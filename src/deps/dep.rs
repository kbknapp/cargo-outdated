#[derive(Debug)]
pub struct Dep {
    pub name: String,
    pub source: String,
    pub project_ver: String,
    pub semver_ver: Option<String>,
    pub latest_ver: Option<String>,
}
