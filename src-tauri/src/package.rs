use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub manager: String,
}