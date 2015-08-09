use std::collections::HashMap;
use std::io::{Read, Write};
use std::error::Error;
use std::fs::File;
use std::path::Path;

use toml::{self, Value, Table};

use deps::RawDep;
use deps::Dep;
use error::CliError;

use CliResult;

pub struct Lockfile {
    pub deps: HashMap<String, RawDep>
}

impl Lockfile {

    pub fn new() -> Self {
        Lockfile { deps: HashMap::new() }
    }

    pub fn from_file<P: AsRef<Path>>(p: P) -> CliResult<Self> {
        debugln!("executing; parse_lockfile");
        let mut f = match File::open(p.as_ref()) {
            Ok(f) => f,
            Err(e) => return Err(CliError::FileOpen(e.description().to_owned()))
        };

        let mut s = String::new();
        if let Err(e) = f.read_to_string(&mut s) {
            return Err(CliError::Generic(format!("Couldn't read the contents of Cargo.lock with error: {}", e.description())))
        }

        let mut parser = toml::Parser::new(&s);
        match parser.parse() {
            Some(toml) => return Lockfile::parse_table(toml),
            None => {}
        }


        // On err
        let mut error_str = format!("could not parse input as TOML\n");
        for error in parser.errors.iter() {
            let (loline, locol) = parser.to_linecol(error.lo);
            let (hiline, hicol) = parser.to_linecol(error.hi);
            error_str.push_str(&format!("{:?}:{}:{}{} {}\n",
                                        f,
                                        loline + 1, locol + 1,
                                        if loline != hiline || locol != hicol {
                                            format!("-{}:{}", hiline + 1,
                                                    hicol + 1)
                                        } else {
                                            "".to_string()
                                        },
                                        error.desc));
        }
        Err(CliError::Generic(error_str))
    }

    fn parse_table(table: Table) -> CliResult<Self> {
        debugln!("executing; parse_table");
        let mut lockfile = Lockfile::new();

        try!(lockfile.get_root_deps(&table));

        try!(lockfile.get_non_root_deps(&table));

        Ok(lockfile)
    }

    pub fn get_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let root_table = match table.get("root") {
            Some(table) => table,
            None        => return Err(CliError::TomlTableRoot)
        };

        match root_table.lookup("dependencies") {
            Some(&Value::Array(ref val)) => {
                debugln!("found root deps table");

                for v in val {
                    let val_str = v.as_str().unwrap_or("");
                    debugln!("adding root dep {}", val_str);
                    let mut raw_dep: RawDep = match val_str.parse() {
                        Ok(val) => val,
                        Err(e)  => return Err(CliError::Generic(e))
                    };
                    raw_dep.is_root = true;
                    self.deps.insert(raw_dep.name.clone(), raw_dep);
                }
            },
            Some(_) => unreachable!(),
            None => return Err(CliError::NoRootDeps)
        };

        debugln!("Root deps: {:?}", self.deps);
        Ok(())
    }

    fn write_manifest_pretext(w: &mut W) -> CliResult<()> where W: Write {
        write!(w, "[package]\n\
                      name = \"temp\"\n\
                      version = \"1.0.0\"\n\
                      [[bin]]\n\
                      name = \"test\"\n\
                      [dependencies]\n").unwrap();
    }

    pub fn write_semver_manifest(w: &mut W) -> CliResult<()> where W: Write {
        try!(Lockfile::write_manifest_pretext(w));

        for dep in all_deps.deps.values() {
            write!(mf, "{} = \"~{}\"\n", dep.name, dep.ver).unwrap();
        }
    }
    pub fn write_allver_manifest(w: &mut W) -> CliResult<()> where W: Write {
        try!(Lockfile::write_manifest_pretext(w));

        for dep in all_deps.deps.values() {
            write!(mf, "{} = \"*\"\n", dep.name).unwrap();
        }
    }

    pub fn get_non_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let arr = match table.get("package") {
            Some(&Value::Array(ref val)) => {
                debugln!("found non root deps");

                for v in val {
                    match v.lookup("dependencies") {
                        Some(&Value::Array(ref deps)) => {
                            for d in deps {
                                let dep_str = d.as_str().unwrap_or("");
                                debugln!("adding non root dep {}", dep_str);
                                let raw_dep: RawDep = match dep_str.parse() {
                                    Ok(val) => val,
                                    Err(e)  => return Err(CliError::Generic(e))
                                };
                                self.deps.insert(raw_dep.name.clone(), raw_dep);
                            }
                        },
                        Some(..) => unreachable!(),
                        None => ()
                    }
                }
            },
            Some(_) => unreachable!(),
            None => return Err(CliError::NoNonRootDeps)
        };

        debugln!("All deps: {:#?}", self.deps);
        Ok(())
    }

    pub fn get_semver_diff<'a, I>(&self, deps: I) -> Option<Vec<Dep>> where I: Iterator<Item=&'a RawDep> {
        let mut res = vec![];
        for dep in deps {
            if let Some(ref old_dep) = self.deps.get(&dep.name) {
                if old_dep.ver != dep.ver { res.push(Dep {
                    name: dep.name.clone(),
                    raw_ver: None,
                    current_ver: Some(old_dep.ver.clone()),
                    possible_ver: Some(dep.ver.clone()),
                    latest_ver: None,
                })}
            }
        }
        if res.is_empty() {return None}
        Some(res)
    }
}