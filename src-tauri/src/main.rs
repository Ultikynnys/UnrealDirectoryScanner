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

// Folders named for an asset type, which Allar 2.6.2 forbids.
const TYPE_FOLDERS: [&str; 3] = ["Meshes", "Textures", "Materials"];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    name: String,
    size: u64,
    asset: bool,
    // Allar issues for this file, so its own row can be marked.
    issues: Vec<Issue>,
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
    // Allar issues on this folder itself, and how many exist anywhere beneath it.
    issues: Vec<Issue>,
    violations: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Issue {
    rule: String,
    message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Violation {
    rule: String,
    message: String,
    path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Payload {
    root: String,
    scanned_at: u64,
    tree: Node,
    violations: Vec<Violation>,
    lint_applied: bool,
    looks_unreal: bool,
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
        issues: Vec::new(),
        violations: 0,
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
                issues: Vec::new(),
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

/* ---- Allar / Gamemakin UE style guide checks -------------------------------
   Rule ids cite https://github.com/Allar/ue5-style-guide section 2 (Content
   Directory Structure). Only filesystem-visible rules live here; the guide's
   Blueprint, mesh, map and texture chapters need the editor and are out of
   scope. Folders under Developers/ or Python/ are exempt: 2.3 makes the former
   a sandbox, and the latter is UE's Python folder rather than project content. */

struct Ctx {
    rel: String,
    exempt: bool,
    under_maps: bool,
    under_matlib: bool,
}

fn is_umap(name: &str) -> bool {
    Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("umap"))
}

fn is_pascal_case(name: &str) -> bool {
    name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
        && name.chars().all(|c| c.is_ascii_alphanumeric())
}

// 2.1.1 to 2.1.3 overlap, so report only the most specific one that applies.
fn folder_name_issue(name: &str) -> Option<(&'static str, String)> {
    if name.contains(' ') {
        Some(("2.1.2", format!("Folder name \"{name}\" contains a space.")))
    } else if let Some(bad) = name.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '_') {
        Some((
            "2.1.3",
            format!(
                "Folder name \"{name}\" contains '{bad}'; only letters, digits and underscore are allowed."
            ),
        ))
    } else if !is_pascal_case(name) {
        Some(("2.1.1", format!("Folder name \"{name}\" is not PascalCase.")))
    } else {
        None
    }
}

fn flag(
    out: &mut Vec<Violation>,
    issues: &mut Vec<Issue>,
    rule: &str,
    message: String,
    path: &str,
) {
    issues.push(Issue {
        rule: rule.to_string(),
        message: message.clone(),
    });
    out.push(Violation {
        rule: rule.to_string(),
        message,
        path: path.to_string(),
    });
}

fn lint_dir(node: &mut Node, ctx: &Ctx, out: &mut Vec<Violation>) -> u64 {
    let root = ctx.rel.is_empty();
    let exempt = ctx.exempt || node.name == "Developers" || node.name == "Python";
    let in_maps = ctx.under_maps || node.name == "Maps";
    let in_matlib = ctx.under_matlib || node.name == "MaterialLibrary";
    let in_content = node.name == "Content";

    let mut issues: Vec<Issue> = Vec::new();

    if !exempt {
        // The scanned root's own name is the user's choice, not a project folder.
        if !root {
            if let Some((rule, message)) = folder_name_issue(&node.name) {
                flag(out, &mut issues, rule, message, &ctx.rel);
            }
            if node.name == "Assets" {
                flag(
                    out,
                    &mut issues,
                    "2.6.1",
                    format!("\"{}\" is a redundant type folder; all assets are assets.", ctx.rel),
                    &ctx.rel,
                );
            } else if TYPE_FOLDERS.contains(&node.name.as_str()) {
                flag(
                    out,
                    &mut issues,
                    "2.6.2",
                    format!(
                        "\"{}\" is a type folder; asset name prefixes already convey the type.",
                        ctx.rel
                    ),
                    &ctx.rel,
                );
            }
            if node.total == 0 && node.children.is_empty() {
                flag(
                    out,
                    &mut issues,
                    "2.9",
                    format!("\"{}\" is an empty folder.", ctx.rel),
                    &ctx.rel,
                );
            }
        }

        for file in &mut node.files {
            let rel = if root {
                file.name.clone()
            } else {
                format!("{}/{}", ctx.rel, file.name)
            };
            if !file.asset {
                continue;
            }
            let mut found: Vec<(&'static str, String)> = Vec::new();
            if in_content {
                found.push((
                    "2.2.1",
                    format!("\"{rel}\" is a global asset; project assets belong in Content/<Project>."),
                ));
            }
            if is_umap(&file.name) && !in_maps {
                found.push(("2.4", format!("\"{rel}\" is a map outside a Maps folder.")));
            }
            if file.name.starts_with("M_") && !in_matlib {
                found.push((
                    "2.8",
                    format!("\"{rel}\" is a base material outside MaterialLibrary."),
                ));
            }
            for (rule, message) in found {
                flag(out, &mut file.issues, rule, message, &rel);
            }
        }
    }

    let own =
        issues.len() as u64 + node.files.iter().map(|f| f.issues.len() as u64).sum::<u64>();
    let mut nested = 0u64;
    for child in &mut node.children {
        let child_ctx = Ctx {
            rel: if root {
                child.name.clone()
            } else {
                format!("{}/{}", ctx.rel, child.name)
            },
            exempt,
            under_maps: in_maps,
            under_matlib: in_matlib,
        };
        nested += lint_dir(child, &child_ctx, out);
    }

    node.issues = issues;
    node.violations = own + nested;
    node.violations
}

// Only lint when the root looks like Unreal content, so pointing the app at a
// random folder does not bury it in naming complaints.
fn looks_like_unreal(root: &Path, name: &str) -> bool {
    if name.eq_ignore_ascii_case("Content") || root.join("Content").is_dir() {
        return true;
    }
    fs::read_dir(root)
        .map(|entries| {
            entries.filter_map(Result::ok).any(|e| {
                e.file_type().is_ok_and(|t| t.is_file())
                    && e.file_name()
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .ends_with(".uproject")
            })
        })
        .unwrap_or(false)
}

#[tauri::command]
fn scan_directory(path: String, lint: Option<bool>) -> Result<Payload, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let looks_unreal = looks_like_unreal(&root, &name);
    let lint_applied = lint.unwrap_or(looks_unreal);

    let mut tree = scan_dir(&root, name).map_err(|e| format!("could not read {path}: {e}"))?;

    let mut violations = Vec::new();
    if lint_applied {
        let ctx = Ctx {
            rel: String::new(),
            exempt: false,
            under_maps: false,
            under_matlib: false,
        };
        lint_dir(&mut tree, &ctx, &mut violations);
    }

    let scanned_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(Payload {
        root: root.display().to_string(),
        scanned_at,
        tree,
        violations,
        lint_applied,
        looks_unreal,
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
