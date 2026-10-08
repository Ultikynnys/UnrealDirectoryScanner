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
- Allar (Gamemakin UE style guide) directory checks with inline issue markers.

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
ai, ui, fx, data, input, level, file. The chip carries the exact prefix (hover it
for the type name), and the legend under the summary is a census of the types in
the current scan.

## Allar checks

When the folder looks like Unreal content (it is named `Content`, it holds a
`Content/` folder, or a `*.uproject` sits beside it), the scan also checks the
[Allar / Gamemakin UE style guide](https://github.com/Allar/ue5-style-guide)
section 2 directory rules. Turn them off with **Allar checks**, and use **issues
only** to hide everything that is fine. Hover a marker to see the rules it is
citing.

| Rule | Check |
| --- | --- |
| 2.1.1 | folder name is PascalCase |
| 2.1.2 | folder name contains no space |
| 2.1.3 / 00.1 | folder name uses only `A-Z a-z 0-9 _` |
| 2.2.1 | no `.uasset`/`.umap` loose in `Content/` |
| 2.4 | every `.umap` lives under a `Maps` folder |
| 2.6.1 | no folder named `Assets` |
| 2.6.2 | no folders named `Meshes`, `Textures`, `Materials` |
| 2.8 | base materials (`M_*`) live under `MaterialLibrary` |
| 2.9 | no empty folders |

`Content/Developers/**` (a sandbox per 2.3) and `Content/Python/**` (UE's Python
folder rather than content) are exempt.

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
