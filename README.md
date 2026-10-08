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
- Allar checks split into two categories, naming and directory structure, each
  with its own rule select, summary line and inline issue markers.
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
adds common engine conventions the guide predates, so real projects get labelled
instead of landing in "unrecognised": `NS_`, `NE_`, `CS_`, `IA_`, `IMC_`, `DA_`,
`AN_`, `SC_`, `NavLink_`, and the underscored `E_`/`F_` forms UE tooling writes.

Colour is per **family**, not per prefix, because 80-odd prefixes cannot each get
a distinguishable colour: blueprint, mesh, material, texture, animation, audio,
ai, ui, fx, data, input, level, file. The chip carries the exact prefix, sits in
its own column at the right edge of the tree so types line up down the page, and
names its type on hover. The legend, also on the right, is a census of the types
in the current scan, most common first.

## Allar checks

When the folder looks like Unreal content (it is named `Content`, it holds a
`Content/` folder, or a `*.uproject` sits beside it), the scan also runs the
[Allar / Gamemakin UE style guide](https://github.com/Allar/ue5-style-guide)
checks. The guide splits into naming (section 1) and Content directory structure
(section 2), so the toolbar carries one rule select per category.

Each picker is multi-select: opening it lists that category's rules as
checkboxes, and a checkbox beside it switches the whole category on or off in one
click, showing a half state when only some of its rules are on. A ticked rule is
the only thing that runs, so unticking one stops it being checked at all: its
violations leave the summary and the tree's counts rather than being filtered out
of the view. **issues only** hides everything that is fine across whichever rules
are on. Hover a marker to see the rules it cites.

| Rule | Category | Check |
| --- | --- | --- |
| 00.1 | naming | file name uses only `A-Z a-z 0-9 _` |
| 1.1 | naming | file name starts with a recognised prefix, and each part after it is PascalCase with two-digit variants |
| 2.1.1 | structure | folder name is PascalCase |
| 2.1.2 | structure | folder name contains no space |
| 2.1.3 | structure | folder name uses only `A-Z a-z 0-9 _` |
| 2.2.1 | structure | no `.uasset`/`.umap` loose in `Content/` |
| 2.4 | structure | every `.umap` lives under a `Maps` folder |
| 2.6.1 | structure | no folder named `Assets` |
| 2.6.2 | structure | no folders named `Meshes`, `Textures`, `Materials` |
| 2.8 | structure | base materials (`M_*`) live under `MaterialLibrary` |
| 2.9 | structure | no empty folders |

`Content/Developers/**` (a sandbox per 2.3) and `Content/Python/**` (UE's Python
folder rather than content) are exempt, and maps are typed by extension so they
are never asked for a filename prefix.

Not yet checked: whether a file's prefix matches the asset's *actual* class. That
class lives in the `.uasset` package header rather than the name, which needs a
UE 5.6 package reader.

Out of scope, because they need the editor rather than the filesystem: 2.3, 2.5
and 2.7 are advisory, and sections 3 to 7 (Blueprint graphs, mesh UVs and
collision, map lighting, texture dimensions) cannot be checked by listing files.

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

A path given on the command line takes priority over the remembered folder.

## Layout

```
src/          app.ts and tauri.d.ts: all of the TypeScript source
web/          index.html and style.css (hand written) plus app.js (tsc output);
              this is the directory Tauri embeds
src-tauri/    Rust backend (the scan_directory command and the Allar rules)
```
