use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub description: String,
    pub repository: String,
    pub manager: String,
    pub icon: Option<String>,
    // Nombre para mostrar cuando el identificador no es legible (p. ej. Flatpak).
    pub title: Option<String>,
}