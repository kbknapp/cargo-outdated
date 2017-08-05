use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::collections::HashMap;
use config::Config;
use super::lockfile::Lockfile;

type PackageCell = RefCell<Package>;

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub dependencies: Option<HashMap<String, Weak<PackageCell>>>,
}

impl Package {
    pub fn new(name: &str, version: &str) -> Package {
        Package {
            name: name.to_owned(),
            version: version.to_owned(),
            dependencies: None,
        }
    }
}

#[derive(Debug)]
pub struct DependencyTree {
    pub root: Weak<PackageCell>,
    pub packages: HashMap<String, Rc<PackageCell>>,
}

impl DependencyTree {
    pub fn from_lockfile(
        lockfile: &mut Lockfile,
        root: Option<&str>,
        depth: i32,
    ) -> DependencyTree {
        lockfile
            .package
            .as_mut()
            .unwrap()
            .push(lockfile.root.clone());
        let root_package_nv = match root {
            Some(r) => if r == lockfile.root.name {
                format!("{} {}", r, lockfile.root.version)
            } else {
                Self::find_root(r, &lockfile.root.dependencies)
            },
            None => lockfile.root.name.clone() + " " + &lockfile.root.version,
        };

        let packages = Rc::new(RefCell::new(HashMap::new()));
        let root_package = Self::generate_tree(&root_package_nv, lockfile, packages.clone(), depth);
        DependencyTree {
            root: root_package,
            packages: Rc::try_unwrap(packages).unwrap().into_inner(),
        }
    }

    pub fn print_list_to_vec(
        tree_curr: &DependencyTree,
        tree_comp: &DependencyTree,
        tree_latest: &DependencyTree,
        cfg: &Config,
    ) -> Vec<String> {
        let mut lines = vec![];
        let root_curr = tree_curr.root.upgrade().unwrap();
        let root_comp = tree_comp.root.upgrade().unwrap();
        let root_latest = tree_latest.root.upgrade().unwrap();
        Self::print_list_to_vec_recursive(
            root_curr,
            Some(root_comp),
            Some(root_latest),
            "",
            &mut lines,
            true,
            cfg,
        );
        lines.sort();
        lines.dedup();
        lines
    }

    fn print_list_to_vec_recursive(
        curr: Rc<PackageCell>,
        comp: Option<Rc<PackageCell>>,
        latest: Option<Rc<PackageCell>>,
        parent: &str,
        lines: &mut Vec<String>,
        curr_is_root: bool,
        cfg: &Config,
    ) {
        if cfg.to_update.is_none() ||
            cfg.to_update
                .as_ref()
                .unwrap()
                .contains(&curr.borrow().name.as_str())
        {
            let name = if !(curr_is_root || parent.is_empty()) {
                format!("{}->{}", parent, curr.borrow().name)
            } else {
                curr.borrow().name.clone()
            };
            let updated_version = |updated: &Option<Rc<PackageCell>>| -> Option<String> {
                match *updated {
                    Some(ref pac) if curr.borrow().version != pac.borrow().version => {
                        Some(pac.borrow().version.clone())
                    }
                    Some(_) => None,
                    None => Some("  RM  ".to_owned()),
                }
            };
            let comp_ver = updated_version(&comp);
            let latest_ver = updated_version(&latest);

            if comp_ver.is_some() || latest_ver.is_some() {
                lines.push(format!(
                    "{}\t   {}\t   {}\t  {}\n",
                    name,
                    curr.borrow().version,
                    comp_ver.unwrap_or_else(|| "  --  ".to_owned()),
                    latest_ver.unwrap_or_else(|| "  --  ".to_owned())
                ));
            }
        }

        fn next_node(name: &str, next: &Option<Rc<PackageCell>>) -> Option<Rc<PackageCell>> {
            match *next {
                Some(ref pac) => if pac.borrow().dependencies.is_some() {
                    pac.borrow()
                        .dependencies
                        .as_ref()
                        .unwrap()
                        .get(name)
                        .map(|v| v.upgrade().unwrap())
                } else {
                    None
                },
                None => None,
            }
        }
        if let Some(ref deps) = curr.borrow().dependencies {
            for next_curr in deps.values() {
                let next_curr = next_curr.upgrade().unwrap();
                let next_comp = next_node(&next_curr.borrow().name, &comp);
                let next_latest = next_node(&next_curr.borrow().name, &latest);
                Self::print_list_to_vec_recursive(
                    next_curr,
                    next_comp,
                    next_latest,
                    &if curr_is_root {
                        "".to_owned()
                    } else {
                        curr.borrow().name.clone()
                    },
                    lines,
                    false,
                    cfg,
                );
            }
        }
    }

    fn find_root(root: &str, dependencies: &Option<Vec<String>>) -> String {
        if let Some(ref deps) = *dependencies {
            for d in deps {
                let splits_vec: Vec<_> = d.split(' ').collect();
                if splits_vec.len() > 1 && root == splits_vec[0] {
                    return format!("{} {}", root, splits_vec[1]);
                }
            }
        }
        panic!("Root is neither the package itself nor a direct dependency");
    }

    fn generate_tree(
        root: &str,
        lockfile: &Lockfile,
        packages: Rc<RefCell<HashMap<String, Rc<PackageCell>>>>,
        depth: i32,
    ) -> Weak<PackageCell> {
        if packages.borrow().contains_key(root) {
            return Rc::downgrade(packages.borrow().get(root).unwrap());
        }
        for raw_pac in lockfile.package.as_ref().unwrap() {
            if root == raw_pac.name.clone() + " " + &raw_pac.version {
                let mut package = Package::new(&raw_pac.name, &raw_pac.version);
                if depth != 0 {
                    if let Some(ref deps) = raw_pac.dependencies {
                        package.dependencies = Some(HashMap::new());
                        for d in deps {
                            let splits_vec: Vec<_> = d.split(' ').collect();
                            if splits_vec.len() > 1 {
                                let next_pac = format!("{} {}", splits_vec[0], splits_vec[1]);
                                match package.dependencies.as_mut() {
                                    Some(map) => {
                                        let _ = map.insert(
                                            splits_vec[0].to_string(),
                                            Self::generate_tree(
                                                &next_pac,
                                                lockfile,
                                                packages.clone(),
                                                depth - 1,
                                            ),
                                        );
                                    }
                                    _ => unreachable!(),
                                }
                            }
                        }
                    }
                }
                packages
                    .borrow_mut()
                    .insert(root.to_owned(), Rc::new(RefCell::new(package)));
                return Rc::downgrade(packages.borrow().get(root).unwrap());
            }
        }
        panic!("Cannot find package {}", root);
    }
}
