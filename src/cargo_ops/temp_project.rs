use std::{
    cell::RefCell,
    collections::HashSet,
    env,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::{anyhow, Context};
use cargo::{
    core::{Dependency, PackageId, Summary, Verbosity, Workspace},
    ops::{update_lockfile, UpdateOptions},
    sources::{
        config::SourceConfigMap,
        source::{QueryKind, Source},
    },
    util::{cache_lock::CacheLockMode, network::PollExt, CargoResult, Config},
};
use semver::{Version, VersionReq};
use tempfile::{Builder, TempDir};
use toml::{value::Table, Value};

use super::{ElaborateWorkspace, Manifest};
use crate::{error::OutdatedError, Options};

/// A temporary project
pub struct TempProject<'tmp> {
    pub workspace: Rc<RefCell<Option<Workspace<'tmp>>>>,
    pub temp_dir: TempDir,
    manifest_paths: Vec<PathBuf>,
    config: Config,
    relative_manifest: String,
    options: &'tmp Options,
    is_workspace_project: bool,
}

impl<'tmp> TempProject<'tmp> {
    /// Copy needed manifest and lock files from an existing workspace
    pub fn from_workspace(
        orig_workspace: &ElaborateWorkspace<'_>,
        orig_manifest: &str,
        options: &'tmp Options,
    ) -> CargoResult<TempProject<'tmp>> {
        // e.g. /path/to/project
        let workspace_root = orig_workspace.workspace.root();
        let workspace_root_str = workspace_root.to_string_lossy();
        let temp_dir = Builder::new().prefix("cargo-outdated").tempdir()?;
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

            // removing default-run key if it exists to check dependencies
            let mut om: Manifest = {
                let mut buf = String::new();
                let mut file = File::open(&dest)?;
                file.read_to_string(&mut buf)?;
                ::toml::from_str(&buf)?
            };

            if om.package.contains_key("default-run") {
                om.package.remove("default-run");
                let om_serialized = ::toml::to_string(&om).expect("Cannot format as toml file");
                let mut cargo_toml = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .open(&dest)?;
                write!(cargo_toml, "{om_serialized}")?;
            }

            // if build script is specified in the original Cargo.toml (from links or build)
            // remove it as we do not need it for checking dependencies
            if om.package.contains_key("links") {
                om.package.remove("links");
                let om_serialized = ::toml::to_string(&om).expect("Cannot format as toml file");
                let mut cargo_toml = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .open(&dest)?;
                write!(cargo_toml, "{om_serialized}")?;
            }

            if om.package.contains_key("build") {
                om.package.remove("build");
                let om_serialized = ::toml::to_string(&om).expect("Cannot format as toml file");
                let mut cargo_toml = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .truncate(true)
                    .open(&dest)?;
                write!(cargo_toml, "{om_serialized}")?;
            }

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

        //.cargo/config.toml
        // this is the preferred way
        // https://doc.rust-lang.org/cargo/reference/config.html
        if workspace_root.join(".cargo/config.toml").is_file() {
            fs::create_dir_all(temp_dir.path().join(".cargo"))?;
            fs::copy(
                workspace_root.join(".cargo/config.toml"),
                temp_dir.path().join(".cargo/config.toml"),
            )?;
        }

        //.cargo/config
        // this is legacy support for config files without the `.toml` extension
        if workspace_root.join(".cargo/config").is_file() {
            fs::create_dir_all(temp_dir.path().join(".cargo"))?;
            fs::copy(
                workspace_root.join(".cargo/config"),
                temp_dir.path().join(".cargo/config"),
            )?;
        }

        let relative_manifest = String::from(&orig_manifest[workspace_root_str.len() + 1..]);
        let config = Self::generate_config(temp_dir.path(), &relative_manifest, options)?;

        Ok(TempProject {
            workspace: Rc::new(RefCell::new(None)),
            temp_dir,
            manifest_paths: tmp_manifest_paths,
            config,
            relative_manifest,
            options,
            is_workspace_project: orig_workspace.workspace_mode,
        })
    }

    fn generate_config(
        root: &Path,
        relative_manifest: &str,
        options: &Options,
    ) -> CargoResult<Config> {
        let shell = ::cargo::core::Shell::new();
        let cwd = env::current_dir()
            .with_context(|| "Cargo couldn't get the current directory of the process")?;

        let homedir = ::cargo::util::homedir(&cwd).ok_or_else(|| {
            anyhow!(
                "Cargo couldn't find your home directory. \
                 This probably means that $HOME was not set.",
            )
        })?;
        let mut cwd = Path::new(root).join(relative_manifest);
        cwd.pop();

        // Check if $CARGO_HOME is set before capturing the config environment
        // if it is, set it in the configure options
        let cargo_home_path = std::env::var_os("CARGO_HOME").map(std::path::PathBuf::from);

        let mut config = Config::new(shell, cwd, homedir);
        config.configure(
            0,
            options.verbose == 0,
            Some(&options.color.to_string().to_ascii_lowercase()),
            options.frozen(),
            options.locked(),
            options.offline,
            &cargo_home_path,
            &[],
            &[],
        )?;
        Ok(config)
    }

    /// Run `cargo update` against the temporary project
    pub fn cargo_update(&self) -> CargoResult<()> {
        let update_opts = UpdateOptions {
            recursive: false,
            precise: None,
            to_update: Vec::new(),
            config: &self.config,
            dry_run: false,
            workspace: self.is_workspace_project,
        };
        update_lockfile(
            self.workspace
                .borrow()
                .as_ref()
                .ok_or(OutdatedError::NoWorkspace)?,
            &update_opts,
        )?;
        Ok(())
    }

    fn write_manifest<P: AsRef<Path>>(manifest: &Manifest, path: P) -> CargoResult<()> {
        let mut file = File::create(path)?;
        let serialized = ::toml::to_string(manifest).expect("Failed to serialized Cargo.toml");
        write!(file, "{serialized}")?;
        Ok(())
    }

    fn manipulate_dependencies<F>(manifest: &mut Manifest, f: &mut F) -> CargoResult<()>
    where
        F: FnMut(&mut Table) -> CargoResult<()>,
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
            for (_key, target) in t.iter_mut() {
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
        if let Some(t) = manifest.patch.as_mut() {
            for (_key, patch) in t.iter_mut() {
                if let Value::Table(ref mut patch) = *patch {
                    f(patch)?;
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
        workspace: &ElaborateWorkspace<'_>,
        skipped: &mut HashSet<String>,
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
            if let Some(lib) = manifest.lib.as_mut() {
                lib.insert("path".to_owned(), Value::String("test_lib.rs".to_owned()));
            }
            Self::manipulate_dependencies(&mut manifest, &mut |deps| {
                Self::replace_path_with_absolute(
                    self,
                    deps,
                    orig_root.as_ref(),
                    tmp_root.as_ref(),
                    manifest_path,
                    skipped,
                )
            })?;

            let package_name = manifest.name();
            let features = manifest.features.clone();
            Self::manipulate_dependencies(&mut manifest, &mut |deps| {
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
    pub fn write_manifest_latest<P: AsRef<Path>>(
        &'tmp self,
        orig_root: P,
        tmp_root: P,
        workspace: &ElaborateWorkspace<'_>,
        skipped: &mut HashSet<String>,
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
            if let Some(lib) = manifest.lib.as_mut() {
                lib.insert("path".to_owned(), Value::String("test_lib.rs".to_owned()));
            }
            Self::manipulate_dependencies(&mut manifest, &mut |deps| {
                Self::replace_path_with_absolute(
                    self,
                    deps,
                    orig_root.as_ref(),
                    tmp_root.as_ref(),
                    manifest_path,
                    skipped,
                )
            })?;

            let package_name = manifest.name();
            let features = manifest.features.clone();
            Self::manipulate_dependencies(&mut manifest, &mut |deps| {
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
        workspace: &ElaborateWorkspace<'_>,
        find_latest: bool,
    ) -> CargoResult<Summary> {
        let package_id = workspace.find_direct_dependency(name, dependent_package_name)?;
        let version = package_id.version();
        let source_id = package_id.source_id().with_locked_precise();
        let query_result = {
            let ws_config = workspace.workspace.config();
            let _lock = ws_config.acquire_package_cache_lock(CacheLockMode::DownloadExclusive)?;
            let source_config = SourceConfigMap::new(ws_config)?;
            let mut source = source_config.load(source_id, &HashSet::new())?;
            if !source_id.is_crates_io() {
                source.invalidate_cache();
            }
            source.block_until_ready()?;
            let dependency = Dependency::parse(name, None, source_id)?;
            let mut query_result = source
                .query_vec(&dependency, QueryKind::Exact)?
                .expect("Source should be ready");
            query_result.sort_by(|a, b| b.version().cmp(a.version()));
            query_result
        };
        let version_req = match requirement {
            Some(requirement) => Some(VersionReq::parse(requirement)?),
            None => None,
        };
        let latest_result = query_result.iter().find(|summary| {
            if summary.version() < version {
                false
            } else if version_req.is_none() {
                true
            } else if find_latest {
                // this unwrap is safe since we check if `version_req` is `None` before this
                // (which is only `None` if `requirement` is `None`)
                self.options.aggressive
                    || valid_latest_version(requirement.unwrap(), summary.version())
            } else {
                // this unwrap is safe since we check if `version_req` is `None` before this
                version_req.as_ref().unwrap().matches(summary.version())
            }
        });

        let latest_summary = match latest_result {
            Some(summary) => summary,
            None => {
                // If the version_req cannot be found use the version
                // this happens when we use a git repository as a dependency, without specifying
                // the version in Cargo.toml, preventing us from needing an unwrap below in the
                // warn
                let ver_req = match version_req {
                    Some(v_r) => format!("{v_r}"),
                    None => format!("{version}"),
                };
                // this should be safe it should only fail if we cannot get
                // access to write to the terminal
                // if this fails it's a cargo (as a dependency) issue
                self.warn(format!(
                    "cannot compare {} crate version found in toml {} with crates.io latest {}",
                    name,
                    ver_req,
                    query_result[0].version()
                ))?;

                // this returns the latest version
                &query_result[0]
            }
        };

        Ok(latest_summary.clone())
    }

    fn feature_includes(&self, name: &str, optional: bool, features_table: &Option<Value>) -> bool {
        if self.options.all_features() {
            return true;
        }
        if !optional && self.options.features.contains(&String::from("default")) {
            return true;
        }
        let features_table = match *features_table {
            Some(Value::Table(ref features_table)) => features_table,
            _ => return false,
        };
        let mut to_resolve: Vec<&str> = self
            .options
            .features
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
                    None => panic!("Feature {feature} does not exist"),
                    Some(Value::Array(ref specified_features)) => specified_features,
                    _ => panic!("Feature {feature} is not mapped to an array"),
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
        workspace: &ElaborateWorkspace<'_>,
        package_name: &str,
        version_to_latest: bool,
    ) -> CargoResult<()> {
        let dep_keys: Vec<_> = dependencies.keys().cloned().collect();
        for dep_key in dep_keys {
            // this, by brute force, allows a user to exclude a dependency by not writing
            // it to the temp project's manifest
            // In short this allows cargo to build the package with semver minor
            // compatibilities issues https://github.com/rust-lang/cargo/issues/6584
            // https://github.com/kbknapp/cargo-outdated/issues/230
            if self.options.exclude.contains(&dep_key) {
                continue;
            }

            let original = dependencies
                .get(&dep_key)
                .cloned()
                .ok_or(OutdatedError::NoMatchingDependency)?;

            match original {
                Value::String(requirement) => {
                    let name = dep_key;
                    if version_to_latest {
                        match self.find_update(
                            &name,
                            package_name,
                            Some(requirement.as_str()),
                            workspace,
                            version_to_latest,
                        ) {
                            Result::Ok(val) => dependencies
                                .insert(name.clone(), Value::String(val.version().to_string())),
                            Result::Err(_err) => {
                                eprintln!(
                                    "Updates to dependency {} could not be found",
                                    name.clone()
                                );
                                None
                            }
                        };
                    }
                }
                Value::Table(ref t) => {
                    let mut name = match t.get("package") {
                        Some(Value::String(ref s)) => s,
                        Some(_) => panic!("'package' of dependency {dep_key} is not a string"),
                        None => &dep_key,
                    };

                    let mut orig_name = "";
                    if t.contains_key("package") {
                        orig_name = name;
                        name = &dep_key;
                    }

                    if !(version_to_latest || t.contains_key("features")) {
                        continue;
                    }
                    let optional = t
                        .get("optional")
                        .map(|optional| {
                            if let Value::Boolean(optional) = *optional {
                                optional
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if !self.feature_includes(name, optional, features) {
                        continue;
                    }
                    let mut replaced = t.clone();
                    let requirement = match t.get("version") {
                        Some(Value::String(ref requirement)) => Some(requirement.as_str()),
                        Some(_) => panic!("Version of {name} is not a string"),
                        _ => None,
                    };
                    let r_summary = self.find_update(
                        if orig_name.is_empty() {
                            name
                        } else {
                            orig_name
                        },
                        package_name,
                        requirement,
                        workspace,
                        version_to_latest,
                    );
                    let summary = match r_summary {
                        Result::Ok(val) => val,
                        Result::Err(_) => {
                            eprintln!("Update for {} could not be found!", name.clone());
                            return Ok(());
                        }
                    };
                    if version_to_latest && t.contains_key("version") {
                        replaced.insert(
                            "version".to_owned(),
                            Value::String(summary.version().to_string()),
                        );
                    }
                    if replaced.contains_key("features") {
                        let features = match replaced.get("features") {
                            Some(Value::Array(ref features)) => features
                                .iter()
                                .filter(|&feature| {
                                    let feature = match *feature {
                                        Value::String(ref feature) => feature,
                                        _ => panic!(
                                            "Features section of {name} is not an array of strings"
                                        ),
                                    };
                                    let retained =
                                        features_and_options(&summary).contains(feature.as_str());
                                    // this unwrap should be safe it should only fail if we cannot
                                    // get access to write to
                                    // the terminal
                                    // if this fails it's a cargo (as a dependency) issue
                                    if !retained {
                                        self.warn(format!(
                                            "Feature {} of package {} \
                                             has been obsolete in version {}",
                                            feature,
                                            name,
                                            summary.version()
                                        ))
                                        .unwrap();
                                    }
                                    retained
                                })
                                .cloned()
                                .collect::<Vec<Value>>(),
                            _ => panic!("Features section of {name} is not an array"),
                        };
                        replaced.insert("features".to_owned(), Value::Array(features));
                    }
                    dependencies.insert(name.clone(), Value::Table(replaced));
                }
                _ => panic!("Dependency spec is neither a string nor a table {dep_key}"),
            }
        }
        Ok(())
    }

    fn replace_path_with_absolute(
        &self,
        dependencies: &mut Table,
        orig_root: &Path,
        tmp_root: &Path,
        tmp_manifest: &Path,
        skipped: &mut HashSet<String>,
    ) -> CargoResult<()> {
        let dep_names: Vec<_> = dependencies.keys().cloned().collect();
        for name in dep_names {
            let original = dependencies
                .get(&name)
                .cloned()
                .ok_or(OutdatedError::NoMatchingDependency)?;
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
                                    PathBuf::from(relative.trim_start_matches(delimiter));
                                relative.pop();
                                relative.join(orig_path)
                            };
                            if !tmp_root.join(&relative).join("Cargo.toml").exists() {
                                if self.options.root_deps_only {
                                    dependencies.remove(&name);

                                    if t.contains_key("package") {
                                        if let Value::String(ref package_name) = t["package"] {
                                            skipped.insert(package_name.to_string());
                                        } else {
                                            skipped.insert(name);
                                        }
                                    } else {
                                        skipped.insert(name);
                                    }
                                } else {
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
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn warn<T: ::std::fmt::Display>(&self, message: T) -> CargoResult<()> {
        let original_verbosity = self.config.shell().verbosity();
        self.config.shell().set_verbosity(if self.options.quiet {
            Verbosity::Quiet
        } else {
            Verbosity::Normal
        });
        self.config.shell().warn(message)?;
        self.config.shell().set_verbosity(original_verbosity);
        Ok(())
    }
}

/// Features and optional dependencies of a Summary
fn features_and_options(summary: &Summary) -> HashSet<&str> {
    let mut result: HashSet<&str> = summary.features().keys().map(|s| s.as_str()).collect();
    summary
        .dependencies()
        .iter()
        .filter(|d| d.is_optional())
        .map(Dependency::package_name)
        .for_each(|d| {
            result.insert(d.as_str());
        });
    result
}

/// Paths of all manifest files in current workspace
fn manifest_paths(elab: &ElaborateWorkspace<'_>) -> CargoResult<Vec<PathBuf>> {
    let mut visited: HashSet<PackageId> = HashSet::new();
    let mut manifest_paths = vec![];

    fn manifest_paths_recursive(
        pkg_id: PackageId,
        elab: &ElaborateWorkspace<'_>,
        workspace_path: &str,
        visited: &mut HashSet<PackageId>,
        manifest_paths: &mut Vec<PathBuf>,
    ) -> CargoResult<()> {
        if visited.contains(&pkg_id) {
            return Ok(());
        }
        visited.insert(pkg_id);
        let pkg = &elab.pkgs[&pkg_id];
        let pkg_path = pkg.root().to_string_lossy();

        // Checking if there's a CARGO_HOME set and that it is not an empty string
        let cargo_home_path = match std::env::var_os("CARGO_HOME") {
            Some(path) if !path.is_empty() => Some(
                path.into_string()
                    .expect("Error getting string from OsString"),
            ),
            _ => None,
        };

        // If there is a CARGO_HOME make sure we do not crawl the registry for more
        // Cargo.toml files Otherwise add all Cargo.toml files to the manifest
        // paths
        if pkg.root().starts_with(PathBuf::from(workspace_path))
            && (cargo_home_path.is_none()
                || !pkg_path
                    .starts_with(&cargo_home_path.expect("Error extracting CARGO_HOME string")))
        {
            manifest_paths.push(pkg.manifest_path().to_owned());
        }

        for &dep in elab.pkg_deps[&pkg_id].keys() {
            manifest_paths_recursive(dep, elab, workspace_path, visited, manifest_paths)?;
        }

        Ok(())
    }

    // executed against a virtual manifest
    let workspace_path = elab.workspace.root().to_string_lossy();
    // if cargo workspace is not explicitly used, the package itself would be a
    // member
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

fn valid_latest_version(mut requirement: &str, version: &Version) -> bool {
    match (requirement.contains('-'), !version.pre.is_empty()) {
        // if user was on a stable channel, it's unlikely for him to update to an unstable one
        (false, true) => false,
        // both are stable, leave for further filters
        // ...or...
        // user was on an unstable one, newer stable ones are still candidates
        (false, false) | (true, false) => true,
        // both are unstable, must be in the same channel
        (true, true) => {
            requirement = requirement.trim_start_matches(&['=', ' ', '~', '^'][..]);
            let requirement_version = Version::parse(requirement)
                .expect("Error could not parse requirement into a semantic version");
            let requirement_channel = requirement_version.pre.split('.').next().unwrap();
            let requirement_channel_numeric =
                requirement_channel.bytes().all(|b| b.is_ascii_digit());

            let version_channel = version.pre.split('.').next().unwrap();
            let version_channel_numeric = version_channel.bytes().all(|b| b.is_ascii_digit());

            match (requirement_channel_numeric, version_channel_numeric) {
                (false, false) => requirement_channel == version_channel,
                (true, true) => true,
                _ => false,
            }
        }
    }
}
