use std::io::Read;
use std::fs::File;
use std::path::Path;
use error::CliResult;

#[derive(Debug, Deserialize, Clone)]
pub struct RawPackage {
    pub name: String,
    pub version: String,
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Lockfile {
    pub root: RawPackage,
    pub package: Option<Vec<RawPackage>>,
}

impl Lockfile {
    pub fn from_lockfile_path<P: AsRef<Path>>(path: P) -> CliResult<Lockfile> {
        let mut lockfile = try!(File::open(path.as_ref()));
        let mut lockfile_contents = String::new();
        let _ = try!(lockfile.read_to_string(&mut lockfile_contents));
        Ok(::toml::from_str(&lockfile_contents).expect(&format!(
            "Cannot parse lockfile {}",
            path.as_ref().to_str().unwrap()
        )))
    }
}
