#[macro_use]
extern crate clap;
extern crate toml;
extern crate semver;

#[macro_use]
mod macros;
mod config;
mod lockfile;
mod deps;

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process;

use clap::{App, SubCommand};
use toml::Table;

use config::Config;
use lockfile::Lockfile;

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
        process::exit(1);
    }
}

//
// FIXME: Remove unwrap()'s
//
fn execute(cfg: Config) -> CliResult<()> {
    debugln!("executing; execute; cfg={:?}", cfg);

    let all_deps = try!(parse_lockfile(try!(find_root_lockfile_for_cwd())));

    let temp_manifest = cfg.tmp_dir.join("Cargo.toml");
    let temp_lockfile = cfg.tmp_dir.join("Cargo.lock");

    let mut lf = match fs::copy(try!(find_root_lockfile_for_cwd()), &temp_lockfile) {
        Ok(f) => f,
        Err(e) => {
            debugln!("temp Cargo.lock failed with error: {}", e);
            return Err(e.description().to_owned())
        }
    };


    let mut mf = match File::create(&temp_manifest) {
        Ok(f) => f,
        Err(e) => {
            debugln!("temp Cargo.toml failed with error: {}", e);
            return Err(e.description().to_owned())
        }
    };

    debugln!("temp Cargo.toml created");
    write!(mf, "[package]\n\
                  name = \"temp\"\n\
                  version = \"1.0.0\"\n\
                  [[bin]]\n\
                  name = \"test\"\n\
                  [dependencies]\n").unwrap();

    for dep in all_deps.deps.values() {
        write!(mf, "{} = \"~{}\"\n", dep.name, dep.ver).unwrap();
    }

    let cwd = env::current_dir().unwrap();
    env::set_current_dir(&cfg.tmp_dir).unwrap();
    process::Command::new("cargo")
                    .arg("update")
                    .output()
                    .unwrap();

    let val_it = try!(parse_lockfile(&temp_lockfile));
    if let Some(new_deps) = all_deps.get_semver_diff(val_it.deps.values()) {
        for d in new_deps.iter() {
            println!("        Name\tCurr\tNew");
            println!("Update: {}\t{}\t{}", d.name, &*all_deps.deps.get(&d.name).unwrap().ver, d.possible_ver.clone().unwrap());
        }
    }

    env::set_current_dir(&cwd).unwrap();

    Ok(())
}


fn parse_lockfile<P: AsRef<Path>>(p: P) -> CliResult<Lockfile> {
    debugln!("executing; parse_lockfile");
    let mut f = match File::open(p.as_ref()) {
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
    let mut lockfile = Lockfile::new();

    try!(lockfile.get_root_deps(&table));

    try!(lockfile.get_non_root_deps(&table));

    Ok(lockfile)
}

