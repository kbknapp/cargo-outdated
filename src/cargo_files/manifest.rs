use toml::value::Table;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub package: Table,
    #[serde(serialize_with = "::toml::ser::tables_last")]
    pub dependencies: Table,
    pub bin: Option<Vec<Table>>,
}
