use std::env;
use std::fs;
use std::path::{PathBuf, Path};

use clap::ArgMatches;

#[derive(Debug)]
pub struct Config<'tu> {
    to_update: Option<Vec<&'tu str>>,
    depth: u8,
    pub tmp_dir: PathBuf
}

impl<'tu> Config<'tu> {
    pub fn from_matches(m: &'tu ArgMatches) -> Self {
        let temp_dir = env::temp_dir().join("cargo-outdated");
        fs::create_dir(&temp_dir).unwrap();
        Config {
            to_update: m.values_of("PKG"),
            depth: m.value_of("DEPTH").unwrap_or("1").parse().unwrap_or(1),
            tmp_dir: temp_dir
        }
    }
}