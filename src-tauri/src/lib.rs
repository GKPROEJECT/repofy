mod package;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use package::Package;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_ICON_SIZE: u64 = 512 * 1024;
const ICON_EXTENSIONS: [&str; 4] = ["png", "svg", "webp", "jpg"];
const ICON_SIZES: [&str; 8] = [
    "scalable", "512x512", "256x256", "128x128", "96x96", "64x64", "48x48", "32x32",
];

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

fn package_icon(package_name: &str) -> Option<String> {
    if package_name.is_empty()
        || !package_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character))
    {
        return None;
    }

    let icon_name = find_desktop_icon_name(package_name)?;
    let icon_path = find_icon_path(&icon_name)?;
    let metadata = fs::metadata(&icon_path).ok()?;

    if metadata.len() > MAX_ICON_SIZE {
        return None;
    }

    let contents = fs::read(&icon_path).ok()?;
    let extension = icon_path.extension()?.to_str()?.to_ascii_lowercase();
    let mime_type = match extension.as_str() {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "jpg" => "image/jpeg",
        _ => return None,
    };

    Some(format!(
        "data:{};base64,{}",
        mime_type,
        BASE64.encode(contents)
    ))
}

fn find_desktop_icon_name(package_name: &str) -> Option<String> {
    let mut application_directories = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];

    if let Some(home) = std::env::var_os("HOME") {
        application_directories.push(PathBuf::from(home).join(".local/share/applications"));
    }

    for directory in application_directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("desktop")
                || !path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.eq_ignore_ascii_case(package_name))
            {
                continue;
            }

            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };
            if let Some(icon_name) = content
                .lines()
                .find_map(|line| line.strip_prefix("Icon="))
                .map(str::trim)
                .filter(|name| !name.is_empty())
            {
                return Some(icon_name.to_string());
            }
        }
    }

    None
}

fn find_icon_path(icon_name: &str) -> Option<PathBuf> {
    let icon_path = Path::new(icon_name);
    if icon_path.is_absolute() {
        return icon_path.is_file().then(|| icon_path.to_path_buf());
    }

    let icon_name = if icon_path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| ICON_EXTENSIONS.contains(&extension))
    {
        icon_path.file_stem()?.to_str()?
    } else {
        icon_name
    };

    let mut icon_roots = vec![
        PathBuf::from("/usr/share/icons"),
        PathBuf::from("/usr/local/share/icons"),
        PathBuf::from("/usr/share/pixmaps"),
        PathBuf::from("/usr/local/share/pixmaps"),
    ];

    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        icon_roots.push(home.join(".local/share/icons"));
        icon_roots.push(home.join(".local/share/pixmaps"));
    }

    for root in &icon_roots {
        for extension in ICON_EXTENSIONS {
            let candidate = root.join(format!("{icon_name}.{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    for root in icon_roots.iter().filter(|root| root.ends_with("icons")) {
        let Ok(themes) = fs::read_dir(root) else {
            continue;
        };
        for theme in themes.flatten() {
            for size in ICON_SIZES {
                for extension in ICON_EXTENSIONS {
                    let candidate = theme
                        .path()
                        .join(size)
                        .join("apps")
                        .join(format!("{icon_name}.{extension}"));
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }

    None
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
                    icon: package_icon(parts[0]),
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