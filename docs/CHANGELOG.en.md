[简体中文](CHANGELOG.md) | **English**

# Changelog

## [2.3.3] — 2026-10-26

### System requirements and documentation

- Explicitly documents the minimum supported systems as **Windows 10 / 11**. Both ship the WebView2 Runtime, and the app no longer actively adapts to or validates older Windows builds without it.
- The installer keeps `downloadBootstrapper` as a network fallback; a machine without a Runtime and without network access is outside the supported scope.
- Synchronized the version in `package.json`, `Cargo.toml`, and `tauri.conf.json` to 2.3.3.
- Corrected the stale version number in the testing document (still said 2.0.1) and the license gate status (`licenses:release` now passes).
- Updated the manual acceptance checklist and the not-yet-verified scope to reflect the completed verification.

### Internal naming unified as GrabMeta

- Frontend package name `yt-dlp` → `grabmeta`; Rust crate `yt-dlp-gui-cn` → `grabmeta` and library `yt_dlp_gui_lib` → `grabmeta_lib`. The main EXE, window title, and installer name were already GrabMeta; this only cleans up internal leftovers.
- The auto-update preference key changed from `ytdlp.auto-update-check` to `grabmeta.auto-update-check`. **The old key is migrated automatically**, so users who turned automatic updates off are not silently opted back in.
- The tool installer staging and backup prefixes are now `.grabmeta-*`; the older `.ytdlp-gui-bin-backup` directory is still read, so a machine with an interrupted install does not lose its `bin/`.
- The app identifier changed from `com.ssk-shandm.ytdlp-gui` to **`com.ssk-shandm.grabmeta`**, fully matching the product name. Because the identifier determines the `%APPDATA%` config directory, the app migrates `settings.json` from the old directory once on startup, so upgrading users keep their download directory and preferences.
- The updater validates the new identifier; the test build (`.install-test`) still cannot launch production updates.

## [2.3.2] — 2026-10-09

### Fixes

- Fixed intermittent "access denied (os error 5)" failures when downloading tools for the first time on Windows. Brief system locks during the install step are now retried automatically, and failure messages include the affected paths.

## [2.3.1] — 2026-10-09

### Update fix

- Fixed automatic updates rejecting download URLs that use GitHub's actual repository casing (`GrabMeta`). Installed 2.3.0 builds would refuse their own update packages; from this version on, update checks and installation work normally.

## [2.3.0] — 2026-10-09

### Rename

- The application is now named GrabMeta everywhere: window title, installer file name (`GrabMeta_2.3.0_x64-setup.exe`), executable name, repository links, and the update source.
- The app identifier and the user configuration directory are unchanged, so settings and download folder configuration are preserved.
- Upstream tools yt-dlp and FFmpeg keep their names and licenses.

## [2.2.0] — 2026-10-09

### Interface language

- Added an English interface. It follows the system language by default: Simplified Chinese when the system is Chinese, otherwise English. You can switch at any time in the sidebar; the choice is saved on this computer.
- The application name in the English interface is GrabMeta. The native window title, installer file name, executable name and repository name are not changed yet and remain yt-dlp GUI.
- The English README is `readme.en.md`; the documentation directory is still in Chinese.

## [2.1.0] — 2026-10-09

Windows x64 NSIS feature release. The installer no longer bundles yt-dlp / FFmpeg; they are downloaded from upstream on first launch as needed.

### Licensing and dependencies

- The upstream `number-precision` 1.6.0 lacked a complete copyright notice. It is replaced by the project's own MIT implementation (`frontend/vendor/number-precision`), which keeps the upstream behavior.

### Update pipeline

- Desktop release checks and installer downloads use the native HTTPS client. Automatic checks start only after initialization completes, keeping local proxy and system network settings consistent.
- Automatic updates accept only the x64 NSIS package from this project's GitHub Releases. The GitHub asset SHA-256 is verified; if it is missing, malformed or mismatched, installation stops. The hash is not a publisher signature, and no Windows code-signing certificate is required.
- Update redirects are limited to supported GitHub HTTPS hosts. A failed download cleans up temporary files and releases the task update lock. The install-started event is sent only after the installer has launched successfully.

### Download speed and stability

- Media DASH/HLS fragments use 8 concurrent connections by default. You can choose 1 / 4 / 8 / 16 under "Content processing → Main usage"; older settings are backfilled with defaults, and the download directory and retry count are preserved.
- Large upstream tool files support up to 4 Range connections with buffered writes. Servers that do not support segmented downloads reuse a single response. HTTPS, size and SHA-256 verification are retained.
- Tool installation shows the actual transfer speed and estimated remaining time. A failed transfer ends the other connections promptly; after cancellation, a retry is possible only once cleanup and the cancellation request have finished.
- The application sets no download rate limit. Fragment concurrency does not guarantee faster speeds for ordinary direct links, for rate limits imposed by target sites, or for proxy routes.

### On-demand third-party tools

- The GUI installer no longer bundles yt-dlp / FFmpeg / ffprobe. On first run you can choose the BtbN LGPL basic build or the GPL full build; they are downloaded from HTTPS upstream sources and verified.
- The basic build can be upgraded to the full build from the About page. If the upgrade fails or is cancelled, existing tools are left unchanged, and the actual tool version, source and license records are saved.

### Local VPN / proxy

- The About → Settings page adds a local VPN client proxy address, which accepts HTTP / SOCKS5 / SOCKS5H loopback addresses. When enabled, link parsing, media downloads, tool downloads and update requests use this proxy. The program does not install or start a VPN.
- The proxy setting is off by default. When it is off, the original system / environment network configuration is still used.

### Tests and Git ignore rules

- The real tool-download smoke test cancels after receiving valid transfer data by default instead of waiting for the full download. The full installation test is enabled explicitly with `--full`.
- Original PNG artwork, screenshots, media download outputs and local installation test evidence are added to Git ignore. Icons required by the Tauri packaging are still kept.

### Project cleanup

- Removed the Python/Eel, Docker and pnpm archives that no longer take part in the current build. The old source remains viewable through the Git tag `v2.0.1`.
- Cleaned up unreferenced sample counters, test images, write-probe files, duplicate icons and non-Windows NSIS icons.
- Removed the unused Element Plus icons, Less, and the redundant direct ESLint parser dependency. The Sass and TypeScript/Vue toolchain in use is retained.
- Cleaned up old local PyInstaller artifacts, Python environments/caches, temporary tool archives, and the 1.1.0/2.0.0 installers. The current installer, downloaded tools, original artwork and the current build cache are retained.
- Updated the project structure, build and migration documents; added file-maintenance rules and resource/documentation link regression tests. The published 2.0.1 installer is unchanged.

### Local development tools cleanup (historical)

- Replaced the FDM-downloaded tools with the Gyan FFmpeg Essentials 9.0.2 Windows x64 static build.
- yt-dlp is kept as the core download tool. FFmpeg / FFprobe are used only for media merging, remuxing and post-processing; the outdated full-build download entry has been removed.
- Updated tool preflight checks, build documentation, third-party hash records and the subsequent release plan. The published v2.0.1 installer has not been repackaged.

### Known limitations

- Installation on a clean Windows machine without the WebView2 Runtime, including online bootstrapping, offline behavior and standard-user scenarios, has not been verified.
- The upgrade path from 2.0.1 to this version has not been verified in a separate environment.
- The installer is unsigned; SmartScreen may show a warning.

### Release and repository information

- Added a user-facing release notes template. Future public Releases will no longer include internal verification, test or build results.
- Adjusted the v2.0.1 release notes to cover update content, download and installation, usage notes, and file information, keeping the real installer size and SHA-256. The installer was not re-released.
- Added a Chinese-version video downloader introduction to the README so readers can understand the project's purpose.

## [2.0.1] — 2026-10-06

Windows x64 NSIS fix release, published on GitHub Releases.

- Fixed a certificate trust problem when downloading the update installer: reqwest now uses the system root certificates, supporting proxy/enterprise certificates that the system already trusts, while keeping HTTPS certificate verification.
- Enabled reqwest's Windows system proxy support, so the Rust downloader uses the system proxy just as the browser's update check does.
- Update download errors now show the full underlying error chain, with guidance for certificate errors, connection failures and timeouts.
- Added a 30-second connection timeout and a 60-second per-read timeout, without limiting the total download time of large installers; redirects to HTTP are forbidden.
- Added regression tests for system trust/proxy configuration, unit tests for the error chain, and an optional GitHub Release network smoke test.
- Version unified to 2.0.1; the terminal startup message uses the configured version number.

## [2.0.0] — 2026-10-06

Release status: the code and version tag have been pushed, and the Windows x64 NSIS installer is published publicly through GitHub Release. Release notes are written as explicit UTF-8 and read back for verification. Third-party distribution materials and the installation/overwrite-upgrade acceptance have not all completed review; see TESTING.md and THIRD_PARTY.md for details.

### Interface improvements

- The cover's initial and reset states are now blank placeholders, with feedback during analysis and a loading-failure message.
- Unified sidebar, page titles, main content area, card spacing and responsive layout; removed the fixed 50% sample progress and show actual task statistics.
- Added page transitions, navigation and button hover effects, card feedback and cover-loading animation, respecting the system "reduce motion" preference.
- Added an About page. It was finally trimmed to two sections, "Project information" and "Settings"; the information keeps only the version number, copyright, MIT license and project repository.
- Removed "Local desktop tool", the sidebar brand icon/name/subtitle, and "Open source · runs locally"; tightened the top spacing of the navigation.
- The desktop app checks the official release from GitHub Releases, both manually on the About page and automatically at startup. When updating, it downloads and launches the NSIS installer whose file name ends with `-setup.exe`. Installation is deferred while tasks are active; browser preview, development builds and portable builds do not perform automatic installation.
- Generated a 1024px RGBA PNG from the root artwork "视频下载工具图标设计.png", plus a multi-size Windows ICO and a Tauri icon set for the EXE build.
- The About page opens project links in the native browser; permissions allow only three preset project URLs. Added a UI browser smoke test.

### Added

- Tauri 2 / Rust Windows desktop entry point and a current-user NSIS installer configuration.
- Native directory picker, persisted settings, a per-session task list, and task cancellation.
- Up to 4 concurrent tasks, parse/download timeouts, process output and exit cleanup.
- Rust unit tests, frontend state/configuration tests, a real WebView2 desktop smoke test, and packaged-tool preflight checks.
- Development, architecture, build, migration, testing and third-party tool documentation.

### Changed

- Kept the Vue frontend; replaced the Python/Eel communication with Tauri IPC.
- Updated the Vue, Vite, Vue plugin, type-checking and ESLint toolchain, removed the ANSI-to-HTML and old ESLint configuration dependencies; the local npm audit reports 0 vulnerabilities.
- External yt-dlp / FFmpeg are distributed as application resources; the application layer is no longer packaged with PyInstaller.
- Detailed project documentation is consolidated under docs; outdated Python/Eel and Docker usage guides, and duplicate UI improvement, PR and work-summary documents, were removed, and the documentation index is organized by purpose.
- The old Python/Eel, Docker and pnpm lock files were moved to legacy/python-eel; dependencies are now managed by npm alone.
- The terminal now renders plain text, and the number of logs held in memory and finished task records is limited.

### Fixed

- The cover download passed the directory instead of the video link.
- The default value, type and branch logic of the retry count.
- Loading state and link-marking problems caused by failed or stale parse responses.
- Download tasks lacked a unified error result and child-process lifecycle management.
- Display of HTTP covers under CSP; directory and retry settings are saved through a patch queue so they no longer overwrite each other.

### Local verification

- Windows x64 NSIS installer build succeeded, about 110.24 MiB.
- 19 frontend tests (including automatic update and multi-size icon checks), the UI browser smoke test, and 10 Rust tests passed; build, ESLint, Rust formatting and Clippy checks passed.
- The real desktop smoke test verified the six download modes, Vue parsing and covers, Chinese paths, the concurrency limit, cancellation and clean process exit.
- Installation/upgrade/uninstallation, clean machines, external sites and release licensing are still pending acceptance; the full scope is in TESTING.md.

### Known limitations

- Only Windows x64 is verified; no installers for other platforms are provided.
- Automatic updates require the publisher to upload a Windows NSIS installer whose file name ends with `-setup.exe`; this currently applies only to installed Windows x64 current-user builds.
- The installer needs network access when WebView2 is missing; the installer is not yet configured for signing.
- Third-party binary licensing and formal release acceptance require independent review.

## [1.0.0] — Historical release

Vue 3 + Python/Eel implementation, using external yt-dlp / FFmpeg and PyInstaller. The exact release date cannot be determined from the current source, so no date is invented. The old implementation source was once archived under legacy/python-eel/ and is now kept only in Git history (the tag v2.0.1 can be viewed); outdated usage documents are no longer retained.
