#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

mod cli;

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

// Beyond Allar 2.6.2's three, the folders community guides (Epic's community
// structure tutorial, Diversion's UE5 guide) also call redundant: each restates
// a name prefix the content browser can already filter on. Rule 2.6.3 flags them.
const EXTENDED_TYPE_FOLDERS: [&str; 8] = [
    "Blueprints",
    "StaticMeshes",
    "SkeletalMeshes",
    "Animations",
    "Sounds",
    "Audio",
    "Particles",
    "Effects",
];

// Asset types follow the Allar style-guide prefixes plus conventions found in UE projects. A few
// assets have no stable filename prefix; for those, a single unambiguous serialized class path can
// label the type without treating that inferred label as a naming-rule prefix.
const ASSET_KINDS: &[(&str, &str, &str, &str)] = &[
    // (filename prefix, colour family, chip label, type name)
    ("BPFL_", "blueprint", "BPFL", "Blueprint Function Library"),
    ("BPML_", "blueprint", "BPML", "Blueprint Macro Library"),
    ("BPI_", "blueprint", "BPI", "Blueprint Interface"),
    ("BI_", "blueprint", "BI", "Blueprint Interface"),
    ("BPC_", "blueprint", "BPC", "Blueprint Component"),
    ("ST_", "blueprint", "ST", "Structure"),
    ("TBP_", "blueprint", "TBP", "Tutorial Blueprint"),
    ("BP_", "blueprint", "BP", "Blueprint"),
    ("E_", "blueprint", "E", "Enumeration"),
    ("F_", "blueprint", "F", "Structure"),
    ("SM_", "mesh", "SM", "Static Mesh"),
    ("SKEL_", "mesh", "SKEL", "Skeleton"),
    ("SK_", "mesh", "SK", "Skeletal Mesh"),
    ("PHYS_", "mesh", "PHYS", "Physics Asset"),
    ("DM_", "mesh", "DM", "Destructible Mesh"),
    ("SKM_", "mesh", "SKM", "Skeletal Mesh"),
    ("PA_", "mesh", "PA", "Physics Asset"),
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
    ("BTT_", "ai", "BTT", "Behavior Tree Task"),
    ("BTS_", "ai", "BTS", "Behavior Tree Service"),
    ("BTD_", "ai", "BTD", "Behavior Tree Decorator"),
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
    ("AS_", "animation", "AS", "Animation Sequence"),
    ("A_", "animation", "A", "Animation Sequence"),
    ("MSW_", "audio", "MSW", "Media Sound Wave"),
    ("Cue_", "audio", "Cue", "Sound Cue"),
    ("SA_", "audio", "SA", "Sound Attenuation"),
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
    ("C_", "data", "C", "Curve"),
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
    // The files this one references, by path, for the row's reference chip.
    references: Vec<String>,
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
    // Allar issues on this folder itself, and how many exist anywhere beneath it,
    // split by category so each one can be tagged in its own colour.
    issues: Vec<Issue>,
    violations: u64,
    naming_violations: u64,
    structure_violations: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Issue {
    rule: String,
    message: String,
    category: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Violation {
    rule: String,
    message: String,
    path: String,
    category: &'static str,
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

// Serialized class names can type an otherwise unrecognised asset, but import names share this
// string table. Only a unique class name, or one of the two known subclass pairs below, is safe.
const SERIALIZED_CLASS_KINDS: &[(&str, &str, &str, &str)] = &[
    ("SoundWave", "audio", "SW", "Sound Wave"),
    ("SoundCue", "audio", "Cue", "Sound Cue"),
    ("SoundAttenuation", "audio", "SA", "Sound Attenuation"),
    ("SkeletalMesh", "mesh", "SKM", "Skeletal Mesh"),
    ("PhysicsAsset", "mesh", "PA", "Physics Asset"),
    ("StaticMesh", "mesh", "SM", "Static Mesh"),
    ("MapBuildDataRegistry", "data", "LBD", "Map Build Data Registry"),
    ("Texture2D", "texture", "T2D", "Texture 2D"),
    ("Blueprint", "blueprint", "BP", "Blueprint"),
    ("BlueprintGeneratedClass", "blueprint", "BP", "Blueprint"),
    ("UserDefinedStruct", "blueprint", "Struct", "Structure"),
    ("CurveLinearColorAtlas", "data", "C", "Curve Atlas"),
    ("CurveLinearColor", "data", "C", "Color Curve"),
    ("Font", "ui", "F", "Font"),
];

fn class_marker_present(bytes: &[u8], marker: &str) -> bool {
    let marker = marker.as_bytes();
    bytes.windows(marker.len()).enumerate().any(|(start, window)| {
        let begins_at_boundary = start == 0
            || (!bytes[start - 1].is_ascii_alphanumeric()
                && bytes[start - 1] != b'_'
                && bytes[start - 1] != b'/');
        window == marker
            && begins_at_boundary
            && bytes.get(start + marker.len()).map_or(true, |next| {
                !next.is_ascii_alphanumeric() && *next != b'_'
            })
    })
}

fn serialized_class_index(bytes: &[u8]) -> Option<usize> {
    let blueprint = SERIALIZED_CLASS_KINDS.iter().position(|(name, ..)| *name == "Blueprint")?;
    if class_marker_present(bytes, "Blueprint") && class_marker_present(bytes, "BlueprintGeneratedClass") {
        return Some(blueprint);
    }

    let mut found: Option<usize> = None;
    for (index, (marker, ..)) in SERIALIZED_CLASS_KINDS.iter().enumerate() {
        if !class_marker_present(bytes, marker) {
            continue;
        }
        if let Some(previous) = found {
            let previous_name = SERIALIZED_CLASS_KINDS[previous].0;
            let is_known_class_pair =
                previous_name == "CurveLinearColorAtlas" && *marker == "CurveLinearColor";
            if is_known_class_pair {
                continue;
            }
            return None;
        }
        found = Some(index);
    }
    found
}

type ClassCache = std::sync::Mutex<std::collections::HashMap<PathBuf, (u64, u128, Option<usize>)>>;

fn cached_serialized_class(path: &Path) -> Option<usize> {
    static CACHE: std::sync::OnceLock<ClassCache> = std::sync::OnceLock::new();
    let metadata = fs::metadata(path).ok()?;
    let modified = metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_nanos();
    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut cache = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((size, time, class_index)) = cache.get(path) {
        if *size == metadata.len() && *time == modified {
            return *class_index;
        }
    }
    let class_index = fs::read(path).ok().and_then(|bytes| serialized_class_index(&bytes));
    cache.insert(path.to_path_buf(), (metadata.len(), modified, class_index));
    class_index
}

// Classifies a file as (colour family, chip label, type name). Maps are decided
// by extension, then known naming prefixes, and then a uniquely identifiable serialized class.
fn asset_kind(name: &str, asset: bool) -> (&'static str, String, String) {
    asset_kind_with_class(name, asset, None)
}

#[cfg(test)]
fn asset_kind_with_bytes(
    name: &str,
    asset: bool,
    class_bytes: Option<&[u8]>,
) -> (&'static str, String, String) {
    asset_kind_with_class(name, asset, class_bytes.and_then(serialized_class_index))
}

fn asset_kind_with_class(
    name: &str,
    asset: bool,
    class_index: Option<usize>,
) -> (&'static str, String, String) {
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

    // These short conventions are shared by curves and fonts, whose classes can distinguish them.
    if base.starts_with("C_") || base.starts_with("F_") {
        if let Some(index) = class_index {
            let kind = SERIALIZED_CLASS_KINDS[index];
            if base.starts_with("C_") && kind.1 == "data"
                || base.starts_with("F_") && kind.1 == "ui"
            {
                return (kind.1, kind.2.to_string(), kind.3.to_string());
            }
        }
        if base.starts_with("F_") {
            return ("blueprint", "F".to_string(), "Structure".to_string());
        }
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

    if let Some(index) = class_index {
        let kind = SERIALIZED_CLASS_KINDS[index];
        return (kind.1, kind.2.to_string(), kind.3.to_string());
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
        naming_violations: 0,
        structure_violations: 0,
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
            let needs_class = asset
                && (asset_kind(&entry_name, asset).0 == "other"
                    || entry_name.starts_with("C_")
                    || entry_name.starts_with("F_"));
            let class_index = if needs_class { cached_serialized_class(&entry.path()) } else { None };
            let (type_family, type_label, type_name) =
                asset_kind_with_class(&entry_name, asset, class_index);
            node.files.push(FileEntry {
                name: entry_name,
                size,
                asset,
                issues: Vec::new(),
                type_family: type_family.to_string(),
                type_label,
                type_name,
                references: Vec::new(),
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

/* ---- References ------------------------------------------------------------ */

/* What an asset points at. A package is written as tables and property data, and the full
   `/Game/...` object path of everything it imports sits in those bytes in the clear, so the graph
   can be read the way one would grep it: no package parser, and nothing guessed, since a path that
   names no file the scan found - or names more than one - is dropped rather than turned into an
   edge. Two things it cannot see: content mounted under a plugin's own name rather than `/Game/`,
   and packages whose tables are compressed, as a shipped build may use. */

/// One file's dependencies, keyed by the path of the file holding the reference, in the same
/// relative form the tree uses.
type References = std::collections::HashMap<String, Vec<String>>;

/// What each package named when it was last read, and when that was, so that moving a checkbox does
/// not read the whole project again.
type PackageCache =
    std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, (u64, u64, Vec<String>)>>;

/// Whether a byte can appear in an object path. Anything else ends the run, which is what stops it
/// at the quote of a path written as `'/Game/A/B.B'`.
fn is_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'_' | b'-' | b'.')
}

/// The `/Game/` object paths a package's bytes name, in order and without repetition. What follows
/// the last dot names the object inside the package rather than the package, so it goes.
fn mounted_paths(bytes: &[u8]) -> Vec<String> {
    const MOUNT: &[u8] = b"/Game/";
    let mut paths: Vec<String> = Vec::new();
    let mut at = 0;
    while let Some(offset) = bytes[at..].windows(MOUNT.len()).position(|run| run == MOUNT) {
        let start = at + offset;
        let mut end = start;
        while end < bytes.len() && is_path_byte(bytes[end]) {
            end += 1;
        }
        let run = std::str::from_utf8(&bytes[start..end]).unwrap_or_default();
        let package = run.rsplit_once('.').map_or(run, |(package, _)| package);
        if !paths.iter().any(|path| path == package) {
            paths.push(package.to_string());
        }
        at = end.max(start + 1);
    }
    paths
}

/// A file's path without its extension, which is how a package is named from the inside.
fn package_path(file: &str) -> &str {
    file.rsplit_once('.').map_or(file, |(head, _)| head)
}

/// The folder a path sits in, the root being the empty one.
fn folder_of(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(head, _)| head)
}

/// Whether one folder is the other or sits below it. The root is the folder everything sits below,
/// while a sibling never is.
fn at_or_below(folder: &str, other: &str) -> bool {
    folder == other
        || other.is_empty()
        || (folder.len() > other.len()
            && folder.starts_with(other)
            && folder.as_bytes()[other.len()] == b'/')
}

/// The scan's packages keyed by the last part of their name, so a reference can be found by the end
/// it has to match rather than by walking every file for each one.
fn reference_index(files: &[String]) -> std::collections::HashMap<String, Vec<String>> {
    let mut index: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for file in files {
        let last = package_path(file).rsplit('/').next().unwrap_or_default();
        index.entry(last.to_string()).or_default().push(file.clone());
    }
    index
}

/// The one file a `/Game/...` path names, or nothing when it names none of them or more than one.
/// The mount point of `/Game/` is `Content` in a game project and a plugin's own name elsewhere, and
/// this does not know which, so the tail is what is matched: the part that is the same either way.
fn resolve_reference(
    path: &str,
    index: &std::collections::HashMap<String, Vec<String>>,
) -> Option<String> {
    let tail = path.strip_prefix("/Game/")?;
    let last = tail.rsplit('/').next()?;
    let mut found: Option<&String> = None;
    for file in index.get(last)? {
        let package = package_path(file);
        if package == tail || package.ends_with(&format!("/{tail}")) {
            if found.is_some() {
                // two packages answer to the same tail, so this is not a reference we can follow
                return None;
            }
            found = Some(file);
        }
    }
    found.cloned()
}

/// Every file in a scanned tree as a path relative to its root, and the packages among them. The
/// tree already knows both, so the filesystem is not walked a second time to find them.
fn tree_files(node: &Node, rel: &str, files: &mut Vec<String>, packages: &mut Vec<String>) {
    for file in &node.files {
        let path = if rel.is_empty() {
            file.name.clone()
        } else {
            format!("{rel}/{}", file.name)
        };
        if file.asset {
            packages.push(path.clone());
        }
        files.push(path);
    }
    for child in &node.children {
        let path = if rel.is_empty() {
            child.name.clone()
        } else {
            format!("{rel}/{}", child.name)
        };
        tree_files(child, &path, files, packages);
    }
}

/// Hang each file's references on the row that shows it, under the same path the edge map is keyed
/// by, so the window can say what an asset points at without asking again.
fn attach_references(node: &mut Node, rel: &str, references: &References) {
    for file in &mut node.files {
        let path = if rel.is_empty() {
            file.name.clone()
        } else {
            format!("{rel}/{}", file.name)
        };
        file.references = references.get(&path).cloned().unwrap_or_default();
    }
    for child in &mut node.children {
        let path = if rel.is_empty() {
            child.name.clone()
        } else {
            format!("{rel}/{}", child.name)
        };
        attach_references(child, &path, references);
    }
}

/// The `/Game/` paths a package names, remembered by path and only re-read when the file's size or
/// modified time says it has changed. The paths are a fraction of the bytes they came from, which is
/// the point: a project is read once, not once per rule tick.
fn cached_paths(path: &Path) -> Vec<String> {
    static CACHE: std::sync::OnceLock<PackageCache> = std::sync::OnceLock::new();
    let Ok(meta) = fs::metadata(path) else {
        return Vec::new();
    };
    let size = meta.len();
    let modified = meta
        .modified()
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |since| since.as_secs());

    let cache = CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut cache = cache.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some((known_size, known_modified, paths)) = cache.get(path) {
        if *known_size == size && *known_modified == modified {
            return paths.clone();
        }
    }
    let paths = fs::read(path).map_or_else(|_| Vec::new(), |bytes| mounted_paths(&bytes));
    cache.insert(path.to_path_buf(), (size, modified, paths.clone()));
    paths
}

/// What every package in a scanned tree points at, resolved against the tree's own files.
fn scan_references(root: &Path, tree: &Node) -> References {
    let mut files: Vec<String> = Vec::new();
    let mut packages: Vec<String> = Vec::new();
    tree_files(tree, "", &mut files, &mut packages);
    let index = reference_index(&files);

    let mut references = References::new();
    for package in packages {
        let mut targets: Vec<String> = Vec::new();
        for path in cached_paths(&root.join(&package)) {
            let Some(file) = resolve_reference(&path, &index) else {
                continue;
            };
            if file != package && !targets.contains(&file) {
                targets.push(file);
            }
        }
        references.insert(package, targets);
    }
    references
}

/* ---- Allar / Gamemakin UE style guide checks -------------------------------
   Rule ids cite https://github.com/Allar/ue5-style-guide. Section 1 is asset
   naming and section 2 is Content directory structure, so the leading number is
   also the rule's category. Only filesystem-visible rules live here; the
   guide's Blueprint, mesh, map and texture chapters need the editor and are out
   of scope. Folders under Developers/ or Python/ are exempt: 2.3 makes the
   former a sandbox, and the latter is UE's Python folder rather than project
   content. */

/* The rules and the guides that group them. This table is the single source of
   truth: cli.rs derives its rule ids from RULES, and the front end renders both the
   guide picker and the rule checkboxes from the rule_catalog command, so the two
   cannot disagree about which ids exist. */

struct RuleSpec {
    id: &'static str,
    label: &'static str,
    category: &'static str,
}

const RULES: &[RuleSpec] = &[
    RuleSpec { id: "00.1", label: "characters", category: "naming" },
    RuleSpec { id: "00.2", label: "Windows names", category: "naming" },
    // PascalCase covers folder names and asset names alike, so it is a rule of its own rather
    // than half of the folder check and half of the asset-name check.
    RuleSpec { id: "00.3", label: "PascalCase", category: "naming" },
    RuleSpec { id: "1.1", label: "base name", category: "naming" },
    RuleSpec { id: "2.1.2", label: "folder spaces", category: "structure" },
    RuleSpec { id: "2.1.3", label: "folder characters", category: "structure" },
    RuleSpec { id: "2.1.4", label: "folder case clash", category: "structure" },
    RuleSpec { id: "2.2.1", label: "global asset", category: "structure" },
    RuleSpec { id: "2.4", label: "map placement", category: "structure" },
    RuleSpec { id: "2.6.1", label: "Assets folder", category: "structure" },
    RuleSpec { id: "2.6.2", label: "type folder", category: "structure" },
    RuleSpec { id: "2.6.3", label: "type folder (extended)", category: "structure" },
    RuleSpec { id: "2.8", label: "base material", category: "structure" },
    RuleSpec { id: "2.9", label: "empty folder", category: "structure" },
    // A category of its own: no guide this tool ships says anything about references, so 3.1 is
    // only checked when it is named. See OPT_IN_RULES.
    RuleSpec { id: "3.1", label: "association", category: "association" },
];

/* A guide is a named preset over the rule set, so "which guide do we follow" is one
   choice instead of fourteen. `allar` is the checks this scanner has always run and
   stays the default; `community` adds the rules that go beyond Allar's guide;
   `minimal` is the high-signal subset. Anything else is read from the folder beside the
   executable - see `FilePreset` - so these three are only what the tool itself brings. */

struct Guide {
    id: &'static str,
    label: &'static str,
    // The guide's own documentation, opened by `open_docs`. `None` for a preset nobody
    // published one for.
    url: Option<&'static str>,
    // What the preset turns on, then what it takes away. A token naming a category covers
    // every rule in it; any other token is a rule id. A preset is therefore a combination of
    // flags and not a private rule list, and the same set can be spelled either way round.
    flags: &'static [&'static str],
    except: &'static [&'static str],
}

/// Whether a rule token covers a rule: a category name covers its whole category, anything
/// else is a rule id. This is the one grammar the presets and the command line both speak.
fn token_covers<S: AsRef<str>>(token: &S, rule: &str) -> bool {
    let token = token.as_ref();
    // A category name covers its category except for the rules that have to be named, so naming
    // everything and subtracting cannot quietly turn one on.
    token == rule || (token == rule_category(rule) && !rule_is_opt_in(rule))
}

/// The checks this tool can run that no guide it ships asks for. Naming a whole category leaves them
/// out; naming the rule itself, or a preset that names it, turns it on.
const OPT_IN_RULES: &[&str] = &["3.1"];

/// Whether a rule is only checked when it is named.
fn rule_is_opt_in(rule: &str) -> bool {
    OPT_IN_RULES.contains(&rule)
}

/// Whether any of `tokens` covers `rule`.
fn covers<S: AsRef<str>>(tokens: &[S], rule: &str) -> bool {
    tokens.iter().any(|token| token_covers(token, rule))
}

/// Whether the flags accept a token, which is what every validation is: a category name, or
/// the id of a rule the checks know.
fn known_token(token: &str) -> bool {
    token == "naming"
        || token == "structure"
        || token == "association"
        || RULES.iter().any(|spec| spec.id == token)
}

/// The rule tokens in a comma separated flag value, or the first one the flags do not accept.
/// Every list-taking flag comes through here, the command line and a preset file alike, so they
/// all split and validate the same way.
fn tokens_of(list: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    for token in list.split(',').map(str::trim).filter(|token| !token.is_empty()) {
        if !known_token(token) {
            return Err(token.to_string());
        }
        tokens.push(token.to_string());
    }
    Ok(tokens)
}

/// A preset written back out as command-line flags, which is both what `--help` shows and what a
/// preset file holds. Rule tokens collapse into one `--rules` list, an equivalent spelling.
fn flags_text<S: AsRef<str>, T: AsRef<str>>(flags: &[S], except: &[T]) -> String {
    let mut parts: Vec<String> = flags
        .iter()
        .map(|token| token.as_ref())
        .filter(|token| *token == "naming" || *token == "structure")
        .map(|token| format!("--{token}"))
        .collect();
    let ids: Vec<&str> = flags
        .iter()
        .map(|token| token.as_ref())
        .filter(|token| *token != "naming" && *token != "structure")
        .collect();
    if !ids.is_empty() {
        parts.push(format!("--rules {}", ids.join(",")));
    }
    let left_out: Vec<&str> = except.iter().map(|token| token.as_ref()).collect();
    if !left_out.is_empty() {
        parts.push(format!("--except {}", left_out.join(",")));
    }
    parts.join(" ")
}

/* The one place a rule set is worked out. Start from a preset - or from nothing, when the
   flags spell the whole set out themselves - add what the include tokens name, then take
   away what the exclude tokens name, so exclusions always win. The presets, the command line,
   the report and the pickers all come through here, and none of them can disagree about what
   a selection means. */
fn resolve<S: AsRef<str>>(base: Option<&dyn Preset>, include: &[S], exclude: &[S]) -> Vec<String> {
    RULES
        .iter()
        .filter(|spec| {
            let rule = spec.id;
            let wanted = base.is_some_and(|preset| preset.has(rule)) || covers(include, rule);
            wanted && !covers(exclude, rule)
        })
        .map(|spec| spec.id.to_string())
        .collect()
}

/// The rules a preset selects, which is what the pickers and the report ask for: the resolver with
/// nothing added and nothing taken away, in table order.
fn rules_of(preset: &dyn Preset) -> Vec<String> {
    let none: &[&str] = &[];
    resolve(Some(preset), none, none)
}

/// What a selection starts from, whoever wrote it: a guide the tool ships and a preset read from
/// the folder both answer through here, so nothing downstream cares which of the two it got.
trait Preset {
    /// Whether this preset turns `rule` on.
    fn has(&self, rule: &str) -> bool;

    /// The preset's own documentation, if its author published any.
    fn docs(&self) -> Option<&str> {
        None
    }

    /// Why the preset cannot be used, if it cannot. A broken preset still exists, so that it can be
    /// reported and deleted rather than quietly vanishing.
    fn problem(&self) -> Option<&str> {
        None
    }
}

impl Preset for Guide {
    fn has(&self, rule: &str) -> bool {
        covers(self.flags, rule) && !covers(self.except, rule)
    }

    fn docs(&self) -> Option<&str> {
        self.url
    }
}

/* The presets the tool ships, each named as its author names it and each written the way round
   that reads best: name everything with the two category flags, then subtract.
   `a_preset_can_be_spelled_either_way` checks the first two against the same set listed out rule
   by rule, and `a_guide_leaves_out_what_its_own_rules_disagree_with` pins what the rest leave out. */

const EVERY_CATEGORY: &[&str] = &["naming", "structure"];
const NO_EXCEPTIONS: &[&str] = &[];
const ALLAR_EXCEPT: &[&str] = &["00.2", "2.1.4", "2.6.3"];
const MINIMAL_EXCEPT: &[&str] = &["00.2", "2.1.4", "2.6.1", "2.6.2", "2.6.3", "2.8"];

/* The guides that are not Allar argue over a handful of questions about folders - whether a
   folder per asset type is allowed, whether the project gets a folder of its own, whether there
   is one for Maps or for a material library - and agree with Allar or say nothing about the rest.
   Almost all of what each publishes beyond that is a table of asset prefixes, and this scanner
   only asks whether a name starts with a prefix it knows rather than which one, so a preset here
   is those folder answers and no more. Each leaves out Allar's exceptions plus its own. */

// Unreal Directive's "by purpose": a _Shared folder organised by asset type, and no MaterialLibrary
// or Maps folder of its own. Its "modular" layout for live-service content comes out the same.
const UNREAL_DIRECTIVE_EXCEPT: &[&str] = &["00.2", "2.1.4", "2.4", "2.6.2", "2.6.3", "2.8"];
// Lyra: Content holds Maps/, System/ and UI/, with no folder named after the project, and the rest
// of its content lives in the plugin that owns it.
const LYRA_EXCEPT: &[&str] = &["00.2", "2.1.4", "2.2.1", "2.6.3", "2.8"];
// "By asset type", the layout most small projects start with: a folder per type is the point.
const BY_TYPE_EXCEPT: &[&str] = &["00.2", "2.1.4", "2.6.2", "2.6.3"];

const ALLAR_DOCS: &str = "https://github.com/Allar/ue5-style-guide";
const COMMUNITY_DOCS: &str =
    "https://dev.epicgames.com/community/learning/tutorials/mX6b/unreal-engine-project-structure-naming-conventions";
const FOLDER_STRUCTURE_DOCS: &str =
    "https://unrealdirective.com/resources/project-standards/folder-structure/";
const LYRA_DOCS: &str = "https://dev.epicgames.com/community/learning/paths/Z4/lyra-starter-game";

const GUIDES: &[Guide] = &[
    Guide {
        id: "allar",
        label: "Allar / Gamemakin",
        url: Some(ALLAR_DOCS),
        flags: EVERY_CATEGORY,
        except: ALLAR_EXCEPT,
    },
    Guide {
        id: "community",
        label: "Community",
        url: Some(COMMUNITY_DOCS),
        flags: EVERY_CATEGORY,
        except: NO_EXCEPTIONS,
    },
    // Minimal is Allar's set trimmed further, so it documents itself with Allar's guide.
    Guide {
        id: "minimal",
        label: "Minimal",
        url: Some(ALLAR_DOCS),
        flags: EVERY_CATEGORY,
        except: MINIMAL_EXCEPT,
    },
    // The guides below are the ones that are common for UE5 without being Allar's, taken at the
    // points where they actually disagree with it. Unreal Directive names its structure "by
    // purpose" and "modular"; both reduce to the same answers here.
    Guide {
        id: "unrealdirective",
        label: "Unreal Directive",
        url: Some(FOLDER_STRUCTURE_DOCS),
        flags: EVERY_CATEGORY,
        except: UNREAL_DIRECTIVE_EXCEPT,
    },
    Guide {
        id: "lyra",
        label: "Lyra",
        url: Some(LYRA_DOCS),
        flags: EVERY_CATEGORY,
        except: LYRA_EXCEPT,
    },
    Guide {
        id: "bytype",
        label: "By asset type",
        url: Some(FOLDER_STRUCTURE_DOCS),
        flags: EVERY_CATEGORY,
        except: BY_TYPE_EXCEPT,
    },
];

fn default_guide() -> &'static Guide {
    &GUIDES[0]
}

/* The presets the user wrote, one file per preset in a folder beside the executable so that the
   window and the command line read the same files. A preset is a combination of flags, so its
   file holds the flags it stands for, exactly as they would be typed:

       --naming --structure --except 2.6.3

   Dropping a file in adds a preset and deleting one removes it; the app does the same through
   `save_preset` and `delete_preset`. */

const PRESET_FOLDER: &str = "presets";
const PRESET_EXT: &str = "preset";

struct FilePreset {
    id: String,
    // Why the file cannot be used, if it cannot. Kept rather than dropped, so that asking for a
    // broken preset says what is wrong with it instead of "unknown guide".
    problem: Option<String>,
    flags: Vec<String>,
    except: Vec<String>,
}

impl Preset for FilePreset {
    fn has(&self, rule: &str) -> bool {
        self.problem.is_none() && covers(&self.flags, rule) && !covers(&self.except, rule)
    }

    fn problem(&self) -> Option<&str> {
        self.problem.as_deref()
    }
}

impl FilePreset {
    /// The file body as the flags it stands for. Blank lines and `#` comments are ignored, and
    /// every token has to be one the flags accept, so a file cannot ask for a rule that is gone.
    fn parse(id: &str, text: &str) -> FilePreset {
        let mut flags: Vec<String> = Vec::new();
        let mut except: Vec<String> = Vec::new();
        let body = text
            .lines()
            .map(|line| line.split('#').next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(" ");
        let mut tokens = body.split_whitespace().map(str::to_string);

        let problem = loop {
            let Some(flag) = tokens.next() else { break None };
            let excludes = flag == "--except";
            match flag.as_str() {
                "--naming" => flags.push("naming".to_string()),
                "--structure" => flags.push("structure".to_string()),
                "--rules" | "--rule" | "--except" => {
                    let Some(list) = tokens.next() else {
                        break Some(format!("{flag} needs a list of rules"));
                    };
                    match tokens_of(&list) {
                        Ok(found) if excludes => except.extend(found),
                        Ok(found) => flags.extend(found),
                        Err(token) => break Some(format!("unknown rule \"{token}\"")),
                    }
                }
                other => break Some(format!("unknown flag \"{other}\"")),
            }
        };

        FilePreset { id: id.to_string(), problem, flags, except }
    }
}

/// The preset a name refers to. A file beside the executable wins over a shipped guide of the same
/// name, which is how a team pins a default locally.
fn find_preset<'a>(name: &str, folder: &'a [FilePreset]) -> Option<&'a dyn Preset> {
    if let Some(preset) = folder.iter().find(|preset| preset.id == name) {
        return Some(preset);
    }
    GUIDES.iter().find(|guide| guide.id == name).map(|guide| guide as &dyn Preset)
}

/// The folder the presets live in: `presets/` beside the executable, so that an installed copy and
/// a development build each keep their own, and the command line finds what the window wrote.
fn presets_dir() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(PRESET_FOLDER))
}

/// Every preset in `dir`, in name order so the picker and `--presets` read the same way.
fn load_presets_from(dir: &Path) -> Vec<FilePreset> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut presets: Vec<FilePreset> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case(PRESET_EXT))
        })
        .filter_map(|entry| {
            let id = entry.path().file_stem()?.to_string_lossy().into_owned();
            Some(match fs::read_to_string(entry.path()) {
                Ok(text) => FilePreset::parse(&id, &text),
                Err(error) => FilePreset {
                    id,
                    problem: Some(error.to_string()),
                    flags: Vec::new(),
                    except: Vec::new(),
                },
            })
        })
        .collect();
    presets.sort_by(|a, b| a.id.cmp(&b.id));
    presets
}

/// The presets beside this executable, or none when there is no such folder.
fn load_presets() -> Vec<FilePreset> {
    presets_dir().map_or_else(Vec::new, |dir| load_presets_from(&dir))
}

fn rule_category(rule: &str) -> &'static str {
    RULES
        .iter()
        .find(|spec| spec.id == rule)
        .map_or("naming", |spec| spec.category)
}

// What the pickers ask the backend for. `None` means the default guide, which is
// also what an untouched UI sends; an explicit list names the rules that are on.
struct Rules {
    ids: Option<Vec<String>>,
}

impl Rules {
    fn on(&self, rule: &str) -> bool {
        match &self.ids {
            None => default_guide().has(rule),
            Some(ids) => ids.iter().any(|id| id == rule),
        }
    }

    fn any_on(&self) -> bool {
        match &self.ids {
            None => !default_guide().flags.is_empty(),
            Some(ids) => !ids.is_empty(),
        }
    }
}

/* The catalog the front end renders from: the rules split into the two categories,
   and the guides as presets over them. */

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogRule {
    id: &'static str,
    label: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogCategory {
    id: &'static str,
    rules: Vec<CatalogRule>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogGuide {
    id: String,
    label: String,
    url: Option<String>,
    rules: Vec<String>,
    // Written by the user rather than shipped, so the picker can offer to delete it.
    custom: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Catalog {
    categories: Vec<CatalogCategory>,
    guides: Vec<CatalogGuide>,
}

#[tauri::command]
fn rule_catalog() -> Catalog {
    let mut categories = Vec::new();
    for category in ["naming", "structure"] {
        categories.push(CatalogCategory {
            id: category,
            rules: RULES
                .iter()
                .filter(|spec| spec.category == category)
                .map(|spec| CatalogRule { id: spec.id, label: spec.label })
                .collect(),
        });
    }
    // The shipped presets first, in the order the table lists them, then whatever is in the folder.
    // A file of the same name replaces that row rather than appearing alongside it, so the picker
    // shows one row per name either way.
    let mut guides: Vec<CatalogGuide> = GUIDES
        .iter()
        .map(|guide| CatalogGuide {
            id: guide.id.to_string(),
            label: guide.label.to_string(),
            url: guide.url.map(str::to_string),
            rules: rules_of(guide),
            custom: false,
        })
        .collect();
    for preset in load_presets() {
        let label = match preset.problem {
            Some(_) => format!("{} (cannot be used)", preset.id),
            None => preset.id.clone(),
        };
        let row = CatalogGuide {
            id: preset.id.clone(),
            label,
            url: None,
            rules: rules_of(&preset),
            custom: true,
        };
        match guides.iter_mut().find(|guide| guide.id == preset.id) {
            Some(shipped) => *shipped = row,
            None => guides.push(row),
        }
    }

    Catalog { categories, guides }
}

/// Opens a guide's documentation in the system browser. The URL is looked up in the guide table
/// rather than taken from the caller, so the front end cannot ask the OS to open an arbitrary
/// string. A preset read from the folder has no documentation unless its file claims one.
#[tauri::command]
fn open_docs(guide: String) -> Result<(), String> {
    let folder = load_presets();
    let url = find_preset(&guide, &folder)
        .and_then(|preset| preset.docs())
        .ok_or_else(|| format!("no documentation for guide \"{guide}\""))?;
    open_external(url).map_err(|error| format!("could not open {url}: {error}"))
}

/// The file a preset name maps to, refusing anything that is not a plain file stem so that a name
/// cannot reach outside the folder.
fn preset_file(dir: &Path, name: &str) -> Result<PathBuf, String> {
    let plain = !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !plain {
        return Err(format!(
            "\"{name}\" is not a preset name; use letters, digits, dash and underscore"
        ));
    }
    Ok(dir.join(format!("{name}.{PRESET_EXT}")))
}

/// Writes a preset into `dir`, in the same flag form a preset file is read from, so that saving one
/// and reading it back give the same rules.
fn write_preset(dir: &Path, name: &str, flags: &[String], except: &[String]) -> Result<(), String> {
    for token in flags.iter().chain(except.iter()) {
        if !known_token(token) {
            return Err(format!("unknown rule \"{token}\""));
        }
    }
    let file = preset_file(dir, name)?;
    fs::create_dir_all(dir).map_err(|error| format!("could not make {}: {error}", dir.display()))?;
    fs::write(&file, format!("{}\n", flags_text(flags, except)))
        .map_err(|error| format!("could not write {}: {error}", file.display()))
}

/// Removes a preset from `dir`. A name that is no longer there is not an error, since the file may
/// have been deleted by hand.
fn remove_preset(dir: &Path, name: &str) -> Result<(), String> {
    let file = preset_file(dir, name)?;
    match fs::remove_file(&file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("could not remove {}: {error}", file.display())),
    }
}

#[tauri::command]
fn save_preset(name: String, flags: Vec<String>, except: Vec<String>) -> Result<(), String> {
    let dir = presets_dir().ok_or("there is no folder beside the executable to save into")?;
    write_preset(&dir, &name, &flags, &except)
}

#[tauri::command]
fn delete_preset(name: String) -> Result<(), String> {
    let dir = presets_dir().ok_or("there is no folder beside the executable to delete from")?;
    remove_preset(&dir, &name)
}

// A webview cannot open the system browser itself: navigating it would replace the app
// with the web page, so the OS opener is asked instead.
#[cfg(target_os = "windows")]
fn open_external(url: &str) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    // Without this the console host flashes a window on every click.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map(|_| ())
}

#[cfg(target_os = "macos")]
fn open_external(url: &str) -> std::io::Result<()> {
    std::process::Command::new("open").arg(url).spawn().map(|_| ())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_external(url: &str) -> std::io::Result<()> {
    std::process::Command::new("xdg-open").arg(url).spawn().map(|_| ())
}

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

/// Whether a name reads as PascalCase: a leading capital, and nothing but letters and digits after
/// it. A part that starts with a digit - an ordinal like `1st`, or a variant like `01` - is not a
/// word, so there is no casing to ask of it.
fn is_pascal_case(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_uppercase() || first.is_ascii_digit())
}

// 2.1.2 and 2.1.3 overlap, so report only the most specific one that applies. Casing is no
// longer part of this: it is 00.3, asked of folder names and asset names in the same words.
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
    } else {
        None
    }
}

// The longest Allar 1.2 prefix the name begins with, if any.
fn known_prefix(base: &str) -> Option<&'static str> {
    ASSET_KINDS
        .iter()
        .map(|(prefix, ..)| *prefix)
        .filter(|prefix| base.starts_with(prefix))
        .max_by_key(|prefix| prefix.len())
}

// 00.2: Windows reserves CON, PRN, AUX, NUL and the COM1-9 / LPT1-9 ports for any
// extension, so the dot-stripped stem is what gets tested.
fn reserved_device_name(base: &str) -> bool {
    let upper = base.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL"
            | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9"
            | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9"
    )
}

// 1.1, the base asset name pattern Prefix_BaseAssetName_Variant_Suffix, using
// only letters, digits and underscore. It checks known naming prefixes, not whether
// the package's actual class matches the prefix.
fn naming_issues(name: &str, rel: &str) -> Vec<(&'static str, String)> {
    let base = base_name(name);
    let mut found = Vec::new();

    if let Some(bad) = base.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '_') {
        found.push((
            "00.1",
            format!("\"{rel}\" contains '{bad}'; only letters, digits and underscore are allowed."),
        ));
    }

    // 00.2: a reserved device name or a name Windows would silently trim is a
    // portability fault the character rule cannot see.
    if name.starts_with(' ') || name.ends_with(' ') {
        found.push((
            "00.2",
            format!("\"{rel}\" has a leading or trailing space, which Windows strips."),
        ));
    }
    if reserved_device_name(base) {
        found.push((
            "00.2",
            format!("\"{rel}\" is a reserved Windows device name, which Windows refuses to create."),
        ));
    }

    let prefix = known_prefix(base);
    if prefix.is_none() && !is_umap(name) {
        found.push((
            "1.1",
            format!("\"{rel}\" does not start with a recognised asset-type prefix."),
        ));
    }

    let rest = match prefix {
        Some(p) => &base[p.len()..],
        None => base,
    };
    for segment in rest.split('_').filter(|s| !s.is_empty()) {
        if !segment.chars().all(|c| c.is_ascii_alphanumeric()) {
            continue; // already reported by the character rule above
        }
        if segment.chars().all(|c| c.is_ascii_digit()) {
            if segment.len() != 2 {
                found.push((
                    "1.1",
                    format!(
                        "\"{rel}\" uses the variant \"{segment}\"; variants are two digits, like _01."
                    ),
                ));
            }
        } else if !is_pascal_case(segment) {
            // 00.3, the one casing rule, rather than 1.1: the base name's shape is 1.1's job
            found.push(("00.3", format!("\"{rel}\" has \"{segment}\", which is not PascalCase.")));
        }
    }

    found
}

fn flag(
    out: &mut Vec<Violation>,
    issues: &mut Vec<Issue>,
    rules: &Rules,
    rule: &str,
    message: String,
    path: &str,
) {
    // A rule the pickers have switched off is not reported at all, so nothing
    // downstream has to know to hide it.
    if !rules.on(rule) {
        return;
    }
    let category = rule_category(rule);
    issues.push(Issue {
        rule: rule.to_string(),
        message: message.clone(),
        category,
    });
    out.push(Violation {
        rule: rule.to_string(),
        message,
        path: path.to_string(),
        category,
    });
}

fn lint_dir(node: &mut Node, ctx: &Ctx, rules: &Rules, refs: &References, out: &mut Vec<Violation>) -> u64 {
    let root = ctx.rel.is_empty();
    let exempt = ctx.exempt
        || node.name == "Developers"
        || node.name == "Python"
        // World Partition writes these beside the map; engine-generated names are
        // not the project's to answer for.
        || node.name == "__ExternalActors__"
        || node.name == "__ExternalObjects__";
    let in_maps = ctx.under_maps || node.name == "Maps";
    let in_matlib = ctx.under_matlib || node.name == "MaterialLibrary";
    let in_content = node.name == "Content";

    let mut issues: Vec<Issue> = Vec::new();

    if !exempt {
        // The scanned root's own name is the user's choice, not a project folder.
        if !root {
            if let Some((rule, message)) = folder_name_issue(&node.name) {
                flag(out, &mut issues, rules, rule, message, &ctx.rel);
            }
            // 00.3 is asked of folder names too, so that casing is one flag wherever a name is
            // written rather than something folded into the folder-character rules.
            if !is_pascal_case(&node.name) {
                flag(
                    out,
                    &mut issues,
                    rules,
                    "00.3",
                    format!("\"{}\" is not PascalCase.", ctx.rel),
                    &ctx.rel,
                );
            }
            if node.name == "Assets" {
                flag(
                    out,
                    &mut issues,
                    rules,
                    "2.6.1",
                    format!("\"{}\" is a redundant type folder; all assets are assets.", ctx.rel),
                    &ctx.rel,
                );
            } else if TYPE_FOLDERS.contains(&node.name.as_str()) {
                flag(
                    out,
                    &mut issues,
                    rules,
                    "2.6.2",
                    format!(
                        "\"{}\" is a type folder; asset name prefixes already convey the type.",
                        ctx.rel
                    ),
                    &ctx.rel,
                );
            } else if EXTENDED_TYPE_FOLDERS.contains(&node.name.as_str()) {
                flag(
                    out,
                    &mut issues,
                    rules,
                    "2.6.3",
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
                    rules,
                    "2.9",
                    format!("\"{}\" is an empty folder.", ctx.rel),
                    &ctx.rel,
                );
            }
        }
    }

    if !exempt {
        // 2.1.4: sibling folders differing only by case collide on Windows and in
        // case-insensitive source control, so each repeat after the first is
        // flagged.
        let mut seen: Vec<(String, String)> = Vec::new();
        for child in &node.children {
            let lower = child.name.to_ascii_lowercase();
            match seen.iter().find(|(l, _)| *l == lower) {
                Some((_, prior)) => {
                    let path = if root {
                        child.name.clone()
                    } else {
                        format!("{}/{}", ctx.rel, child.name)
                    };
                    flag(
                        out,
                        &mut issues,
                        rules,
                        "2.1.4",
                        format!(
                            "\"{path}\" collides with sibling \"{prior}\"; folder names must differ by more than case."
                        ),
                        &path,
                    );
                }
                None => seen.push((lower, child.name.clone())),
            }
        }
    }

    if !exempt {
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
            // 3.1: what an asset references belongs in its own folder or one inside it, never in a
            // sibling folder. A pair is reported by whichever of the two comes first in path order,
            // so a reference and the answer to it cannot report the same pair twice.
            for target in refs.get(&rel).into_iter().flatten() {
                if rel.as_str() >= target.as_str() {
                    continue;
                }
                if at_or_below(folder_of(&rel), folder_of(target))
                    || at_or_below(folder_of(target), folder_of(&rel))
                {
                    continue;
                }
                found.push((
                    "3.1",
                    format!(
                        "\"{rel}\" references \"{target}\", so they belong together: keep them in one folder or one inside the other."
                    ),
                ));
            }
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
            found.extend(
                naming_issues(&file.name, &rel)
                    .into_iter()
                    .filter(|(rule, _)| rules.on(rule)),
            );
            for (rule, message) in found {
                flag(out, &mut file.issues, rules, rule, message, &rel);
            }
        }
    }

    let own =
        issues.len() as u64 + node.files.iter().map(|f| f.issues.len() as u64).sum::<u64>();
    let own_naming = issues
        .iter()
        .chain(node.files.iter().flat_map(|f| f.issues.iter()))
        .filter(|issue| issue.category == "naming")
        .count() as u64;
    let mut nested = 0u64;
    let mut nested_naming = 0u64;
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
        nested += lint_dir(child, &child_ctx, rules, refs, out);
        nested_naming += child.naming_violations;
    }

    node.issues = issues;
    node.violations = own + nested;
    node.naming_violations = own_naming + nested_naming;
    node.structure_violations = (own - own_naming) + (nested - nested_naming);
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
fn scan_directory(path: String, rules: Option<Vec<String>>) -> Result<Payload, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(format!("not a directory: {path}"));
    }

    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let looks_unreal = looks_like_unreal(&root, &name);
    // The pickers send the rules they want; with nothing sent, check everything,
    // but only when the folder looks like Unreal content.
    let rules = match rules {
        Some(ids) => Rules { ids: Some(ids) },
        None if looks_unreal => Rules { ids: None },
        None => Rules { ids: Some(Vec::new()) },
    };
    let lint_applied = rules.any_on();

    let mut tree = scan_dir(&root, name).map_err(|e| format!("could not read {path}: {e}"))?;

    // References are only worth reading when something is going to ask about them.
    let references = if lint_applied { scan_references(&root, &tree) } else { References::new() };
    attach_references(&mut tree, "", &references);

    let mut violations = Vec::new();
    if lint_applied {
        let ctx = Ctx {
            rel: String::new(),
            exempt: false,
            under_maps: false,
            under_matlib: false,
        };
        lint_dir(&mut tree, &ctx, &rules, &references, &mut violations);
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
    // `--check` runs the same checks as a one-shot report and exits, so the binary is
    // usable from a script; anything else opens the window.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = cli::run(&args) {
        std::process::exit(code);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            startup_directory,
            default_directory,
            rule_catalog,
            open_docs,
            save_preset,
            delete_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running the application");
}

#[cfg(test)]
mod tests {
    use super::{
        asset_kind, asset_kind_with_bytes, at_or_below, cli, default_guide, find_preset, folder_of,
        is_pascal_case,
        known_token, lint_dir,
        naming_issues, open_docs, fs, load_presets_from, preset_file, remove_preset,
        reserved_device_name, resolve, rule_category, rules_of, rule_is_opt_in, write_preset, Ctx,
        FileEntry, FilePreset, Node, Path, Preset, References, Rules, Violation, ALLAR_EXCEPT,
        GUIDES,
        OPT_IN_RULES, RULES,
    };

    /// A preset by name with no folder in the way, which is the table the tool ships on its own.
    fn shipped(name: &str) -> &'static dyn Preset {
        find_preset(name, &[]).expect("a shipped preset")
    }

    #[test]
    fn a_preset_file_is_the_flags_it_stands_for() {
        // the file allar is spelled with is the preset allar, rule for rule
        let mine = FilePreset::parse("allar", "--naming --structure --except 00.2,2.1.4,2.6.3\n");
        assert_eq!(mine.problem, None);
        assert_eq!(rules_of(&mine), rules_of(shipped("allar")));

        // blank lines and comments are ignored, so a file can explain itself
        let commented = FilePreset::parse("x", "# the two that matter\n\n--rules 00.1,2.9\n");
        assert_eq!(commented.problem, None);
        assert_eq!(commented.flags, ["00.1", "2.9"]);
        assert_eq!(rules_of(&commented).len(), 2);

        // and a file that will not parse says which token is wrong
        assert_eq!(
            FilePreset::parse("x", "--naming --nope").problem.as_deref(),
            Some("unknown flag \"--nope\"")
        );
        assert_eq!(
            FilePreset::parse("x", "--rules 9.9").problem.as_deref(),
            Some("unknown rule \"9.9\"")
        );
        assert_eq!(
            FilePreset::parse("x", "--except").problem.as_deref(),
            Some("--except needs a list of rules")
        );
        // a broken preset selects nothing, rather than quietly selecting everything
        assert!(!FilePreset::parse("x", "--nope").has("1.1"));
        assert!(rules_of(&FilePreset::parse("x", "--nope")).is_empty());
    }

    #[test]
    fn a_file_of_a_shipped_name_takes_its_place() {
        let folder = [FilePreset::parse("allar", "--rules 2.9")];
        let overridden = find_preset("allar", &folder).expect("found");
        assert!(overridden.has("2.9"));
        // the shipped allar is still the wider set it always was, and the file has taken its place
        assert!(shipped("allar").has("1.1"));
        assert!(!overridden.has("1.1"));
        // a name that is in neither place is not found at all
        assert!(find_preset("nope", &[]).is_none());
    }

    #[test]
    fn a_preset_name_stays_in_its_own_folder() {
        let dir = Path::new("presets");
        assert_eq!(preset_file(dir, "team").expect("plain"), dir.join("team.preset"));
        for name in ["", ".hidden", "../escape", "a/b", "a\\b", "with space", "dot.ted"] {
            assert!(preset_file(dir, name).is_err(), "{name:?} should be refused");
        }
    }

    #[test]
    fn a_saved_preset_reads_back_the_same() {
        let dir = std::env::temp_dir().join("unreal-directory-scanner-tests").join("presets");
        let _ = fs::remove_dir_all(&dir);

        // saving writes the flags, and reading the folder gives the same selection back
        let flags = vec!["naming".to_string(), "structure".to_string()];
        let except = vec!["2.6.3".to_string()];
        write_preset(&dir, "team", &flags, &except).expect("saved");
        let folder = load_presets_from(&dir);
        assert_eq!(folder.len(), 1);
        assert_eq!(folder[0].id, "team");
        assert_eq!(folder[0].problem, None);
        assert_eq!(folder[0].flags, flags);
        assert_eq!(folder[0].except, except);
        assert_eq!(rules_of(&folder[0]), resolve(None, &flags, &except));

        // removing it takes it away, and removing it twice is not an error
        remove_preset(&dir, "team").expect("removed");
        assert!(load_presets_from(&dir).is_empty());
        remove_preset(&dir, "team").expect("removing twice is fine");

        // a name that would leave the folder is refused, and nothing is written
        assert!(write_preset(&dir, "../escape", &flags, &except).is_err());
        assert!(remove_preset(&dir, "../escape").is_err());
        assert!(load_presets_from(&dir).is_empty());
    }

    fn kind(name: &str, asset: bool) -> (String, String, String) {
        let (family, label, type_name) = asset_kind(name, asset);
        (family.to_string(), label, type_name)
    }

    fn kind_with_bytes(name: &str, bytes: &[u8]) -> (String, String, String) {
        let (family, label, type_name) = asset_kind_with_bytes(name, true, Some(bytes));
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
        assert_eq!(kind("Cue_Hammer.uasset", true), ("audio".into(), "Cue".into(), "Sound Cue".into()));
        assert_eq!(kind("SKM_Hero.uasset", true), ("mesh".into(), "SKM".into(), "Skeletal Mesh".into()));
        assert_eq!(kind("PA_Hero.uasset", true), ("mesh".into(), "PA".into(), "Physics Asset".into()));
        assert_eq!(kind("SA_Steps.uasset", true), ("audio".into(), "SA".into(), "Sound Attenuation".into()));
        assert_eq!(kind("BTT_Attack.uasset", true), ("ai".into(), "BTT".into(), "Behavior Tree Task".into()));
        assert_eq!(kind("BTS_Sight.uasset", true), ("ai".into(), "BTS".into(), "Behavior Tree Service".into()));
        assert_eq!(kind("BTD_Range.uasset", true), ("ai".into(), "BTD".into(), "Behavior Tree Decorator".into()));
        assert_eq!(kind("BPC_Inventory.uasset", true), ("blueprint".into(), "BPC".into(), "Blueprint Component".into()));
        assert_eq!(kind("BI_Interactable.uasset", true), ("blueprint".into(), "BI".into(), "Blueprint Interface".into()));
        assert_eq!(kind("ST_Item.uasset", true), ("blueprint".into(), "ST".into(), "Structure".into()));
        assert_eq!(kind("AS_Run.uasset", true), ("animation".into(), "AS".into(), "Animation Sequence".into()));
        assert_eq!(kind("C_Fire.uasset", true), ("data".into(), "C".into(), "Curve".into()));
    }

    #[test]
    fn serialized_classes_type_only_unprefixed_unambiguous_packages() {
        assert_eq!(
            kind_with_bytes("Button.uasset", b"SoundWave"),
            ("audio".into(), "SW".into(), "Sound Wave".into())
        );
        assert_eq!(
            kind_with_bytes("content_tree.uasset", b"Texture2D"),
            ("texture".into(), "T2D".into(), "Texture 2D".into())
        );
        assert_eq!(
            kind_with_bytes("AssetZoo_Thing.uasset", b"SoundWave SoundCue"),
            ("other".into(), "?".into(), "Unrecognised prefix".into())
        );
        assert_eq!(
            kind_with_bytes("AssetZoo_Thing.uasset", b"SoundWaveProxy"),
            ("other".into(), "?".into(), "Unrecognised prefix".into())
        );
        assert_eq!(
            kind_with_bytes(
                "C_FireAtlas.uasset",
                b"CurveLinearColorAtlas CurveLinearColor",
            ),
            ("data".into(), "C".into(), "Curve Atlas".into())
        );
        assert_eq!(
            kind_with_bytes("AssetZoo_Thing.uasset", b"SkeletalMesh PhysicsAsset"),
            ("other".into(), "?".into(), "Unrecognised prefix".into())
        );
        assert_eq!(
            kind_with_bytes("AssetZoo_Thing.uasset", b"prefixSoundWave"),
            ("other".into(), "?".into(), "Unrecognised prefix".into())
        );
        assert_eq!(
            kind_with_bytes(
                "C_FireAtlas.uasset",
                b"CurveLinearColorAtlas CurveLinearColor",
            ),
            ("data".into(), "C".into(), "Curve Atlas".into())
        );
        assert_eq!(
            kind_with_bytes("C_Fire.uasset", b"CurveLinearColor"),
            ("data".into(), "C".into(), "Color Curve".into())
        );
        assert_eq!(
            kind_with_bytes("F_Gothic.uasset", b"Font"),
            ("ui".into(), "F".into(), "Font".into())
        );
        // A class hint types an asset, but does not invent a naming prefix for rule 1.1.
        assert!(rules("Button.uasset").contains(&"1.1"));
        assert_eq!(
            kind_with_bytes("SpikeTrap.uasset", b"StaticMesh Blueprint BlueprintGeneratedClass"),
            ("blueprint".into(), "BP".into(), "Blueprint".into())
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

    fn rules(name: &str) -> Vec<&'static str> {
        naming_issues(name, name).into_iter().map(|(rule, _)| rule).collect()
    }

    #[test]
    fn naming_accepts_conventional_asset_names() {
        assert!(rules("T_Rock_D.uasset").is_empty());
        assert!(rules("MI_RedProjectile.uasset").is_empty());
        for name in [
            "Cue_EnemyFootstep.uasset",
            "SKM_MeleeDummy.uasset",
            "PA_MeleeDummy.uasset",
            "SA_Medium.uasset",
            "BTT_BasicAttack.uasset",
            "BTS_TankFacing.uasset",
            "BTD_IsPlayerTooClose.uasset",
            "BPC_WeaponManager.uasset",
            "BI_Damageable.uasset",
            "ST_EnemyData.uasset",
            "AS_Run.uasset",
        ] {
            assert!(rules(name).is_empty(), "{name} uses a recognised project prefix");
        }
        assert!(rules("BP_AirCollideComponent.uasset").is_empty());
        assert!(rules("BTDecorator_IsPlayerTooClose.uasset").is_empty());
        // a chained modifier after the prefix is fine, and maps need no prefix
        assert!(rules("M_PP_Highlight.uasset").is_empty());
        assert!(rules("ArenaLevel1.umap").is_empty());
    }

    #[test]
    fn naming_flags_broken_asset_names() {
        assert_eq!(rules("MyAsset.uasset"), ["1.1"]); // no recognised prefix
        assert_eq!(rules("Button.uasset"), ["1.1"]); // an inferred type is not a name prefix
        assert_eq!(rules("T_rock.uasset"), ["00.3"]); // lowercase base name: the casing rule
        assert_eq!(rules("T_Rock_1.uasset"), ["1.1"]); // one-digit variant
        // a space is reported once, by 00.1, not again as a segment problem
        assert_eq!(rules("T_Rock Name.uasset"), ["00.1"]);
    }

    #[test]
    fn an_ordinal_is_not_asked_to_be_pascal_case() {
        // `1st` and `2nd` are parts of ordinary animation names, not words with a casing to
        // capitalise, so 00.3 has nothing to say about them.
        for name in ["A_Hammer_Attack_1st_Anim.uasset", "A_Hammer_Attack_2nd_Anim.uasset"] {
            assert!(!rules(name).contains(&"00.3"), "{name} is not a casing violation");
        }

        // a word's casing is still asked for, and a one-digit variant is still 1.1's business
        assert!(rules("A_Hammer_attack_Anim.uasset").contains(&"00.3"));
        assert!(rules("A_Hammer_Attack_1_Anim.uasset").contains(&"1.1"));

        // and the question itself: a part starting with a digit is not a word, in a folder name or
        // an asset name alike
        assert!(is_pascal_case("1st") && is_pascal_case("01") && is_pascal_case("3rdPlace"));
        assert!(!is_pascal_case("rock") && !is_pascal_case("rock_1") && !is_pascal_case(""));
    }

    #[test]
    fn rules_split_into_three_categories() {
        assert_eq!(rule_category("00.1"), "naming");
        assert_eq!(rule_category("1.1"), "naming");
        assert_eq!(rule_category("2.1.2"), "structure");
        assert_eq!(rule_category("2.9"), "structure");
        assert_eq!(rule_category("3.1"), "association");
    }

    fn leaf_file(name: &str) -> FileEntry {
        let (family, label, type_name) = asset_kind(name, true);
        FileEntry {
            name: name.to_string(),
            size: 1,
            asset: true,
            issues: Vec::new(),
            type_family: family.to_string(),
            type_label: label,
            type_name,
            references: Vec::new(),
        }
    }

    // Content/Haeretica/M_Loose.uasset: a base material outside MaterialLibrary, so
    // 2.8 fires and nothing else does.
    fn tree_with_one_offender() -> Node {
        Node {
            name: "Content".to_string(),
            is_dir: true,
            files: Vec::new(),
            children: vec![Node {
                name: "Haeretica".to_string(),
                is_dir: true,
                files: vec![leaf_file("M_Loose.uasset")],
                children: Vec::new(),
                assets: 1,
                total: 1,
                bytes: 1,
                issues: Vec::new(),
                violations: 0,
                naming_violations: 0,
                structure_violations: 0,
            }],
            assets: 1,
            total: 1,
            bytes: 1,
            issues: Vec::new(),
            violations: 0,
            naming_violations: 0,
            structure_violations: 0,
        }
    }

    fn lint_with(ids: Option<Vec<String>>) -> Vec<Violation> {
        let mut tree = tree_with_one_offender();
        let ctx = Ctx {
            rel: String::new(),
            exempt: false,
            under_maps: false,
            under_matlib: false,
        };
        let mut out = Vec::new();
        lint_dir(&mut tree, &ctx, &Rules { ids }, &References::new(), &mut out);
        out
    }

    #[test]
    fn a_disabled_rule_reports_nothing_at_all() {
        let everything = lint_with(None);
        assert_eq!(everything.len(), 1);
        assert_eq!(everything[0].rule, "2.8");

        // The same tree with 2.8 switched off, as unchecking it in the picker does:
        // the violation has to disappear rather than merely be hidden by the UI.
        let without_28 = lint_with(Some(vec!["2.9".to_string(), "1.1".to_string()]));
        assert!(!without_28.iter().any(|v| v.rule == "2.8"));
        assert_eq!(without_28.len(), 0);

        // and switching everything off reports nothing while still running
        let nothing = lint_with(Some(Vec::new()));
        assert_eq!(nothing.len(), 0);
    }

    #[test]
    fn violations_are_rolled_up_per_category() {
        let mut tree = tree_with_one_offender();
        let ctx = Ctx {
            rel: String::new(),
            exempt: false,
            under_maps: false,
            under_matlib: false,
        };
        let mut out = Vec::new();
        lint_dir(&mut tree, &ctx, &Rules { ids: None }, &References::new(), &mut out);

        // the only offender is the base material, which is a structure rule, and
        // the name itself is fine, so the naming roll-up stays empty
        assert_eq!(tree.children[0].structure_violations, 1);
        assert_eq!(tree.children[0].naming_violations, 0);
        assert_eq!(tree.structure_violations, 1);
        assert_eq!(tree.naming_violations, 0);
        assert_eq!(tree.violations, 1);
    }

    fn dir(name: &str, children: Vec<Node>, files: Vec<FileEntry>) -> Node {
        Node {
            name: name.to_string(),
            is_dir: true,
            files,
            children,
            assets: 0,
            total: 0,
            bytes: 0,
            issues: Vec::new(),
            violations: 0,
            naming_violations: 0,
            structure_violations: 0,
        }
    }

    fn fired(tree: &mut Node, ids: &[&str]) -> Vec<String> {
        let ctx = Ctx {
            rel: String::new(),
            exempt: false,
            under_maps: false,
            under_matlib: false,
        };
        let ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        let mut out = Vec::new();
        lint_dir(tree, &ctx, &Rules { ids: Some(ids) }, &References::new(), &mut out);
        out.into_iter().map(|v| v.rule).collect()
    }

    #[test]
    fn every_preset_is_a_combination_of_flags() {
        for guide in GUIDES {
            // every token, on either side, has to be one the flags accept
            for token in guide.flags.iter().chain(guide.except) {
                assert!(known_token(token), "{} names the unknown flag \"{token}\"", guide.id);
            }

            let rules = rules_of(guide);
            assert!(!rules.is_empty(), "{} selects nothing", guide.id);
            // both categories have to come out of it, however the preset is spelled
            for category in ["naming", "structure"] {
                assert!(
                    rules.iter().any(|rule| rule_category(rule) == category),
                    "{} covers no {category} rule",
                    guide.id
                );
            }
            // and the flat list the pickers get is exactly what `has` answers per rule
            for rule in RULES {
                assert_eq!(
                    rules.iter().any(|chosen| chosen == rule.id),
                    guide.has(rule.id),
                    "{} disagrees about {}",
                    guide.id,
                    rule.id
                );
            }
        }

        // community names everything a guide describes with the two category flags, and subtracts
        // nothing; the rules no guide describes are the ones that are not asked for
        assert_eq!(
            rules_of(shipped("community")).len(),
            RULES.iter().filter(|spec| !rule_is_opt_in(spec.id)).count()
        );
        // and the default is a preset like any other
        assert_eq!(rules_of(default_guide()).len(), 11);
    }

    #[test]
    fn a_preset_can_be_spelled_either_way() {
        // the presets subtract from everything; these are the same sets listed out by hand
        let allar = [
            "00.1", "00.3", "1.1", "2.1.2", "2.1.3", "2.2.1", "2.4", "2.6.1", "2.6.2", "2.8",
            "2.9",
        ];
        let minimal = ["00.1", "00.3", "1.1", "2.1.2", "2.1.3", "2.2.1", "2.4", "2.9"];

        assert_eq!(rules_of(shipped("allar")), allar);
        assert_eq!(rules_of(shipped("minimal")), minimal);

        // and naming those rules one by one resolves to the very same set
        let none: &[&str] = &[];
        assert_eq!(resolve(None, &allar, none), allar);
        assert_eq!(resolve(None, &minimal, none), minimal);
    }

    #[test]
    fn a_guide_leaves_out_what_its_own_rules_disagree_with() {
        // What each of the other guides disagrees with Allar about, and nothing else. Each of them
        // is a whole style guide; what this scanner can take from one is the answers it gives about
        // folders, the rest of it being a table of asset prefixes it does not tell apart.
        let cases: [(&str, &[&str]); 3] = [
            ("unrealdirective", &["2.4", "2.6.2", "2.8"]),
            ("lyra", &["2.2.1", "2.8"]),
            ("bytype", &["2.6.2"]),
        ];
        for (id, own) in cases {
            let guide = GUIDES.iter().find(|guide| guide.id == id).expect("a shipped guide");
            let left_out: Vec<&str> =
                RULES.iter().map(|spec| spec.id).filter(|rule| !guide.has(rule)).collect();
            let mut expected = ALLAR_EXCEPT.to_vec();
            expected.extend(own);
            // and the rules no guide asks for, which no preset turns on
            expected.extend(OPT_IN_RULES);
            assert_eq!(left_out.len(), expected.len(), "{id} leaves out the wrong number of rules");
            for rule in expected {
                assert!(left_out.contains(&rule), "{id} should leave out {rule}");
            }
        }
    }

    #[test]
    fn exclusions_take_rules_away() {
        let none: &[&str] = &[];
        let allar = shipped("allar");

        // trimming a preset is exactly its own set less what was excluded
        let trimmed = resolve(Some(allar), none, &["2.9"]);
        assert!(!trimmed.iter().any(|rule| rule == "2.9"));
        assert_eq!(trimmed.len(), rules_of(allar).len() - 1);

        // the other way round: name everything, then subtract. A category token takes the
        // whole category with it.
        let named = resolve(None, &["naming", "structure"], &["2.6.3", "naming"]);
        assert!(named.iter().all(|rule| rule_category(rule) == "structure"));
        assert!(!named.iter().any(|rule| rule == "2.6.3"));

        // nothing named and nothing excluded is the default preset, to the rule
        assert_eq!(resolve(Some(default_guide()), none, none), rules_of(default_guide()));

        // an exclusion may repeat, and one that excludes nothing has nothing to do
        assert_eq!(
            resolve(Some(allar), none, &["1.1", "1.1"]),
            resolve(Some(allar), none, &["1.1"])
        );
        assert_eq!(resolve(Some(allar), none, none), rules_of(allar));

        // excluding everything leaves nothing, rather than falling back to a preset
        assert!(resolve(None, &["naming", "structure"], &["naming", "structure"]).is_empty());
    }

    #[test]
    fn an_association_wants_the_two_in_one_folder_or_one_inside_the_other() {
        // the rule is about folders, so it is checked on paths rather than on a whole scan
        let cases = [
            ("A/BP_X.uasset", "A/T_X.uasset", true),
            ("A/BP_X.uasset", "A/Sub/T_X.uasset", true),
            ("A/Sub/BP_X.uasset", "A/T_X.uasset", true),
            ("A/BP_X.uasset", "B/T_X.uasset", false),
            ("A/Sub/BP_X.uasset", "A/Other/T_X.uasset", false),
            ("BP_X.uasset", "A/T_X.uasset", true),
            ("BP_X.uasset", "T_X.uasset", true),
            // a folder whose name merely starts the same is not a folder inside it
            ("AB/BP_X.uasset", "A/T_X.uasset", false),
        ];
        for (left, right, fine) in cases {
            let nested = at_or_below(folder_of(left), folder_of(right))
                || at_or_below(folder_of(right), folder_of(left));
            assert_eq!(nested, fine, "{left} against {right}");
        }
    }

    #[test]
    fn only_naming_a_rule_turns_the_opt_in_ones_on() {
        // a preset naming a whole category leaves them out
        assert!(!resolve(None, &["structure"], &[] as &[&str])
            .iter()
            .any(|rule| rule == "3.1"));
        assert!(!shipped("community").has("3.1"));

        // naming the rule itself turns it on, and an exclusion still takes it away again
        assert!(resolve(None, &["3.1"], &[] as &[&str]).contains(&"3.1".to_string()));
        assert!(!resolve(Some(shipped("allar")), &["3.1"], &["3.1"])
            .iter()
            .any(|rule| rule == "3.1"));
    }

    fn run_cli(args: &[&str]) -> Option<i32> {
        cli::run(&args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn a_preset_is_never_mixed_with_the_manual_flags() {
        // a preset is a whole rule set already, so every one of these is turned away while
        // the arguments are parsed, before any folder is touched
        let cases: [&[&str]; 5] = [
            &["--guide", "allar", "--naming"],
            &["--guide", "allar", "--structure"],
            &["--guide", "allar", "--rules", "2.4"],
            &["--guide", "minimal", "--rule", "2.9"],
            &["--guide", "allar", "--guide", "minimal"],
        ];
        for args in cases {
            assert_eq!(run_cli(args), Some(2), "{args:?} should be a usage error");
        }
    }

    #[test]
    fn the_cli_rejects_what_it_does_not_know() {
        assert_eq!(run_cli(&["--guide", "nope"]), Some(2));
        assert_eq!(run_cli(&["--rules", "9.9"]), Some(2));
        assert_eq!(run_cli(&["--nonsense"]), Some(2));
        // a rule flag with no folder is a usage error rather than a window too
        assert_eq!(run_cli(&["--guide", "community"]), Some(2));
        // and --help is not an error at all
        assert_eq!(run_cli(&["--help"]), Some(0));
    }

    /// A folder with nothing in it, so a run that gets as far as scanning finds no violations
    /// and reports 0. Under the system temp dir, made on demand.
    fn empty_dir() -> String {
        let dir = std::env::temp_dir().join("unreal-directory-scanner-tests").join("empty");
        std::fs::create_dir_all(&dir).expect("could not make the temp folder");
        dir.to_string_lossy().into_owned()
    }

    #[test]
    fn there_is_only_one_guide_at_a_time() {
        // a second --guide is turned away while the arguments are parsed, whatever it names
        assert_eq!(run_cli(&["--guide", "allar", "--guide", "minimal"]), Some(2));
        assert_eq!(run_cli(&["--guide", "community", "--guide", "allar"]), Some(2));
    }

    #[test]
    fn a_guide_may_be_trimmed_but_not_added_to() {
        let dir = empty_dir();
        let dir = dir.as_str();

        // trimming a preset, and naming a set of your own and then trimming that, are both fine
        assert_eq!(run_cli(&["--guide", "allar", "--except", "2.9", dir]), Some(0));
        assert_eq!(run_cli(&["--naming", "--structure", "--except", "2.6.3", dir]), Some(0));
        // a category name is a token like any other, so a whole category can go at once
        assert_eq!(run_cli(&["--rules", "structure", "--except", "structure", dir]), Some(0));
        // and an exclusion stands on its own, trimming the default preset
        assert_eq!(run_cli(&["--except", "2.9", dir]), Some(0));

        // adding to a preset is not allowed: it would be neither the preset nor a set of your own
        assert_eq!(run_cli(&["--guide", "allar", "--structure", dir]), Some(2));
        assert_eq!(run_cli(&["--guide", "allar", "--rules", "2.9", dir]), Some(2));
        assert_eq!(run_cli(&["--guide", "allar", "--naming", dir]), Some(2));

        // and an exclusion the flags do not know is rejected like any other list
        assert_eq!(run_cli(&["--guide", "allar", "--except", "9.9", dir]), Some(2));
    }

    #[test]
    fn pascal_case_is_one_flag_over_both_kinds_of_name() {
        // the same rule answers for an asset name and for a folder name
        assert_eq!(rules("T_rock.uasset"), ["00.3"]);

        // a folder named the way an asset is asked to be named, with a file inside it
        let a_folder = || {
            dir(
                "Content",
                vec![dir("weapons", Vec::new(), vec![leaf_file("BP_Pistol.uasset")])],
                Vec::new(),
            )
        };

        let mut cased = a_folder();
        assert_eq!(fired(&mut cased, &["00.3"]), ["00.3"]);

        // so turning casing off silences the folder too: the folder-character rules have no
        // casing left in them to complain with
        let mut uncased = a_folder();
        assert!(fired(&mut uncased, &["1.1", "2.1.2", "2.1.3"]).is_empty());

        // nor does the old folder-only casing rule still exist
        assert!(!RULES.iter().any(|spec| spec.id == "2.1.1"));
    }

    #[test]
    fn the_markup_has_a_checkbox_for_every_rule() {
        // The pickers are hand-written markup while the rules live in the table above, so hold
        // the two together here rather than only at run time, where the front end throws.
        let html = include_str!("../../web/index.html");

        for spec in RULES {
            assert!(
                html.contains(&format!("value=\"{}\"", spec.id)),
                "index.html has no checkbox for {}",
                spec.id
            );
        }

        // and no checkbox is left behind for a rule that has gone
        let boxed: Vec<&str> = html
            .split("value=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect();
        for id in &boxed {
            assert!(
                RULES.iter().any(|spec| spec.id == *id),
                "index.html has a checkbox for the unknown rule {id}"
            );
        }
        assert_eq!(boxed.len(), RULES.len(), "the markup and RULES disagree in size");
    }

    #[test]
    fn the_picker_has_no_pretend_preset() {
        // "Custom" stood for a hand-edited selection. The picker now works out which preset the
        // boxes add up to and says so, or says "(edited)" when none of them matches, so there is
        // no such entry and nothing to keep in step.
        let app = include_str!("../../src/app.ts");
        assert!(!app.contains("customGuideId"));
        assert!(app.contains("function matchingGuide"));
    }

    #[test]
    fn stored_guides_point_at_documentation() {
        for guide in GUIDES {
            let url = guide.url.expect("a stored guide needs a docs link");
            assert!(url.starts_with("https://"), "{url} should be https");
        }
        // the default has to be one of them, and the table has to resolve by id
        assert!(find_preset(default_guide().id, &[]).is_some());
    }

    #[test]
    fn open_docs_refuses_a_guide_without_documentation() {
        // only the refusal is tested: opening a real guide would launch a browser
        assert!(open_docs("custom".to_string()).is_err());
        assert!(open_docs("nope".to_string()).is_err());
    }

    #[test]
    fn reserved_windows_device_names() {
        for name in ["CON", "con", "Nul", "COM4", "lpt9"] {
            assert!(reserved_device_name(name), "{name} is reserved");
        }
        for name in ["CONSOLE", "COM", "COM10", "LPT0", "T_Console"] {
            assert!(!reserved_device_name(name), "{name} is not reserved");
        }
    }

    #[test]
    fn windows_hostile_names_are_flagged_by_00_2() {
        // a reserved stem, even though the rest of the name is well-formed
        assert_eq!(rules("CON.uasset"), ["00.2", "1.1"]);
        // a space trailing the whole name, which Windows silently strips
        assert_eq!(rules("T_Rock.uasset "), ["00.2"]);
    }

    #[test]
    fn extended_type_folders_fire_2_6_3_not_2_6_2() {
        let mut tree = dir("Content", vec![dir("Blueprints", Vec::new(), Vec::new())], Vec::new());
        let got = fired(&mut tree, &["2.6.2", "2.6.3"]);
        assert!(got.contains(&"2.6.3".to_string()));
        assert!(!got.contains(&"2.6.2".to_string()));
    }

    #[test]
    fn case_only_sibling_folders_collide() {
        let mut tree = dir(
            "Content",
            vec![
                dir("Weapons", Vec::new(), Vec::new()),
                dir("weapons", Vec::new(), Vec::new()),
            ],
            Vec::new(),
        );
        assert!(fired(&mut tree, &["2.1.4"]).contains(&"2.1.4".to_string()));
    }

    #[test]
    fn world_partition_folders_are_exempt() {
        let mut tree = dir(
            "Content",
            vec![dir("__ExternalActors__", Vec::new(), Vec::new())],
            Vec::new(),
        );
        assert!(fired(
            &mut tree,
            &["00.3", "2.1.2", "2.1.3", "2.1.4", "2.6.1", "2.6.2", "2.6.3", "2.9"]
        )
        .is_empty());
    }
}
