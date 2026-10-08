mod package;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use package::Package;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

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
    managers: Vec<ManagerInfo>,
}

#[derive(serde::Serialize)]
struct ManagerInfo {
    name: String,
    installed: bool,
    // Se puede instalar desde los repositorios del sistema con pacman.
    installable: bool,
}

/// Gestores que Repofy puede ofrecer instalar desde los repositorios de pacman.
const INSTALLABLE_MANAGERS: [&str; 4] = ["paru", "yay", "flatpak", "snap"];

/// Nombre del paquete que instala cada gestor (snap se instala con snapd).
fn manager_package(manager: &str) -> &str {
    if manager == "snap" {
        "snapd"
    } else {
        manager
    }
}

/// Existe en el AUR y hay un ayudante (paru o yay) para instalarlo.
fn is_in_aur(package: &str) -> bool {
    aur_helper().is_some_and(|helper| {
        Command::new(helper)
            .args(["-Si", package])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    })
}

fn is_in_pacman_repos(package: &str) -> bool {
    Command::new("pacman")
        .args(["-Si", package])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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

    let has_pacman = command_exists("pacman");
    let mut managers: Vec<ManagerInfo> = Vec::new();

    for name in possible_managers {
        let installed = command_exists(name);
        let offered = INSTALLABLE_MANAGERS.contains(&name) && has_pacman;

        if installed || offered {
            managers.push(ManagerInfo {
                name: name.to_string(),
                installed,
                installable: !installed && {
                    let package = manager_package(name);
                    is_in_pacman_repos(package) || is_in_aur(package)
                },
            });
        }
    }
    // En Arch, pacman siempre se muestra primero.
    managers.sort_by_key(|manager| manager.name != "pacman");

    SystemInfo {
        distribution,
        family,
        architecture,
        package_managers,
        managers,
    }
}

/// Busca en todos los gestores disponibles: pacman, AUR, Flatpak y Snap.
#[tauri::command]
async fn search_all(query: String) -> Result<Vec<Package>, String> {
    run_blocking(move || search_all_blocking(&query)).await
}

const SOURCE_RESULT_LIMIT: usize = 10;

fn search_all_blocking(query: &str) -> Result<Vec<Package>, String> {
    let query = query.trim();
    if query.is_empty() || query.starts_with('-') {
        return Ok(Vec::new());
    }

    std::thread::scope(|scope| {
        let aur = scope.spawn(|| search_aur(query));
        let flatpak = scope.spawn(|| search_flatpak(query));
        let snap = scope.spawn(|| search_snap(query));

        let mut packages = if command_exists("pacman") {
            search_pacman_internal(query)?
        } else {
            Vec::new()
        };

        let flatpak = flatpak.join().unwrap_or_default();
        let snap = snap.join().unwrap_or_default();
        let known: HashSet<String> = packages.iter().map(|package| package.name.clone()).collect();
        let others_empty = packages.is_empty() && flatpak.is_empty() && snap.is_empty();

        packages.extend(select_aur(
            aur.join().unwrap_or_default(),
            query,
            others_empty,
            &known,
        ));
        packages.extend(flatpak);
        packages.extend(snap);

        Ok(packages)
    })
}

const AUR_VARIANT_SUFFIXES: [&str; 10] = [
    "bin", "git", "appimage", "beta", "nightly", "dev", "stable", "preview", "wayland", "electron",
];

/// El AUR tiene miles de paquetes auxiliares: solo se muestra el programa buscado y sus
/// variantes (-bin, -git...). Si no hay nada más, también los nombres que empiezan igual.
fn select_aur(
    candidates: Vec<Package>,
    query: &str,
    show_partial: bool,
    known: &HashSet<String>,
) -> Vec<Package> {
    let query = query.trim().to_ascii_lowercase();

    let mut selected: Vec<Package> = candidates
        .into_iter()
        .filter(|package| !known.contains(&package.name))
        .filter(|package| {
            let name = package.name.to_ascii_lowercase();
            let is_variant = name == query
                || name
                    .strip_prefix(&format!("{query}-"))
                    .is_some_and(|rest| AUR_VARIANT_SUFFIXES.contains(&rest));
            is_variant || (show_partial && name.starts_with(&query))
        })
        .collect();

    selected.truncate(SOURCE_RESULT_LIMIT);
    selected
}

/// Paquetes del AUR (paru o yay). Solo se admiten búsquedas sencillas porque el AUR no usa regex.
fn search_aur(query: &str) -> Vec<Package> {
    let Some(helper) = aur_helper() else {
        return Vec::new();
    };

    if !query
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
    {
        return Vec::new();
    }

    let Ok(output) = Command::new(helper)
        .arg("-Ssa")
        .args(query.split_whitespace())
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let packages = parse_pacman_search_output(&String::from_utf8_lossy(&output.stdout), query, None);
    let mut packages = filter_by_name(packages, query);
    packages.truncate(50);

    for package in &mut packages {
        package.manager = "aur".to_string();
        package.repository = "aur".to_string();
    }

    packages
}

const FLATPAK_NON_APP_MARKERS: [&str; 15] = [
    ".Locale", ".Debug", ".Sdk", ".Platform", ".Utility.", ".Plugin.", ".Extension", ".Gtk3theme",
    ".Gtk4theme", ".GtkTheme", ".KStyle", ".Icontheme", ".Codecs", ".VAAPI", ".Addon",
];

/// Carpetas `appstream/<remoto>/<arquitectura>/active` de Flatpak (sistema y usuario).
fn flatpak_appstream_dirs() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/var/lib/flatpak/appstream")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/flatpak/appstream"));
    }

    let mut directories = Vec::new();
    for root in roots {
        let Ok(remotes) = fs::read_dir(root) else {
            continue;
        };
        for remote in remotes.flatten() {
            let active = remote.path().join(std::env::consts::ARCH).join("active");
            if active.is_dir() {
                directories.push(active);
            }
        }
    }
    directories
}

fn flatpak_icon(app_id: &str) -> Option<String> {
    for directory in flatpak_appstream_dirs() {
        for size in ["128x128", "64x64"] {
            let path = directory.join("icons").join(size).join(format!("{app_id}.png"));
            if path.is_file() {
                return package_icon(path.to_str()?);
            }
        }
    }
    None
}

fn search_flatpak(query: &str) -> Vec<Package> {
    if !command_exists("flatpak") {
        return Vec::new();
    }

    let Ok(output) = Command::new("flatpak")
        .args(["search", "--columns=name,description,application,version"])
        .arg(query)
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    let mut packages: Vec<Package> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let columns: Vec<&str> = line.split('\t').collect();
            if columns.len() < 3 {
                return None;
            }

            let (title, description, id) = (columns[0].trim(), columns[1].trim(), columns[2].trim());
            if id.is_empty()
                || !is_valid_package_name(id)
                || FLATPAK_NON_APP_MARKERS.iter().any(|marker| id.contains(marker))
            {
                return None;
            }

            // Se busca en el título y en el último tramo del id, no en el nombre del autor.
            let last_segment = id.rsplit('.').next().unwrap_or(id);
            let haystack = format!("{} {}", title.to_lowercase(), last_segment.to_lowercase());
            if !terms.iter().all(|term| haystack.contains(term)) {
                return None;
            }

            Some(Package {
                name: id.to_string(),
                version: columns.get(3).map(|value| value.trim()).unwrap_or("").to_string(),
                description: description.to_string(),
                repository: "flathub".to_string(),
                manager: "flatpak".to_string(),
                icon: None,
                title: Some(title.to_string()),
            })
        })
        .collect();

    let rank = |package: &Package| {
        let title = package.title.as_deref().unwrap_or("").to_lowercase();
        if title == query_lower {
            0
        } else if title.starts_with(&query_lower) {
            1
        } else {
            2
        }
    };

    if packages.iter().any(|package| rank(package) == 0) {
        packages.retain(|package| rank(package) == 0);
    }
    packages.sort_by_key(rank);
    packages.truncate(SOURCE_RESULT_LIMIT);

    for package in &mut packages {
        package.icon = flatpak_icon(&package.name);
    }

    packages
}

/// Snap (requiere snapd en marcha). La salida es una tabla con columnas alineadas.
fn search_snap(query: &str) -> Vec<Package> {
    if !command_exists("snap") {
        return Vec::new();
    }

    let Ok(output) = Command::new("snap").arg("find").arg(query).output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut lines = stdout.lines();
    let Some(header) = lines.next() else {
        return Vec::new();
    };

    let (Some(version_at), Some(publisher_at), Some(notes_at), Some(summary_at)) = (
        header.find("Version"),
        header.find("Publisher"),
        header.find("Notes"),
        header.find("Summary"),
    ) else {
        return Vec::new();
    };

    let column = |line: &str, from: usize, to: Option<usize>| -> String {
        let chars = line.chars().skip(from);
        match to {
            Some(to) => chars.take(to.saturating_sub(from)).collect::<String>(),
            None => chars.collect::<String>(),
        }
        .trim()
        .to_string()
    };

    let packages: Vec<Package> = lines
        .filter_map(|line| {
            let name = column(line, 0, Some(version_at));
            if name.is_empty() || !is_valid_package_name(&name) {
                return None;
            }

            Some(Package {
                name,
                version: column(line, version_at, Some(publisher_at)),
                description: column(line, summary_at, None),
                repository: "snapcraft".to_string(),
                manager: "snap".to_string(),
                icon: None,
                title: None,
            })
        })
        .collect();

    let _ = notes_at;
    let mut packages = filter_by_name(packages, query);
    packages.truncate(SOURCE_RESULT_LIMIT);
    packages
}

/// Ejecuta trabajo bloqueante fuera del hilo de la interfaz para que no se congele.
async fn run_blocking<T, F>(task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|error| format!("Error interno: {}", error))?
}

const APP_INDEX_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Nombres de paquetes que instalan un lanzador .desktop. `None` si no se pudo construir.
static APP_INDEX: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn cache_directory() -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .map(|directory| directory.join("repofy"))
}

/// Devuelve los paquetes con lanzador, usando la caché si es reciente.
/// Bloquea mientras se construye, de modo que solo se construye una vez.
fn app_index() -> Option<HashSet<String>> {
    let mut index = APP_INDEX.lock().ok()?;

    if index.is_none() {
        *index = load_or_build_app_index();
    }

    index.clone()
}

fn load_or_build_app_index() -> Option<HashSet<String>> {
    let cache_file = cache_directory()?.join("apps.txt");

    let is_fresh = fs::metadata(&cache_file)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .is_some_and(|age| age < APP_INDEX_MAX_AGE);

    if is_fresh {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            let names: HashSet<String> = content.lines().map(str::to_string).collect();
            if !names.is_empty() {
                return Some(names);
            }
        }
    }

    match build_app_index() {
        Some(names) => {
            let mut sorted: Vec<&String> = names.iter().collect();
            sorted.sort();
            let content = sorted
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let _ = fs::write(&cache_file, content);
            Some(names)
        }
        // Si falla, se usa la caché antigua antes que no filtrar nada.
        None => fs::read_to_string(&cache_file)
            .ok()
            .map(|content| content.lines().map(str::to_string).collect())
            .filter(|names: &HashSet<String>| !names.is_empty()),
    }
}

/// Descarga las bases de datos de ficheros de pacman en una carpeta propia (sin root)
/// y lista los paquetes que instalan un archivo .desktop.
fn build_app_index() -> Option<HashSet<String>> {
    let directory = cache_directory()?.join("db");
    let sync_directory = directory.join("sync");

    fs::create_dir_all(&sync_directory).ok()?;

    let local_link = directory.join("local");
    if !local_link.exists() {
        std::os::unix::fs::symlink("/var/lib/pacman/local", &local_link).ok()?;
    }

    for entry in fs::read_dir("/var/lib/pacman/sync").ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("db") {
            fs::copy(&path, sync_directory.join(entry.file_name())).ok()?;
        }
    }

    let sync = Command::new("unshare")
        .args(["-r", "pacman", "-Fy", "--disable-sandbox", "--dbpath"])
        .arg(&directory)
        .output()
        .ok()?;

    if !sync.status.success() {
        return None;
    }

    let listing = Command::new("pacman")
        .args(["-Fx", "--machinereadable", "--dbpath"])
        .arg(&directory)
        .arg(r"^usr/share/applications/[^/]+\.desktop$")
        .output()
        .ok()?;

    let names: HashSet<String> = String::from_utf8_lossy(&listing.stdout)
        .lines()
        .filter_map(|line| line.split('\0').nth(1))
        .map(str::to_string)
        .collect();

    (!names.is_empty()).then_some(names)
}

/// Iconos y capturas de AppStream de Arch, incluso de programas sin instalar.
#[derive(Default)]
struct AppstreamData {
    icons: HashMap<String, PathBuf>,
    screenshots: HashMap<String, Vec<String>>,
}

static APPSTREAM: OnceLock<AppstreamData> = OnceLock::new();

/// Descarga (sin root) los datos de AppStream de Arch y los indexa. Se llama en segundo plano.
fn prepare_appstream_icons() {
    let _ = APPSTREAM.set(load_appstream_icons().unwrap_or_default());
}

fn load_appstream_icons() -> Option<AppstreamData> {
    let base = cache_directory()?;
    let data = base.join("appstream");

    if !data.join("usr/share/swcatalog/xml").is_dir() {
        let packages = base.join("pkg");
        fs::create_dir_all(&packages).ok()?;

        let download = Command::new("unshare")
            .args([
                "-r", "pacman", "-Sw", "--noconfirm", "--disable-sandbox", "--dbpath",
            ])
            .arg(base.join("db"))
            .arg("--cachedir")
            .arg(&packages)
            .arg("archlinux-appstream-data")
            .output()
            .ok()?;
        if !download.status.success() {
            return None;
        }

        let archive = fs::read_dir(&packages)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("archlinux-appstream-data-") && name.ends_with(".zst")
                    })
            })
            .max()?;

        fs::create_dir_all(&data).ok()?;
        let extracted = Command::new("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(&data)
            .output()
            .ok()?;
        if !extracted.status.success() {
            return None;
        }
    }

    let catalog = data.join("usr/share/swcatalog");
    let mut icons = HashMap::new();
    let mut screenshots: HashMap<String, Vec<String>> = HashMap::new();

    for origin in ["core", "extra", "multilib"] {
        let xml_path = catalog.join(format!("xml/{origin}.xml.gz"));
        let Ok(xml) = Command::new("gzip").arg("-dc").arg(&xml_path).output() else {
            continue;
        };
        let xml = String::from_utf8_lossy(&xml.stdout);
        let icon_directory = catalog.join(format!("icons/archlinux-arch-{origin}/64x64"));

        for component in xml.split("<component ").skip(1) {
            let Some(package) = tag_text(component, "<pkgname>", "</pkgname>") else {
                continue;
            };

            let shots = extract_screenshots(component);
            if !shots.is_empty() {
                let entry = screenshots.entry(package.to_string()).or_default();
                for url in shots {
                    if !entry.contains(&url) && entry.len() < MAX_SCREENSHOTS {
                        entry.push(url);
                    }
                }
            }

            let Some(start) = component.find("<icon type=\"cached\" width=\"64\"") else {
                continue;
            };
            let Some(file) = tag_text(&component[start..], ">", "</icon>") else {
                continue;
            };

            let path = icon_directory.join(file);
            if path.is_file() {
                icons.entry(package.to_string()).or_insert(path);
            }
        }
    }

    Some(AppstreamData { icons, screenshots })
}

const MAX_SCREENSHOTS: usize = 8;

/// Extrae las URL de capturas de un componente AppStream. Prefiere las miniaturas
/// más grandes (cargan antes) y usa la imagen original si no hay.
fn extract_screenshots(component: &str) -> Vec<String> {
    let Some(start) = component.find("<screenshots>") else {
        return Vec::new();
    };
    let end = component[start..]
        .find("</screenshots>")
        .map_or(component.len(), |offset| start + offset);

    let mut urls: Vec<String> = Vec::new();

    for screenshot in component[start..end].split("<screenshot").skip(1) {
        let mut best: Option<(u32, String)> = None;

        for image in screenshot.split("<image").skip(1) {
            let Some(tag_end) = image.find('>') else {
                continue;
            };
            let attributes = &image[..tag_end];
            let Some(url) = tag_text(&image[tag_end..], ">", "</image>") else {
                continue;
            };
            if !url.starts_with("https://") && !url.starts_with("http://") {
                continue;
            }

            let score = if attributes.contains("type=\"thumbnail\"") {
                attributes
                    .split("width=\"")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|width| width.parse::<u32>().ok())
                    .unwrap_or(1)
                    .max(1)
            } else {
                0
            };

            if best.as_ref().is_none_or(|(current, _)| score > *current) {
                best = Some((score, url.to_string()));
            }
        }

        if let Some((_, url)) = best {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
    }

    urls.truncate(MAX_SCREENSHOTS);
    urls
}

fn flatpak_screenshots(app_id: &str) -> Vec<String> {
    for directory in flatpak_appstream_dirs() {
        let Ok(bytes) = fs::read(directory.join("appstream.xml")) else {
            continue;
        };
        let xml = String::from_utf8_lossy(&bytes);

        let position = xml
            .find(&format!("<id>{app_id}</id>"))
            .or_else(|| xml.find(&format!("<id>{app_id}.desktop</id>")));

        if let Some(position) = position {
            let end = xml[position..]
                .find("</component>")
                .map_or(xml.len(), |offset| position + offset);
            let urls = extract_screenshots(&xml[position..end]);
            if !urls.is_empty() {
                return urls;
            }
        }
    }

    Vec::new()
}

fn snap_screenshots(name: &str) -> Vec<String> {
    let url = format!("https://api.snapcraft.io/v2/snaps/info/{name}?fields=media");
    let Ok(output) = Command::new("curl")
        .args(["-s", "-m", "8", "-H", "Snap-Device-Series: 16"])
        .arg(url)
        .output()
    else {
        return Vec::new();
    };

    let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return Vec::new();
    };

    json["snap"]["media"]
        .as_array()
        .map(|media| {
            media
                .iter()
                .filter(|item| item["type"] == "screenshot")
                .filter_map(|item| item["url"].as_str())
                .filter(|url| url.starts_with("https://"))
                .take(MAX_SCREENSHOTS)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Capturas de pantalla de un programa, según su gestor.
#[tauri::command]
async fn get_screenshots(package_name: String, manager: Option<String>) -> Vec<String> {
    run_blocking(move || {
        let manager = validated_manager(manager)?;
        let name = package_name.trim();
        if !is_valid_package_name(name) {
            return Ok(Vec::new());
        }

        Ok(match manager.as_str() {
            "flatpak" => flatpak_screenshots(name),
            "snap" => snap_screenshots(name),
            "pacman" => APPSTREAM
                .get()
                .and_then(|data| data.screenshots.get(name))
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        })
    })
    .await
    .unwrap_or_default()
}

fn tag_text<'a>(text: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = text.find(open)? + open.len();
    let end = text[start..].find(close)? + start;
    Some(text[start..end].trim())
}

/// Icono de un paquete según AppStream. Los .jxl se convierten a PNG (con djxl) y se cachean.
/// No bloquea: si los datos todavía se están preparando, no devuelve nada.
fn appstream_icon(package: &str) -> Option<String> {
    let source = APPSTREAM.get()?.icons.get(package)?;

    let extension = source.extension()?.to_str()?;
    let png = if extension == "jxl" {
        let target = cache_directory()?
            .join("icons")
            .join(format!("{}.png", package));
        if !target.is_file() {
            fs::create_dir_all(target.parent()?).ok()?;
            let converted = Command::new("djxl").arg(source).arg(&target).output().ok()?;
            if !converted.status.success() {
                let _ = fs::remove_file(&target);
                return None;
            }
        }
        target
    } else {
        source.clone()
    };

    package_icon(png.to_str()?)
}

const NON_APP_PREFIXES: [&str; 22] = [
    "lib", "lib32-", "python-", "python2-", "perl-", "ruby-", "lua-", "lua51-", "lua52-",
    "lua53-", "nodejs-", "php-", "haskell-", "ocaml-", "mingw-w64-", "gst-plugin-", "gst-",
    "qt5-", "qt6-", "r-", "xorg-", "vulkan-",
];

const NON_APP_MARKERS: [&str; 16] = [
    "-plugin", "-plugins", "-devel", "-dev", "-docs", "-doc", "-debug", "-headers", "-data",
    "-common", "-lang", "-i18n", "-l10n", "-locale", "-static", "-git-debug",
];

/// Solo deja programas: los que instalan un lanzador .desktop. Si no hay índice, filtra por nombre.
fn is_application(
    name: &str,
    query: &str,
    desktop_icons: &HashMap<String, String>,
    app_index: Option<&HashSet<String>>,
) -> bool {
    if let Some(index) = app_index {
        return index.contains(name);
    }

    let lower = name.to_ascii_lowercase();

    if lower == query.trim().to_ascii_lowercase() || desktop_icons.contains_key(&lower) {
        return true;
    }

    !NON_APP_PREFIXES.iter().any(|prefix| lower.starts_with(prefix))
        && !NON_APP_MARKERS
            .iter()
            .any(|marker| lower.ends_with(marker) || lower.contains(&format!("{marker}-")))
}

/// Lee una sola vez los archivos .desktop y asocia cada nombre con su icono.
fn load_desktop_icons() -> HashMap<String, String> {
    let mut directories = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
    ];

    if let Some(home) = std::env::var_os("HOME") {
        directories.push(PathBuf::from(home).join(".local/share/applications"));
    }

    let mut icons = HashMap::new();

    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("desktop") {
                continue;
            }

            let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                continue;
            };
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };

            let icon = content
                .lines()
                .find_map(|line| line.strip_prefix("Icon="))
                .map(str::trim)
                .unwrap_or("");

            icons.insert(stem.to_ascii_lowercase(), icon.to_string());
        }
    }

    icons
}

/// pacman interpreta la búsqueda como expresión regular; se escapa para buscar texto literal.
fn escape_regex(term: &str) -> String {
    let mut escaped = String::with_capacity(term.len());
    for character in term.chars() {
        if "\\.^$*+?()[]{}|".contains(character) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn search_pacman_internal(query: &str) -> Result<Vec<Package>, String> {
    let output = Command::new("pacman")
        .arg("-Ss")
        .args(query.split_whitespace().map(escape_regex))
        .output()
        .map_err(|error| format!("No se pudo ejecutar pacman: {}", error))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        // pacman termina con error y sin mensaje cuando simplemente no hay coincidencias.
        if stderr.is_empty() {
            return Ok(Vec::new());
        }
        return Err(stderr);
    }

    let packages = parse_pacman_search_output(
        &String::from_utf8_lossy(&output.stdout),
        query,
        app_index().as_ref(),
    );

    Ok(filter_by_name(packages, query))
}

/// Deja solo los programas cuyo nombre coincide con la búsqueda (no su descripción).
/// Si existe una coincidencia exacta, es la única que se muestra.
fn filter_by_name(packages: Vec<Package>, query: &str) -> Vec<Package> {
    let query = query.trim().to_ascii_lowercase();
    let terms: Vec<&str> = query.split_whitespace().collect();

    let mut seen = HashSet::new();
    let mut matches: Vec<Package> = packages
        .into_iter()
        .filter(|package| {
            let name = package.name.to_ascii_lowercase();
            terms.iter().all(|term| name.contains(term))
        })
        .filter(|package| seen.insert(package.name.clone()))
        .collect();

    if matches
        .iter()
        .any(|package| package.name.eq_ignore_ascii_case(&query))
    {
        matches.retain(|package| package.name.eq_ignore_ascii_case(&query));
    }

    matches
}

fn package_icon(icon_name: &str) -> Option<String> {
    if icon_name.is_empty() {
        return None;
    }

    let icon_path = find_icon_path(icon_name)?;
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

fn parse_pacman_search_output(
    stdout: &str,
    query: &str,
    app_index: Option<&HashSet<String>>,
) -> Vec<Package> {
    let desktop_icons = load_desktop_icons();
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

                if !is_application(parts[0], query, &desktop_icons, app_index) {
                    continue;
                }

                let icon = desktop_icons
                    .get(&parts[0].to_ascii_lowercase())
                    .and_then(|icon_name| package_icon(icon_name))
                    .or_else(|| appstream_icon(parts[0]));

                packages.push(Package {
                    repository: repository.to_string(),
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    description,
                    manager: "pacman".to_string(),
                    icon,
                    title: None,
                });
            }
        }
    }

    packages
}

#[derive(serde::Serialize)]
struct UpdateInfo {
    name: String,
    current: String,
    latest: String,
    source: String,
}

/// Lista las actualizaciones pendientes. `checkupdates` sincroniza una copia
/// temporal de las bases de datos, así que no necesita permisos de administrador.
#[tauri::command]
async fn list_updates() -> Result<Vec<UpdateInfo>, String> {
    run_blocking(|| {
        // checkupdates usa una base de datos temporal compartida: se serializan las llamadas.
        static CHECK_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _guard = CHECK_LOCK.lock().unwrap_or_else(|e| e.into_inner());

        let mut output = None;
        if command_exists("checkupdates") {
            for _ in 0..2 {
                if let Ok(result) = Command::new("checkupdates").output() {
                    let failed = result.stdout.is_empty() && !result.stderr.is_empty();
                    output = Some(result);
                    if !failed {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
        // Si checkupdates falla, se usa la base de datos local de pacman.
        let output = match output {
            Some(o) if !(o.stdout.is_empty() && !o.stderr.is_empty()) => o,
            _ => Command::new("pacman")
                .arg("-Qu")
                .output()
                .map_err(|error| format!("No se pudo comprobar las actualizaciones: {}", error))?,
        };

        // Ambos comandos terminan con un código distinto de 0 si no hay actualizaciones,
        // así que solo es un error real si además no hay salida y hay mensaje de error.
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stdout.trim().is_empty() && !stderr.is_empty() {
            return Err(stderr);
        }

        let mut updates: Vec<UpdateInfo> = stdout.lines().filter_map(parse_update_line).collect();
        updates.extend(list_aur_updates());
        updates.extend(list_flatpak_updates());
        Ok(updates)
    })
    .await
}

fn parse_update_line(line: &str) -> Option<UpdateInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    match parts.as_slice() {
        [name, current, "->", latest, ..] => Some(UpdateInfo {
            name: name.to_string(),
            current: current.to_string(),
            latest: latest.to_string(),
            source: "pacman".to_string(),
        }),
        _ => None,
    }
}

/// Opciones para que el ayudante del AUR no se quede esperando preguntas y use pkexec.
fn aur_install_args(helper: &str) -> Vec<&'static str> {
    let mut args = vec!["--noconfirm", "--needed", "--sudo", "pkexec"];
    if helper == "paru" {
        args.push("--skipreview");
    } else {
        args.extend(["--answerdiff", "None", "--answerclean", "None", "--answeredit", "None"]);
    }
    args
}

fn aur_helper() -> Option<&'static str> {
    ["paru", "yay"].into_iter().find(|helper| command_exists(helper))
}

/// Actualizaciones de paquetes del AUR (paru o yay), si hay un ayudante instalado.
fn list_aur_updates() -> Vec<UpdateInfo> {
    let Some(helper) = aur_helper() else {
        return Vec::new();
    };

    Command::new(helper)
        .arg("-Qua")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(parse_update_line)
                .map(|mut update| {
                    update.source = "aur".to_string();
                    update
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Actualizaciones de aplicaciones y runtimes de Flatpak.
fn list_flatpak_updates() -> Vec<UpdateInfo> {
    if !command_exists("flatpak") {
        return Vec::new();
    }

    Command::new("flatpak")
        .args(["remote-ls", "--updates", "--columns=name,application,version"])
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let mut columns = line.split('\t');
                    let name = columns.next()?.trim();
                    let application = columns.next()?.trim();
                    if name.is_empty() || application.is_empty() {
                        return None;
                    }
                    Some(UpdateInfo {
                        name: application.to_string(),
                        current: String::new(),
                        latest: columns.next().unwrap_or("").trim().to_string(),
                        source: "flatpak".to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn command_error(output: &std::process::Output, fallback: &str) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        fallback.to_string()
    } else {
        stderr
    }
}

/// Actualiza todo: repositorios oficiales, AUR y Flatpak. pacman resuelve las dependencias.
#[tauri::command]
async fn update_system() -> Result<String, String> {
    run_blocking(|| {
        if !command_exists("pkexec") {
            return Err("No se encontró pkexec en el sistema. \
                 Instala polkit para poder realizar actualizaciones gráficas."
                .to_string());
        }

        let mut errors: Vec<String> = Vec::new();

        match Command::new("pkexec")
            .arg("pacman")
            .args(["-Syu", "--noconfirm"])
            .output()
        {
            Ok(output) if output.status.success() => {}
            Ok(output) => errors.push(command_error(&output, "La actualización no se completó.")),
            Err(error) => errors.push(format!("No se pudo iniciar la actualización: {}", error)),
        }

        // Si pacman falla (p. ej. se canceló la contraseña) no se sigue con el AUR.
        if errors.is_empty() {
            if let Some(helper) = aur_helper() {
                if !list_aur_updates().is_empty() {
                    match Command::new(helper)
                        .arg("-Sua")
                        .args(aur_install_args(helper))
                        .output()
                    {
                        Ok(output) if output.status.success() => {}
                        Ok(output) => errors.push(command_error(&output, "Falló la actualización del AUR.")),
                        Err(error) => errors.push(format!("AUR: {}", error)),
                    }
                }
            }
        }

        if command_exists("flatpak") {
            match Command::new("flatpak").args(["update", "-y", "--noninteractive"]).output() {
                Ok(output) if output.status.success() => {}
                Ok(output) => errors.push(command_error(&output, "Falló la actualización de Flatpak.")),
                Err(error) => errors.push(format!("Flatpak: {}", error)),
            }
        }

        if errors.is_empty() {
            Ok("success".to_string())
        } else {
            Err(errors.join("\n"))
        }
    })
    .await
}

/// Instala un gestor de paquetes (paru, yay, flatpak...) desde los repositorios del sistema.
#[tauri::command]
async fn install_manager(name: String) -> Result<String, String> {
    run_blocking(move || {
        if !INSTALLABLE_MANAGERS.contains(&name.as_str()) {
            return Err("Gestor no admitido.".to_string());
        }
        if command_exists(&name) {
            return Ok("success".to_string());
        }
        if !command_exists("pkexec") {
            return Err("No se encontró pkexec en el sistema.".to_string());
        }

        let package = manager_package(&name).to_string();
        let flathub_remote = name == "flatpak";

        let output = if is_in_pacman_repos(&package) {
            Command::new("pkexec")
                .arg("pacman")
                .args(["-S", "--noconfirm", "--needed", &package])
                .output()
        } else if let Some(helper) = aur_helper().filter(|_| is_in_aur(&package)) {
            Command::new(helper)
                .args(["-S", "--noconfirm", "--needed", "--sudo", "pkexec", &package])
                .output()
        } else {
            return Err(format!("{} no está disponible para instalar.", name));
        }
        .map_err(|error| format!("No se pudo iniciar la instalación: {}", error))?;

        if !output.status.success() {
            return Err(command_error(&output, "La instalación no se completó."));
        }

        // Flatpak necesita el repositorio Flathub para encontrar programas.
        if flathub_remote {
            let _ = Command::new("pkexec")
                .args([
                    "flatpak", "remote-add", "--if-not-exists", "flathub",
                    "https://dl.flathub.org/repo/flathub.flatpakrepo",
                ])
                .output();
        }

        // snapd necesita su servicio activo para funcionar.
        if name == "snap" {
            let _ = Command::new("pkexec")
                .args(["systemctl", "enable", "--now", "snapd.socket"])
                .output();
        }

        Ok("success".to_string())
    })
    .await
}

fn is_installed(package_name: &str, manager: &str) -> bool {
    let mut command = match manager {
        "flatpak" => {
            let mut command = Command::new("flatpak");
            command.arg("info");
            command
        }
        "snap" => {
            let mut command = Command::new("snap");
            command.arg("list");
            command
        }
        _ => {
            let mut command = Command::new("pacman");
            command.arg("-Q");
            command
        }
    };

    command
        .arg(package_name)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn validated_manager(manager: Option<String>) -> Result<String, String> {
    let manager = manager.unwrap_or_else(|| "pacman".to_string());
    match manager.as_str() {
        "pacman" | "aur" | "flatpak" | "snap" => Ok(manager),
        _ => Err("Gestor no admitido.".to_string()),
    }
}

/// Comprueba si un paquete está instalado.
#[tauri::command]
async fn is_package_installed(package_name: String, manager: Option<String>) -> bool {
    run_blocking(move || {
        let manager = validated_manager(manager)?;
        Ok(is_installed(package_name.trim(), &manager))
    })
    .await
    .unwrap_or(false)
}

/// Comprueba que el nombre del paquete tenga un formato válido.
fn is_valid_package_name(package_name: &str) -> bool {
    if package_name.is_empty() || package_name.len() > 100 || package_name.starts_with('-') {
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

fn require_pkexec() -> Result<(), String> {
    if command_exists("pkexec") {
        Ok(())
    } else {
        Err("No se encontró pkexec en el sistema. \
             Instala polkit para poder realizar cambios gráficamente."
            .to_string())
    }
}

fn run_pkexec(program: &str, args: &[&str]) -> Result<std::process::Output, String> {
    require_pkexec()?;
    Command::new("pkexec")
        .arg(program)
        .args(args)
        .output()
        .map_err(|error| format!("No se pudo iniciar la operación: {}", error))
}

/// Instala un paquete con el gestor indicado.
#[tauri::command]
async fn install_package(package_name: String, manager: Option<String>) -> Result<String, String> {
    run_blocking(move || {
        let manager = validated_manager(manager)?;
        install_package_blocking(&package_name, &manager)
    })
    .await
}

fn install_package_blocking(package_name: &str, manager: &str) -> Result<String, String> {
    let package_name = package_name.trim();

    if !is_valid_package_name(package_name) {
        return Err("El nombre del paquete no es válido.".to_string());
    }

    if is_installed(package_name, manager) {
        return Ok(format!("El paquete {} ya está instalado.", package_name));
    }

    let output = match manager {
        "aur" => {
            let helper = aur_helper().ok_or("No hay paru ni yay instalado para usar el AUR.")?;
            Command::new(helper)
                .arg("-S")
                .args(aur_install_args(helper))
                .arg(package_name)
                .output()
                .map_err(|error| format!("No se pudo iniciar la instalación: {}", error))?
        }
        "flatpak" => Command::new("flatpak")
            .args(["install", "-y", "--noninteractive", "flathub", package_name])
            .output()
            .map_err(|error| format!("No se pudo iniciar la instalación: {}", error))?,
        "snap" => {
            let output = run_pkexec("snap", &["install", package_name])?;
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() && stderr.contains("--classic") {
                run_pkexec("snap", &["install", "--classic", package_name])?
            } else {
                output
            }
        }
        _ => run_pkexec("pacman", &["-S", "--noconfirm", "--needed", package_name])?,
    };

    if output.status.success() {
        Ok(format!("El paquete {} se ha instalado correctamente.", package_name))
    } else {
        Err(command_error(
            &output,
            &format!("La instalación de {} no se completó.", package_name),
        ))
    }
}

/// Desinstala un paquete con el gestor indicado.
#[tauri::command]
async fn remove_package(package_name: String, manager: Option<String>) -> Result<String, String> {
    run_blocking(move || {
        let manager = validated_manager(manager)?;
        remove_package_blocking(&package_name, &manager)
    })
    .await
}

fn remove_package_blocking(package_name: &str, manager: &str) -> Result<String, String> {
    let package_name = package_name.trim();

    if !is_valid_package_name(package_name) {
        return Err("El nombre del paquete no es válido.".to_string());
    }

    if !is_installed(package_name, manager) {
        return Ok(format!("El paquete {} no está instalado.", package_name));
    }

    let output = match manager {
        "flatpak" => Command::new("flatpak")
            .args(["uninstall", "-y", "--noninteractive", package_name])
            .output()
            .map_err(|error| format!("No se pudo iniciar la desinstalación: {}", error))?,
        "snap" => run_pkexec("snap", &["remove", package_name])?,
        _ => run_pkexec("pacman", &["-R", "--noconfirm", package_name])?,
    };

    if output.status.success() {
        Ok(format!("El paquete {} se ha desinstalado correctamente.", package_name))
    } else {
        Err(command_error(
            &output,
            &format!("La desinstalación de {} no se completó.", package_name),
        ))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // WebKitGTK con GPUs NVIDIA en Wayland deja la ventana a franjas al moverla o
    // redimensionarla; desactivar el renderizador DMABUF evita el problema.
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    // En Wayland con NVIDIA WebKitGTK no repinta bien al mover o redimensionar la ventana,
    // así que se fuerza XWayland (aunque la sesión defina GDK_BACKEND=wayland).
    // Con REPOFY_WAYLAND=1 se mantiene Wayland nativo.
    #[cfg(target_os = "linux")]
    if std::env::var_os("REPOFY_WAYLAND").is_none() && std::env::var_os("DISPLAY").is_some() {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    // Construye el índice de programas en segundo plano para que la primera búsqueda sea rápida.
    std::thread::spawn(|| {
        let _ = app_index();
        prepare_appstream_icons();
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_system_info,
            search_all,
            get_screenshots,
            is_package_installed,
            list_updates,
            update_system,
            install_manager,
            install_package,
            remove_package
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_libraries_and_plugins_but_keeps_applications() {
        let icons = HashMap::new();

        for name in ["vlc", "firefox", "steam", "gimp"] {
            assert!(is_application(name, "x", &icons, None), "{name}");
        }

        for name in [
            "libvlc",
            "vlc-plugin-alsa",
            "python-requests",
            "qt6-base",
            "foo-devel",
            "foo-docs",
        ] {
            assert!(!is_application(name, "x", &icons, None), "{name}");
        }
    }

    #[test]
    fn exact_match_and_desktop_entries_are_always_kept() {
        let mut icons = HashMap::new();
        icons.insert("libreoffice-fresh".to_string(), String::new());

        assert!(is_application("libvlc", "libvlc", &icons, None));
        assert!(is_application("libreoffice-fresh", "office", &icons, None));
    }

    #[test]
    fn index_decides_what_is_an_application() {
        let icons = HashMap::new();
        let index: HashSet<String> = ["steam".to_string()].into();

        assert!(is_application("steam", "steam", &icons, Some(&index)));
        assert!(!is_application("steam-devices", "steam", &icons, Some(&index)));
        assert!(!is_application("python-steam", "steam", &icons, Some(&index)));
    }

    fn package(name: &str) -> Package {
        Package {
            title: None,
            name: name.to_string(),
            version: "1".to_string(),
            description: String::new(),
            repository: "extra".to_string(),
            manager: "pacman".to_string(),
            icon: None,
        }
    }

    #[test]
    fn exact_match_is_the_only_result() {
        let found = filter_by_name(
            vec![package("steam-jupiter-stable"), package("sc-controller"), package("steam")],
            "steam",
        );

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "steam");
    }

    #[test]
    fn without_exact_match_name_must_contain_query() {
        let found = filter_by_name(
            vec![package("vlc-gui-qt"), package("syncplay"), package("vlc-gui-qt")],
            "vlc",
        );

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "vlc-gui-qt");
    }

    #[test]
    fn parses_update_lines() {
        let update = parse_update_line("steam 1.0.0.87-3 -> 1.0.0.88-1").unwrap();

        assert_eq!(update.name, "steam");
        assert_eq!(update.current, "1.0.0.87-3");
        assert_eq!(update.latest, "1.0.0.88-1");
        assert!(parse_update_line("basura").is_none());
    }

    #[test]
    fn parses_only_applications_from_pacman_output() {
        let output = "extra/vlc 3.0-1\n    Player\nextra/libvlc 3.0-1 [instalado]\n    Lib\nextra/vlc-plugin-alsa 3.0-1\n    Plugin\n";
        let packages = parse_pacman_search_output(output, "vlc", None);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "vlc");
    }
}
