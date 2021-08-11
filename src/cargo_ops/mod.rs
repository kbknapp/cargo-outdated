use super::Options;
use toml::value::{Table, Value};

mod elaborate_workspace;
mod pkg_status;
mod temp_project;
pub use self::elaborate_workspace::ElaborateWorkspace;
pub use self::pkg_status::*;
pub use self::temp_project::TempProject;

/// A continent struct for quick parsing and manipulating manifest
#[derive(Debug, serde_derive::Serialize, serde_derive::Deserialize)]
struct Manifest {
    #[serde(
        rename = "cargo-features",
        skip_serializing_if = "Option::is_none"
    )]
    pub cargo_features: Option<Value>,
    #[serde(serialize_with = "::toml::ser::tables_last")]
    pub package: Table,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_tables_last"
    )]
    pub dependencies: Option<Table>,
    #[serde(
        rename = "dev-dependencies",
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_tables_last"
    )]
    pub dev_dependencies: Option<Table>,
    #[serde(
        rename = "build-dependencies",
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_tables_last"
    )]
    pub build_dependencies: Option<Table>,
    pub lib: Option<Table>,
    pub bin: Option<Vec<Table>>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_tables_last"
    )]
    pub workspace: Option<Table>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "opt_tables_last"
    )]
    pub target: Option<Table>,
    pub features: Option<Value>,
}

impl Manifest {
    pub fn name(&self) -> String {
        match self.package["name"] {
            Value::String(ref name) => name.clone(),
            _ => unreachable!(),
        }
    }
}

pub fn opt_tables_last<S>(data: &Option<Table>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: ::serde::ser::Serializer,
{
    match *data {
        Some(ref d) => ::toml::ser::tables_last(d, serializer),
        None => unreachable!(),
    }
}
