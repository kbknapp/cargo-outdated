use std::env;
use std::path::{Path, PathBuf};
use std::fs;

use error::{CliError, CliResult};

pub fn find_file(file: &str, usr_override: bool) -> CliResult<PathBuf> {
    debugln!("util:find_file;file={:?};usr_override={:?}", file, usr_override);
    if usr_override {
        return Ok(Path::new(file).to_path_buf());
    }
    let cwd = try!(env::current_dir());
    let mut pwd = cwd.as_path();

    loop {
        let ret = pwd.join(file);
        if let Ok(metadata) = fs::metadata(&ret) {
            if metadata.is_file() {
                return Ok(ret);
            }
        }

        match pwd.parent() {
            Some(p) => pwd = p,
            None => break,
        }
    }

    Err(CliError::Generic(format!("Could not find `{}` in `{}` or any parent directory",
                                    file,
                                    pwd.display())))
}
