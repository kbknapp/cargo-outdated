use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use tempdir::TempDir;
use toml::Value;
use toml::value::Table;
use cargo::util::errors::CargoResultExt;
use cargo::core::{Dependency, PackageId, Summary, Verbosity, Workspace};
use cargo::util::{CargoResult, Config};
use cargo::ops::{update_lockfile, UpdateOptions};
use semver::{Identifier, Version, VersionReq};

use Options;
use super::{ElaborateWorkspace, Manifest};

/// A temporary project
pub struct TempProject<'tmp> {
    pub workspace: Rc<RefCell<Option<Workspace<'tmp>>>>,
    pub temp_dir: TempDir,
    manifest_paths: Vec<PathBuf>,
    config: Config,
    relative_manifest: String,
    options: &'tmp Options,
}

impl<'tmp> TempProject<'tmp> {
    /// Copy needed manifest and lock files from an existing workspace
    pub fn from_workspace(
        orig_workspace: &ElaborateWorkspace,
        orig_manifest: &str,
        options: &'tmp Options,
    ) -> CargoResult<TempProject<'tmp>> {
        // e.g. /path/to/project
        let workspace_root = orig_workspace.workspace.root();
        let workspace_root_str = workspace_root.to_string_lossy();

        let temp_dir = TempDir::new("cargo-outdated")?;
        let manifest_paths = manifest_paths(orig_workspace)?;
        let mut tmp_manifest_paths = vec![];
        for from in &manifest_paths {
            // e.g. /path/to/project/src/sub
            let mut from_dir = from.clone();
            from_dir.pop();
            let from_dir_str = from_dir.to_string_lossy();
            // e.g. /tmp/cargo.xxx/src/sub
            let mut dest = if workspace_root_str.len() < from_dir_str.len() {
                temp_dir
                    .path()
                    .join(&from_dir_str[workspace_root_str.len() + 1..])
            } else {
                temp_dir.path().to_owned()
            };
            fs::create_dir_all(&dest)?;
            // e.g. /tmp/cargo.xxx/src/sub/Cargo.toml
            dest.push("Cargo.toml");
            tmp_manifest_paths.push(dest.clone());
            fs::copy(from, &dest)?;
            let lockfile = from_dir.join("Cargo.lock");
            if lockfile.is_file() {
                dest.pop();
                dest.push("Cargo.lock");
                fs::copy(lockfile, dest)?;
            }
        }

        // virtual root
        let mut virtual_root = workspace_root.join("Cargo.toml");
        if !manifest_paths.contains(&virtual_root) && virtual_root.is_file() {
            fs::copy(&virtual_root, temp_dir.path().join("Cargo.toml"))?;
            virtual_root.pop();
            virtual_root.push("Cargo.lock");
            if virtual_root.is_file() {
                fs::copy(&virtual_root, temp_dir.path().join("Cargo.lock"))?;
            }
        }

        let relative_manifest = String::from(&orig_manifest[workspace_root_str.len() + 1..]);
        let config = Self::generate_config(temp_dir.path(), &relative_manifest, options)?;
        Ok(TempProject {
            workspace: Rc::new(RefCell::new(None)),
            temp_dir: temp_dir,
            manifest_paths: tmp_manifest_paths,
            config: config,
            relative_manifest: relative_manifest,
            options: options,
        })
    }

    fn generate_config(
        root: &Path,
        relative_manifest: &str,
        options: &Options,
    ) -> CargoResult<Config> {
        let shell = ::cargo::core::Shell::new();
        let cwd = env::current_dir()
            .chain_err(|| "Cargo couldn't get the current directory of the process")?;

        let homedir = ::cargo::util::homedir(&cwd).ok_or_else(|| {
            "Cargo couldn't find your home directory. \
             This probably means that $HOME was not set."
        })?;
        let mut cwd = Path::new(root).join(relative_manifest);
        cwd.pop();
        let config = Config::new(shell, cwd, homedir);
        config.configure(
            0,
            if options.flag_verbose > 0 {
                None
            } else {
                Some(true)
            },
            &options.flag_color,
            options.flag_frozen,
            options.flag_locked,
            &[],
        )?;
        Ok(config)
    }

    /// Run `cargo update` against the temporary project
    pub fn cargo_update(&self) -> CargoResult<()> {
        let update_opts = UpdateOptions {
            aggressive: false,
            precise: None,
            to_update: &[],
            config: &self.config,
        };
        update_lockfile(self.workspace.borrow().as_ref().unwrap(), &update_opts)?;
        Ok(())
    }

    fn write_manifest<P: AsRef<Path>>(manifest: &Manifest, path: P) -> CargoResult<()> {
        let mut file = try!(File::create(path));
        let serialized = ::toml::to_string(manifest).expect("Failed to serialized Cargo.toml");
        try!(write!(file, "{}", serialized));
        Ok(())
    }

    fn manipulate_dependencies<F>(manifest: &mut Manifest, f: &F) -> CargoResult<()>
    where
        F: Fn(&mut Table) -> CargoResult<()>,
    {
        if let Some(dep) = manifest.dependencies.as_mut() {
            f(dep)?;
        }
        if let Some(dep) = manifest.dev_dependencies.as_mut() {
            f(dep)?;
        }
        if let Some(dep) = manifest.build_dependencies.as_mut() {
            f(dep)?;
        }
        if let Some(t) = manifest.target.as_mut() {
            for target in t.values_mut() {
                if let Value::Table(ref mut target) = *target {
                    for dependency_tables in
                        &["dependencies", "dev-dependencies", "build-dependencies"]
                    {
                        if let Some(&mut Value::Table(ref mut dep_table)) =
                            target.get_mut(*dependency_tables)
                        {
                            f(dep_table)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Write manifests with SemVer requirements
    pub fn write_manifest_semver<P: AsRef<Path>>(
        &'tmp self,
        orig_root: P,
        tmp_root: P,
        workspace: &ElaborateWorkspace,
    ) -> CargoResult<()> {
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
            Self::manipulate_dependencies(&mut manifest, &|deps| {
                Self::replace_path_with_absolute(
                    deps,
                    orig_root.as_ref(),
                    tmp_root.as_ref(),
                    manifest_path,
                )
            })?;
            let package_name = manifest.name();
            let features = manifest.features.clone();
            Self::manipulate_dependencies(&mut manifest, &|deps| {
                self.update_version_and_feature(deps, &features, workspace, &package_name, false)
            })?;
            Self::write_manifest(&manifest, manifest_path)?;
        }
        let root_manifest = self.temp_dir.path().join(&self.relative_manifest);
        *self.workspace.borrow_mut() =
            Some(Workspace::new(Path::new(&root_manifest), &self.config)?);
        Ok(())
    }

    /// Write manifests with wildcard requirements
    pub fn write_manifest_latest(&'tmp self, workspace: &ElaborateWorkspace) -> CargoResult<()> {
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
            let package_name = manifest.name();
            let features = manifest.features.clone();
            Self::manipulate_dependencies(&mut manifest, &|deps| {
                self.update_version_and_feature(deps, &features, workspace, &package_name, true)
            })?;
            Self::write_manifest(&manifest, manifest_path)?;
        }

        let root_manifest = self.temp_dir.path().join(&self.relative_manifest);
        *self.workspace.borrow_mut() =
            Some(Workspace::new(Path::new(&root_manifest), &self.config)?);
        Ok(())
    }

    fn find_update(
        &self,
        name: &str,
        dependent_package_name: &str,
        requirement: Option<&str>,
        workspace: &ElaborateWorkspace,
        find_latest: bool,
    ) -> CargoResult<Summary> {
        let package_id = workspace.find_direct_dependency(name, dependent_package_name)?;
        let source_id = package_id.source_id();
        let mut source = source_id.load(&self.config)?;
        if !source_id.is_default_registry() {
            source.update()?;
        }
        let dependency = Dependency::parse_no_deprecated(name, None, source_id)?;
        let query_result = source.query_vec(&dependency)?;
        let version_req = match requirement {
            Some(requirement) => Some(VersionReq::parse(requirement)?),
            None => None,
        };
        let summaries: BTreeMap<&Version, &Summary> = query_result
            .iter()
            .filter(|&summary| if version_req.is_none() {
                true
            } else if find_latest {
                self.options.flag_aggressive
                    || valid_latest_version(requirement.unwrap(), summary.version())
            } else {
                version_req.as_ref().unwrap().matches(summary.version())
            })
            .map(|summary| (summary.version(), summary))
            .collect();
        Ok(
            summaries
                .values()
                .last()
                .cloned()
                .expect(&format!(
                    "Cannot find matched versions of package {} from source {}",
                    name,
                    source_id
                ))
                .clone(),
        )
    }

    fn feature_includes(&self, name: &str, optional: bool, features_table: &Option<Value>) -> bool {
        if self.options.flag_all_features {
            return true;
        }
        if !optional
            && self.options
                .flag_features
                .contains(&String::from("default"))
        {
            return true;
        }
        let features_table = match *features_table {
            Some(Value::Table(ref features_table)) => features_table,
            _ => return false,
        };
        let mut to_resolve: Vec<&str> = self.options
            .flag_features
            .iter()
            .filter(|f| !f.is_empty())
            .map(String::as_str)
            .collect();
        let mut visited: HashSet<&str> = HashSet::new();
        while let Some(feature) = to_resolve.pop() {
            if feature == name {
                return true;
            }
            if visited.contains(feature) {
                continue;
            }
            visited.insert(feature);
            if features_table.contains_key(feature) {
                let specified_features = match features_table.get(feature) {
                    None => panic!("Feature {} does not exist", feature),
                    Some(&Value::Array(ref specified_features)) => specified_features,
                    _ => panic!("Feature {} is not mapped to an array", feature),
                };
                for spec in specified_features {
                    if let Value::String(ref spec) = *spec {
                        to_resolve.push(spec.as_str());
                    }
                }
            }
        }
        false
    }

    fn update_version_and_feature(
        &self,
        dependencies: &mut Table,
        features: &Option<Value>,
        workspace: &ElaborateWorkspace,
        package_name: &str,
        version_to_latest: bool,
    ) -> CargoResult<()> {
        let dep_names: Vec<_> = dependencies.keys().cloned().collect();
        for name in dep_names {
            let original = dependencies.get(&name).cloned().unwrap();
            match original {
                Value::String(requirement) => if version_to_latest {
                    dependencies.insert(
                        name.clone(),
                        Value::String(
                            self.find_update(
                                &name,
                                package_name,
                                Some(requirement.as_str()),
                                workspace,
                                version_to_latest,
                            )?
                                .version()
                                .to_string(),
                        ),
                    );
                },
                Value::Table(ref t) => {
                    if !(version_to_latest || t.contains_key("features")) {
                        continue;
                    }
                    let optional = t.get("optional")
                        .map(|optional| if let Value::Boolean(optional) = *optional {
                            optional
                        } else {
                            false
                        })
                        .unwrap_or(false);
                    if !self.feature_includes(&name, optional, features) {
                        continue;
                    }
                    let mut replaced = t.clone();
                    let requirement = match t.get("version") {
                        Some(&Value::String(ref requirement)) => Some(requirement.as_str()),
                        Some(_) => panic!("Version of {} is not a string", name),
                        _ => None,
                    };
                    let summary = self.find_update(
                        &name,
                        package_name,
                        requirement,
                        workspace,
                        version_to_latest,
                    )?;
                    if version_to_latest && t.contains_key("version") {
                        replaced.insert(
                            "version".to_owned(),
                            Value::String(summary.version().to_string()),
                        );
                    }
                    if replaced.contains_key("features") {
                        let features = match replaced.get("features") {
                            Some(&Value::Array(ref features)) => features
                                .iter()
                                .filter(|&feature| {
                                    let feature = match *feature {
                                        Value::String(ref feature) => feature,
                                        _ => panic!(
                                            "Features section of {} is not an array of strings",
                                            name
                                        ),
                                    };
                                    let retained = summary.features().contains_key(feature);
                                    if !retained {
                                        self.warn(format!(
                                            "Feature {} of package {} \
                                             has been obsolete in version {}",
                                            feature,
                                            name,
                                            summary.version()
                                        )).unwrap();
                                    }
                                    retained
                                })
                                .cloned()
                                .collect::<Vec<Value>>(),
                            _ => panic!("Features section of {} is not an array", name),
                        };
                        replaced.insert("features".to_owned(), Value::Array(features));
                    }
                    dependencies.insert(name.clone(), Value::Table(replaced));
                }
                _ => panic!("Dependency spec is neither a string nor a table {}", name),
            }
        }
        Ok(())
    }

    fn replace_path_with_absolute(
        dependencies: &mut Table,
        orig_root: &Path,
        tmp_root: &Path,
        tmp_manifest: &Path,
    ) -> CargoResult<()> {
        let dep_names: Vec<_> = dependencies.keys().cloned().collect();
        for name in dep_names {
            let original = dependencies.get(&name).cloned().unwrap();
            match original {
                Value::Table(ref t) if t.contains_key("path") => {
                    if let Value::String(ref orig_path) = t["path"] {
                        let orig_path = Path::new(orig_path);
                        if orig_path.is_relative() {
                            let relative = {
                                let delimiter: &[_] = &['/', '\\'];
                                let relative = &tmp_manifest.to_string_lossy()
                                    [tmp_root.to_string_lossy().len()..];
                                let mut relative =
                                    PathBuf::from(relative.trim_left_matches(delimiter));
                                relative.pop();
                                relative.join(orig_path)
                            };
                            if !tmp_root.join(&relative).join("Cargo.toml").exists() {
                                let mut replaced = t.clone();
                                replaced.insert(
                                    "path".to_owned(),
                                    Value::String(
                                        fs::canonicalize(orig_root.join(relative))?
                                            .to_string_lossy()
                                            .to_string(),
                                    ),
                                );
                                dependencies.insert(name, Value::Table(replaced));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn warn<T: ::std::fmt::Display>(&self, message: T) -> CargoResult<()> {
        let original_verbosity = self.config.shell().verbosity();
        self.config
            .shell()
            .set_verbosity(if self.options.flag_quiet {
                Verbosity::Quiet
            } else {
                Verbosity::Normal
            });
        self.config.shell().warn(message)?;
        self.config.shell().set_verbosity(original_verbosity);
        Ok(())
    }
}

/// Paths of all manifest files in current workspace
fn manifest_paths(elab: &ElaborateWorkspace) -> CargoResult<Vec<PathBuf>> {
    let mut visited: HashSet<PackageId> = HashSet::new();
    let mut manifest_paths = vec![];

    fn manifest_paths_recursive(
        pkg_id: &PackageId,
        elab: &ElaborateWorkspace,
        workspace_path: &str,
        visited: &mut HashSet<PackageId>,
        manifest_paths: &mut Vec<PathBuf>,
    ) -> CargoResult<()> {
        if visited.contains(pkg_id) {
            return Ok(());
        }
        visited.insert(pkg_id.clone());
        let pkg = &elab.pkgs[pkg_id];
        let pkg_path = pkg.root().to_string_lossy();
        if pkg_path.starts_with(workspace_path) {
            manifest_paths.push(pkg.manifest_path().to_owned());
        }

        for dep in elab.pkg_deps[pkg_id].keys() {
            manifest_paths_recursive(dep, elab, workspace_path, visited, manifest_paths)?;
        }

        Ok(())
    };

    // executed against a virtual manifest
    let workspace_path = elab.workspace.root().to_string_lossy();
    // if cargo workspace is not explicitly used, the pacakge itself would be a member
    for member in elab.workspace.members() {
        let root_pkg_id = member.package_id();
        manifest_paths_recursive(
            root_pkg_id,
            elab,
            &workspace_path,
            &mut visited,
            &mut manifest_paths,
        )?;
    }

    Ok(manifest_paths)
}

fn valid_latest_version(requirement: &str, version: &Version) -> bool {
    match (requirement.contains('-'), version.is_prerelease()) {
        // if user was on a stable channel, it's unlikely for him to update to an unstable one
        (false, true) => false,
        // both are stable, leave for further filters
        // ...or...
        // user was on an unstable one, newer stable ones are still candidates
        (false, false) | (true, false) => true,
        // both are unstable, must be in the same channel
        (true, true) => {
            let requirement_channel = {
                let pre = requirement.split(&['-', '+'][..]).nth(1).unwrap();
                pre.trim_matches(|c| !char::is_alphabetic(c))
            };
            match (requirement_channel.is_empty(), &version.pre[0]) {
                (true, &Identifier::Numeric(_)) => true,
                (false, &Identifier::AlphaNumeric(ref pre)) => {
                    requirement_channel == pre.trim_matches(char::is_numeric)
                }
                _ => false,
            }
        }
    }
}
