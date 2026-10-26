[简体中文](DEVELOPMENT.md) | **English**

# Development Guide

## Environment

The current target is Windows x64. Install Node.js (22/24 LTS recommended), the Rust stable `x86_64-pc-windows-msvc` toolchain, Visual Studio 2022 Build Tools with the "Desktop development with C++" workload (including the Windows SDK), and the Microsoft Edge WebView2 Runtime.

Dependencies are locked by `frontend/package-lock.json` and `frontend/src-tauri/Cargo.lock`. npm is the only package manager in use; the old pnpm lock file is kept only in Git history.

## Initialization

1. Run `npm ci` in `frontend/`.
2. Run `npm run desktop:dev`. The first time the desktop app starts, choose the basic or full edition in the tool installation window.

For local debugging, if you already have the tools, you can place `yt-dlp.exe`, `ffmpeg.exe` and `ffprobe.exe` in the root `bin/` directory and then run `npm run tools:check` to check them. These files are not included in the installer.

Vite always listens on `127.0.0.1:1420`; if the port is already in use, it fails immediately. Tauri starts the frontend and the Rust window. After closing the development window, check the terminal and, if needed, press Ctrl+C to stop the development server.

## Common commands (run in frontend/)

| Command | Purpose |
| --- | --- |
| `npm run dev` | Browser-only UI preview; no desktop capabilities |
| `npm run desktop:dev` | Full desktop development mode |
| `npm run build` | TypeScript check and frontend production build |
| `npm run lint:check` | ESLint check; does not modify source |
| `npm test` | Node built-in tests; requires Node.js 22.18+ / 24 |
| `npm run test:ui` | Headless browser UI smoke test; does not access real sites |
| `npm run tools:check` | Checks the optional local development tools; does not affect the GUI build |
| `npm run check:rust` | Rust compile check |
| `npm run test:rust` | Rust unit tests |
| `npm run test:desktop` | Runs a real desktop smoke test after building (briefly opens a window and temporarily modifies and then restores settings) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Rust format check |
| `npm run desktop:build` | Frontend, Rust release build, NSIS packaging |
| `npm run desktop:build:install-test` | Builds an isolated test installer that does not affect the official app identity |
| `npm run test:install-resources` | Isolated NSIS silent install; byte-by-byte verification of all license resources, then uninstall; does not touch the official app or WebView2 |
| `npm run test:tools` | Networked tool install/cancel/upgrade smoke test; uses only the isolated test EXE and requires downloading large upstream archives |
| `npm run licenses:installed -- <install directory>` | Read-only verification of the root LICENSE and all files under licenses/; this is not a license review |
| `npm run licenses:check` | Checks license materials, inventories, hashes and packaging configuration |
| `npm run licenses:release` | Release gate; fails while `pending` is non-empty (it is currently empty, so this passes) |
| `npm audit` | Checks JS dependency advisories; do not force-downgrade or upgrade all dependencies directly |

## Changing code

Frontend pages must not call low-level commands directly: use `src/services/desktop.ts`, and keep `src/types/desktop.ts` in sync. New Rust commands must be registered in `invoke_handler`. Rust child processes must not be run through a shell with concatenated strings, and no arbitrary executable path may be exposed.

When releasing, keep the versions in `package.json`, `Cargo.toml` and `tauri.conf.json` consistent, and update `docs/CHANGELOG.md`. The repository commits only the icons in `frontend/src-tauri/icons/` that packaging needs; the original PNG artwork and `frontend/app-icon.png` are local design/generation inputs and are ignored by Git. If you have a local icon source, you can regenerate the icons as follows:

```powershell
# Generate in a temporary directory first, then copy only the icons the current Windows build needs.
$iconOutput = Join-Path $env:TEMP ('ytdlp-icons-' + [guid]::NewGuid())
npm run tauri -- icon app-icon.png --output $iconOutput --fit contain
if ($LASTEXITCODE -ne 0) { throw 'Icon generation failed' }
'icon.ico', 'icon.png', '32x32.png', '128x128.png', '128x128@2x.png' |
  ForEach-Object {
    Copy-Item -LiteralPath (Join-Path $iconOutput $_) -Destination 'src-tauri/icons/'
  }
```

## Project file maintenance

- The valid entry points are `frontend/src/main.ts` and `frontend/src-tauri/src/main.rs`. The Python/Eel, Docker and PyInstaller entry points are no longer maintained.
- `frontend/vendor/` holds project-owned dependencies; currently this is the replacement implementation of `number-precision`, referenced through a `file:` dependency and `overrides` in `package.json`.
- `frontend/dist/` is the Vite build output and is unrelated to the `dist/` directory at the root of the old version.
- Locally generated artifacts ignored by Git include: the root `bin/`, `frontend/node_modules/`, `frontend/dist/`, `frontend/src-tauri/target/`, the `.grabmeta-install-*/` test evidence directories and `tmp_ffmpeg_download/` (the older `.ytdlp-gui-install-*/` name stays ignored too). These directories are not committed and are not distributed with the installer; test conclusions that need to be kept are governed by [Testing and acceptance](TESTING.en.md).
- The repository commits only the Windows ICO/PNG files referenced by the packaging configuration in `frontend/src-tauri/icons/`, along with the general `icon.png`. Original artwork, screenshots, downloaded media and test installation evidence are not committed.
- Frontend business code is written in TypeScript; the `.mjs` files in `scripts/` and `tests/` are build, smoke-test and test scripts, not leftovers from the old version.
- Relative links in the documentation are checked by `npm test`. When you add, delete or rename a file in `docs/`, update the [Documentation index](README.en.md) as well. Release notes use only the [Release notes template](RELEASE_TEMPLATE.en.md).

The old implementation is kept in the Git tag `v2.0.1` and can be viewed read-only from the project root without overwriting the current working tree:

```powershell
git ls-tree -r --name-only v2.0.1 legacy/python-eel/
git show v2.0.1:legacy/python-eel/main.py
```

After adding or deleting files, run `npm test`, `npm run lint:check` and `npm run build`, and check the icons, documentation links and Tauri resource mappings. Before release, additionally run the tool preflight and the NSIS packaging.

## Settings and data

The settings file is `settings.json` under Tauri's `app_config_dir()`; on Windows it is usually `%APPDATA%/com.ssk-shandm.grabmeta/settings.json`. The runtime-resolved path is authoritative. The default download directory comes from the system Downloads folder and does not depend on the working directory. Corrupt or invalid settings fall back to defaults. Old configurations without concurrentFragments are filled in with 8 automatically; the valid range is 1–16, the UI offers 1/4/8/16 fragments, and the backend validates the value again on save.

Task records are in-memory state for the current run and keep at most 200 finished tasks; the terminal keeps at most 2000 lines. Clearing task records does not delete files in the download directory.
