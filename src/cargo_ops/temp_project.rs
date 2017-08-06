use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::process;
use std::error::Error;
use std::collections::HashSet;

use tempdir::TempDir;
use toml::Value;
use toml::value::Table;
use cargo::core::{PackageId, Workspace};
use cargo::util::{CargoError, CargoErrorKind, CargoResult, Config};

use super::{ElaborateWorkspace, Manifest};

/// A temporary project
pub struct TempProject<'tmp> {
    pub workspace: Workspace<'tmp>,
    pub temp_dir: TempDir,
    manifest_paths: Vec<PathBuf>,
}

impl<'tmp> TempProject<'tmp> {
    /// Copy needed manifest and lock files from an existing workspace
    pub fn from_workspace(
        orig_workspace: &ElaborateWorkspace,
        config: &'tmp Config,
    ) -> CargoResult<TempProject<'tmp>> {
        // e.g. /path/to/project
        let workspace_root = orig_workspace.workspace.root().to_str().ok_or_else(|| {
            CargoError::from_kind(CargoErrorKind::Msg(format!(
                "Invalid character found in path {}",
                orig_workspace.workspace.root().to_string_lossy()
            )))
        })?;

        let temp_dir = TempDir::new("cargo-outdated")?;
        let manifest_paths = manifest_paths(orig_workspace)?;
        let mut tmp_manifest_paths = vec![];
        for from in &manifest_paths {
            // e.g. /path/to/project/src/sub
            let mut from_dir = from.clone();
            from_dir.pop();
            let from_dir = from_dir.to_string_lossy();
            // e.g. /tmp/cargo.xxx/src/sub
            let mut dest = PathBuf::from(format!(
                "{}/{}",
                temp_dir.path().to_string_lossy(),
                &from_dir[workspace_root.len()..]
            ));
            fs::create_dir_all(&dest)?;
            // e.g. /tmp/cargo.xxx/src/sub/Cargo.toml
            dest.push("Cargo.toml");
            tmp_manifest_paths.push(dest.clone());
            fs::copy(from, &dest)?;
            let lockfile = PathBuf::from(format!("{}/Cargo.lock", from_dir));
            if lockfile.is_file() {
                dest.pop();
                dest.push("Cargo.lock");
                fs::copy(lockfile, dest)?;
            }
        }
        Self::write_manifest_semver_with_paths(&tmp_manifest_paths)?;

        let root_manifest = PathBuf::from(
            String::from(temp_dir.path().to_string_lossy()) + "/Cargo.toml",
        );
        Ok(TempProject {
            workspace: Workspace::new(&root_manifest, config)?,
            temp_dir: temp_dir,
            manifest_paths: tmp_manifest_paths,
        })
    }

    // TODO: instead of process::Command, call cargo update internally
    /// Run `cargo update` against the temporary project
    pub fn cargo_update(&mut self, config: &'tmp Config) -> CargoResult<()> {
        let root_manifest = String::from(self.workspace.root().to_string_lossy()) + "/Cargo.toml";
        if let Err(e) = process::Command::new("cargo")
            .arg("update")
            .arg("--manifest-path")
            .arg(&root_manifest)
            .output()
            .and_then(|v| if v.status.success() {
                Ok(v)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    "did not exit successfully",
                ))
            }) {
            return Err(CargoError::from_kind(CargoErrorKind::Msg(format!(
                "Failed to run 'cargo update' with error '{}'",
                e.description()
            ))));
        }
        self.workspace = Workspace::new(Path::new(&root_manifest), config)?;
        Ok(())
    }

    fn write_manifest<P: AsRef<Path>>(manifest: &Manifest, path: P) -> CargoResult<()> {
        let mut file = try!(File::create(path));
        let serialized = ::toml::to_string(manifest).expect("Failed to serialized Cargo.toml");
        try!(write!(file, "{}", serialized));
        Ok(())
    }

    /// Write manifests with SemVer requirements
    pub fn write_manifest_semver(&self) -> CargoResult<()> {
        Self::write_manifest_semver_with_paths(&self.manifest_paths)
    }

    fn write_manifest_semver_with_paths(manifest_paths: &Vec<PathBuf>) -> CargoResult<()> {
        let bin = {
            let mut bin = Table::new();
            bin.insert("name".to_owned(), Value::String("test".to_owned()));
            bin.insert("path".to_owned(), Value::String("test.rs".to_owned()));
            bin
        };
        for manifest_path in manifest_paths {
            let mut manifest: Manifest = {
                let mut buf = String::new();
                let mut file = File::open(manifest_path)?;
                file.read_to_string(&mut buf)?;
                ::toml::from_str(&buf)?
            };
            manifest.bin = Some(vec![bin.clone()]);
            // provide lib.path
            manifest.lib.as_mut().map(|lib| {
                lib.insert("path".to_owned(), Value::String("test_lib.rs".to_owned()));
            });
            Self::write_manifest(&manifest, manifest_path)?;
        }

        Ok(())
    }

    /// Write manifests with wildcard requirements
    pub fn write_manifest_latest(&self) -> CargoResult<()> {
        let bin = {
            let mut bin = Table::new();
            bin.insert("name".to_owned(), Value::String("test".to_owned()));
            bin.insert("path".to_owned(), Value::String("test.rs".to_owned()));
            bin
        };
        for manifest_path in &self.manifest_paths {
            let mut manifest: Manifest = {
                let mut buf = String::new();
                let mut file = File::open(manifest_path)?;
                file.read_to_string(&mut buf)?;
                ::toml::from_str(&buf)?
            };
            manifest.bin = Some(vec![bin.clone()]);

            // provide lib.path
            manifest.lib.as_mut().map(|lib| {
                lib.insert("path".to_owned(), Value::String("test_lib.rs".to_owned()));
            });

            // replace versions of direct dependencies
            manifest
                .dependencies
                .as_mut()
                .map(Self::replace_version_with_wildcard);
            manifest
                .dev_dependencies
                .as_mut()
                .map(Self::replace_version_with_wildcard);
            manifest
                .build_dependencies
                .as_mut()
                .map(Self::replace_version_with_wildcard);

            // replace target-specific dependencies
            manifest.target.as_mut().map(
                |ref mut t| for target in t.values_mut() {
                    if let Value::Table(ref mut target) = *target {
                        for dependency_tables in
                            &["dependencies", "dev-dependencies", "build-dependencies"]
                        {
                            target.get_mut(*dependency_tables).map(|dep_table| {
                                if let Value::Table(ref mut dep_table) = *dep_table {
                                    Self::replace_version_with_wildcard(dep_table);
                                }
                            });
                        }
                    }
                },
            );
            Self::write_manifest(&manifest, manifest_path)?;
        }
        Ok(())
    }

    fn replace_version_with_wildcard(dependencies: &mut Table) {
        let dep_names: Vec<_> = dependencies.keys().cloned().collect();
        for name in dep_names {
            let original = dependencies.get(&name).cloned().unwrap();
            match original {
                Value::String(_) => {
                    dependencies.insert(name, Value::String("*".to_owned()));
                }
                Value::Table(ref t) => {
                    let mut replaced = t.clone();
                    if replaced.contains_key("version") {
                        replaced.insert("version".to_owned(), Value::String("*".to_owned()));
                    }
                    dependencies.insert(name, Value::Table(replaced));
                }
                _ => panic!("Dependency spec is neither a string nor a table {}", name),
            }
        }
    }
}

fn manifest_paths(elab: &ElaborateWorkspace) -> CargoResult<Vec<PathBuf>> {
    let mut visited: HashSet<PackageId> = HashSet::new();
    let mut manifest_paths = vec![];
    let workspace_members: HashSet<_> = elab.workspace
        .members()
        .map(|pkg| pkg.package_id())
        .collect();
    for member in &workspace_members {
        manifest_paths.push(elab.pkgs[member].manifest_path().to_owned());
    }
    let root_pkg = elab.workspace.current()?.package_id();
    // e.g. /path/to/project
    let workspace_path = elab.workspace.current()?.root().to_string_lossy();

    fn manifest_paths_recursive(
        pkg_id: &PackageId,
        elab: &ElaborateWorkspace,
        workspace_path: &str,
        members: &HashSet<&PackageId>,
        visited: &mut HashSet<PackageId>,
        manifest_paths: &mut Vec<PathBuf>,
    ) -> CargoResult<()> {
        if visited.contains(pkg_id) {
            return Ok(());
        }
        visited.insert(pkg_id.clone());
        if !members.contains(pkg_id) {
            let pkg = &elab.pkgs[pkg_id];
            let pkg_path = pkg.root().to_string_lossy();
            if &pkg_path[..workspace_path.len()] == workspace_path {
                manifest_paths.push(pkg.manifest_path().to_owned());
            }
        }

        for dep in elab.pkg_deps[pkg_id].keys() {
            manifest_paths_recursive(dep, elab, workspace_path, members, visited, manifest_paths)?;
        }

        Ok(())
    };
    manifest_paths_recursive(
        root_pkg,
        elab,
        &workspace_path,
        &workspace_members,
        &mut visited,
        &mut manifest_paths,
    )?;


    Ok(manifest_paths)
}
