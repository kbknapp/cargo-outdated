use std::collections::{BTreeMap, HashMap};
use std::io::{self, Read, Write};
use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process;

use toml::{self, Table, Value};
use tempdir::TempDir;

use deps::RawDep;
use deps::Dep;
use error::CliError;
use config::Config;
use fmt::Format;

use CliResult;

pub struct Lockfile {
    pub deps: HashMap<String, RawDep>,
    toml: Box<Table>,
    proj_lockfile_path: PathBuf,
}

impl Lockfile {
    pub fn new() -> CliResult<Self> {
        Lockfile::from_file(try!(Lockfile::find_root_lockfile_for_cwd()))
    }

    pub fn from_file<P: AsRef<Path>>(p: P) -> CliResult<Self> {
        debugln!("executing; from_file; file={:?}", p.as_ref());
        let mut f = match File::open(p.as_ref()) {
            Ok(f) => f,
            Err(e) => return Err(CliError::FileOpen(e.description().to_owned())),
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return Err(CliError::Generic(format!(
                        "Couldn't read the contents of Cargo.lock with error: {}",
                        e.description()
                    )));
        }

        let mut parser = toml::Parser::new(&s);
        if let Some(toml) = parser.parse() {
            return Ok(Lockfile {
                deps: HashMap::new(),
                toml: Box::new(toml),
                proj_lockfile_path: p.as_ref().to_path_buf(),
            });
        }

        // On err
        let mut error_str = String::from("could not parse input as TOML\n");
        for error in parser.errors.iter() {
            let (loline, locol) = parser.to_linecol(error.lo);
            let (hiline, hicol) = parser.to_linecol(error.hi);
            error_str.push_str(&format!("{:?}:{}:{}{} {}\n",
                                        f,
                                        loline + 1,
                                        locol + 1,
                                        if loline != hiline || locol != hicol {
                                            format!("-{}:{}", hiline + 1, hicol + 1)
                                        } else {
                                            "".to_owned()
                                        },
                                        error.desc));
        }
        Err(CliError::Generic(error_str))
    }

    fn find_root_lockfile_for_cwd() -> CliResult<PathBuf> {
        debugln!("executing; find_root_lockfile_for_cwd;");
        let cwd = match env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(CliError::Generic(format!(
                        "Couldn't determine the current working directory with error:\n\t{}",
                        e.description()))),
        };

        Lockfile::find_project_lockfile(&cwd, "Cargo.lock")
    }

    fn find_project_lockfile(pwd: &Path, file: &str) -> CliResult<PathBuf> {
        debugln!("executing; find_project_lockfile; pwd={:?}; file={}",
                 pwd,
                 file);
        let mut current = pwd;

        loop {
            let manifest = current.join(file);
            if fs::metadata(&manifest).is_ok() {
                return Ok(manifest);
            }

            match current.parent() {
                Some(p) => current = p,
                None => break,
            }
        }

        Err(CliError::Generic(format!("Could not find `{}` in `{}` or any parent directory",
                                      file,
                                      pwd.display())))
    }

    #[cfg_attr(feature = "lints", allow(cyclomatic_complexity))]
    pub fn get_updates(&mut self, cfg: &Config) -> CliResult<Option<BTreeMap<String, Dep>>> {
        try!(self.parse_deps_to_depth(cfg.depth));

        // try!(self.get_non_root_deps(self.toml));
        let tmp = match TempDir::new("cargo-outdated") {
            Ok(t) => t,
            Err(e) => return Err(CliError::Generic(e.description().to_owned())),
        };

        verbose!(cfg, "Setting up temp space...");
        let tmp_manifest = tmp.path().join("Cargo.toml");
        let tmp_lockfile = tmp.path().join("Cargo.lock");

        let mut mf = match File::create(&tmp_manifest) {
            Ok(f) => f,
            Err(e) => {
                debugln!("temp Cargo.toml failed with error: {}", e);
                return Err(CliError::Generic(e.description().to_owned()));
            }
        };

        debugln!("temp Cargo.toml created");
        try!(self.write_semver_manifest(&mut mf));
        verboseln!(cfg, "{}", Format::Good("Done"));

        debugln!("\n{}\n", {
            let mut f = File::open(&tmp_manifest)
                            .unwrap_or_else(|e| panic!("cannot open file: {}", e));

            let mut s = String::new();
            f.read_to_string(&mut s).ok();
            s
        });

        match fs::copy(&self.proj_lockfile_path, &tmp_lockfile) {
            Ok(..) => (),
            Err(e) => {
                debugln!("temp Cargo.lock failed with error: {}", e);
                return Err(CliError::Generic(e.description().to_owned()));
            }
        }

        debugln!("\n{}\n", {
            let mut f = File::open(&tmp_lockfile)
                            .unwrap_or_else(|e| panic!("cannot open file: {}", e));

            let mut s = String::new();
            f.read_to_string(&mut s).ok();
            s
        });

        let cwd = env::current_dir()
                      .unwrap_or_else(|e| panic!("current working dir opening error: {}", e));
        debugln!("executing; cargo update");
        env::set_current_dir(tmp.path())
            .unwrap_or_else(|e| panic!("cannot set current dir: {}", e));
        print!("Checking for SemVer compatible updates...");
        let mut out = io::stdout();
        out.flush().unwrap_or_else(|e| panic!("failed to flush stdout: {}", e));
        if let Err(e) =
               process::Command::new("cargo")
                   .arg("update")
                   .arg("--manifest-path")
                   .arg(tmp_manifest.to_str()
                                    .expect("failed to convert temp Cargo.toml path to string"))
                   .output()
                   .and_then(|v| {
                       if v.status.success() {
                           Ok(v)
                       } else {
                           Err(io::Error::new(io::ErrorKind::Other, "did not exit successfully"))
                       }
                   }) {

            return Err(CliError::Generic(format!("Failed to run 'cargo update' with error '{}'",
                                                 e.description())));
        }
        println!("{}", Format::Good("Done"));

        verbose!(cfg, "Parsing the results...");
        debugln!("creating new lockfile from tmp results");
        let mut updated_lf = try!(Lockfile::from_file(&tmp_lockfile));
        try!(updated_lf.parse_deps_to_depth(0));
        let mut res = BTreeMap::new();
        debugln!("parsing semver results");
        for (d_name, d) in self.deps.iter() {
            debugln!("iter; name={}; ver={}", d_name, d.ver);
            if let Some(semver_dep) = updated_lf.deps.get(&d.name) {
                if semver_dep.ver != d.ver {
                    res.insert(d_name.to_owned(),
                               Dep {
                                   name: d_name.to_owned(),
                                   project_ver: d.ver.clone(),
                                   semver_ver: Some(semver_dep.ver.clone()),
                                   latest_ver: None,
                               });
                }
            }
        }
        verboseln!(cfg, "{}", Format::Good("Done"));

        verbose!(cfg, "Creating temp space for latest versions...");
        let mut mf = match File::create(&tmp_manifest) {
            Ok(f) => f,
            Err(e) => {
                debugln!("temp Cargo.toml failed with error: {}", e);
                return Err(CliError::Generic(e.description().to_owned()));
            }
        };

        try!(self.write_latest_manifest(&mut mf));

        match fs::copy(&self.proj_lockfile_path, &tmp_lockfile) {
            Ok(..) => (),
            Err(e) => {
                debugln!("temp Cargo.lock failed with error: {}", e);
                return Err(CliError::Generic(e.description().to_owned()));
            }
        }
        verboseln!(cfg, "{}", Format::Good("Done"));

        print!("Checking for the latest updates...");
        out.flush().expect("failed to flush stdout");
        if let Err(e) =
               process::Command::new("cargo")
                   .arg("update")
                   .arg("--manifest-path")
                   .arg(tmp_manifest.to_str()
                                    .expect("failed to convert temp Cargo.toml path to string"))
                   .output()
                   .and_then(|v| {
                       if v.status.success() {
                           Ok(v)
                       } else {
                           Err(io::Error::new(io::ErrorKind::Other, "did not exit successfully"))
                       }
                   }) {

            return Err(CliError::Generic(format!("Failed to run 'cargo update' with error '{}'",
                                                 e.description())));
        }
        println!("{}", Format::Good("Done"));

        verbose!(cfg, "Parsing the results...");
        let mut updated_lf = try!(Lockfile::from_file(&tmp_lockfile));
        try!(updated_lf.parse_deps_to_depth(0));
        for (d_name, d) in self.deps.iter() {
            debugln!("iter; name={}", d_name);
            if let Some(latest_dep) = updated_lf.deps.get(&d.name) {
                if latest_dep.ver != d.ver {
                    let exists = if let Some(d) = res.get_mut(d_name) {
                        d.latest_ver = Some(latest_dep.ver.clone());
                        true
                    } else {
                        false
                    };

                    if !exists {
                        res.insert(d_name.to_owned(),
                                   Dep {
                                       name: d_name.to_owned(),
                                       project_ver: d.ver.clone(),
                                       semver_ver: None,
                                       latest_ver: Some(latest_dep.ver.clone()),
                                   });
                    }
                }
            }
        }
        verboseln!(cfg, "{}", Format::Good("Done"));

        env::set_current_dir(&cwd).unwrap_or_else(|e| panic!("cannot set current dir: {}", e));

        if res.is_empty() {
            debugln!("returning; res=Ok(None)");
            Ok(None)
        } else {
            if let Some(ref dep_v) = cfg.to_update {
                let mut safe = vec![];
                for dep in dep_v {
                    if res.contains_key(dep.to_owned()) {
                        safe.push(dep);
                    }
                }
                let mut ret = BTreeMap::new();
                for dep in safe.into_iter() {
                    ret.insert((*dep).to_owned(),
                               res.remove(&**dep)
                                  .expect("failed to get dependency from results set"));
                }
                return Ok(Some(ret));
            }
            debugln!("returning; res={:#?}", res);
            Ok(Some(res))
        }
    }


    fn parse_deps_to_depth(&mut self, mut depth: i32) -> CliResult<()> {
        debugln!("executing; parse_deps_to_depth; depth={}", depth);
        let mut all_deps = depth == 0;

        try!(self.parse_root_deps());

        while depth > 1 || all_deps {
            debugln!("iter; depth={};", depth);
            match self.toml.get("package") {
                Some(&Value::Array(ref tables)) => {
                    for table in tables {
                        let name = table.lookup("name")
                                        .expect("no 'name' field in Cargo.lock [package] table")
                                        .as_str()
                                        .expect("'name' field of [package] table in Cargo.lock was \
                                             not a valid string");
                        if !self.deps.contains_key(name) {
                            continue;
                        }
                        let ver = table.lookup("version")
                                       .expect("no 'version' field in Cargo.lock [package] table")
                                       .as_str()
                                       .expect("'version' field of [package] table in Cargo.lock \
                                                was not a valid string");
                        let mut next_level = vec![];
                        if let Some(existing_dep) = self.deps.get_mut(name) {
                            if existing_dep.ver != ver {
                                // probably a child of another dep...skip!
                                continue;
                            }
                            match table.lookup("dependencies") {
                                Some(&Value::Array(ref deps)) => {
                                    let mut children = vec![];
                                    for d in deps {
                                        let dep_str = d.as_str().unwrap_or("");
                                        let mut child: RawDep = match dep_str.parse() {
                                            Ok(val) => val,
                                            Err(e) => return Err(CliError::Generic(e)),
                                        };
                                        if !child.source.starts_with("(registry+") {
                                            continue;
                                        }
                                        child.parent = Some(name.to_owned());
                                        children.push(child.name.clone());
                                        if all_deps || depth > 1 {
                                            debugln!("adding sub dep {}->{}", name, child.name);
                                            next_level.push(child);
                                        }
                                    }
                                    existing_dep.children = Some(children);
                                }
                                Some(..) => unreachable!(),
                                None => (),
                            }
                        }
                        for child in next_level.into_iter() {
                            self.deps
                                .insert(format!("{}->{}",
                                                child.parent
                                                     .as_ref()
                                                     .expect("child dependency has no parent node")
                                                     .clone(),
                                                child.name.clone()),
                                        child);
                        }
                    }
                    depth -= 1;
                    if depth == 1 || depth < 0 {
                        all_deps = false;
                    }
                }
                Some(_) => unreachable!(),
                None => return Err(CliError::NoNonRootDeps),
            }
        }

        debugln!("All deps: {:#?}", self.deps);
        Ok(())
    }

    fn parse_root_deps(&mut self) -> CliResult<()> {
        debugln!("executing; parse_root_deps;");
        let root_table = match self.toml.get("root") {
            Some(table) => table,
            None => return Err(CliError::TomlTableRoot),
        };
        match root_table.lookup("dependencies") {
            Some(&Value::Array(ref val)) => {
                debugln!("found root deps table");

                for v in val {
                    let val_str = v.as_str().unwrap_or("");
                    debugln!("adding root dep {}", val_str);
                    let raw_dep: RawDep = match val_str.parse() {
                        Ok(val) => val,
                        Err(e) => return Err(CliError::Generic(e)),
                    };
                    if raw_dep.source.starts_with("(registry+") {
                        self.deps.insert(raw_dep.name.clone(), raw_dep);
                    }
                }
            }
            Some(_) => unreachable!(),
            None => return Err(CliError::NoRootDeps),
        }

        debugln!("Root deps: {:#?}", self.deps);
        Ok(())
    }

    fn write_manifest_pretext<W>(&self, w: &mut W) -> CliResult<()>
        where W: Write
    {
        write!(w,
               "[package]\n\
                      name = \"temp\"\n\
                      version = \"1.0.0\"\n\
                      [[bin]]\n\
                      name = \"test\"\n\
                      [dependencies]\n")
            .unwrap();

        Ok(())
    }

    fn unique_deps(&self) -> HashMap<String, RawDep> {
        self.deps.iter().map(|(_, ref dep)| (dep.name.clone(), (**dep).clone())).collect()
    }

    pub fn write_semver_manifest<W>(&self, w: &mut W) -> CliResult<()>
        where W: Write
    {
        debugln!("executing; write_semver_manifest;");
        try!(self.write_manifest_pretext(w));

        for dep in self.unique_deps().values() {
            debugln!("iter; name={}; ver=~{}", dep.name, dep.ver);
            if let Err(e) = write!(w, "{} = \"~{}\"\n", dep.name, dep.ver) {
                return Err(CliError::Generic(format!("Failed to write Cargo.toml with error '{}'",
                                                     e.description())));
            }
        }

        Ok(())
    }
    pub fn write_latest_manifest<W>(&self, w: &mut W) -> CliResult<()>
        where W: Write
    {
        debugln!("executing; write_latest_manifest;");
        try!(self.write_manifest_pretext(w));

        for dep in self.unique_deps().values() {
            debugln!("iter; name={}; ver=*", dep.name);
            if let Err(e) = write!(w, "{} = \"*\"\n", dep.name) {
                return Err(CliError::Generic(format!("Failed to write Cargo.toml with error '{}'",
                                                     e.description())));
            }
        }

        Ok(())
    }
}
