use std::io::{self, Write};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap, VecDeque};

use cargo::core::{Dependency, Package, PackageId, Workspace};
use cargo::ops::{self, Packages};
use cargo::util::{CargoError, CargoErrorKind, CargoResult, Config};
use tabwriter::TabWriter;

use super::Options;
use super::pkg_status::*;

/// An elaborate workspace containing resolved dependencies and
/// the update status of packages
pub struct ElaborateWorkspace<'ela> {
    pub workspace: &'ela Workspace<'ela>,
    pub pkgs: HashMap<PackageId, Package>,
    pub pkg_deps: HashMap<PackageId, HashMap<PackageId, Dependency>>,
    /// Map of package status
    pub pkg_status: RefCell<HashMap<Vec<&'ela PackageId>, PkgStatus>>,
    /// Whether using workspace mode
    pub workspace_mode: bool,
}

impl<'ela> ElaborateWorkspace<'ela> {
    /// Elaborate a `Workspace`
    pub fn from_workspace(
        workspace: &'ela Workspace,
        options: &Options,
    ) -> CargoResult<ElaborateWorkspace<'ela>> {
        let specs = Packages::All.into_package_id_specs(workspace)?;
        let (packages, resolve) = ops::resolve_ws_precisely(
            workspace,
            None,
            &options.flag_features,
            options.all_features(),
            options.no_default_features(),
            &specs,
        )?;
        let mut pkgs = HashMap::new();
        let mut pkg_deps = HashMap::new();
        for pkg_id in packages.package_ids() {
            let pkg = packages.get(pkg_id)?;
            pkgs.insert(pkg_id.clone(), pkg.clone());
            let deps = pkg.dependencies();
            let mut dep_map = HashMap::new();
            for dep_id in resolve.deps(pkg_id) {
                for d in deps {
                    if d.matches_id(dep_id) {
                        dep_map.insert(dep_id.clone(), d.clone());
                        break;
                    }
                }
            }
            pkg_deps.insert(pkg_id.clone(), dep_map);
        }

        Ok(ElaborateWorkspace {
            workspace: workspace,
            pkgs: pkgs,
            pkg_deps: pkg_deps,
            pkg_status: RefCell::new(HashMap::new()),
            workspace_mode: options.flag_workspace || workspace.current().is_err(),
        })
    }

    /// Determine root package based on current workspace and CLI options
    pub fn determine_root(&self, options: &Options) -> CargoResult<&PackageId> {
        if let Some(ref root_name) = options.flag_root {
            if let Ok(workspace_root) = self.workspace.current() {
                if root_name == workspace_root.name() {
                    Ok(workspace_root.package_id())
                } else {
                    for direct_dep in self.pkg_deps[workspace_root.package_id()].keys() {
                        if self.pkgs[direct_dep].name() == root_name {
                            return Ok(direct_dep);
                        }
                    }
                    return Err(CargoError::from_kind(CargoErrorKind::Msg(
                        "Root is neither the workspace root nor a direct dependency".to_owned(),
                    )));
                }
            } else {
                Err(CargoError::from_kind(CargoErrorKind::Msg(
                    "--root is not allowed when running against a virtual manifest".to_owned(),
                )))
            }
        } else {
            Ok(self.workspace.current()?.package_id())
        }
    }

    /// Find a member based on member name
    fn find_member(&self, member: &PackageId) -> CargoResult<&PackageId> {
        for m in self.workspace.members() {
            // members with the same name in a workspace is not allowed
            // even with different paths
            if member.name() == m.name() {
                return Ok(m.package_id());
            }
        }
        Err(CargoError::from_kind(CargoErrorKind::Msg(
            format!("Workspace member {} not found", member.name()),
        )))
    }

    /// Find a contained package, which is a member or dependency inside the workspace
    fn find_contained_package(&self, name: &str) -> CargoResult<PackageId> {
        let root_path = self.workspace.root();
        for (pkg_id, pkg) in &self.pkgs {
            if pkg.manifest_path().starts_with(root_path) && pkg.name() == name {
                return Ok(pkg_id.clone());
            }
        }
        Err(CargoError::from_kind(CargoErrorKind::Msg(
            format!("Cannot find package {} in workspace", name),
        )))
    }

    /// Find a direct dependency of a contained package
    pub fn find_direct_dependency(
        &self,
        dependency_name: &str,
        dependent_package_name: &str,
    ) -> CargoResult<PackageId> {
        let dependent_package = self.find_contained_package(dependent_package_name)?;
        for direct_dep in self.pkg_deps[&dependent_package].keys() {
            if direct_dep.name() == dependency_name {
                return Ok(direct_dep.clone());
            }
        }
        Err(CargoError::from_kind(CargoErrorKind::Msg(format!(
            "Direct dependency {} not found for package {}",
            dependency_name,
            dependent_package_name
        ))))
    }

    /// Resolve compatible and latest status from the corresponding `ElaborateWorkspace`s
    pub fn resolve_status(
        &'ela self,
        compat: &ElaborateWorkspace,
        latest: &ElaborateWorkspace,
        options: &Options,
        _config: &Config,
        root: &'ela PackageId,
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
            let pkg = path.last().unwrap();
            let depth = path.len() as i32;
            // generate pkg_status
            let status = PkgStatus {
                compat: Status::from_versions(pkg.version(), compat_pkg.map(|p| p.version())),
                latest: Status::from_versions(pkg.version(), latest_pkg.map(|p| p.version())),
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
            if options.flag_depth.is_none() || &depth < options.flag_depth.as_ref().unwrap() {
                self.pkg_deps[pkg]
                    .keys()
                    .filter(|dep| !path.contains(dep))
                    .for_each(|dep| {
                        let name = dep.name();
                        let compat_pkg = compat_pkg
                            .and_then(|id| compat.pkg_deps.get(id))
                            .map(|dep_map| dep_map.keys())
                            .and_then(|ref mut deps| deps.find(|dep| dep.name() == name));
                        let latest_pkg = latest_pkg
                            .and_then(|id| latest.pkg_deps.get(id))
                            .map(|dep_map| dep_map.keys())
                            .and_then(|ref mut deps| deps.find(|dep| dep.name() == name));
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
        root: &'ela PackageId,
        preceding_line: bool,
    ) -> CargoResult<i32> {
        let mut lines = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(vec![root]);
        while let Some(path) = queue.pop_front() {
            let pkg = path.last().unwrap();
            let depth = path.len() as i32;
            // generate lines
            let status = &self.pkg_status.borrow_mut()[&path];
            if (status.compat.is_changed() || status.latest.is_changed())
                && (options.flag_packages.is_empty()
                    || options.flag_packages.contains(&pkg.name().to_string()))
            {
                // name version compatible latest kind platform
                let parent = path.get(path.len() - 2);
                if let Some(parent) = parent {
                    let dependency = &self.pkg_deps[parent][pkg];
                    let label = if self.workspace_mode
                        || parent == &self.workspace.current()?.package_id()
                    {
                        pkg.name().to_string()
                    } else {
                        format!("{}->{}", self.pkgs[parent].name(), pkg.name())
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
                            .map(|p| p.to_string())
                            .unwrap_or_else(|| "---".to_owned())
                    );
                    lines.insert(line);
                } else {
                    let line = format!(
                        "{}\t{}\t{}\t{}\t---\t---\n",
                        pkg.name(),
                        pkg.version(),
                        status.compat.to_string(),
                        status.latest.to_string()
                    );
                    lines.insert(line);
                }
            }
            // next layer
            if options.flag_depth.is_none() || &depth < options.flag_depth.as_ref().unwrap() {
                self.pkg_deps[pkg]
                    .keys()
                    .filter(|dep| !path.contains(dep))
                    .filter(|dep| {
                        !self.workspace_mode
                            || !self.workspace.members().any(|mem| &mem.package_id() == dep)
                    })
                    .for_each(|dep| {
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
            write!(&mut tw, "Name\tProject\tCompat\tLatest\tKind\tPlatform\n")?;
            write!(&mut tw, "----\t-------\t------\t------\t----\t--------\n")?;
            for line in &lines {
                write!(&mut tw, "{}", line)?;
            }
            tw.flush()?;
            write!(
                io::stdout(),
                "{}",
                String::from_utf8(tw.into_inner().unwrap()).unwrap()
            )?;
            io::stdout().flush()?;
        }

        Ok(lines.len() as i32)
    }
}
