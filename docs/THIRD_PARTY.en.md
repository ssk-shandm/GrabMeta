[简体中文](THIRD_PARTY.md) | **English**

# Third-party tools and licenses

The project's MIT license covers only the project's own code. It does not cover third-party components such as yt-dlp, FFmpeg or WebView2.

## Download tools

- yt-dlp: the official Windows release EXE is recommended. A single-file build contains its runtime and dependencies, so the distribution terms of the whole binary cannot be inferred from the main project's license alone.
- FFmpeg / ffprobe: should be a Windows x64 build with a traceable source. Run `-version`, `-L` and `-buildconf` to check the version and build options.
- WebView2: use the Microsoft official bootstrapper/runtime and the distribution mode supported by Tauri.

This repository does not host the EXE files above, and the GUI installer does not contain these third-party binaries. After the first launch, users can choose in the GUI and download the tools from supported upstream addresses. The licenses and terms of use of third-party programs are the responsibility of the respective upstream projects or build providers.

## Upstream and download policy

The application no longer writes yt-dlp, FFmpeg and ffprobe as fixed EXE files into the installer. On first launch, the user chooses the "basic" or "full" build, and the application downloads them over HTTPS from the configured upstream release addresses and installs them to the local `bin/` folder. The About page can download the full build again.

- yt-dlp upstream: [yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp).
- FFmpeg upstream: [ffmpeg.org](https://ffmpeg.org/).
- Windows binary build providers and their license materials must be recorded with each version. Exact addresses, versions and build options should follow the actual download results.
- The basic and full builds must each record their build type, applicable licenses, third-party notices and the entry point to their corresponding source.

Release notes do not list the SHA-256 of the installer or tools. Hashes can serve as a local aid for maintainers in troubleshooting and cache verification, but they are not release information that users must read, and they cannot replace source, license or corresponding-source materials.

For development debugging, you can still use the following commands to check local tools:

```powershell
../bin/yt-dlp.exe --version
../bin/ffmpeg.exe -version
../bin/ffmpeg.exe -L
../bin/ffmpeg.exe -buildconf
../bin/ffprobe.exe -version
```

## Notices shipped with the package

The project packages the root `licenses/` folder as a Tauri resource into the installation directory. It contains:

- the user-facing `NOTICE.txt`;
- links to the upstream projects and build providers;
- the collected third-party licenses and source notes. Materials for the old yt-dlp / Gyan FFmpeg are explicitly marked as historical tool records and do not represent the runtime-downloaded versions.

These files are sources and usage notes. They do not mean that the installer contains yt-dlp or FFmpeg, nor do they provide complete corresponding-source archives for yt-dlp / FFmpeg. The exact-version MPL source for the GUI is stored separately in `licenses/gui/sources/`.

This does not amount to completing all legal review automatically. If the download provider, tool version or build type changes, the versions, sources and license notices must be updated again. After the application downloads and verifies a tool, it writes the actual installation record to `bin/licenses/installed-tools.json` on the user's machine.

## GUI dependencies and WebView2 notices

Exact-version MPL source archives, composite-license and Unicode notices, and the original Microsoft Runtime terms have been added and are distributed with `licenses/gui/`. The NSIS license page and the About page provide copyright and terms notices; after tool installation, the GUI notices remain separately accessible.

number-precision has been replaced by the project's own MIT implementation (`frontend/vendor/number-precision`) and no longer depends on upstream 1.6.0. Upstream 1.6.0 lacks a complete copyright notice, and the project does not fill in that gap from the npm author field. See the [GUI license record](../licenses/gui/README.md) for details.

## Release checks

For the specific review actions and the current gaps, see the [license review checklist](../licenses/RELEASE-CHECKLIST.md). This is a check of the delivered materials and applicable terms, not a paid certification.

- Dependency composite terms and WebView2 notices are recorded in `licenses/gui/`.
- The minimum supported systems are Windows 10 / 11, both of which ship the WebView2 Runtime; this project no longer adapts to or validates older Windows builds without it. The installer keeps `downloadBootstrapper` as a network fallback.

Run `npm run licenses:check` in the `frontend/` directory to check the license notice materials in the package. A successful tool download does not mean the GUI assumes the third-party projects' license obligations on their behalf; clear upstream links and user notices should be kept at release time.
