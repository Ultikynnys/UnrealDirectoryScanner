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

// Asset type by filename prefix. Prefixes and type names follow the Allar style
// guide section 1.2 (Asset Name Modifiers), extended with engine conventions the
// guide predates so real projects get labelled instead of dropped into
// "unrecognised": NS_, NE_, CS_, IA_, IMC_, DA_, AN_, SC_, NavLink_, and the
// underscored E_/F_ forms that UE tooling writes. No prefix here is a prefix of
// any other, so the order of this table does not matter.
const ASSET_KINDS: &[(&str, &str, &str, &str)] = &[
    // (filename prefix, colour family, chip label, type name)
    ("BPFL_", "blueprint", "BPFL", "Blueprint Function Library"),
    ("BPML_", "blueprint", "BPML", "Blueprint Macro Library"),
    ("BPI_", "blueprint", "BPI", "Blueprint Interface"),
    ("TBP_", "blueprint", "TBP", "Tutorial Blueprint"),
    ("BP_", "blueprint", "BP", "Blueprint"),
    ("E_", "blueprint", "E", "Enumeration"),
    ("F_", "blueprint", "F", "Structure"),
    ("SM_", "mesh", "SM", "Static Mesh"),
    ("SKEL_", "mesh", "SKEL", "Skeleton"),
    ("SK_", "mesh", "SK", "Skeletal Mesh"),
    ("PHYS_", "mesh", "PHYS", "Physics Asset"),
    ("DM_", "mesh", "DM", "Destructible Mesh"),
    ("S_", "mesh", "S", "Static Mesh"),
    ("MPC_", "material", "MPC", "Material Parameter Collection"),
    ("MI_", "material", "MI", "Material Instance"),
    ("MF_", "material", "MF", "Material Function"),
    ("PP_", "material", "PP", "Post Process Material"),
    ("SP_", "material", "SP", "Subsurface Profile"),
    ("PM_", "material", "PM", "Physical Material"),
    ("M_", "material", "M", "Material"),
    ("RTC_", "texture", "RTC", "Cube Render Target"),
    ("TC_", "texture", "TC", "Texture Cube"),
    ("MT_", "texture", "MT", "Media Texture"),
    ("RT_", "texture", "RT", "Render Target"),
    ("TLP", "texture", "TLP", "Texture Light Profile"),
    ("T_", "texture", "T", "Texture"),
    ("BTDecorator_", "ai", "BTDecorator", "Behavior Tree Decorator"),
    ("BTService_", "ai", "BTService", "Behavior Tree Service"),
    ("BTTask_", "ai", "BTTask", "Behavior Tree Task"),
    ("AIC_", "ai", "AIC", "AI Controller"),
    ("EQS_", "ai", "EQS", "Environment Query"),
    ("BT_", "ai", "BT", "Behavior Tree"),
    ("BB_", "ai", "BB", "Blackboard"),
    ("ABP_", "animation", "ABP", "Animation Blueprint"),
    ("AM_", "animation", "AM", "Animation Montage"),
    ("AO_", "animation", "AO", "Aim Offset"),
    ("AC_", "animation", "AC", "Animation Composite"),
    ("BS_", "animation", "BS", "Blend Space"),
    ("LS_", "animation", "LS", "Level Sequence"),
    ("CR_", "animation", "CR", "Control Rig"),
    ("Rig_", "animation", "Rig", "Rig"),
    ("PFB_", "animation", "PFB", "Paper Flipbook"),
    ("AN_", "animation", "AN", "Animation Sequence"),
    ("A_", "animation", "A", "Animation Sequence"),
    ("MSW_", "audio", "MSW", "Media Sound Wave"),
    ("ATT_", "audio", "ATT", "Sound Attenuation"),
    ("Reverb_", "audio", "Reverb", "Reverb Effect"),
    ("Mix_", "audio", "Mix", "Sound Mix"),
    ("SC_", "audio", "SC", "Sound Class"),
    ("WBP_", "ui", "WBP", "Widget Blueprint"),
    ("Font_", "ui", "Font", "Font"),
    ("Brush_", "ui", "Brush", "Slate Brush"),
    ("Style_", "ui", "Style", "Slate Widget Style"),
    ("PS_", "fx", "PS", "Particle System"),
    ("NS_", "fx", "NS", "Niagara System"),
    ("NE_", "fx", "NE", "Niagara Emitter"),
    ("CS_", "fx", "CS", "Camera Shake"),
    ("VFA_", "data", "VFA", "Animated Vector Field"),
    ("SGI_", "data", "SGI", "Substance Graph Instance"),
    ("SIF_", "data", "SIF", "Substance Instance Factory"),
    ("NavLink_", "data", "NavLink", "Nav Link Proxy"),
    ("OL_", "data", "OL", "Object Library"),
    ("FT_", "data", "FT", "Foliage Type"),
    ("LG_", "data", "LG", "Landscape Grass Type"),
    ("LL_", "data", "LL", "Landscape Layer"),
    ("VF_", "data", "VF", "Static Vector Field"),
    ("MP_", "data", "MP", "Media Player"),
    ("DA_", "data", "DA", "Data Asset"),
    ("DT_", "data", "DT", "Data Table"),
    ("Curve_", "data", "Curve", "Curve"),
    ("Matinee_", "data", "Matinee", "Matinee Data"),
    ("TI_", "data", "TI", "Touch Interface Setup"),
    ("SPRG_", "data", "SPRG", "Sprite Atlas Group"),
    ("SPR_", "data", "SPR", "Sprite"),
    ("SS_", "data", "SS", "Sprite Sheet"),
    ("TM_", "data", "TM", "Tile Map"),
    ("TS_", "data", "TS", "Tile Set"),
    ("CA_", "data", "CA", "Camera Anim"),
    ("FFE_", "data", "FFE", "Force Feedback Effect"),
    ("FF_", "data", "FF", "Force Feedback"),
    ("DV_", "data", "DV", "Dialogue Voice"),
    ("DW_", "data", "DW", "Dialogue Wave"),
    ("IA_", "input", "IA", "Input Action"),
    ("IMC_", "input", "IMC", "Input Mapping Context"),
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FileEntry {
    name: String,
    size: u64,
    asset: bool,
    // Allar issues for this file, so its own row can be marked.
    issues: Vec<Issue>,
    // Asset type, for the chip and the colour coding in the UI.
    type_family: String,
    type_label: String,
    type_name: String,
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

fn base_name(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) => &name[..i],
        None => name,
    }
}

// Classifies a file as (colour family, chip label, type name). Maps are decided
// by extension; everything else by the Allar section 1.2 filename prefix.
fn asset_kind(name: &str, asset: bool) -> (&'static str, String, String) {
    let base = base_name(name);
    let ext = Path::new(name).extension().and_then(|e| e.to_str()).unwrap_or("");

    if asset && ext.eq_ignore_ascii_case("umap") {
        return ("level", "MAP".to_string(), "Level / Map".to_string());
    }

    // Non-asset files are labelled by extension, never by an asset prefix: a
    // stray "T_notes.txt" is a text file, not a texture.
    if !asset {
        if ext.is_empty() {
            return ("file", "FILE".to_string(), "File".to_string());
        }
        let upper = ext.to_ascii_uppercase();
        return ("file", upper.clone(), format!("{upper} file"));
    }

    for &(prefix, family, label, type_name) in ASSET_KINDS {
        if base.starts_with(prefix) {
            return (family, label.to_string(), type_name.to_string());
        }
    }

    // Allar's no-underscore forms: E for enumerations, F or S for structures.
    if !base.contains('_') && base.len() >= 2 {
        let mut chars = base.chars();
        let first = chars.next().unwrap_or(' ');
        let second = chars.next().unwrap_or(' ');
        if first.is_ascii_uppercase()
            && second.is_ascii_uppercase()
            && base.chars().all(|c| c.is_ascii_alphanumeric())
        {
            match first {
                'E' => return ("blueprint", "E".to_string(), "Enumeration".to_string()),
                'F' | 'S' => {
                    return (
                        "blueprint",
                        first.to_string(),
                        "Structure".to_string(),
                    )
                }
                _ => {}
            }
        }
    }

    ("other", "?".to_string(), "Unrecognised prefix".to_string())
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
            let (type_family, type_label, type_name) = asset_kind(&entry_name, asset);
            node.files.push(FileEntry {
                name: entry_name,
                size,
                asset,
                issues: Vec::new(),
                type_family: type_family.to_string(),
                type_label,
                type_name,
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

#[cfg(test)]
mod tests {
    use super::asset_kind;

    fn kind(name: &str, asset: bool) -> (String, String, String) {
        let (family, label, type_name) = asset_kind(name, asset);
        (family.to_string(), label, type_name)
    }

    #[test]
    fn assets_are_typed_by_prefix() {
        assert_eq!(kind("T_Rock_D.uasset", true), ("texture".into(), "T".into(), "Texture".into()));
        assert_eq!(
            kind("MI_Rock.uasset", true),
            ("material".into(), "MI".into(), "Material Instance".into())
        );
        assert_eq!(kind("M_Rock.uasset", true), ("material".into(), "M".into(), "Material".into()));
        assert_eq!(kind("S_Rock.uasset", true), ("mesh".into(), "S".into(), "Static Mesh".into()));
        assert_eq!(kind("BP_Bob.uasset", true), ("blueprint".into(), "BP".into(), "Blueprint".into()));
        assert_eq!(
            kind("NS_Fire.uasset", true),
            ("fx".into(), "NS".into(), "Niagara System".into())
        );
        assert_eq!(
            kind("A_Run.uasset", true),
            ("animation".into(), "A".into(), "Animation Sequence".into())
        );
        assert_eq!(
            kind("E_Damage.uasset", true),
            ("blueprint".into(), "E".into(), "Enumeration".into())
        );
        assert_eq!(
            kind("SC_Combat.uasset", true),
            ("audio".into(), "SC".into(), "Sound Class".into())
        );
    }

    #[test]
    fn maps_are_levels_whatever_their_name() {
        assert_eq!(
            kind("Town.umap", true),
            ("level".into(), "MAP".into(), "Level / Map".into())
        );
        assert_eq!(
            kind("A_Streaming.umap", true),
            ("level".into(), "MAP".into(), "Level / Map".into())
        );
    }

    #[test]
    fn underscoreless_enum_and_structure_forms() {
        assert_eq!(
            kind("EWeaponType.uasset", true),
            ("blueprint".into(), "E".into(), "Enumeration".into())
        );
        assert_eq!(
            kind("FInventorySlot.uasset", true),
            ("blueprint".into(), "F".into(), "Structure".into())
        );
    }

    #[test]
    fn unknown_prefixes_and_non_assets() {
        assert_eq!(
            kind("AssetZoo_Thing.uasset", true),
            ("other".into(), "?".into(), "Unrecognised prefix".into())
        );
        assert_eq!(kind("notes.txt", false), ("file".into(), "TXT".into(), "TXT file".into()));
        // an asset prefix on a non-asset file must not win
        assert_eq!(kind("T_notes.txt", false), ("file".into(), "TXT".into(), "TXT file".into()));
        assert_eq!(
            kind("Splash.png", false),
            ("file".into(), "PNG".into(), "PNG file".into())
        );
    }
}
