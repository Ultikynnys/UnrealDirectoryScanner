#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

const ASSET_EXT: [&str; 2] = ["uasset", "umap"];

// Directories that never hold project content (VCS, caches, build output).
const SKIP_DIRS: [&str; 14] = [
    ".git",
    ".svn",
    ".hg",
    ".vs",
    ".idea",
    "node_modules",
    "__pycache__",
    "DerivedDataCache",
    "Intermediate",
    "Saved",
    "Binaries",
    "Build",
    "target",
    "dist",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    name: String,
    size: u64,
    asset: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Node {
    name: String,
    is_dir: bool,
    files: Vec<FileEntry>,
    children: Vec<Node>,
    assets: u64,
    total: u64,
    bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    root: String,
    scanned_at: u64,
    tree: Node,
}

fn is_asset(name: &str) -> bool {
    match Path::new(name).extension().and_then(|e| e.to_str()) {
        Some(ext) => ASSET_EXT.iter().any(|a| ext.eq_ignore_ascii_case(a)),
        None => false,
    }
}

fn scan_dir(abs: &Path, name: String) -> std::io::Result<Node> {
    let mut node = Node {
        name,
        is_dir: true,
        files: Vec::new(),
        children: Vec::new(),
        assets: 0,
        total: 0,
        bytes: 0,
    };

    let mut dir_names: Vec<String> = Vec::new();
    for entry in fs::read_dir(abs)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let entry_name = entry.file_name().to_string_lossy().into_owned();

        if file_type.is_dir() {
            if !SKIP_DIRS.contains(&entry_name.as_str()) {
                dir_names.push(entry_name);
            }
        } else if file_type.is_file() {
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let asset = is_asset(&entry_name);
            node.files.push(FileEntry {
                name: entry_name,
                size,
                asset,
            });
            node.total += 1;
            node.bytes += size;
            if asset {
                node.assets += 1;
            }
        }
    }

    // Directories sorted once, then folded in; unreadable subdirs are skipped so
    // one locked folder cannot abort the whole scan.
    dir_names.sort_by_key(|n| n.to_lowercase());
    for dir_name in dir_names {
        if let Ok(child) = scan_dir(&abs.join(&dir_name), dir_name) {
            node.assets += child.assets;
            node.total += child.total;
            node.bytes += child.bytes;
            node.children.push(child);
        }
    }

    node.files.sort_by_key(|f| f.name.to_lowercase());
    Ok(node)
}

#[tauri::command]
fn scan_directory(path: String) -> Result<Payload, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let tree = scan_dir(&root, name).map_err(|e| format!("could not read {path}: {e}"))?;
    let scanned_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(Payload {
        root: root.display().to_string(),
        scanned_at,
        tree,
    })
}

#[tauri::command]
fn startup_directory() -> Option<String> {
    // A folder passed on the command line (`UnrealDirectoryScanner <path>`) wins
    // over the remembered one, so an explicit launch is never overridden.
    match std::env::args().nth(1) {
        Some(arg) if Path::new(&arg).is_dir() => Some(arg),
        _ => None,
    }
}

#[tauri::command]
fn default_directory() -> String {
    std::env::current_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_default()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            startup_directory,
            default_directory
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}
