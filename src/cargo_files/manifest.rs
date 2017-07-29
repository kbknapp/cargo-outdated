use toml::value::Table;

#[derive(Debug, Serialize, Deserialize)]
pub struct Manifest {
    pub package: Table,
    pub dependencies: Table,
    pub bin: Option<Vec<Table>>,
}
