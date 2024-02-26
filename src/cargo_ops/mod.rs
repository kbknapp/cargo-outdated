use super::Options;
use toml::value::{Table, Value};

mod elaborate_workspace;
mod pkg_status;
mod temp_project;
pub use self::{elaborate_workspace::ElaborateWorkspace, temp_project::TempProject};

/// A continent struct for quick parsing and manipulating manifest
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Manifest {
    #[serde(rename = "cargo-features", skip_serializing_if = "Option::is_none")]
    pub cargo_features: Option<Value>,
    pub package: Table,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Table>,
    #[serde(rename = "dev-dependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Table>,
    #[serde(rename = "build-dependencies", skip_serializing_if = "Option::is_none")]
    pub build_dependencies: Option<Table>,
    pub lib: Option<Table>,
    pub bin: Option<Vec<Table>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Table>,
    pub features: Option<Value>,
    #[serde(rename = "patch", skip_serializing_if = "Option::is_none")]
    pub patch: Option<Table>,
}

impl Manifest {
    pub fn name(&self) -> String {
        match self.package["name"] {
            Value::String(ref name) => name.clone(),
            _ => unreachable!(),
        }
    }
}
