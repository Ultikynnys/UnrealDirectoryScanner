# Unreal Directory Scanner

A small desktop app that scans any folder and shows its contents as a collapsible
tree, highlighting Unreal assets (`.uasset`, `.umap`) and rolling up asset, file
and size totals per folder.

Built with [Tauri 2](https://tauri.app): a Rust backend does the scanning, and the
UI is HTML/CSS plus TypeScript compiled by `tsc`. There is no bundler and no
runtime npm dependency; TypeScript is a devDependency only.

## Features

- Native folder picker, or open a folder straight from the command line.
- Per-folder pills: the asset count when the folder holds assets, otherwise the
  plain file count, so a scripts-only folder never reads as empty.
- Asset files are listed by default and colour-coded by type; **all files** adds
  non-asset files on top, and the legend is a type census of the scan.
- Skips noise directories: `.git`, `.svn`, `.hg`, `.vs`, `.idea`, `node_modules`,
  `__pycache__`, `DerivedDataCache`, `Intermediate`, `Saved`, `Binaries`, `Build`,
  `target`, `dist`.
- Remembers the last folder between runs.
- Unreadable subfolders are skipped instead of aborting the scan.
- Checks split into two categories, naming and directory structure, each with its
  own rule select, summary line and inline issue markers, all sitting under a
  **guide** selector that presets which rules run.
- Light theme by default, with a `dark` toggle remembered between runs; square,
  borderless controls.

## Theme

Light is the default, and `dark` in the toolbar switches to the dark palette; the
choice is remembered between runs. Both palettes are custom properties in
`web/style.css`, light on `:root` and dark on `:root[data-theme='dark']`, so the
app still renders light if the preference cannot be read.

Controls are deliberately flat: no rounded corners and no borders or outlines on
the toolbar controls, count pills, type chips or issue markers. Separation comes
from surface colour instead, and focus is shown by a surface change rather than a
focus ring. Two things are kept, because they are structure rather than chrome:
the tree's indent guides and the caret glyph.

All text carries a drop shadow, which keeps the paler type colours legible on the
white background. The shadow colour is themed with the rest of the palette: dark
in the light theme and white in the dark one (`--text-shadow`).

## Asset types

Asset files are always listed, and each is tagged with a prefix chip and a colour
for its type. The type comes from the filename prefix, which is exactly what the
style guide's naming convention encodes, so the colours and the checks read the
same names.

The prefix table follows [Allar 1.2](https://github.com/Allar/ue5-style-guide) and
adds conventions found in UE projects: `NS_`, `NE_`, `CS_`, `IA_`, `IMC_`, `DA_`,
`AN_`, `SC_`, `NavLink_`, the underscored `E_`/`F_` forms UE tooling writes, and
`Cue_`, `SKM_`, `PA_`, `SA_`, `BTT_`, `BTS_`, `BTD_`, `BPC_`, `BI_`, and `ST_`,
which occur in the checked UE5 project. `AS_` is the Allar animation-sequence convention.

Some assets, notably SoundWaves, have no consistent filename prefix. For an otherwise unrecognised
asset, the scanner checks the package's serialized name table for a supported class name and uses a
type only when it is the sole supported class. Blueprint plus generated class and curve atlas plus
base curve are the two known class pairs. This inferred type is not a naming-prefix claim and does
not make the name pass rule 1.1. Ambiguous class names are left unclassified rather than guessed.

Colour is per **family**, not per prefix: blueprint, mesh, material, texture,
animation, audio, ai, ui, fx, data, input, level, file. The chip carries the
prefix when one is known, sits in its own column at the right edge of the tree so
types line up down the page, and names its type on hover. The legend is a census
of the types in the current scan, most common first.

## Checks and guides

When the folder looks like Unreal content (it is named `Content`, it holds a
`Content/` folder, or a `*.uproject` sits beside it), the scan also runs style
checks. They split into naming and Content directory structure, so the toolbar
carries one rule select per category, under a **guide** selector.

A guide is a named preset over the rule set, so "which style guide do we follow" is
one choice rather than a dozen. A preset is not a private rule list: it is spelled with
the same flags the command line takes, where a category name covers every rule in it and
any other token is a rule id. The same set can be written either way round, which is why
`allar` and `minimal` name everything and subtract while `community` simply names
everything - `a_preset_can_be_spelled_either_way` holds them to it.

| Guide | Spelled as | Rules |
| --- | --- | --- |
| **Allar / Gamemakin** (default) | `--naming --structure --except 00.2,2.1.4,2.6.3` | [Allar's UE style guide](https://github.com/Allar/ue5-style-guide) |
| **Community** | `--naming --structure` | every rule, including the 00.2, 2.1.4 and 2.6.3 that other widely used UE structure guides add |
| **Minimal** | `--naming --structure --except 00.2,2.1.4,2.6.1,2.6.2,2.6.3,2.8` | the high-signal subset |
| **Unreal Directive** | `--naming --structure --except 00.2,2.1.4,2.4,2.6.2,2.6.3,2.8` | [Unreal Directive](https://unrealdirective.com/resources/project-standards/folder-structure/): a `_Shared` folder organised by type, so no `Maps` or `MaterialLibrary` of its own |
| **Lyra** | `--naming --structure --except 00.2,2.1.4,2.2.1,2.6.3,2.8` | [Epic's Lyra](https://dev.epicgames.com/community/learning/paths/Z4/lyra-starter-game): `Content` is `Maps`, `System` and `UI`, with no folder named after the project |
| **By asset type** | `--naming --structure --except 00.2,2.1.4,2.6.2,2.6.3` | the layout most small projects start with, where a folder per type is the point |
| *yours* | whatever flags its file holds | any preset in the folder beside the executable, see below |

The last three are the guides that are common for UE5 without being Allar's. Each is a whole style
guide, and almost everything each publishes beyond folder layout is a table of asset prefixes -
`SM_` against `S_`, `WBP_` against `W_` - which this scanner does not tell apart: rule 1.1 asks
whether a name starts with a prefix it knows, not which one. So a preset takes a guide at the points
where it genuinely disagrees about folders and leaves the rest of it alone, which is why the six
differ by a handful of rules rather than by much.

A guide only presets the checkboxes: any rule can still be ticked or unticked by hand.
Which guide the picker shows is worked out from the boxes rather than kept as state of its
own, so a hand-edited selection matches none of them and reads `(edited)`; tick it back and
the preset re-selects itself. Allar is the default for a window that has never been touched,
and after that the window opens on whatever it was last on: the preset and the flags are kept
together, so a set picked once does not have to be picked again on every launch, while a
shipped preset still follows its own definition if that ever changes. Both the rule ids and
the presets come from one table in the Rust backend and the toolbar is built from it, so the
two cannot drift apart. Next to the picker, **guidelines** opens the selected guide's own
documentation in your browser, and it is hidden for presets that have none.

### Presets of your own

The three above are what the tool ships. Your own live in a `presets` folder beside the
executable - `src-tauri/target/release/presets/` for a development build, and next to
`unreal-directory-scanner.exe` for an installed one. One file per preset, named
`<name>.preset`, holding the flags it stands for:

```
# the team's set: everything, minus the two we do not enforce yet
--naming --structure --except 2.6.3,2.9
```

Blank lines and `#` comments are ignored, and every token has to be one the flags accept, so a
file cannot ask for a rule that is gone. The name is the file's own, so `--guide team` works as
`--guide minimal` does and `--presets` lists what is there. A file that shares a shipped name
replaces it, which is how a team pins a default locally; a file that will not parse is reported
by name rather than quietly checking nothing. In the window the picker lists them below the
shipped three, each with **save** and **delete** controls that write the file again or remove it,
and a **save as preset** box that writes the rules ticked right now out as a new one. Writing over
a preset and deleting one both throw away what was there, so each takes two clicks: the first
arms the control, the second carries it out, and closing the picker puts it back.

Each picker is multi-select: opening it lists that category's rules as
checkboxes, and a checkbox beside it switches the whole category on or off in one
click, showing a half state when only some of its rules are on. A ticked rule is
the only thing that runs, so unticking one stops it being checked at all: its
violations leave the summary and the tree's counts rather than being filtered out
of the view. **issues only** hides everything that is fine across whichever rules
are on. A folder that is itself in the wrong keeps what is inside it, since that is
the folder you have to act on and a lone row would expand to nothing; a folder shown
only for something further down keeps just the offending rows. Hover a marker to see
the rules it cites.

| Rule | Category | Source | Check |
| --- | --- | --- | --- |
| 00.1 | naming | Allar 00.1 | file name uses only `A-Z a-z 0-9 _` |
| 00.2 | naming | community | file name is not a reserved Windows name (`CON`, `NUL`, `COM1`...) and has no leading or trailing space |
| 00.3 | naming | Allar 2.1.1, 1.1 | folder and asset names are PascalCase, wherever a name is written; a part beginning with a digit, like `1st` or `01`, is not a word and is not asked |
| 1.1 | naming | Allar 1.1 | file name starts with a recognised prefix, and its variants are two digits |
| 2.1.2 | structure | Allar 2.1.2 | folder name contains no space |
| 2.1.3 | structure | Allar 2.1.3 | folder name uses only `A-Z a-z 0-9 _` |
| 2.1.4 | structure | community | sibling folder names differ by more than case |
| 2.2.1 | structure | Allar 2.2.1 | no `.uasset`/`.umap` loose in `Content/` |
| 2.4 | structure | Allar 2.4 | every `.umap` lives under a `Maps` folder |
| 2.6.1 | structure | Allar 2.6.1 | no folder named `Assets` |
| 2.6.2 | structure | Allar 2.6.2 | no folders named `Meshes`, `Textures`, `Materials` |
| 2.6.3 | structure | community | no folders named `Blueprints`, `StaticMeshes`, `SkeletalMeshes`, `Animations`, `Sounds`, `Audio`, `Particles`, `Effects` |
| 2.8 | structure | Allar 2.8 | base materials (`M_*`) live under `MaterialLibrary` |
| 2.9 | structure | Allar 2.9 | no empty folders |
| 2.10 | structure | this tool | an asset and what it references sit in the same folder or one inside the other, never in sibling folders |

2.10 is the one rule no guide this tool ships asks for, so it is **off unless it is named**: naming
`--structure` does not turn it on, while `--rules 2.10` or a preset that spells it out does. Without
that, every preset that names a category would have acquired a rule about references the moment it
arrived, and every existing window would have opened on a wall of new violations.

What an asset references is read straight out of the package's bytes, where the full `/Game/...`
object path of everything it imports sits in the clear. A path that names no file the scan found, or
names more than one, is dropped rather than turned into an edge. Two things it cannot see: content
mounted under a plugin's own name rather than `/Game/`, and packages whose tables are compressed, as
a shipped build may use.

`Content/Developers/**` (a sandbox per 2.3) and `Content/Python/**` (UE's Python
folder rather than content) are exempt, as are the World Partition folders
`__ExternalActors__` and `__ExternalObjects__` that the engine generates beside a
map. Maps are typed by extension so they are never asked for a filename prefix.

Prefix and serialized-class hints are used for display and for deciding whether a type can be
identified. The scanner does not yet validate that a recognized prefix matches an asset's actual
class; it does not parse the package export map, where that identity is authoritative.

Out of scope, because they need the editor rather than the filesystem: 2.3, 2.5
and 2.7 are advisory, and sections 3 to 7 (Blueprint graphs, mesh UVs and
collision, map lighting, texture dimensions) cannot be checked by listing files.
The plugin and Game Feature Plugin layout, and keeping third-party content in its
own top-level folder, are out of scope too: both need a scan of the project root
rather than of `Content`.

## Requirements

- Rust (stable) with the MSVC toolchain on Windows.
- Node.js, for the Tauri CLI and the TypeScript compiler.
- The WebView2 runtime on Windows (preinstalled on Windows 11).

## Run

```sh
npm install
npm run dev
```

## Build

```sh
npm run build
```

Artifacts land under `src-tauri/target/release/bundle/`.

`npm run build:web` is just `tsc`, which compiles the TypeScript sources into the
`web/` assets directory. `npm run dev` runs it first, and `npm run build`
triggers it through Tauri's `beforeBuildCommand`, so `web/app.js` is always
current before it is embedded.

## Usage

Pick a folder with **Choose folder...**, or open one directly:

```sh
unreal-directory-scanner <path>
```

A path given on the command line takes priority over the remembered folder, and becomes the
remembered one, so the window reopens wherever it was last looking however it got there.

## Command line

The same binary runs the checks headlessly, which is what a script or a CI job
wants:

```sh
unreal-directory-scanner --check ./MyProject/Content
unreal-directory-scanner --check ./MyProject/Content --structure
unreal-directory-scanner --check ./MyProject/Content --guide community
unreal-directory-scanner --check ./MyProject/Content --guide community --except 2.6.3
unreal-directory-scanner --check ./MyProject/Content --rules 1.1,2.8
unreal-directory-scanner --rules 1.1,2.8 ./MyProject/Content
unreal-directory-scanner --rule 1.1 --rule 2.8 ./MyProject/Content
unreal-directory-scanner --guide team ./MyProject/Content     # a preset of your own
unreal-directory-scanner --presets                            # what is in the folder
```
Every violation prints as `rule <tab> path <tab> message`, the path being relative
to the folder being checked, so the output greps and diffs cleanly. The summary
goes to standard error instead, leaving standard output pipeable. The exit status
is `0` when nothing was found, `1` when something was, and `2` for bad usage, so a
CI step fails on its own. `--help` lists the options, every rule id and every preset it can
see, `--presets` lists just the presets that are there, and `--rules` rejects an id it does
not know.

One preset at a time, and it is never combined with the flags that build a set up: pairing
`--guide` with `--naming`, `--structure`, `--rules` or `--rule` is a usage error, because the
result would be neither the preset nor a set of your own. `--except` is welcome either way,
since it only ever takes rules away, so `--guide community --except 2.6.3` trims a preset and
`--naming --structure --except 2.6.3,2.9` is every rule but those two. A category name is a
token like any other, so `--except naming` leaves just the structure rules. The manual flags
add up among themselves, so `--structure --rules 2.4` checks the structure rules plus 2.4.
Passing no rule flag at all checks the default preset (Allar), which is also what an untouched
window uses. `--help` prints each preset as the flags it expands to. `--guide` takes the name of
a preset you wrote just as readily as one of the three the tool ships. Any rule flag implies
`--check`, so the last two examples above need no `--check` of their own.
One Windows caveat: the release build declares the Windows GUI subsystem, so it
has no console to print an interactive run to. Output still reaches a pipe or a
file - which is how a CI job or `--check ... > report.txt` uses it - and the debug
build and `cargo run` print to the terminal as usual.

## Test mode

In a plain browser there is no Tauri runtime and no way to read the file system, so the
app normally reports that it has no backend. Adding `?test` to the URL swaps in a bundled
demo backend, `src/test-mode.ts`, which serves a synthetic Unreal project and the rule
catalog. The whole UI - tree, type chips, markers, both summary lines, the guide picker -
then runs with no Rust process and no folder access, which is how it is driven headlessly:

```sh
npm run build:web
npx serve web          # or any static server over the web/ directory
# open http://localhost:3000/index.html?test
```

The window title gains a "- test mode" suffix, so demo data is never mistaken for a real
scan. The fixture carries a fixed set of issues per item and filters them by whichever
rules are on, so it shows what a violation looks like rather than deciding that one
exists: the real checks still only run in the app. It also covers the two cases a Windows
disk cannot hold, a `weapons`/`Weapons` case collision and a reserved `CON.uasset`, which
is the point of having it.

## Pre-commit

`.githooks/pre-commit` runs the Rust suite and the frontend compile on every commit, so a
commit cannot land with a failing check or a file that does not build. It is committed
rather than left in `.git/hooks` so that it travels with the repository; `npm install`
points git at it through the `prepare` script, or by hand:

```sh
git config core.hooksPath .githooks
```

It fails closed: if `cargo` or `npm` is not on `PATH` it stops with a message instead of
quietly checking nothing, which matters with a GUI git client that does not inherit your
shell's environment. On Unix, `install.mjs` also sets the executable bit, which git on
Windows does not track.

## Layout

```
src/          app.ts, test-mode.ts and tauri.d.ts: all of the TypeScript source
web/          index.html and style.css (hand written) plus the compiled app.js and
              test-mode.js; this is the directory Tauri embeds
src-tauri/    Rust backend (the scan_directory command and the Allar rules)
.githooks/    the pre-commit autotests and the script that wires them up
```
