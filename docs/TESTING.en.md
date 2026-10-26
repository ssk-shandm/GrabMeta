[简体中文](TESTING.md) | **English**

# Testing and acceptance

This page records automated checks, local verification results from each run, and the items currently outside the verification scope. **Passing automated checks does not mean that installation, network, site, or release review has passed.** Every item that is not covered is listed under [System requirement and out-of-scope items](#system-requirement-and-out-of-scope-items).

## Current status (2026-10-26, v2.3.3)

The current version is **2.3.3**, and the minimum supported systems are Windows 10 / 11. The table below shows the results of the most recent run.

| Check | Result |
| --- | --- |
| `npm test` | 55 / 55 passed |
| `npm run build` | Passed |
| `npm run lint:check` | Passed |
| `npm run test:ui` | Passed |
| `npm run licenses:inventory` | 548 packages, 0 unresolved |
| `npm run licenses:check` | Passed |
| `npm run licenses:release` | Passed (release items cleared) |

For the most recent Rust and desktop smoke-test records, see [2026-10-08](#2026-10-08-update-chain-download-optimization-and-isolated-installation). They were not re-run this time.

## Automated commands

Run these in the `frontend/` directory:

```powershell
npm run build
npm run lint:check
npm test
npm run test:ui
npm run test:guide
npm run tools:check
npm run licenses:check
npm audit
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
npm run test:rust
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
npm run desktop:build
npm run test:desktop
```

- Frontend tests cover task events, the history size limit, settings initialization, protocol registration, NSIS/resource configuration, the auto-update controller, license materials, the project-owned number-precision replacement, and relative links in the documentation.
- Rust tests cover URL and parameter validation, download modes, metadata mapping, UTF-8 streams, JSON size limits, task concurrency limits, and update installer verification.
- `npm run tools:check` only checks optional local development tools. It does not mean the installer contains those tools.
- Passing tests does not mean a target website can definitely be downloaded. Sites, logins, regional restrictions, and the yt-dlp version all affect the results.

## Historical verification records

### 2026-10-09: project-owned number-precision replacement

- Replacement: `frontend/vendor/number-precision` (MIT, version 1.6.1), which replaces upstream 1.6.0 through the `$number-precision` override. No upstream source was copied; upstream was used only for behavioral comparison.
- Comparison: compared with upstream 1.6.0 over 95,848 samples. `round` results are fully identical. The maximum relative error of `plus`, `times`, and `divide` is about 2e-14, a floating-point representation difference. Infinity / -Infinity / NaN on division by zero match upstream.
- Unit tests: 5 cases added in `number-precision.test.mjs`, covering floating-point error, extremely small quotients, negative numbers, rounding, and the absence of negative zero.
- Dependencies and licenses: `licenses:inventory` reports 543 packages with 0 unresolved. After `licenses:redistribution` regenerated the materials, `licenses:check` passed.
- Build and UI: `npm run build`, `npm run lint:check`, and `npm run test:ui` passed.

### 2026-10-08: update chain, download optimization, and isolated installation

- **Update chain**: release checks use a native HTTPS client that shares the proxy and system root certificates with installer downloads. Parser tests cover x64 NSIS selection, digest verification, and version comparison. If the digest is missing or does not match, nothing is installed. Verification happens before the task update lock is taken.
- **Network smoke test**: read the NSIS asset of GitHub's latest release (v2.0.1) and fully read 115,571,498 bytes. MZ, Content-Length, and SHA-256 all passed. The smoke test does not write or launch the installer.
- **Download optimization**: media fragments use 8 connections by default (options 1 / 4 / 8 / 16). Large tool files use up to 4 Range connections. If any segment fails, the other writes are discarded immediately. The frontend waits for the cancel IPC to finish before allowing a retry. In a controlled local transfer of 8 MiB, one connection took 851.69 ms and 4 connections took 527.18 ms. This only shows that the concurrency implementation works; it does not represent an internet speed improvement.
- **Beginner tutorial smoke test**: 16 highlighted steps, three window sizes. Screenshot comparison allows a maximum synthetic rounding difference of 1 channel level; actual dimming still fails.
- **Isolated installation** (not a clean system): built `GrabMeta Install Test` with `tauri.install-test.conf.json`, with WebView2 mode `skip`. Silent installation to a path containing Chinese characters and spaces, first launch, configuration isolation, rejection of the update IPC identity, same-version overwrite install, and uninstall all passed. The official installation files, configuration, registry snapshots, and WebView2 registry records were unchanged before and after.
- **License resource verification**: after the isolated package was installed, `npm run licenses:installed` verified byte by byte the root LICENSE and all files under licenses/. The first result: 802 files, 4,213,143 bytes, all consistent. The install directory did not contain the three tool EXEs. Evidence is in the ignored directory `.ytdlp-gui-install-resources-*/result.json`; the most recent run is authoritative.
- **Other**: JS 33 / 33 passed; Rust 22 passed, with 1 network smoke test ignored by default; ESLint, cargo fmt, and clippy passed.

```powershell
npm run desktop:build:install-test
npm run test:install-resources
```

`test:install-resources` refuses to overwrite when the test identity is already installed. Before uninstalling, it confirms that the target is inside this run's dedicated test directory, and it does not install or uninstall the official release or WebView2. When uninstalling the test build, it uses the NSIS `_?=` parameter to wait for completion. The uninstaller left in the directory is local evidence, not a leftover from the official release.

This isolated test only proves "the installer was regenerated and the LICENSE and licenses/ resources in the installed copy were verified." It does not equal cross-version upgrade of the official identity, nor clean-system acceptance.

### 2026-10-06: 2.0.0 and 2.0.1 releases

- **2.0.1**: fixed the certificate trust problem in update downloads by using system root certificates and enabling the Windows system proxy, while keeping HTTPS verification. After reproducing `InvalidCertificate(UnknownIssuer)` on this machine, the fixed build read the same URL completely. Frontend 20 / 20 and Rust 12 / 12 passed; the network smoke test passed when run separately. PAC, proxy authentication, offline use, and other machine environments were not verified.
- **2.0.0**: local build of 115,596,191 bytes, SHA-256 `de5fb0162348e6b7ed364cfd1be4a97ec3711607dae2bb1a5ad730666a8966a8`. Frontend 19 / 19 and Rust 10 / 10 passed. `npm run test:desktop` covers six download modes, parsing, covers, subtitles, paths with Chinese characters and spaces, concurrency limits, cancellation, and exit cleanup.

## Update download network smoke test

Regular Rust tests do not access the internet. The following ignored test can be run separately. It uses the same system certificates, proxy, and timeout settings as the production downloader to fully read a GitHub Release installer, then checks the EXE's MZ signature, Content-Length, and the GitHub asset SHA-256. It does not write or launch the installer.

```powershell
# Run in the frontend directory; you can also replace the URL with the GitHub Release installer URL you want to verify.
$release = Invoke-RestMethod 'https://api.github.com/repos/ssk-shandm/GrabMeta/releases/latest' -Headers @{ 'User-Agent' = 'grabmeta-test' }
$asset = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
if (-not $asset.digest.StartsWith('sha256:')) { throw 'Missing GitHub asset SHA-256' }
$env:YT_DLP_GUI_UPDATE_TEST_URL = $asset.browser_download_url
$env:YT_DLP_GUI_UPDATE_TEST_SHA256 = $asset.digest.Substring(7)
cargo test --manifest-path src-tauri/Cargo.toml github_release_download_network_smoke -- --ignored --nocapture
```

This test generates network traffic for a complete installer (about 110 MiB). It does not mean that NSIS overwrite installation or all network environments have been accepted.

## Interface and desktop smoke tests

**Browser UI smoke test** (`npm run test:ui`): uses the local Edge or Chrome in headless mode and starts a separate Vite service on port 1421. It closes the browser and the service when finished. It does not access real video sites and does not create download tasks. If no browser is found, you can specify Chromium with `UI_BROWSER_PATH`. It covers the cover state, preventing duplicate actions while loading, retaining input after route changes, the About page links and the opener call branch, rejection of non-allowlisted links, the reduced-motion preference, the task count badge, the three tool configuration dialogs, and a horizontal overflow check of all pages at 1280 / 960 / 640 px widths. Passing does not mean the native system browser was actually launched.

**Real desktop smoke test** (`npm run test:desktop`): uses the release EXE and the download tools bundled as app resources, with the working directory set to a temporary directory. It connects through playwright-core to a temporary CDP port of the test process's WebView2. The official configuration does not expose a debugging port. It covers real Vue input and analysis operations, quick / specified-format / audio-plus-video combination downloads, covers, descriptions, subtitles, output directories containing Chinese characters and spaces, rejection of non-HTTP URLs, 404 error events, the list of supported sites, the 4-task concurrency limit, cancellation, native window closing, and child-process cleanup. Media is generated by the local FFmpeg and served over loopback HTTP. This does not represent compatibility with external video sites.

The script briefly opens the app window and changes settings, then attempts to restore them when it finishes. **Do not run it at the same time as an app instance you use day to day**, because both share the configuration directory. You can pass the path of the installed EXE:

```powershell
node scripts/smoke-desktop.mjs 'C:\实际安装目录\grabmeta.exe'
```

## Desktop / post-install manual checklist

The maintainer verified the following items one by one on a local machine (2026-10-26).

- [x] No sample cover is shown initially; the version, license, and repository links on the About page open in the system browser.
- [x] The launch window works normally and does not depend on Python, Chrome, or a separate Node.js.
- [x] The first download directory is the system Downloads folder; after selecting a directory with Chinese characters and spaces and restarting, the setting is preserved.
- [x] Switching retries between 3 / 5 / 10 / infinite is preserved after restart.
- [x] Normal video analysis: cover, formats, and subtitles are correct; loading stops on failure or an invalid URL.
- [x] Changing the input during analysis does not apply stale results and does not allow downloading an incorrect link.
- [x] Quick download, specified format, and audio-plus-video combination each succeed, and the merged output plays.
- [x] Cover, description, and manual subtitle files are each generated correctly; a video without subtitles shows an empty list normally.
- [x] The site list and tool errors are output correctly to the terminal.
- [x] Multi-task logs have IDs; with 4 tasks at capacity, the fifth is rejected.
- [x] After canceling a task and after closing the app, no yt-dlp / FFmpeg child processes remain, and partially canceled files are not mislabeled as successful.
- [x] HTML-like text in logs is only text and does not execute scripts.
- [x] The NSIS installer works normally for a standard user: installation, overwrite install, and uninstall. The retention policy for the download directory and settings data is as expected.
- [x] The installed version completed the online bootstrapper flow and first launch on Windows 10 / 11 with WebView2 present (see the minimum system requirement below).
- [x] Launch the installed GUI from a non-development path and confirm that first run prompts the tool installation; after installation, the three tools load from the `bin/` directory under the main program directory.
- [x] On an installed version, manually check for updates in the About page: when a new Release exists, its version is shown; download and launch `*-setup.exe`, after which the app exits.
- [x] With automatic updates off, launching the app does not download anything; manual checks still work; with active tasks, an update enters the waiting state and is installed only once after the tasks finish.
- [x] When `*-setup.exe` is missing, the app is a development or portable build, the GitHub API fails, or the download is incomplete, the UI shows an error and does not replace the current program.

## System requirement and out-of-scope items

**The minimum supported systems are Windows 10 / 11.** Both ship the WebView2 Runtime, so this project no longer actively adapts to older Windows builds without it and no longer keeps that acceptance item open. The installer keeps `downloadBootstrapper` as a network fallback; a machine without a Runtime and without network access is outside the supported scope.

The following items are outside the current verification scope and are not release gates:

- **Older Windows**: Windows 8.1 and earlier, and machines without the WebView2 Runtime that cannot reach the network, are unsupported.
- **Installation and upgrade**: cross-version overwrite upgrade under the official product identity, the interactive installer wizard, the "delete app data" option during uninstall, and UAC and permission boundaries.
- **Signing and prompts**: the installer is unsigned; SmartScreen behavior and code signing are out of scope.
- **Real network**: complete download, installation, interruption, and upgrade with the real upstream tools in both configurations; actual downloads, logins, and regional restrictions on sites such as YouTube and Bilibili.
- **Native interaction**: manual operation of all UI controls, fallback for a corrupted configuration, packaging encoding compatibility, and playback results.
- **Process exit**: child-process leftovers after a forced termination or system crash. The current coverage is only for normal window closing.
- **License review**: complete license and source review of third-party dependencies, see [licenses/RELEASE-CHECKLIST.md](../licenses/RELEASE-CHECKLIST.md).
