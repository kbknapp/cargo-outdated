use std::collections::HashMap;

use toml::{Value, Table};

use deps::RawDep;
use deps::Dep;

use CliResult;

pub struct Lockfile {
    pub deps: HashMap<String, RawDep>
}

impl Lockfile {

    pub fn new() -> Self {
        Lockfile { deps: HashMap::new() }
    }

    pub fn get_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let root_table = match table.get("root") {
            Some(table) => table,
            None        => {
                return Err(String::from("couldn't find '[root]' table in Cargo.lock"));
            }
        };

        match root_table.lookup("dependencies") {
            Some(&Value::Array(ref val)) => {
                debugln!("found root deps table");

                for v in val {
                    let val_str = v.as_str().unwrap_or("");
                    debugln!("adding root dep {}", val_str);
                    let mut raw_dep: RawDep = try!(val_str.parse());
                    raw_dep.is_root = true;
                    self.deps.insert(raw_dep.name.clone(), raw_dep);
                }
            },
            Some(_) => unreachable!(),
            None => return Err(String::from("No root dependencies"))
        };

        debugln!("Root deps: {:?}", self.deps);
        Ok(())
    }

    pub fn get_non_root_deps(&mut self, table: &Table) -> CliResult<()> {
        let arr = match table.get("package") {
            Some(&Value::Array(ref val)) => {
                debugln!("found non root deps");

                for v in val {
                    let name_str = v.lookup("name").unwrap().as_str().unwrap_or("");
                    let ver_str = v.lookup("version").unwrap().as_str().unwrap_or("");
                    match v.lookup("dependencies") {
                        Some(&Value::Array(ref deps)) => {
                            for d in deps {
                                let dep_str = d.as_str().unwrap_or("");
                                debugln!("adding non root dep {}", dep_str);
                                let raw_dep: RawDep = try!(dep_str.parse());
                                self.deps.insert(raw_dep.name.clone(), raw_dep);
                            }
                        },
                        Some(..) => unreachable!(),
                        None => ()
                    }
                }
            },
            Some(_) => unreachable!(),
            None => return Err(String::from("No non root dependencies"))
        };

        debugln!("All deps: {:#?}", self.deps);
        Ok(())
    }

    pub fn get_semver_diff<'a, I>(&self, deps: I) -> Option<Vec<Dep>> where I: Iterator<Item=&'a RawDep> {
        let mut res = vec![];
        for dep in deps {
            if let Some(ref old_dep) = self.deps.get(&dep.name) {
                if old_dep.ver != dep.ver { res.push(Dep {
                    name: dep.name.clone(),
                    raw_ver: None,
                    current_ver: Some(old_dep.ver.clone()),
                    possible_ver: Some(dep.ver.clone()),
                    latest_ver: None,
                })}
            }
        }
        if res.is_empty() {return None}
        Some(res)
    }
}