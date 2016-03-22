use std::str::FromStr;

#[derive(Clone, Debug, PartialEq)]
pub struct RawDep {
    pub name: String,
    pub ver: String,
    pub source: String,
    pub children: Option<Vec<String>>,
    pub parent: Option<String>,
}

impl FromStr for RawDep {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let raw_dep_vec: Vec<_> = s.split(' ').collect();
        if raw_dep_vec.len() < 2 {
            return Err(format!("failed to parse dependency string '{}'", s));
        }
        Ok(RawDep {
            name: raw_dep_vec[0].to_owned(),
            ver: raw_dep_vec[1].to_owned(),
            source: raw_dep_vec.get(2).map_or(String::new(), |v| (*v).to_owned()),
            children: None,
            parent: None,
        })
    }
}
