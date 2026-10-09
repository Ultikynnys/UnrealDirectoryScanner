//! A one-shot report, so the same checks can run from a script or a CI job with no
//! window: `--check <folder>` - or any rule flag on its own - prints one line per
//! violation and exits non-zero when there is anything to report.

use crate::{
    default_guide, find_preset, flags_text, lint_dir, load_presets, presets_dir, resolve,
    scan_references, scan_dir, tokens_of, Ctx, FilePreset, Preset, Rules, Violation, GUIDES, RULES,
};
use std::path::PathBuf;

fn rule_list(category: &str) -> String {
    RULES
        .iter()
        .filter(|spec| spec.category == category)
        .map(|spec| spec.id)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The names `--guide` will take: the ones the tool ships, then whatever the folder holds.
fn guide_names(folder: &[FilePreset]) -> String {
    GUIDES
        .iter()
        .map(|guide| guide.id)
        .chain(folder.iter().map(|preset| preset.id.as_str()))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The presets beside the executable, one per line and each written as the flags it stands for, so
/// that a file can be read against what it means.
fn preset_list() -> String {
    let folder = load_presets();
    if folder.is_empty() {
        return match presets_dir() {
            Some(dir) => format!("no presets in {}", dir.display()),
            None => "there is no preset folder beside this executable".to_string(),
        };
    }
    folder
        .iter()
        .map(|preset| match preset.problem() {
            Some(problem) => format!("{:<12} cannot be used: {problem}", preset.id),
            None => format!("{:<12} {}", preset.id, flags_text(&preset.flags, &preset.except)),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Built rather than a const, so the rule ids, the preset names and their flag spellings all come
/// from the one table in `main.rs` and the one folder beside the executable, rather than being
/// retyped here.
fn help() -> String {
    let mut rows: Vec<String> = GUIDES
        .iter()
        .map(|guide| {
            let default = if guide.id == default_guide().id { " (the default)" } else { "" };
            format!("  {:<12} {}{}", guide.id, flags_text(guide.flags, guide.except), default)
        })
        .collect();
    for preset in load_presets() {
        rows.push(match preset.problem() {
            Some(problem) => format!("  {:<12} cannot be used: {problem}", preset.id),
            None => format!("  {:<12} {}", preset.id, flags_text(&preset.flags, &preset.except)),
        });
    }
    let presets = rows.join("\n");
    format!(
        "\
Unreal Directory Scanner

  unreal-directory-scanner [folder]          open the window (the default)
  unreal-directory-scanner --check <folder>  print the violations and exit

What to check - a preset, or the flags to build one, but never both:
  --guide <id>    start from a whole preset, one at a time
  --except <ids>  leave these rules out of whatever is selected, repeatable
  --naming        add every asset naming rule
  --structure     add every content directory structure rule
  --rules <ids>   add these rules, comma separated, and repeatable
  --rule <id>     add one rule; --rule 1.1 --rule 2.8 is --rules 1.1,2.8
  --presets       list the presets in the folder beside the executable
  -h, --help      this text

A preset is a combination of the same flags, so it is not mixed with the flags that build
one up: --guide names a whole set and may be trimmed with --except, while --naming,
--structure, --rules and --rule name a set of your own. Whichever way round you go,
--except only ever takes rules away, so `--naming --structure --except 2.6.3` is every rule
but 2.6.3, and `--except 2.9` on its own trims the default preset.

Presets of your own live in a `presets` folder beside the executable, one file per preset named
`<name>.preset` and holding the flags it stands for, so a file reads exactly like the command
line that would do the same thing:

    --naming --structure --except 2.6.3

With no rule flag at all the default preset is checked. Any rule flag implies --check, so
`--guide minimal ./Content` checks on its own; a rule flag with no folder is a usage error
rather than a window.

Presets:
{}
Rules: {} (naming); {} (structure)

Each violation is printed as: rule <tab> path <tab> message. The summary goes to
standard error, so the output can be piped straight into another tool.

Exit status: 0 nothing found, 1 violations found, 2 bad usage.",
        presets,
        rule_list("naming"),
        rule_list("structure"),
    )
}

/// Returns the process exit code when the arguments ask for a report, or `None`
/// when they should carry on into the windowed app.
pub fn run(args: &[String]) -> Option<i32> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("{}", help());
        return Some(0);
    }
    if args.iter().any(|arg| arg == "--presets") {
        println!("{}", preset_list());
        return Some(0);
    }

    let mut path: Option<String> = None;
    // What the flags name, and what they take away. Both are rule tokens, so a category name is
    // as welcome as a rule id, and both are worked out in one place at the end.
    let mut include: Vec<String> = Vec::new();
    let mut exclude: Vec<String> = Vec::new();
    let mut check = false;
    // `manual` is set by the flags that build a rule set up, which a preset may not be mixed
    // with. `--except` only ever takes rules away, so it is welcome either way.
    let mut manual = false;
    // The name is resolved after the loop, since that is where the preset folder gets read.
    let mut preset_name: Option<String> = None;

    let mut rest = args.iter();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "--check" => check = true,
            "--naming" => {
                include.push("naming".to_string());
                manual = true;
            }
            "--structure" => {
                include.push("structure".to_string());
                manual = true;
            }
            "--guide" => {
                let Some(id) = rest.next() else {
                    eprintln!("--guide needs a guide id\n");
                    eprintln!("{}", help());
                    return Some(2);
                };
                if preset_name.is_some() {
                    eprintln!("--guide takes one preset, not several\n");
                    return Some(2);
                }
                preset_name = Some(id.clone());
            }
            "--rules" | "--rule" | "--except" => {
                let Some(list) = rest.next() else {
                    eprintln!(
                        "{arg} needs {}\n",
                        if arg == "--rule" { "a rule id" } else { "a list of rules" }
                    );
                    eprintln!("{}", help());
                    return Some(2);
                };
                let tokens = match tokens_of(list) {
                    Ok(tokens) => tokens,
                    Err(token) => {
                        eprintln!(
                            "unknown rule \"{token}\"; the rules are {}\n",
                            RULES.iter().map(|spec| spec.id).collect::<Vec<_>>().join(" ")
                        );
                        return Some(2);
                    }
                };
                if arg == "--except" {
                    exclude.extend(tokens);
                } else {
                    manual = true;
                    include.extend(tokens);
                }
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option \"{other}\"\n\n{}", help());
                return Some(2);
            }
            other => path = Some(other.to_string()),
        }
    }

    // A preset is a whole rule set already. Trimming it with --except is the point of it being a
    // combination of flags; adding to it is not, because then it would be neither the preset nor
    // a set of your own - name yours with --naming, --structure, --rules or --rule instead.
    if preset_name.is_some() && manual {
        eprintln!(
            "--guide names a whole preset, so it cannot be combined with --naming, --structure, --rules or --rule; trim it with --except, or name a set of your own\n"
        );
        eprintln!("{}", help());
        return Some(2);
    }

    /* The name is resolved here rather than while parsing, because this is where the preset folder
       gets read: a name may come from the table the tool ships or from a file beside the executable,
       and a file of a shipped name wins. */
    let folder = load_presets();
    let base = match &preset_name {
        Some(name) => {
            let Some(preset) = find_preset(name, &folder) else {
                eprintln!("unknown guide \"{name}\"; the guides are {}\n", guide_names(&folder));
                return Some(2);
            };
            if let Some(problem) = preset.problem() {
                eprintln!("the preset \"{name}\" cannot be used: {problem}\n");
                return Some(2);
            }
            Some(preset)
        }
        // with no preset and nothing named to include, the base is the default preset, so that
        // `--except 2.9` trims what an untouched window would check rather than nothing at all
        None if include.is_empty() => Some(default_guide() as &dyn Preset),
        None => None,
    };

    // --except is a rule flag too, even though it only leaves things out.
    let rules_given = manual || preset_name.is_some() || !exclude.is_empty();

    // A rule flag is itself a request for the report, so it does not have to be
    // paired with --check - which used to mean a rule flag was quietly ignored and
    // the window opened with every rule on. Anything with no rule flag at all is a
    // window launch.
    if !check && !rules_given {
        return None;
    }

    let Some(path) = path else {
        eprintln!(
            "{} needs a folder to check\n\n{}",
            if check { "--check" } else { "a rule flag" },
            help()
        );
        return Some(2);
    };

    let rules = Rules { ids: Some(resolve(base, &include, &exclude)) };

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

    let ctx = Ctx {
        rel: String::new(),
        exempt: false,
        under_maps: false,
        under_matlib: false,
    };

    let references = scan_references(&root, &tree);
    let mut violations: Vec<Violation> = Vec::new();
    lint_dir(&mut tree, &ctx, &rules, &references, &mut violations);

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
