[简体中文](readme.md) | **English**

<div align="center">

# GrabMeta

**A Windows graphical interface for the yt-dlp video downloader**

[![Version](https://img.shields.io/github/v/release/ssk-shandm/GrabMeta?label=version)](https://github.com/ssk-shandm/GrabMeta/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6?logo=windows&logoColor=white)](#development)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

</div>

## Overview

GrabMeta is a Windows desktop application built with **Vue 3 + TypeScript + Tauri 2 / Rust**. It provides a graphical interface for the yt-dlp video downloader, allowing you to download videos, extract audio, and fetch subtitles and covers without typing commands. The interface follows your system language (Simplified Chinese or English) and can be switched in the sidebar.

Downloading and media merging are performed by the external **yt-dlp** and **FFmpeg** tools. The Windows installer is distributed in **NSIS** format.

## Features

- Analyze a video link to view its cover, available video and audio formats, and the list of manual subtitles.
- Download the best quality in one click, or choose a specific format and video/audio combination, with MP4 / MKV / WebM as the container.
- Download covers, descriptions, and subtitles in a chosen language, and view tool output and download progress in the terminal.
- Choose the download folder with a native dialog. The folder and retry count settings are saved.
- Under "About → Settings", configure an HTTP or SOCKS5 proxy provided by a local VPN client. Only localhost and loopback addresses are accepted. The application does not install or start a VPN.
- View the task list for the current session, cancel tasks, and receive failure notices. On normal exit, download process trees are cleaned up automatically.
- Windows x64 NSIS installer. On first launch, choose the basic or full set of tools; they are installed automatically into the local `bin/` folder, so Python, Node.js, and Rust do not need to be installed separately. After choosing the basic set, you can upgrade to the full set from the About page.
- The desktop application can check GitHub Releases for updates. When a new version is found, the matching `*-setup.exe` is downloaded and started. Installation is postponed while a task is running.

## Technology Stack

| Layer | Technology |
| --- | --- |
| Frontend | Vue 3, TypeScript, Vite, Pinia, vue-i18n, Arco Design Vue, TDesign Vue Next |
| Desktop framework | Tauri 2 |
| Backend | Rust (Tauri commands, process management, tool download and updates) |
| External tools | yt-dlp, FFmpeg / ffprobe (downloaded to the local machine on first launch) |
| Packaging | NSIS (Windows x64 installer) |

## Development

Requirements: Windows, Node.js **22.18+ or 24+**, Rust stable (MSVC), Visual Studio C++ Build Tools, and WebView2. Node.js 22 or 24 LTS is recommended. This project is only tested on Windows x64.

```powershell
cd frontend
npm ci
npm run desktop:dev
```

To preview the interface only, run `npm run dev`. A regular browser has no Tauri IPC, so downloads and local folder selection will not work.

## Updates

Automatic updates are only available in the Tauri desktop application. Browser previews, development builds, and portable builds never attempt to replace the running program. The application reads the project's GitHub Releases API directly, compares stable version numbers, and looks for a Windows NSIS installer whose file name ends with `-setup.exe`.

- The About page can check for and install updates manually. The automatic check at startup can be turned off in Settings.
- Installation does not start while a download or parsing task is running; it continues after the task finishes. Download progress is displayed.
- The downloader uses the system's trusted root certificates and the Windows system proxy, and keeps HTTPS certificate verification enabled. When a download fails, the message shows the underlying certificate, connection, or timeout reason.
- Update links only accept HTTPS GitHub Release assets. After the installer starts, the application exits, and NSIS completes the overwrite installation and relaunches the application.
- When publishing a release, upload the `*-setup.exe` produced by `npm run desktop:build`. The current implementation does not read `latest.json`, `.sig` files, or a Tauri updater endpoint.

## Packaging the NSIS Installer

```powershell
cd frontend
npm run desktop:build
```

The default output folder is `frontend/src-tauri/target/release/bundle/nsis/`. The installer installs for the current user and attempts to download the WebView2 runtime over the network if it is missing. For the offline installation policy, signing, and third-party tool licenses, see [BUILD.md](docs/BUILD.md) (in Chinese).

## Documentation

The project documentation is currently written in Chinese.

- [Documentation index](docs/README.md)
- [Development guide](docs/DEVELOPMENT.md)
- [Architecture and IPC](docs/ARCHITECTURE.md)
- [Windows / NSIS build](docs/BUILD.md)
- [Testing and acceptance](docs/TESTING.md)
- [Third-party tools and redistribution licenses](docs/THIRD_PARTY.md)
- [Changelog](docs/CHANGELOG.md)

## Project Structure

```text
frontend/
  src/                 Vue pages, Pinia stores, desktop services, i18n
  src-tauri/           Rust backend, Tauri config, permissions, and icons
  scripts/             Build checks, license tools, desktop and UI smoke tests
  tests/               Node built-in tests
  vendor/              Project-owned dependency (number-precision replacement)
licenses/              License and source notices shipped with the installer
docs/                  Project documentation, test records, and changelog
bin/                   Locally downloaded tools; not committed to Git
```

The application icons used by Tauri/Windows packaging are located in `frontend/src-tauri/icons/`. Original PNG artwork and screenshots are not committed to Git.

The legacy Python/Eel, Docker, and PyInstaller implementations have been removed from the working tree and remain only in Git history. For the rules on retaining these directories and viewing the legacy source, see the [development guide](docs/DEVELOPMENT.md#项目文件维护) (in Chinese).

## Scope and Safety

- Automatic updates are only available in the desktop application. Windows NSIS installers published to GitHub Releases must have a file name ending with `-setup.exe`.
- Only download content you are authorized to access, and comply with the terms of service of the target sites.
- The project code is licensed under the [MIT License](LICENSE). Third-party tools fetched from upstream after installation are governed by their own licenses; the project's MIT license does not replace them. The GUI installer does not bundle the yt-dlp, FFmpeg, or ffprobe executables.

## Upstream Projects

This project is a graphical interface. It is not an official project of yt-dlp or FFmpeg and does not represent them. The core functionality relies on the following upstream projects:

- [yt-dlp](https://github.com/yt-dlp/yt-dlp): website parsing and downloading.
- [FFmpeg](https://ffmpeg.org/): audio and video merging, container conversion, and post-processing.
- Windows builds are provided by the corresponding release pages or build providers. On first launch, the application downloads the tools from configured HTTPS upstream addresses. Third-party executables are not committed to this source repository.

For issues with yt-dlp or FFmpeg themselves, please contact the respective upstream projects first. This project is responsible only for the interface, task management, and tool download integration.

## Licenses and Third-Party Notices

The `LICENSE` file in the installer is the MIT license for this project's code. yt-dlp, FFmpeg, ffprobe, and WebView2 are separate third-party components. They are not covered by this project's MIT license and remain under their own licenses. After the first successful installation, the GUI downloads the tools from upstream into the local `bin/` folder; the installer itself no longer bundles these third-party executables.

The `licenses/` folder in the installer provides only upstream source information and usage notes. It does not indicate that the installer contains the third-party tools, and it is not the complete source material for those projects. For details, see [third-party tools and redistribution licenses](docs/THIRD_PARTY.md) (in Chinese).

## System requirements

- Minimum supported systems: **Windows 10 / 11** (x64). Both ship the WebView2 Runtime, so this project no longer actively adapts to older Windows builds without it.
- The installer keeps `downloadBootstrapper` as a network fallback; a machine without a Runtime and without network access is outside the supported scope.

## TODO

- **Licensing**: `number-precision` has been replaced by the project's own MIT implementation (`frontend/vendor/number-precision`), and the current version no longer depends on upstream 1.6.0. The complete copyright notice missing from upstream is not being pursued at this time and must not be filled in from npm author fields. If the project switches back to the upstream package, this gap must be addressed again.

The redistribution TODO for the historical tools (yt-dlp / FFmpeg) has been cleared: the current installer contains no third-party tool binaries, and the tools are downloaded from upstream on first launch. If the tools are ever bundled with the installer again, a separate license and corresponding-source review must be completed first. See [third-party tools and licenses](docs/THIRD_PARTY.md) and the [license review checklist](licenses/RELEASE-CHECKLIST.md).
