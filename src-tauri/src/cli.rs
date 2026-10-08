//! A one-shot report, so the same checks can run from a script or a CI job with no
//! window: `--check <folder>` prints one line per violation and exits non-zero when
//! there is anything to report.

use crate::{lint_dir, rule_category, scan_dir, Ctx, Rules, Violation};
use std::path::PathBuf;

/// Every rule the checks know about, which is also what `--help` lists and what
/// `--rules` is validated against.
const RULE_IDS: [&str; 11] = [
    "00.1", "1.1", "2.1.1", "2.1.2", "2.1.3", "2.2.1", "2.4", "2.6.1", "2.6.2", "2.8", "2.9",
];

const HELP: &str = "\
Unreal Directory Scanner

  unreal-directory-scanner [folder]          open the window (the default)
  unreal-directory-scanner --check <folder>  print the Allar violations and exit

Options for --check:
  --rules <ids>  add these rules, comma separated, and repeatable
  --naming       add every asset naming rule
  --structure    add every content directory structure rule
  -h, --help     this text

The flags add up rather than override each other: --naming --structure checks
everything, as does passing none of them.

Rules: 00.1 1.1 (naming); 2.1.1 2.1.2 2.1.3 2.2.1 2.4 2.6.1 2.6.2 2.8 2.9 (structure)

Each violation is printed as: rule <tab> path <tab> message. The summary goes to
standard error, so the output can be piped straight into another tool.

Exit status: 0 nothing found, 1 violations found, 2 bad usage.";

/// Returns the process exit code when the arguments ask for a report, or `None`
/// when they should carry on into the windowed app.
pub fn run(args: &[String]) -> Option<i32> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("{HELP}");
        return Some(0);
    }

    let mut path: Option<String> = None;
    let mut ids: Vec<String> = Vec::new();
    let mut naming = false;
    let mut structure = false;
    let mut check = false;

    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--naming" => naming = true,
            "--structure" => structure = true,
            "--rules" => {
                let Some(list) = rest.next() else {
                    eprintln!("--rules needs a comma separated list of rule ids\n\n{HELP}");
                    return Some(2);
                };
                for id in list.split(',').map(str::trim).filter(|id| !id.is_empty()) {
                    if !RULE_IDS.contains(&id) {
                        eprintln!(
                            "unknown rule \"{id}\"; the rules are {}\n",
                            RULE_IDS.join(" ")
                        );
                        return Some(2);
                    }
                    ids.push(id.to_string());
                }
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option \"{other}\"\n\n{HELP}");
                return Some(2);
            }
            other => path = Some(other.to_string()),
        }
    }

    if !check {
        return None;
    }

    let Some(path) = path else {
        eprintln!("--check needs a folder to check\n\n{HELP}");
        return Some(2);
    };

    // The flags add up rather than override each other: --rules names individual
    // rules, and the category flags add whole categories to them. No flags at all
    // means every rule, which is also what the window starts with.
    if naming || structure {
        for id in RULE_IDS {
            let wanted = (naming && rule_category(id) == "naming")
                || (structure && rule_category(id) == "structure");
            if wanted && !ids.iter().any(|chosen| chosen == id) {
                ids.push(id.to_string());
            }
        }
    }

    let root = PathBuf::from(&path);
    if !root.is_dir() {
        eprintln!("not a directory: {path}");
        return Some(2);
    }
    let name = root
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    let mut tree = match scan_dir(&root, name) {
        Ok(tree) => tree,
        Err(err) => {
            eprintln!("could not read {path}: {err}");
            return Some(2);
        }
    };

    // No flags at all means every rule, the same default the window starts with.
    let rules = Rules {
        ids: if ids.is_empty() { None } else { Some(ids) },
    };
    let ctx = Ctx {
        rel: String::new(),
        exempt: false,
        under_maps: false,
        under_matlib: false,
    };

    let mut violations: Vec<Violation> = Vec::new();
    lint_dir(&mut tree, &ctx, &rules, &mut violations);

    for violation in &violations {
        println!("{}\t{}\t{}", violation.rule, violation.path, violation.message);
    }

    let found = violations.iter().filter(|v| v.category == "naming").count();
    eprintln!(
        "{} in {}{}",
        match violations.len() {
            0 => "no violations".to_string(),
            1 => "1 violation".to_string(),
            n => format!("{n} violations"),
        },
        root.display(),
        match violations.len() {
            0 => String::new(),
            n => format!(" (naming {found}, structure {})", n - found),
        }
    );

    Some(if violations.is_empty() { 0 } else { 1 })
}
