# Unreal Directory Scanner

A small desktop app that scans any folder and shows its contents as a collapsible
tree, highlighting Unreal assets (`.uasset`, `.umap`) and rolling up asset, file
and size totals per folder.

Built with [Tauri 2](https://tauri.app): a Rust backend does the scanning, and the
UI is plain HTML/CSS/JS with no bundler and no runtime npm dependencies.

## Features

- Native folder picker, or open a folder straight from the command line.
- Per-folder pills: the asset count when the folder holds assets, otherwise the
  plain file count, so a scripts-only folder never reads as empty.
- Filter by name, expand/collapse all, and optional listing of non-asset files.
- Skips noise directories: `.git`, `.svn`, `.hg`, `.vs`, `.idea`, `node_modules`,
  `__pycache__`, `DerivedDataCache`, `Intermediate`, `Saved`, `Binaries`, `Build`,
  `target`, `dist`.
- Remembers the last folder between runs.
- Unreadable subfolders are skipped instead of aborting the scan.

## Requirements

- Rust (stable) with the MSVC toolchain on Windows.
- Node.js, only for the Tauri CLI.
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

## Usage

Pick a folder with **Choose folder...**, or open one directly:

```sh
unreal-directory-scanner <path>
```

A path given on the command line takes priority over the remembered folder.

## Layout

```
src/          UI (static: index.html, style.css, app.js)
src-tauri/    Rust backend (the scan_directory command) and Tauri config
```
