use super::Options;
use toml::value::{Table, Value};

mod elaborate_workspace;
mod pkg_status;
mod temp_project;
pub use self::{elaborate_workspace::ElaborateWorkspace, pkg_status::*, temp_project::TempProject};

/// A continent struct for quick parsing and manipulating manifest
#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct Manifest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_features: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_dependencies: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lib: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bin: Option<Vec<Table>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<Table>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<Value>,
}

impl Manifest {
    pub fn name(&self) -> Option<String> {
        match self.package.as_ref()?["name"] {
            Value::String(ref name) => Some(name.clone()),
            _ => unreachable!(),
        }
    }
}
