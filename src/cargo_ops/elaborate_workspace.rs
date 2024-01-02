use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet, VecDeque},
    io::{self, Write},
    rc::Rc,
};

use anyhow::anyhow;
use cargo::{
    core::{
        compiler::{CompileKind, RustcTargetData},
        dependency::DepKind,
        resolver::{
            features::{ForceAllTargets, HasDevUnits},
            CliFeatures,
        },
        Dependency, FeatureValue, Package, PackageId, Workspace,
    },
    ops::{self, Packages},
    util::{interning::InternedString, CargoResult, Config},
};
use serde::{Deserialize, Serialize};
use tabwriter::TabWriter;

use crate::error::OutdatedError;

use super::{pkg_status::*, Options};

/// An elaborate workspace containing resolved dependencies and
/// the update status of packages
pub struct ElaborateWorkspace<'ela> {
    pub workspace: &'ela Workspace<'ela>,
    pub pkgs: HashMap<PackageId, Package>,
    pub pkg_deps: HashMap<PackageId, HashMap<PackageId, Dependency>>,
    /// Map of package status
    pub pkg_status: RefCell<HashMap<Vec<PackageId>, PkgStatus>>,
    /// Whether using workspace mode
    pub workspace_mode: bool,
}

/// A struct to serialize to json with serde
#[derive(Serialize, Deserialize)]
pub struct CrateMetadata {
    pub crate_name: String,
    pub dependencies: BTreeSet<Metadata>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq)]
pub struct Metadata {
    pub name: String,
    pub project: String,
    pub compat: String,
    pub latest: String,
    pub kind: Option<String>,
    pub platform: Option<String>,
}

impl Ord for Metadata {
    fn cmp(&self, other: &Self) -> Ordering { self.name.cmp(&other.name) }
}

impl PartialOrd for Metadata {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl<'ela> ElaborateWorkspace<'ela> {
    /// Elaborate a `Workspace`
    pub fn from_workspace(
        workspace: &'ela Workspace<'_>,
        options: &Options,
    ) -> CargoResult<ElaborateWorkspace<'ela>> {
        // new in cargo 0.54.0
        let flag_features: BTreeSet<FeatureValue> = options
            .features
            .iter()
            .map(|feature| FeatureValue::new(InternedString::from(feature)))
            .collect();
        let specs = Packages::All.to_package_id_specs(workspace)?;

        let cli_features = CliFeatures {
            features: Rc::new(flag_features),
            all_features: options.all_features(),
            uses_default_features: options.no_default_features(),
        };

        // The CompileKind, this has no target since it's the temp workspace
        // targets are blank since we don't need to fully build for the targets to get
        // the dependencies
        let compile_kind = CompileKind::from_requested_targets(workspace.config(), &[])?;
        let mut target_data = RustcTargetData::new(workspace, &compile_kind)?;
        let ws_resolve = ops::resolve_ws_with_opts(
            workspace,
            &mut target_data,
            &compile_kind,
            &cli_features,
            &specs,
            HasDevUnits::Yes,
            ForceAllTargets::Yes,
            // TODO: respect MSRV here
            None,
        )?;
        let packages = ws_resolve.pkg_set;
        let resolve = ws_resolve
            .workspace_resolve
            .expect("Error getting workspace resolved");
        let mut pkgs = HashMap::new();
        let mut pkg_deps = HashMap::new();
        for pkg in packages.get_many(packages.package_ids())? {
            let pkg_id = pkg.package_id();
            pkgs.insert(pkg_id, pkg.clone());
            let deps = pkg.dependencies();
            let mut dep_map = HashMap::new();
            for dep_id in resolve.deps(pkg_id) {
                for d in deps {
                    if d.matches_id(dep_id.0) {
                        dep_map.insert(dep_id.0, d.clone());
                        break;
                    }
                }
            }
            pkg_deps.insert(pkg_id, dep_map);
        }

        Ok(ElaborateWorkspace {
            workspace,
            pkgs,
            pkg_deps,
            pkg_status: RefCell::new(HashMap::new()),
            workspace_mode: options.workspace || workspace.current().is_err(),
        })
    }

    /// Determine root package based on current workspace and CLI options
    pub fn determine_root(&self, options: &Options) -> CargoResult<PackageId> {
        if let Some(ref root_name) = options.root {
            if let Ok(workspace_root) = self.workspace.current() {
                if root_name == workspace_root.name().as_str() {
                    Ok(workspace_root.package_id())
                } else {
                    for direct_dep in self.pkg_deps[&workspace_root.package_id()].keys() {
                        if self.pkgs[direct_dep].name().as_str() == root_name {
                            return Ok(*direct_dep);
                        }
                    }
                    Err(anyhow!(
                        "Root is neither the workspace root nor a direct dependency",
                    ))
                }
            } else {
                Err(anyhow!(
                    "--root is not allowed when running against a virtual manifest",
                ))
            }
        } else {
            Ok(self.workspace.current()?.package_id())
        }
    }

    /// Find a member based on member name
    fn find_member(&self, member: PackageId) -> CargoResult<PackageId> {
        for m in self.workspace.members() {
            // members with the same name in a workspace is not allowed
            // even with different paths
            if member.name() == m.name() {
                return Ok(m.package_id());
            }
        }
        Err(anyhow!("Workspace member {} not found", member.name()))
    }

    /// Find a contained package, which is a member or dependency inside the
    /// workspace
    fn find_contained_package(&self, name: &str) -> CargoResult<PackageId> {
        let root_path = self.workspace.root();
        for (pkg_id, pkg) in &self.pkgs {
            if pkg.manifest_path().starts_with(root_path) && pkg.name().as_str() == name {
                return Ok(*pkg_id);
            }
        }
        Err(anyhow!("Cannot find package {} in workspace", name))
    }

    /// Find a direct dependency of a contained package
    pub fn find_direct_dependency(
        &self,
        dependency_name: &str,
        dependent_package_name: &str,
    ) -> CargoResult<PackageId> {
        let dependent_package = self.find_contained_package(dependent_package_name)?;

        for direct_dep in self.pkg_deps[&dependent_package].keys() {
            if direct_dep.name().as_str() == dependency_name {
                return Ok(*direct_dep);
            }
        }

        for (pkg_id, pkg) in &self.pkgs {
            if pkg.name().as_str() == dependency_name {
                return Ok(*pkg_id);
            }
        }

        Err(anyhow!(
            "Direct dependency {} not found for package {}",
            dependency_name,
            dependent_package_name
        ))
    }

    /// Resolve compatible and latest status from the corresponding
    /// `ElaborateWorkspace`s
    pub fn resolve_status(
        &'ela self,
        compat: &ElaborateWorkspace<'_>,
        latest: &ElaborateWorkspace<'_>,
        options: &Options,
        _config: &Config,
        root: PackageId,
        skip: &HashSet<String>,
    ) -> CargoResult<()> {
        self.pkg_status.borrow_mut().clear();
        let (compat_root, latest_root) = if self.workspace_mode {
            (compat.find_member(root)?, latest.find_member(root)?)
        } else {
            (
                compat.determine_root(options)?,
                latest.determine_root(options)?,
            )
        };

        let mut queue = VecDeque::new();
        queue.push_back((vec![root], Some(compat_root), Some(latest_root)));
        while let Some((path, compat_pkg, latest_pkg)) = queue.pop_front() {
            let pkg = path.last().ok_or(OutdatedError::EmptyPath)?;
            let depth = path.len() as i32 - 1;
            // generate pkg_status
            let status = PkgStatus {
                compat: Status::from_versions(pkg.version(), compat_pkg.map(PackageId::version)),
                latest: Status::from_versions(pkg.version(), latest_pkg.map(PackageId::version)),
            };
            debug!(
                _config,
                "STATUS => PKG: {}; PATH: {:?}; COMPAT: {:?}; LATEST: {:?}; STATUS: {:?}",
                pkg,
                path,
                compat_pkg,
                latest_pkg,
                status
            );
            self.pkg_status.borrow_mut().insert(path.clone(), status);
            // next layer
            // this unwrap is safe since we first check if it is None :)
            if options.depth.is_none() || depth < options.depth.unwrap() {
                self.pkg_deps[pkg]
                    .keys()
                    .filter(|dep| !path.contains(dep))
                    .filter(|&dep| !skip.contains(dep.name().as_str()))
                    .for_each(|&dep| {
                        let name = dep.name();
                        let compat_pkg = compat_pkg
                            .and_then(|id| compat.pkg_deps.get(&id))
                            .map(HashMap::keys)
                            .and_then(|mut deps| deps.find(|dep| dep.name() == name))
                            .cloned();
                        let latest_pkg = latest_pkg
                            .and_then(|id| latest.pkg_deps.get(&id))
                            .map(HashMap::keys)
                            .and_then(|mut deps| deps.find(|dep| dep.name() == name))
                            .cloned();
                        let mut path = path.clone();
                        path.push(dep);
                        queue.push_back((path, compat_pkg, latest_pkg));
                    });
            }
        }

        Ok(())
    }

    /// Print package status to `TabWriter`
    pub fn print_list(
        &'ela self,
        options: &Options,
        root: PackageId,
        preceding_line: bool,
        skip: &HashSet<String>,
    ) -> CargoResult<i32> {
        let mut lines = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(vec![root]);
        while let Some(path) = queue.pop_front() {
            let pkg = path.last().ok_or(OutdatedError::EmptyPath)?;
            let name = pkg.name().to_string();

            if options.ignore.contains(&name) {
                continue;
            }

            let depth = path.len() as i32 - 1;
            // generate lines
            let status = &self.pkg_status.borrow_mut()[&path];
            if (status.compat.is_changed() || status.latest.is_changed())
                && (options.packages.is_empty() || options.packages.contains(&name))
            {
                // name version compatible latest kind platform
                let parent = path.get(path.len() - 2);
                if let Some(parent) = parent {
                    let dependency = &self.pkg_deps[parent][pkg];
                    let label = if self.workspace_mode
                        || parent == &self.workspace.current()?.package_id()
                    {
                        name
                    } else {
                        format!("{}->{}", self.pkgs[parent].name(), name)
                    };
                    let line = format!(
                        "{}\t{}\t{}\t{}\t{:?}\t{}\n",
                        label,
                        pkg.version(),
                        status.compat.to_string(),
                        status.latest.to_string(),
                        dependency.kind(),
                        dependency
                            .platform()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| "---".to_owned())
                    );
                    lines.insert(line);
                } else {
                    let line = format!(
                        "{}\t{}\t{}\t{}\t---\t---\n",
                        name,
                        pkg.version(),
                        status.compat.to_string(),
                        status.latest.to_string()
                    );
                    lines.insert(line);
                }
            }
            // next layer
            // this unwrap is safe since we first check if it is None :)
            if options.depth.is_none() || depth < options.depth.unwrap() {
                self.pkg_deps[pkg]
                    .keys()
                    .filter(|dep| !path.contains(dep))
                    .filter(|&dep| {
                        !self.workspace_mode
                            || !self.workspace.members().any(|mem| &mem.package_id() == dep)
                    })
                    .filter(|&dep| !skip.contains(dep.name().as_str()))
                    .for_each(|&dep| {
                        let mut path = path.clone();
                        path.push(dep);
                        queue.push_back(path);
                    });
            }
        }

        if lines.is_empty() {
            if !self.workspace_mode {
                println!("All dependencies are up to date, yay!");
            }
        } else {
            if preceding_line {
                println!();
            }
            if self.workspace_mode {
                println!("{}\n================", root.name());
            }
            let mut tw = TabWriter::new(vec![]);
            writeln!(&mut tw, "Name\tProject\tCompat\tLatest\tKind\tPlatform")?;
            writeln!(&mut tw, "----\t-------\t------\t------\t----\t--------")?;
            for line in &lines {
                write!(&mut tw, "{line}")?;
            }
            tw.flush()?;
            write!(io::stdout(), "{}", String::from_utf8(tw.into_inner()?)?)?;
            io::stdout().flush()?;
        }

        Ok(lines.len() as i32)
    }

    pub fn print_json(
        &'ela self,
        options: &Options,
        root: PackageId,
        skip: &HashSet<String>,
    ) -> CargoResult<i32> {
        let mut crate_graph = CrateMetadata {
            crate_name: root.name().to_string(),
            dependencies: BTreeSet::new(),
        };
        let mut queue = VecDeque::new();
        queue.push_back(vec![root]);

        while let Some(path) = queue.pop_front() {
            let pkg = path.last().ok_or(OutdatedError::EmptyPath)?;
            let name = pkg.name().to_string();

            if options.ignore.contains(&name) {
                continue;
            }

            let depth = path.len() as i32 - 1;
            // generate lines
            let status = &self.pkg_status.borrow_mut()[&path];
            if (status.compat.is_changed() || status.latest.is_changed())
                && (options.packages.is_empty() || options.packages.contains(&name))
            {
                // name version compatible latest kind platform
                // safely get the parent index
                let parent = if path.len() > 1 {
                    path.get(path.len() - 2)
                } else {
                    None
                };

                let line = if let Some(parent) = parent {
                    let dependency = &self.pkg_deps[parent][pkg];
                    let label = if self.workspace_mode
                        || parent == &self.workspace.current()?.package_id()
                    {
                        name
                    } else {
                        format!("{}->{}", self.pkgs[parent].name(), name)
                    };

                    let dependency_type = match dependency.kind() {
                        DepKind::Normal => "Normal",
                        DepKind::Development => "Development",
                        DepKind::Build => "Build",
                    };

                    Metadata {
                        name: label,
                        project: pkg.version().to_string(),
                        compat: status.compat.to_string(),
                        latest: status.latest.to_string(),
                        kind: Some(dependency_type.to_string()),
                        platform: dependency.platform().map(|p| p.to_string()),
                    }
                } else {
                    Metadata {
                        name,
                        project: pkg.version().to_string(),
                        compat: status.compat.to_string(),
                        latest: status.latest.to_string(),
                        kind: None,
                        platform: None,
                    }
                };

                crate_graph.dependencies.insert(line);
            }
            // next layer
            // this unwrap is safe since we first check if it is None :)
            if options.depth.is_none() || depth < options.depth.unwrap() {
                self.pkg_deps[pkg]
                    .keys()
                    .filter(|dep| !path.contains(dep))
                    .filter(|dep| {
                        !self.workspace_mode
                            || !self
                                .workspace
                                .members()
                                .any(|mem| mem.package_id() == **dep)
                    })
                    .filter(|&dep| !skip.contains(dep.name().as_str()))
                    .for_each(|dep| {
                        let mut path = path.clone();
                        path.push(*dep);
                        queue.push_back(path);
                    });
            }
        }

        println!("{}", serde_json::to_string(&crate_graph)?);

        Ok(crate_graph.dependencies.len() as i32)
    }
}
