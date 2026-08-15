mod package;

use package::Package;
use std::process::Command;

#[derive(serde::Serialize)]
struct SystemInfo {
    distribution: String,
    family: String,
    architecture: String,
    package_managers: Vec<String>,
}

fn command_exists(command: &str) -> bool {
    Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", command))
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut distribution = String::from("Linux");
    let mut family = String::from("Linux");

    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                distribution = value.trim_matches('"').to_string();
            }

            if let Some(value) = line.strip_prefix("ID_LIKE=") {
                family = value.trim_matches('"').to_string();
            }

            if let Some(value) = line.strip_prefix("ID=") {
                if family == "Linux" {
                    family = value.trim_matches('"').to_string();
                }
            }
        }
    }

    let architecture = std::env::consts::ARCH.to_string();

    let possible_managers = [
        "pacman",
        "yay",
        "paru",
        "flatpak",
        "apt",
        "dnf",
        "zypper",
        "apk",
        "snap",
    ];

    let package_managers = possible_managers
        .iter()
        .filter(|manager| command_exists(manager))
        .map(|manager| manager.to_string())
        .collect();

    SystemInfo {
        distribution,
        family,
        architecture,
        package_managers,
    }
}

#[tauri::command]
fn search_pacman(query: String) -> Result<Vec<Package>, String> {
    search_pacman_internal(&query)
}

fn search_pacman_internal(query: &str) -> Result<Vec<Package>, String> {
    let output = Command::new("pacman")
        .args(["-Ss", query])
        .output()
        .map_err(|error| format!("No se pudo ejecutar pacman: {}", error))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }

    Ok(parse_pacman_search_output(
        &String::from_utf8_lossy(&output.stdout),
    ))
}

fn parse_pacman_search_output(stdout: &str) -> Vec<Package> {
    let mut packages = Vec::new();

    let mut lines = stdout.lines();

    while let Some(package_line) = lines.next() {
        let package_line = package_line.trim();

        if package_line.is_empty() {
            continue;
        }

        if let Some((repository, name_and_version)) =
            package_line.split_once('/')
        {
            let parts: Vec<&str> =
                name_and_version.split_whitespace().collect();

            if parts.len() >= 2 {
                let description = lines
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();

                packages.push(Package {
                    repository: repository.to_string(),
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    description,
                    manager: "pacman".to_string(),
                });
            }
        }
    }

    packages
}

/// Comprueba si un paquete está instalado.
#[tauri::command]
fn is_package_installed(package_name: String) -> bool {
    Command::new("pacman")
        .args(["-Q", package_name.trim()])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Comprueba que el nombre del paquete tenga un formato válido.
fn is_valid_package_name(package_name: &str) -> bool {
    if package_name.is_empty() || package_name.len() > 100 {
        return false;
    }

    package_name.chars().all(|character| {
        character.is_ascii_alphanumeric()
            || character == '-'
            || character == '_'
            || character == '.'
            || character == '+'
    })
}

/// Instala un paquete mediante pacman utilizando pkexec.
#[tauri::command]
fn install_package(package_name: String) -> Result<String, String> {
    let package_name = package_name.trim();

    if !is_valid_package_name(package_name) {
        return Err("El nombre del paquete no es válido.".to_string());
    }

    // Comprobamos si ya está instalado.
    let already_installed = Command::new("pacman")
        .args(["-Q", package_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if already_installed {
        return Ok(format!(
            "El paquete {} ya está instalado.",
            package_name
        ));
    }

    // Comprobamos que pkexec esté disponible.
    if !command_exists("pkexec") {
        return Err(
            "No se encontró pkexec en el sistema. \
             Instala polkit para poder realizar instalaciones gráficas."
                .to_string(),
        );
    }

    let output = Command::new("pkexec")
        .arg("pacman")
        .args(["-S", "--noconfirm", package_name])
        .output()
        .map_err(|error| {
            format!("No se pudo iniciar la instalación: {}", error)
        })?;

    if output.status.success() {
        Ok(format!(
            "El paquete {} se ha instalado correctamente.",
            package_name
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string();

        let stdout = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        if !stderr.is_empty() {
            Err(stderr)
        } else if !stdout.is_empty() {
            Err(stdout)
        } else {
            Err(format!(
                "La instalación de {} no se completó.",
                package_name
            ))
        }
    }
}

/// Desinstala un paquete mediante pacman utilizando pkexec.
#[tauri::command]
fn remove_package(package_name: String) -> Result<String, String> {
    let package_name = package_name.trim();

    if !is_valid_package_name(package_name) {
        return Err("El nombre del paquete no es válido.".to_string());
    }

    // Comprobamos que el paquete esté instalado.
    let installed = Command::new("pacman")
        .args(["-Q", package_name])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !installed {
        return Ok(format!(
            "El paquete {} no está instalado.",
            package_name
        ));
    }

    // Comprobamos que pkexec esté disponible.
    if !command_exists("pkexec") {
        return Err(
            "No se encontró pkexec en el sistema. \
             Instala polkit para poder realizar desinstalaciones gráficas."
                .to_string(),
        );
    }

    let output = Command::new("pkexec")
        .arg("pacman")
        .args(["-R", "--noconfirm", package_name])
        .output()
        .map_err(|error| {
            format!("No se pudo iniciar la desinstalación: {}", error)
        })?;

    if output.status.success() {
        Ok(format!(
            "El paquete {} se ha desinstalado correctamente.",
            package_name
        ))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_string();

        let stdout = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        if !stderr.is_empty() {
            Err(stderr)
        } else if !stdout.is_empty() {
            Err(stdout)
        } else {
            Err(format!(
                "La desinstalación de {} no se completó.",
                package_name
            ))
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            search_pacman,
            is_package_installed,
            install_package,
            remove_package
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}