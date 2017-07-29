//! A cargo subcommand for checking the latest version on crates.io of a particular dependency
//!
//! ## About
//!
//! `cargo-outdated` is a very early proof-of-concept for displaying when dependencies have newer versions available.
//!
//! ## Compiling
//!
//! Follow these instructions to compile `cargo-outdated`, then skip down to Installation.
//!
//! 1. Ensure you have current version of `cargo` and [Rust](https://www.rust-lang.org) installed
//! 2. Clone the project `$ git clone https://github.com/kbknapp/cargo-outdated && cd cargo-outdated`
//! 3. Build the project `$ cargo build --release`
//! 4. Once complete, the binary will be located at `target/release/cargo-outdated`
//!
//! ## Installation and Usage
//!
//! All you need to do is place `cargo-outdated` somewhere in your `$PATH`. Then run `cargo outdated` anywhere in your project directory. For full details see below.
//!
//! ### Linux / OS X
//!
//! You have two options, place `cargo-outdated` into a directory that is already located in your `$PATH` variable (To see which directories those are, open a terminal and type `echo "${PATH//:/\n}"`, the quotation marks are important), or you can add a custom directory to your `$PATH`
//!
//! **Option 1**
//! If you have write permission to a directory listed in your `$PATH` or you have root permission (or via `sudo`), simply copy the `cargo-outdated` to that directory `# sudo cp cargo-outdated /usr/local/bin`
//!
//! **Option 2**
//! If you do not have root, `sudo`, or write permission to any directory already in `$PATH` you can create a directory inside your home directory, and add that. Many people use `$HOME/.bin` to keep it hidden (and not clutter your home directory), or `$HOME/bin` if you want it to be always visible. Here is an example to make the directory, add it to `$PATH`, and copy `cargo-outdated` there.
//!
//! Simply change `bin` to whatever you'd like to name the directory, and `.bashrc` to whatever your shell startup file is (usually `.bashrc`, `.bash_profile`, or `.zshrc`)
//!
//! ```ignore
//! $ mkdir ~/bin
//! $ echo "export PATH=$PATH:$HOME/bin" >> ~/.bashrc
//! $ cp cargo-outdated ~/bin
//! $ source ~/.bashrc
//! ```
//!
//! ### Windows
//!
//! On Windows 7/8 you can add directory to the `PATH` variable by opening a command line as an administrator and running
//!
//! ```ignore
//! C:\> setx path "%path%;C:\path\to\cargo-outdated\binary"
//! ```
//!
//! Otherwise, ensure you have the `cargo-outdated` binary in the directory which you operating in the command line from, because Windows automatically adds your current directory to PATH (i.e. if you open a command line to `C:\my_project\` to use `cargo-outdated` ensure `cargo-outdated.exe` is inside that directory as well).
//!
//!
//! ### Options
//!
//! There are a few options for using `cargo-outdated` which should be somewhat self explanitory.
//!
//! ```ignore
//! USAGE:
//!     cargo outdated [FLAGS] [OPTIONS]
//!
//! FLAGS:
//!     -h, --help              Prints help information
//!     -R, --root-deps-only    Only check root dependencies (Equivalent to --depth=1)
//!     -V, --version           Prints version information
//!     -v, --verbose           Print verbose output
//!
//! OPTIONS:
//!     -d, --depth <NUM>             How deep in the dependency chain to search (Defaults to all dependencies when omitted)
//!         --exit-code <NUM>         The exit code to return on new versions found [default: 0]
//!     -l, --lockfile-path <PATH>    An absolute path to the Cargo.lock to use (Defaults to Cargo.lock in project root)
//!     -m, --manifest-path <PATH>    An absolute path to the Cargo.toml to use (Defaults to Cargo.toml in project root)
//!     -p, --package <PKG>...        Package to inspect for updates
//!     -r, --root <ROOT>             Package to treat as the root package
//! ```
//!
//! ## License
//!
//! `cargo-outdated` is released under the terms of the MIT license. See the LICENSE-MIT file for the details.

#[macro_use]
extern crate clap;
extern crate toml;
extern crate tempdir;
#[cfg(feature = "color")]
extern crate ansi_term;
extern crate tabwriter;
extern crate serde;
#[macro_use]
extern crate serde_derive;

#[macro_use]
mod macros;
mod config;
mod error;
mod fmt;
mod util;
mod cargo_files;
mod cargo_ops;

use std::io::{Write, stdout};
use std::path::Path;
#[cfg(feature="debug")]
use std::env;
use std::process;

use clap::{App, AppSettings, Arg, SubCommand, ArgMatches};
use tabwriter::TabWriter;

use config::Config;
use error::{CliResult, CliError};
use fmt::Format;

fn main() {
    debugln!("main:args={:?}", env::args().collect::<Vec<_>>());
    let m = App::new("cargo-outdated")
        .author("Kevin K. <kbknapp@gmail.com>")
        .about("Displays information about project dependency versions")
        .version(concat!("v", crate_version!()))
        // We have to lie about our binary name since this will be a third party
        // subcommand for cargo
        .bin_name("cargo")
        // Global version uses the version we supplied (Cargo.toml) for all subcommands
        // as well
        .settings(&[AppSettings::GlobalVersion,
                    AppSettings::SubcommandRequired])
        // We use a subcommand because parsed after `cargo` is sent to the third party
        // plugin
        // which will be interpreted as a subcommand/positional arg by clap
        .subcommand(SubCommand::with_name("outdated")
            .about("Displays information about project dependency versions")
            .args_from_usage(
                "-p, --package [PKG]...     'Package to inspect for updates'
                 -r, --root [ROOT]         'Package to treat as the root package'
                 -v, --verbose              'Print verbose output'
                 -d, --depth [NUM]          'How deep in the dependency chain to search \
                                            (Defaults to all dependencies when omitted)'")
            .args(&[
                Arg::from_usage("--exit-code [NUM]     'The exit code to return on new versions found'")
                    .default_value("0"),
                Arg::from_usage(
                    "-R, --root-deps-only  'Only check root dependencies (Equivalent to --depth=1)'")
                    .conflicts_with("depth"),
                Arg::from_usage("-m, --manifest-path [PATH] 'An absolute path to the Cargo.toml file to use \
                                                             (Defaults to Cargo.toml in project root)'")
                    .validator(is_file),
                Arg::from_usage("-l, --lockfile-path [PATH] 'An absolute path to the Cargo.lock to use \
                                                             (Defaults to Cargo.lock in project root)'")
                    .validator(is_file)]))
        .get_matches();

    if let Some(m) = m.subcommand_matches("outdated") {
        match execute(m) {
            Ok(code) => {
                debugln!("main:exit_code={}", code);
                process::exit(code)
            }
            Err(e) => e.exit(),
        }
    }
}

fn execute(m: &ArgMatches) -> CliResult<i32> {
    debugln!("execute:m={:#?}", m);
    let cfg = try!(Config::from_matches(m));

    // parse original lockfile
    verbose!(cfg, "Parsing {}...", Format::Warning(cfg.lockfile.to_string_lossy()));
    let dep_tree = {
        let mut parsed_lock = cargo_files::Lockfile::from_lockfile_path(&cfg.lockfile)?;
        if parsed_lock.package.is_none() {
            return Err(CliError::NoRootDeps);
        }
        parsed_lock
            .package
            .as_mut()
            .unwrap()
            .push(parsed_lock.root.clone());
        cargo_files::DependencyTree::from_lockfile(&mut parsed_lock, None, cfg.depth)
    };
    verboseln!(cfg, "{}", Format::Good("Done"));
    // create a temp project in tmp
    let tmp_proj = cargo_ops::TempProject::new(&cfg.manifest, &cfg.lockfile)?;
    // write semver to the tmp Cargo.toml
    tmp_proj.write_manifest_semver()?;
    // update it
    tmp_proj.cargo_update()?;
    // parse lockfile with semver compatible dependencies
    verbose!(cfg, "Parsing semver compatible lockfile {}...", Format::Warning(tmp_proj.lockfile.to_string_lossy()));
    let dep_tree_compat = {
        let mut parsed_lock = cargo_files::Lockfile::from_lockfile_path(&tmp_proj.lockfile)?;
        parsed_lock
            .package
            .as_mut()
            .unwrap()
            .push(parsed_lock.root.clone());
        cargo_files::DependencyTree::from_lockfile(&mut parsed_lock, None, -1)
    };
    verboseln!(cfg, "{}", Format::Good("Done"));
    // merge them to the original tree
    dep_tree.merge_compatible_from(&dep_tree_compat);
    // rewrite the manifest with "*" semver dependencies
    tmp_proj.write_manifest_latest()?;
    // update it
    tmp_proj.cargo_update()?;
    // parse lockfile with latest dependencies
    verbose!(cfg, "Parsing latest lockfile {}...", Format::Warning(tmp_proj.lockfile.to_string_lossy()));
    let dep_tree_latest = {
        let mut parsed_lock = cargo_files::Lockfile::from_lockfile_path(&tmp_proj.lockfile)?;
        parsed_lock
            .package
            .as_mut()
            .unwrap()
            .push(parsed_lock.root.clone());
        cargo_files::DependencyTree::from_lockfile(&mut parsed_lock, None, -1)
    };
    verboseln!(cfg, "{}", Format::Good("Done"));
    // merge them to the original tree
    dep_tree.merge_latest_from(&dep_tree_latest);

    let mut tw = TabWriter::new(vec![]);
    write!(&mut tw, "Name\tProject Ver\tSemVer Compat\tLatest Ver\n")
        .unwrap_or_else(|e| panic!("write! error: {}", e));
    let mut lines = dep_tree.print_to_vec();
    if lines.is_empty() {
        println!("All dependencies are up to date, yay!");
        return Ok(0);
    }
    lines.sort();
    lines.dedup();
    for l in lines {
        write!(&mut tw, "{}", l);
    }
    tw.flush()
        .unwrap_or_else(|e| panic!("failed to flush TabWriter: {}", e));
    write!(
        stdout(),
        "{}",
        String::from_utf8(tw.into_inner().unwrap())
            .unwrap_or_else(|e| panic!("from_utf8 error: {}", e))
    ).unwrap_or_else(|e| panic!("write! error: {}", e));

    Ok(cfg.exit_code)
}

fn is_file(s: String) -> Result<(), String> {
    let p = Path::new(&*s);
    if p.file_name().is_none() {
        return Err(format!("'{}' doesn't appear to be a valid file name", &*s));
    }
    Ok(())
}
