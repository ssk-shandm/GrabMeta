[English](readme.en.md) | **简体中文**

<div align="center">

# GrabMeta 中文版

**yt-dlp 视频下载工具的 Windows 中文图形界面**

[![Version](https://img.shields.io/github/v/release/ssk-shandm/GrabMeta?label=version)](https://github.com/ssk-shandm/GrabMeta/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20x64-0078D6?logo=windows&logoColor=white)](#开发启动)
[![Vue](https://img.shields.io/badge/Vue-3-4FC08D?logo=vuedotjs&logoColor=white)](https://vuejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?logo=typescript&logoColor=white)](https://www.typescriptlang.org/)
[![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

</div>

## 项目简介

GrabMeta 中文版是基于 **Vue 3 + TypeScript + Tauri 2 / Rust** 构建的 Windows 桌面应用，为 yt-dlp 视频下载器提供中文图形界面。用户无需输入命令，即可完成视频下载、音频提取以及字幕与封面获取。

下载与音视频合并由外部的 **yt-dlp** 与 **FFmpeg** 完成；Windows 安装包采用 **NSIS** 格式发布。

## 主要功能

- 解析视频链接，查看封面、可用的视频/音频格式以及人工字幕列表。
- 支持一键下载最佳画质，或自定义格式与音视频组合，并可选择 MP4 / MKV / WebM 封装格式。
- 下载封面、简介及指定语言的字幕，并在终端中查看工具输出与下载进度。
- 通过系统原生对话框选择下载目录，并保存目录与重试次数设置。
- 在“关于 → 设置”中可配置本地 VPN 客户端提供的 HTTP / SOCKS5 代理。仅接受 localhost 及回环地址，本应用不会安装或启动 VPN。
- 提供当前会话的任务列表，支持取消任务及失败提示；正常退出时自动清理下载进程树。
- 提供 Windows x64 NSIS 安装包。首次启动时可选择基础版或完整版工具，工具将自动安装至本机 `bin/` 目录，无需单独安装 Python、Node.js 或 Rust。选择基础版后，可在“关于”页升级为完整版。
- 桌面端支持检查 GitHub Releases 更新。发现新版本时，将下载并启动匹配的 `*-setup.exe`；任务运行期间会延后安装。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 界面 | Vue 3、TypeScript、Vite、Pinia、vue-i18n、Arco Design Vue、TDesign Vue Next |
| 桌面框架 | Tauri 2 |
| 后端 | Rust（Tauri 命令、进程管理、工具下载与更新） |
| 外部工具 | yt-dlp、FFmpeg / ffprobe（首次启动时下载至本机） |
| 打包 | NSIS（Windows x64 安装包） |

## 开发启动

运行环境要求：Windows、Node.js **22.18+ 或 24+**、Rust stable（MSVC）、Visual Studio C++ Build Tools 及 WebView2。推荐使用 Node.js 22 或 24 LTS。本项目仅在 Windows x64 平台上验证。

```powershell
cd frontend
npm ci
npm run desktop:dev
```

如仅需预览界面，可运行 `npm run dev`。普通浏览器不提供 Tauri IPC，因此无法下载或选择本地目录。

## 更新机制

自动更新仅在 Tauri 桌面端可用；浏览器预览、开发版及便携版不会尝试替换当前程序。应用直接读取本项目的 GitHub Releases API，比较正式版本号，并在 Release 资产中查找文件名以 `-setup.exe` 结尾的 Windows NSIS 安装包。

- “关于”页可手动检查并安装更新；启动时的自动检查可在“设置”中关闭。
- 下载或解析任务运行期间不会启动安装，任务结束后再继续。下载过程中显示进度。
- 下载器使用系统信任的根证书及 Windows 系统代理，并保持 HTTPS 证书校验。下载失败时，提示会显示底层的证书、连接或超时原因。
- 更新链接仅接受 HTTPS GitHub Release 资产。安装器启动后应用退出，由 NSIS 完成覆盖安装并重新启动应用。
- 发布新版本时，须上传由 `npm run desktop:build` 生成的 `*-setup.exe`。当前实现不读取 `latest.json`、`.sig` 文件或 Tauri updater 端点。

## 打包 NSIS 安装包

```powershell
cd frontend
npm run desktop:build
```

默认产物目录为 `frontend/src-tauri/target/release/bundle/nsis/`。安装器为当前用户安装；若缺少 WebView2 运行时，将尝试联网安装。离线安装策略、签名及第三方工具许可详见 [BUILD.md](docs/BUILD.md)。

## 文档

- [项目文档索引](docs/README.md)
- [开发指南](docs/DEVELOPMENT.md)
- [架构与 IPC](docs/ARCHITECTURE.md)
- [Windows / NSIS 构建](docs/BUILD.md)
- [测试与验收](docs/TESTING.md)
- [第三方工具与分发许可](docs/THIRD_PARTY.md)
- [更新日志](docs/CHANGELOG.md)

## 项目结构

```text
frontend/
  src/                 Vue 页面、Pinia、desktop 服务
  src-tauri/           Rust 后端、Tauri 配置、权限和图标
  scripts/             构建预检、许可材料工具与桌面/UI 烟测
  tests/               Node 内置测试
  vendor/              项目自有依赖（number-precision 替换实现）
licenses/              随安装包分发的许可与来源说明
docs/                  项目文档、测试记录与更新日志
bin/                   本地下载工具，不提交 Git
```

Tauri/Windows 打包所需的应用图标位于 `frontend/src-tauri/icons/`；原始 PNG 图稿与截图等文件不提交至 Git。

旧版 Python/Eel、Docker 与 PyInstaller 实现已从当前工作树移除，仅保留在 Git 历史中。目录保留规则与旧源码查看方式详见 [开发指南](docs/DEVELOPMENT.md#项目文件维护)。

## 使用边界与安全

- 自动更新仅在桌面端可用；发布到 GitHub Release 的 Windows NSIS 安装包文件名必须以 `-setup.exe` 结尾。
- 请仅下载您有权获取的内容，并遵守目标网站的服务条款。
- 项目代码采用 [MIT](LICENSE) 许可。安装后从上游获取的第三方工具具有独立许可，不受本项目 MIT 许可约束。GUI 安装包不内置 yt-dlp、FFmpeg 及 ffprobe 可执行文件。

## 上游项目

本项目为图形界面工具，并非 yt-dlp 或 FFmpeg 的官方项目，亦不代表上游项目。核心功能依托以下上游项目实现：

- [yt-dlp](https://github.com/yt-dlp/yt-dlp)：负责网站解析与下载。
- [FFmpeg](https://ffmpeg.org/)：负责音视频合并、转封装及后处理。
- Windows 构建包由对应的发布页或构建供应方提供。应用在首次启动时，从配置的 HTTPS 上游地址下载工具，第三方可执行文件不会直接写入本项目源码仓库。

如遇 yt-dlp 或 FFmpeg 本身的问题，请优先联系相应的上游项目。本项目仅负责中文界面、任务管理及工具下载集成。

## 许可证与第三方声明

安装包内的 `LICENSE` 为本项目代码的 MIT 许可证。yt-dlp、FFmpeg、ffprobe 及 WebView2 均为独立的第三方组件，不能以本项目的 MIT 许可证代替其自身许可。首次安装完成后，GUI 从上游下载工具至本机 `bin/` 目录，安装包本身不再捆绑这些第三方可执行文件。

安装包内的 `licenses/` 目录仅提供上游来源与使用说明，并不代表安装包包含第三方工具，也不构成第三方项目的完整源码材料。详情请参阅 [第三方工具与分发许可](docs/THIRD_PARTY.md)。

## 系统要求

- 最低支持 **Windows 10 / 11**（x64）。这两个系统自带 WebView2 Runtime，本项目不再为缺少 WebView2 的旧版 Windows 做主动适配。
- 安装器保留 `downloadBootstrapper` 联网引导作为兜底；缺少运行时且无法联网的机器不在支持范围内。

## 待办事项

- **许可证**：`number-precision` 已由本项目自有的 MIT 实现替换（`frontend/vendor/number-precision`），当前版本不再依赖上游 1.6.0。上游缺失的完整版权声明暂不跟进，且不得依据 npm 作者字段补写；若将来改回上游包，需重新处理该缺口。

历史工具（yt-dlp / FFmpeg）的再分发材料待办已清除：当前安装包不包含任何第三方工具二进制，工具在首次启动时从上游下载。若将来改为随包分发工具，须重新完成独立的许可与对应源码审核，详见 [第三方工具与许可](docs/THIRD_PARTY.md) 与 [许可审核清单](licenses/RELEASE-CHECKLIST.md)。
