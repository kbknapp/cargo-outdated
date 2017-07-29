use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::collections::HashMap;
use super::lockfile::Lockfile;

type PackageCell = RefCell<Package>;

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub compatible: Option<String>,
    pub latest: Option<String>,
    pub dependencies: Option<HashMap<String, Weak<PackageCell>>>,
}

impl Package {
    pub fn new(name: &str, version: &str) -> Package {
        Package {
            name: name.to_owned(),
            version: version.to_owned(),
            compatible: None,
            latest: None,
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
            Some(r) => if r == &lockfile.root.name {
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

    pub fn print_to_vec(&self) -> Vec<String> {
        let mut lines = vec![];
        let root = self.root.upgrade().unwrap();
        if let Some(ref deps) = root.borrow().dependencies {
            for (_, dep_pac) in deps {
                Self::print(dep_pac.upgrade().unwrap(), "", &mut lines);
            }
        }

        lines
    }

    fn print(root: Rc<PackageCell>, parent: &str, lines: &mut Vec<String>) {
        let name = if parent.len() > 0 {
            format!("{}->{}", parent, root.borrow().name)
        } else {
            root.borrow().name.clone()
        };
        let compatible = match root.borrow().compatible {
            Some(ref v) if v == &root.borrow().version => None,
            Some(ref v) => Some(v.clone()),
            None => Some("  RM  ".to_owned()),
        };
        let latest = match root.borrow().latest {
            Some(ref v) if v == &root.borrow().version => None,
            Some(ref v) => Some(v.clone()),
            None => Some("  RM  ".to_owned()),
        };
        if compatible.is_some() || latest.is_some() {
            lines.push(format!(
                "{}\t   {}\t   {}\t  {}\n",
                name,
                root.borrow().version,
                compatible.unwrap_or_else(|| "  --  ".to_owned()),
                latest.unwrap_or_else(|| "  --  ".to_owned())
            ));
        }
        if let Some(ref deps) = root.borrow().dependencies {
            for (_, dep_pac) in deps {
                Self::print(
                    dep_pac.upgrade().unwrap(),
                    root.borrow().name.as_str(),
                    lines,
                );
            }
        }
    }

    pub fn merge_compatible_from(&self, from: &DependencyTree) {
        let to_root = self.root.upgrade().unwrap();
        let from_root = from.root.upgrade().unwrap();
        Self::merge(to_root, from_root, true);
    }

    pub fn merge_latest_from(&self, from: &DependencyTree) {
        let to_root = self.root.upgrade().unwrap();
        let from_root = from.root.upgrade().unwrap();
        Self::merge(to_root, from_root, false);
    }

    fn merge(to: Rc<PackageCell>, from: Rc<PackageCell>, compatible_or_latest: bool) {
        if compatible_or_latest {
            to.borrow_mut().compatible = Some(from.borrow().version.clone());
        } else {
            to.borrow_mut().latest = Some(from.borrow().version.clone());
        }
        if !from.borrow().dependencies.is_some() {
            return;
        }
        if let Some(ref deps) = to.borrow().dependencies {
            for (dep_name, dep_pac) in deps {
                match from.borrow().dependencies.as_ref() {
                    Some(from_deps) => if from_deps.contains_key(dep_name) {
                        let next_from = from_deps.get(dep_name).unwrap().upgrade().unwrap();
                        Self::merge(dep_pac.upgrade().unwrap(), next_from, compatible_or_latest);
                    },
                    None => {}
                }
            }
        }
    }

    fn find_root(root: &str, dependencies: &Option<Vec<String>>) -> String {
        if let &Some(ref deps) = dependencies {
            for d in deps {
                let splits_vec: Vec<_> = d.split(' ').collect();
                if splits_vec.len() > 1 && root == *splits_vec.get(0).unwrap() {
                    return format!("{} {}", root, splits_vec.get(1).unwrap());
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
                                let next_pac = format!(
                                    "{} {}",
                                    splits_vec.get(0).unwrap(),
                                    splits_vec.get(1).unwrap()
                                );
                                match package.dependencies.as_mut() {
                                    Some(map) => {
                                        let _ = map.insert(
                                            splits_vec.get(0).unwrap().to_string(),
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
