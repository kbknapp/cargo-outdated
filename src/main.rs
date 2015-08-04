#[macro_use]
extern crate clap;
extern crate toml;
extern crate semver;

use std::env;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::str::FromStr;

use clap::{App, ArgMatches, SubCommand};
use toml::{Table, Value};

#[macro_use]
mod macros;

#[derive(Debug)]
struct Config<'tu> {
    to_update: Option<Vec<&'tu str>>,
    depth: u8
}

impl<'tu> Config<'tu> {
    fn from_matches(m: &'tu ArgMatches) -> Self {
        Config {
            to_update: m.values_of("PKG"),
            depth: m.value_of("DEPTH").unwrap_or("1").parse().unwrap_or(1)
        }
    }
}

struct Dep {
    name: String,
    raw_ver: Option<String>,
    current_ver: Option<semver::Version>,
    possible_ver: Option<semver::Version>,
    latest_ver: Option<semver::Version>,
    sub_deps: Vec<String>
}

#[derive(Debug)]
struct RawDep {
    name: String,
    ver: String,
    is_root: bool,
    depth: u8
}

impl FromStr for RawDep {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
       let raw_dep_vec: Vec<_> = s.split(" ").collect();
       if raw_dep_vec.len() < 2 { 
           return Err(format!("failed to parse dependency string '{}'", s)) 
       }
       Ok(RawDep {
           name: raw_dep_vec[0].to_owned(),
           ver: raw_dep_vec[1].to_owned(),
           is_root: false,
           depth: 1
       })
    }
}

struct Lockfile {
    deps: Vec<RawDep>
}

impl Lockfile {
    fn get_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let root_table = match table.get("root") {
            Some(table) => table,
            None        => {
                return Err(String::from("couldn't find '[root]' table in Cargo.lock"));
            }
        };

        match root_table.lookup("dependencies") {
            Some(&Value::Array(ref val)) => {
                debugln!("found root deps table");

                for v in val {
                    let val_str = v.as_str().unwrap_or("");
                    debugln!("adding root dep {}", val_str);
                    let mut raw_dep: RawDep = try!(val_str.parse());
                    raw_dep.is_root = true;
                    self.deps.push(raw_dep);
                }
            },
            Some(_) => unreachable!(),
            None => return Err(String::from("No root dependencies"))
        };

        debugln!("Root deps: {:?}", self.deps);
        Ok(())
    }

    fn get_non_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let arr = match table.get("package") {
            Some(&Value::Array(ref val)) => {
                debugln!("found non root deps");

                for v in val {
                    let name_str = v.lookup("name").unwrap().as_str().unwrap_or("");
                    let ver_str = v.lookup("version").unwrap().as_str().unwrap_or("");
                    match v.lookup("dependencies") {
                        Some(&Value::Array(ref deps)) => {
                            for d in deps {
                                let dep_str = d.as_str().unwrap_or("");
                                debugln!("adding non root dep {}", dep_str);
                                self.deps.push(try!(dep_str.parse()));
                            }
                        },
                        Some(..) => unreachable!(),
                        None => ()
                    }
                }
            },
            Some(_) => unreachable!(),
            None => return Err(String::from("No non root dependencies"))
        };

        debugln!("All deps: {:#?}", self.deps);
        Ok(())
    }
}

type CliResult<T> = Result<T, String>;

fn main() {
    debugln!("executing; cmd=cargo-outdated; args={:?}", env::args().collect::<Vec<_>>());
    let m = App::new("cargo-outdated")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Displays information about project dependency versions")
        .version(&*format!("v{}", crate_version!()))
        // We have to lie about our binary name since this will be a third party
        // subcommand for cargo
        .bin_name("cargo")
        .global_version(true)
        // We use a subcommand because parsed after `cargo` is sent to the third party plugin
        // which will be interpreted as a subcommand/positional arg by clap
        .subcommand(SubCommand::with_name("outdated")
            .about("Displays information about project dependency versions")
            .args_from_usage("-p, --package [PKG]...    'Package to inspect for updates'
                              -d, --depth [DEPTH]       'How deep in the dependency chain to search{n}\
                                                         (Defaults to 1, or root deps only)'"))
        .get_matches();

    let cfg = Config::from_matches(&m);

    if let Err(e) = execute(cfg) {
        wlnerr!("cargo-outdated: {}", e);
        exit(1);
    }
}

fn execute(cfg: Config) -> CliResult<()> {
    debugln!("executing; execute; cfg={:?}", cfg);

    let all_deps = try!(parse_lockfile());

    Ok(())
}


fn parse_lockfile() -> CliResult<Lockfile> {
    debugln!("executing; parse_lockfile");
    let lock_path = try!(find_root_lockfile_for_cwd());

    let mut f = match File::open(lock_path) {
        Ok(f) => f,
        Err(_) => return Err(String::from("Couldn't open Cargo.lock for reading"))
    };

    let mut s = String::new();
    if let Err(..) = f.read_to_string(&mut s) {
        return Err(String::from("Couldn't read the contents of Cargo.lock"))
    }

    let mut parser = toml::Parser::new(&s);
    match parser.parse() {
        Some(toml) => return parse_table(toml),
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
    Err(error_str)
}

fn find_root_lockfile_for_cwd() -> CliResult<PathBuf> {
    debugln!("executing; find_root_lockfile_for_cwd;");
    let cwd = match env::current_dir() {
        Ok(dir) => dir,
        Err(..) => return Err(String::from("Couldn't determine the current working directory"))
    };

    find_project_lockfile(&cwd, "Cargo.lock")
}

fn find_project_lockfile(pwd: &Path, file: &str) -> CliResult<PathBuf> {
    debugln!("executing; find_project_lockfile; pwd={:?}; file={}", pwd, file);
    let mut current = pwd;

    loop {
        let manifest = current.join(file);
        if fs::metadata(&manifest).is_ok() {
            return Ok(manifest)
        }

        match current.parent() {
            Some(p) => current = p,
            None => break,
        }
    }

    Err(format!("Could not find `{}` in `{}` or any parent directory",
                      file, pwd.display()))
}

fn parse_table(table: Table) -> CliResult<Lockfile> {
    debugln!("executing; parse_table");
    let mut lockfile = Lockfile { deps: vec![] };

    try!(lockfile.get_root_deps(&table));

    try!(lockfile.get_non_root_deps(&table));

    Ok(lockfile)
}

