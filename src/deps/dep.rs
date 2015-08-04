use semver::Version;

pub struct Dep {
    pub name: String,
    pub raw_ver: Option<String>,
    pub current_ver: Option<String>,
    pub possible_ver: Option<String>,
    pub latest_ver: Option<String>,
}