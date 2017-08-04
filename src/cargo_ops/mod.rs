use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process;
use std::error::Error;

use tempdir::TempDir;
use toml::Value;
use toml::value::Table;

use error::{CliError, CliResult};
use cargo_files::Manifest;

#[derive(Debug)]
pub struct TempProject {
    pub manifest: PathBuf,
    pub lockfile: PathBuf,
    parsed_manifest: Manifest,
    temp_dir: TempDir,
}

impl TempProject {
    pub fn new<P: AsRef<Path>>(orig_manifest: P, orig_lockfile: P) -> CliResult<TempProject> {
        let temp_dir = try!(TempDir::new("cargo-outdated"));
        let manifest = temp_dir.path().join("Cargo.toml");
        let lockfile = temp_dir.path().join("Cargo.lock");

        let mut buf = String::new();
        let mut orig_manifest_file = try!(File::open(&orig_manifest));
        try!(orig_manifest_file.read_to_string(&mut buf));
        let parsed_manifest: Manifest = ::toml::from_str(&buf).expect("Cannot parse Cargo.toml");
        try!(fs::copy(&orig_lockfile, &lockfile));

        Ok(TempProject {
            manifest: manifest,
            lockfile: lockfile,
            parsed_manifest: parsed_manifest,
            temp_dir: temp_dir,
        })
    }

    pub fn cargo_update(&self) -> CliResult<()> {
        if let Err(e) = process::Command::new("cargo")
            .arg("update")
            .arg("--manifest-path")
            .arg(
                self.manifest
                    .to_str()
                    .expect("failed to convert temp Cargo.toml path to string"),
            )
            .output()
            .and_then(|v| if v.status.success() {
                Ok(v)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "did not exit successfully",
                ))
            }) {
            return Err(CliError::Generic(format!(
                "Failed to run 'cargo update' with error '{}'",
                e.description()
            )));
        }
        Ok(())
    }

    fn write_manifest(&self, contents: &Manifest) -> CliResult<()> {
        let mut file = try!(File::create(&self.manifest));
        let serialized = ::toml::to_string(contents).expect("Failed to serialized Cargo.toml");
        try!(write!(file, "{}", serialized));
        Ok(())
    }

    pub fn write_manifest_semver(&self) -> CliResult<()> {
        let name = self.parsed_manifest
            .package
            .get("name")
            .expect("Cannot find package name in Cargo.toml");
        let version = self.parsed_manifest
            .package
            .get("version")
            .expect("Cannot find package version in Cargo.toml");
        let mut package = Table::new();
        package.insert("name".to_owned(), name.clone());
        package.insert("version".to_owned(), version.clone());
        let mut bin = Table::new();
        bin.insert("name".to_owned(), Value::String("test".to_owned()));
        bin.insert("path".to_owned(), Value::String("test.rs".to_owned()));

        let manifest_semver = Manifest {
            package: package,
            dependencies: self.parsed_manifest.dependencies.clone(),
            bin: Some(vec![bin]),
        };
        try!(self.write_manifest(&manifest_semver));

        Ok(())
    }

    pub fn write_manifest_latest(&self) -> CliResult<()> {
        let name = self.parsed_manifest
            .package
            .get("name")
            .expect("Cannot find package name in Cargo.toml");
        let version = self.parsed_manifest
            .package
            .get("version")
            .expect("Cannot find package version in Cargo.toml");
        let mut package = Table::new();
        package.insert("name".to_owned(), name.clone());
        package.insert("version".to_owned(), version.clone());
        let mut dependencies = Table::new();
        for (dep_name, dep_pac) in &self.parsed_manifest.dependencies {
            match *dep_pac {
                Value::Table(ref t) => {
                    let mut t = t.clone();
                    t.insert("version".to_owned(), Value::String("*".to_owned()));
                    let _ = dependencies.insert(dep_name.clone(), Value::Table(t));
                }
                Value::String(_) => {
                    let _ = dependencies.insert(dep_name.clone(), Value::String("*".to_owned()));
                }
                _ => unreachable!(),
            }
        }
        let mut bin = Table::new();
        bin.insert("name".to_owned(), Value::String("test".to_owned()));
        bin.insert("path".to_owned(), Value::String("test.rs".to_owned()));

        let manifest_semver = Manifest {
            package: package,
            dependencies: dependencies,
            bin: Some(vec![bin]),
        };
        try!(self.write_manifest(&manifest_semver));

        Ok(())
    }
}
