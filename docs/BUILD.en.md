[简体中文](BUILD.md) | **English**

# Windows x64 / NSIS build

## Dependencies

The development machine needs Node.js, Rust MSVC, C++ Build Tools / Windows SDK, and WebView2. The first Rust compile and NSIS packaging require network access to download crates, the Tauri packaging tools, and runtime bootstrap files, so they may take a long time.

## External tools

The installer contains only the GUI itself. It does not bundle `yt-dlp.exe`, `ffmpeg.exe`, or `ffprobe.exe`. After first launch, the user chooses the basic or full edition, and the app downloads them over HTTPS from supported upstream sources and installs them into `bin/` on the local machine.

A clean clone can therefore build an installer that contains no third-party EXE files. `npm run tools:check` only checks local development tools in a development directory (if present); it does not mean the installer contains those tools. For download sources, providers, and user notices, see [THIRD_PARTY.en.md](THIRD_PARTY.en.md).

## Build commands

```powershell
cd frontend
npm ci
npm run build
npm test
npm run test:rust
npm run desktop:build
```

Output is in `src-tauri/target/release/bundle/nsis/`. The release main EXE is in `src-tauri/target/release/`. The NSIS installer deploys the main EXE, `LICENSE`, and `licenses/`, but not the tool `bin/` directory. Publish the complete installer, not just the main EXE.

## Isolated install-test build

When the official release is already installed on the development machine, do not uninstall the official release or WebView2 for testing. Use a separate configuration:

```powershell
cd frontend
npm run desktop:build:install-test
```

This command merges `src-tauri/tauri.install-test.conf.json` and does not modify the official `tauri.conf.json`:

| Item | Test value |
| --- | --- |
| Product name / NSIS registry subkey | `GrabMeta Install Test` |
| App identifier / config directory name | `com.ssk-shandm.grabmeta.install-test` |
| Main EXE | `grabmeta-install-test.exe` |
| Start menu folder | `GrabMeta Install Test` |
| WebView2 install mode | `skip`, which does not install, uninstall, or upgrade the existing runtime |

Install into a newly created, separate test directory (it may contain Chinese characters and spaces). Do not choose the official installation directory. First inspect the generated `target/release/nsis/x64/installer.nsi`: the product name, main EXE, and app identifier must all be the test values, and `INSTALLWEBVIEW2MODE` and `MINIMUMWEBVIEW2VERSION` should be empty. The generator template represents the `skip` mode in the configuration as an empty mode.

The test build inherits the official release resources, but it must not download or launch the official update installer. Use `/S` for a silent install; put the NSIS `/D=<absolute test directory>` parameter last. Before uninstalling, verify the test build's registry `InstallLocation`, and run only the `uninstall.exe` in that directory. Do not run the official installer to test "upgrade".

Before and after installation, save the official installation files, configuration files, and related registry snapshots, and compare them with the WebView2 records to confirm nothing changed. The test build and the official build share the Rust target directory. The next release must run `npm run desktop:build` again; do not publish the test EXE or test installer as the official release.

Isolated testing can only verify installation, launch, same-version reinstall, and uninstall in an environment where WebView2 is already present. It does not prove a clean system, a missing WebView2, the offline bootstrapper, or cross-version upgrade of the official release. For results, see [TESTING.en.md](TESTING.en.md).

The isolated configuration's `silent: null` is a JSON merge patch. When the type is changed to `skip`, the `silent` field is removed from the official configuration. This avoids generating the `skip + silent` combination, which does not conform to the schema, and does not affect the official network policy.

## Current installation configuration

- Product name: `GrabMeta`; the window title is the Chinese edition name (`GrabMeta 中文版`).
- App identifier: `com.ssk-shandm.grabmeta`. Renamed from `com.ssk-shandm.ytdlp-gui` in 2.3.3 to match the product name; on startup the app migrates `settings.json` from the old config directory once, so upgrading users keep their download directory and preferences.
- NSIS: per-user installation, with Simplified Chinese or English installer UI, selectable at install time.
- The installer contains only the GUI, the project LICENSE, and the documentation. Third-party tools are downloaded after first launch into the current user's app tools directory (`bin/`). The main EXE does not embed FFmpeg.
- WebView2: `downloadBootstrapper`. When the runtime is missing, network access is required; this is not a fully offline installer.
- Automatic updates are implemented in the app layer. The desktop app queries the project's GitHub Releases API, selects NSIS assets whose version and file name end with `-setup.exe`, downloads them, and launches the installer. For a release, upload the `*-setup.exe` produced by `npm run desktop:build`. The app does not currently read `latest.json`, `.sig` files, or a Tauri updater endpoint.

## App icon

The original icon artwork is the root file `视频下载工具图标设计.png`, and the generation source is `frontend/app-icon.png`. The Windows EXE, installer, and uninstaller read `frontend/src-tauri/icons/icon.ico`, which contains 16 / 32 / 48 / 256 px sizes and other sizes. The packaging configuration also references `32x32.png`, `128x128.png`, and `128x128@2x.png`, and keeps a generic `icon.png`. For how to regenerate them, see the [Development guide](DEVELOPMENT.en.md#changing-code).

After replacing the icon, run `npm run desktop:build` to rebuild. Existing EXE and installer files do not change automatically.

## Offline deployment option

For offline deployment, change `bundle.windows.webviewInstallMode.type` to `offlineInstaller` in the Windows build configuration, rebuild, and acceptance-test on a Windows test machine with no network and no WebView2. An offline WebView2 runtime increases the installer size. FFmpeg is still not bundled, and downloading tools for the first time requires network access.

## Installer size

The installer does not include third-party tools, so its size is determined mainly by the GUI, the Tauri/WebView2 installation bootstrapper, and license materials. The size of tools downloaded on first use varies with the upstream version and with the basic or full edition chosen by the user.

## Before release

1. Install on a non-development machine or in an isolated Windows user environment. Verify a path containing Chinese characters and spaces, standard user privileges, and first launch.
2. Complete the download, merge, cancel, and exit acceptance checks in [TESTING.en.md](TESTING.en.md).
3. Verify overwrite install, uninstall, a missing WebView2, and offline scenarios, and confirm that user download files are not deleted.
4. Review the sources, license notices, and download verification policy of the third-party tools.
5. **Optional**: if there is budget, connect a code-signing certificate through the Tauri Windows signing configuration. For a community project, it is acceptable to publish unsigned packages first, without making certificate cost a hard prerequisite for building or automatic updates. Unsigned packages may trigger SmartScreen prompts, which should be stated honestly in the release notes. Keep the SHA-256 metadata of the GitHub Release assets so the updater can verify them. Do not put the hash into the user-facing release body.
6. Synchronize the three version numbers, the changelog, and the build verification records, then publish.

Only a successful compilation or a local NSIS artifact does not prove that overwrite install, uninstall, signing, and downloads from all sites have been tested.

## Publishing an update package

1. In `frontend/`, update the version numbers in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`, and synchronize `docs/CHANGELOG.md`.
2. Run `npm run desktop:build` to build the GUI installer, which does not include third-party EXE files.
3. From `src-tauri/target/release/bundle/nsis/`, take the installer ending in `-setup.exe` and complete the test checklist.
4. First create a draft Release for the corresponding tag. After checking the installer name, size, and notes, publish it publicly. Automatic updates check the official version through GitHub's latest Release API. Pre-releases should not be used as official update targets.
5. Record honestly the code-signing status (state explicitly when unsigned), the third-party license materials, and the overwrite-install acceptance status. Publishing does not mean these checks pass automatically. This project's updater does not require paid code signing. Unsigned installers may still be flagged or blocked by SmartScreen, Smart App Control, or organizational policy, so they cannot be guaranteed to install directly on every Windows environment. A Release that uploads only the main EXE does not meet the automatic-update requirements.

Automatic updates accept only the Windows x64 NSIS installer from this project's GitHub Releases, and require a valid `sha256:` asset digest from the GitHub API. If the download size or SHA-256 verification fails, or the digest is missing or malformed, automatic installation stops and does not fall back to running without verification. For older assets that lack a digest, the user must verify them on the release page and install manually. Hash verification is an integrity check and is not the same as publisher signing.

Automatic updates apply only to an installed, per-user NSIS version. Development builds, portable builds, and programs in a running directory without `uninstall.exe` refuse automatic installation to avoid accidentally replacing development files.

### Release content format

Release notes follow the [release notes template](RELEASE_TEMPLATE.en.md) and keep only these four user-facing sections:

1. **What's changed**: new features, experience improvements, and bug fixes.
2. **Download and install**: installer selection and the installation or upgrade method.
3. **Notes**: platform limits, signing, WebView2, and necessary security notices.
4. **File information**: the real installer name, platform, and size.

Public Releases do not include sections such as "Verification", "Test results", or "Acceptance records", and do not list test counts, builds, lint, Clippy, or smoke-test pass lists. Internal verification records are in [TESTING.en.md](TESTING.en.md) and are not copied into the release body. Template placeholders must be replaced with this release's actual information. Source changes that are not included in the installer must not be described as published features.

When only the text of a published Release is adjusted, synchronize the corresponding local release notes. Keep the tag, version number, publication time, and installer assets unchanged.

### Release notes encoding

Write release notes according to the [release notes template](RELEASE_TEMPLATE.en.md), and maintain them directly on the GitHub Release page. When using the GitHub API, explicitly convert the JSON to UTF-8 bytes and set `Content-Type: application/json; charset=utf-8`. Do not rely on the default string encoding of Windows PowerShell.

## License materials

`npm run licenses:check` checks the project's own files and the notice materials shipped with it. The packaging configuration copies the project's `licenses/` directory into the installation directory, so the installer contains:

```text
LICENSE
licenses/
  NOTICE.txt
  README.md
  yt-dlp/
  ffmpeg/
```

For release, use the complete NSIS installer. The `bin/` directory is created on the user's machine only after the user installs the tools. When the download upstream or build type changes, first update [THIRD_PARTY.en.md](THIRD_PARTY.en.md) and the notices in `licenses/`, then run the check and build commands.

The project does not distribute third-party tool binaries in the installer. `licenses/` provides only upstream sources, user notices, and project dependency notes; it does not replace the license of each upstream project. Before a formal release, still run `npm run licenses:check` and confirm that the download sources and the user-interface notices are accurate.

## GUI copyright, source, and WebView2 pre-installation notice

Production distribution keeps WebView2 `downloadBootstrapper` and explicitly enables silent runtime installation. The NSIS interactive installer shows, through `bundle.licenseFile`, the Chinese notice, the original GUI MIT text, and Microsoft's official end-user terms. The shared Runtime is not removed when the GUI is uninstalled. Installing without network access when the Runtime is missing is outside the supported scope.


`licenses/gui/sources/` ships the exact versions of the MPL and composite-license component source code together with the resources. `SOURCE-NOTICE.txt` explains how to extract them and how to obtain the source. After updating dependencies or notice materials, run `npm run licenses:inventory` (exits non-zero if there are unresolved items) and `npm run licenses:redistribution` in order, then run `licenses:check` and `licenses:release`. number-precision has been replaced by the project's own MIT implementation; see [GUI license record](../licenses/gui/README.md) for details. The clean environment without a Runtime has not been acceptance-tested, and the release gate remains in effect.
