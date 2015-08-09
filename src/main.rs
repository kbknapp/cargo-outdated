#[macro_use]
extern crate clap;
extern crate toml;
extern crate semver;
extern crate tempdir;
#[cfg(feature = "color")]
extern crate ansi_term;
extern crate tabwriter;

#[macro_use]
mod macros;
mod config;
mod lockfile;
mod deps;
mod error;
mod fmt;

use std::env;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Write, stdout};
use std::path::{Path, PathBuf};
use std::process;

use clap::{App, SubCommand};
use tempdir::TempDir;
use tabwriter::TabWriter;

use config::Config;
use lockfile::Lockfile;
use error::CliError;
use fmt::Format;

pub type CliResult<T> = Result<T, CliError>;

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
                              -v, --verbose             'Print verbose output'
                              -d, --depth [DEPTH]       'How deep in the dependency chain to search{n}\
                                                         (Defaults to 1, or root deps only)'"))
        .get_matches();

    if let Some(m) = m.subcommand_matches("outdated") {
        let cfg = Config::from_matches(m);
        if let Err(e) = execute(cfg) {
            e.exit();
        }
    }
}

//
// FIXME: Remove unwrap()'s
//
fn execute(cfg: Config) -> CliResult<()> {
    debugln!("executing; execute; cfg={:?}", cfg);

    verbose!(cfg, "Parsing {}...", Format::Warning("Cargo.lock"));
    let all_deps = try!(Lockfile::from_file(try!(find_root_lockfile_for_cwd())));
    verboseln!(cfg, "{}", Format::Good("Done"));

    let tmp = match TempDir::new("cargo-outdated") {
        Ok(t)  => t,
        Err(e) => return Err(CliError::Generic(e.description().to_owned())),
    };

    verbose!(cfg, "Setting up temp space...");
    let temp_manifest = tmp.path().join("Cargo.toml");
    let temp_lockfile = tmp.path().join("Cargo.lock");

    let lf = match fs::copy(try!(find_root_lockfile_for_cwd()), &temp_lockfile) {
        Ok(f) => f,
        Err(e) => {
            debugln!("temp Cargo.lock failed with error: {}", e);
            return Err(CliError::Generic(e.description().to_owned()))
        }
    };

    let mut mf = match File::create(&temp_manifest) {
        Ok(f) => f,
        Err(e) => {
            debugln!("temp Cargo.toml failed with error: {}", e);
            return Err(CliError::Generic(e.description().to_owned()))
        }
    };

    debugln!("temp Cargo.toml created");
    try!(all_deps.write_semver_manifest(&mut mf));
    verboseln!(cfg, "{}", Format::Good("Done"));

    verbose!(cfg, "Checking for updates...");
    let cwd = env::current_dir().unwrap();
    env::set_current_dir(tmp.path()).unwrap();
    process::Command::new("cargo")
                    .arg("update")
                    .output()
                    .unwrap();
    verboseln!(cfg, "{}", Format::Good("Done"));

    verbose!(cfg, "Parsing the results...");
    let val_it = try!(Lockfile::from_file(&temp_lockfile));
    verboseln!(cfg, "{}", Format::Good("Done"));

    verboseln!(cfg, "Displaying the results:\n");
    if let Some(new_deps) = all_deps.get_semver_diff(val_it.deps.values()) {
        let mut tw = TabWriter::new(vec![]);
        write!(&mut tw, "\tName\tCurr\tNew\n").unwrap();
        for d in new_deps.iter() {
            write!(&mut tw, "\t{}\t{}\t{}\n", d.name, &*all_deps.deps.get(&d.name).unwrap().ver, d.possible_ver.clone().unwrap()).unwrap();
        }
        tw.flush().unwrap();
        write!(stdout(), "{}", String::from_utf8(tw.unwrap()).unwrap()).unwrap();
    }

    env::set_current_dir(&cwd).unwrap();

    Ok(())
}

fn find_root_lockfile_for_cwd() -> CliResult<PathBuf> {
    debugln!("executing; find_root_lockfile_for_cwd;");
    let cwd = match env::current_dir() {
        Ok(dir) => dir,
        Err(e)  => return Err(CliError::Generic(format!("Couldn't determine the current working directory with error: {}", e.description())))
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

    Err(CliError::Generic(format!("Could not find `{}` in `{}` or any parent directory",
                      file, pwd.display())))
}


