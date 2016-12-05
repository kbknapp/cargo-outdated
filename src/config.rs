use std::path::PathBuf;

use clap::ArgMatches;

use fmt::Format;
use util;
use error::CliResult;

#[derive(Debug)]
pub struct Config<'tu> {
    pub to_update: Option<Vec<&'tu str>>,
    pub depth: u32,
    pub verbose: bool,
    pub exit_code: i32,
    pub manifest: PathBuf,
    pub lockfile: PathBuf,
}

impl<'tu> Config<'tu> {
    pub fn from_matches(m: &'tu ArgMatches) -> CliResult<Self> {
        debugln!("Config:from_matches");
        let depth = match m.value_of("depth") {
            Some(d_str) => {
                match d_str.parse::<u32>() {
                    Ok(num) => num,
                    Err(..) => {
                        wlnerr!("{} Couldn't parse '{}' as a valid depth (Valid depths are 0 (infinite) to ~4,000,000,000)",
                                Format::Error("error:"),
                                d_str);
                        ::std::process::exit(1);
                    }
                }
            }
            None => if m.is_present("root-deps-only") { 1 } else { 0 },
        };

        let cfg = Config {
            to_update: m.values_of("package").map(|v| v.collect()),
            depth: depth,
            verbose: m.is_present("verbose"),
            exit_code: {
                debugln!("Config:from_matches:exit-code={:?}", m.value_of("exit-code"));
                value_t!(m, "exit-code", i32).unwrap_or(0)
            },
            manifest: try!(util::find_file(m.value_of("manifest-path").unwrap_or("Cargo.toml"), m.is_present("manifest-path"))),
            lockfile: try!(util::find_file(m.value_of("lockfile-path").unwrap_or("Cargo.lock"), m.is_present("lockfile-path"))),
        };
        debugln!("Config:from_matches:cfg={:#?}", cfg);
        Ok(cfg)
    }
}
