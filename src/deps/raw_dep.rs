use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct RawDep {
    pub name: String,
    pub ver: String,
    pub is_root: bool,
    depth: u8
}

impl FromStr for RawDep {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
       let raw_dep_vec: Vec<_> = s.split(" ").collect();
       if raw_dep_vec.len() < 2 {
           return Err(format!("failed to parse dependency string '{}'", s))
       }
       Ok(RawDep {
           name: raw_dep_vec[0].to_owned(),
           ver: raw_dep_vec[1].to_owned(),
           is_root: false,
           depth: 1
       })
    }
}