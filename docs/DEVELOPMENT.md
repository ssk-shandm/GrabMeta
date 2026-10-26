**简体中文** | [English](DEVELOPMENT.en.md)

# 开发指南

## 环境

当前目标为 Windows x64。安装 Node.js（推荐 22/24 LTS）、Rust stable 的 `x86_64-pc-windows-msvc` 工具链、Visual Studio 2022 Build Tools 的“使用 C++ 的桌面开发”（含 Windows SDK）和 Microsoft Edge WebView2 Runtime。

依赖分别由 `frontend/package-lock.json` 和 `frontend/src-tauri/Cargo.lock` 锁定。当前统一使用 npm；旧 pnpm lock 仅在 Git 历史中保留。

## 初始化

1. 在 `frontend/` 执行 `npm ci`。
2. 执行 `npm run desktop:dev`。首次启动桌面端后，按工具安装窗口选择基础版或完整版。

本地调试如已准备工具，可将 `yt-dlp.exe`、`ffmpeg.exe`、`ffprobe.exe` 放入根目录 `bin/`，再运行 `npm run tools:check` 检查；这些文件不会进入安装包。

Vite 固定监听 `127.0.0.1:1420`，端口冲突时直接报错；Tauri 负责启动前端与 Rust 窗口。关闭开发窗口后检查终端，必要时用 Ctrl+C 结束开发服务。

## 常用命令（在 frontend/ 执行）

| 命令 | 作用 |
| --- | --- |
| `npm run dev` | 仅浏览器界面预览，不具备桌面能力 |
| `npm run desktop:dev` | 完整桌面开发模式 |
| `npm run build` | TypeScript 检查和前端生产构建 |
| `npm run lint:check` | ESLint 检查，不修改源码 |
| `npm test` | Node 内置测试；需 Node.js 22.18+ / 24 |
| `npm run test:ui` | 无头浏览器界面烟测，不访问真实站点 |
| `npm run tools:check` | 检查本地可选的开发工具，不影响 GUI 构建 |
| `npm run check:rust` | Rust 编译检查 |
| `npm run test:rust` | Rust 单元测试 |
| `npm run test:desktop` | 构建后运行真实桌面烟测（会短暂打开窗口，临时修改并恢复设置） |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Rust 格式检查 |
| `npm run desktop:build` | 前端、Rust release、NSIS 打包 |
| `npm run desktop:build:install-test` | 构建隔离的测试安装包，不影响正式版身份 |
| `npm run test:install-resources` | 隔离 NSIS 静默安装，逐字节核验全部许可资源并卸载；不操作正式版或 WebView2 |
| `npm run test:tools` | 联网工具安装/取消/升级烟测；仅使用隔离测试 EXE，需要下载较大的上游归档 |
| `npm run licenses:installed -- <安装目录>` | 只读核验根 LICENSE 与 licenses/ 全部文件；不等于许可审核 |
| `npm run licenses:check` | 检查许可材料、清单、哈希及打包配置 |
| `npm run licenses:release` | 发布门禁；`pending` 非空时失败（当前为空，通过） |
| `npm audit` | 检查 JS 依赖公告；不要直接强制降级或升级全部依赖 |

## 修改代码

前端页面不得直接调用底层命令：使用 `src/services/desktop.ts`，并同步维护 `src/types/desktop.ts`。新增 Rust 命令需注册到 `invoke_handler`。Rust 子进程不能通过 shell 执行拼接字符串，也不能开放任意可执行文件路径。

发布时保持 `package.json`、`Cargo.toml`、`tauri.conf.json` 的版本一致，并更新 `docs/CHANGELOG.md`。仓库只提交 `frontend/src-tauri/icons/` 中打包所需的图标；原始 PNG 图稿和 `frontend/app-icon.png` 是本地设计/生成输入，已被 Git 忽略。若本地存在图标源，重新生成可运行：

```powershell
# 先在临时目录生成，再仅复制当前 Windows 构建所需图标。
$iconOutput = Join-Path $env:TEMP ('ytdlp-icons-' + [guid]::NewGuid())
npm run tauri -- icon app-icon.png --output $iconOutput --fit contain
if ($LASTEXITCODE -ne 0) { throw '图标生成失败' }
'icon.ico', 'icon.png', '32x32.png', '128x128.png', '128x128@2x.png' |
  ForEach-Object {
    Copy-Item -LiteralPath (Join-Path $iconOutput $_) -Destination 'src-tauri/icons/'
  }
```

## 项目文件维护

- 有效入口为 `frontend/src/main.ts` 与 `frontend/src-tauri/src/main.rs`。Python/Eel、Docker 和 PyInstaller 入口不再维护。
- `frontend/vendor/` 存放项目自有依赖，目前是 `number-precision` 的替换实现，通过 `package.json` 的 `file:` 依赖和 `overrides` 引用。
- `frontend/dist/` 是 Vite 构建产物，与旧版根目录 `dist/` 无关。
- Git 忽略的本地产物包括：根目录 `bin/`、`frontend/node_modules/`、`frontend/dist/`、`frontend/src-tauri/target/`、`.grabmeta-install-*/` 测试证据目录和 `tmp_ffmpeg_download/`（旧名 `.ytdlp-gui-install-*/` 仍保持忽略）。这些目录不提交，也不随安装包分发；需要保留的测试结论以 [测试与验收](TESTING.md) 为准。
- 仓库只提交 `frontend/src-tauri/icons/` 中打包配置引用的 Windows ICO/PNG 及通用 `icon.png`。原始图稿、截图、下载媒体和测试安装证据不提交。
- 前端业务代码使用 TypeScript；`scripts/` 与 `tests/` 中的 `.mjs` 文件是构建、烟测和测试脚本，不是旧版残留。
- 文档中的相对链接由 `npm test` 检查。新增、删除或重命名 `docs/` 文件时，同步更新 [文档索引](README.md)。发布说明只使用 [发布说明模板](RELEASE_TEMPLATE.md)。

旧实现保留在 Git 标签 `v2.0.1` 中，可在项目根目录只读查看，不会覆盖当前工作树：

```powershell
git ls-tree -r --name-only v2.0.1 legacy/python-eel/
git show v2.0.1:legacy/python-eel/main.py
```

新增或删除文件后运行 `npm test`、`npm run lint:check`、`npm run build`，并核对图标、文档链接和 Tauri 资源映射。发布前另执行工具预检和 NSIS 打包。

## 设置与数据

设置文件位于 Tauri `app_config_dir()` 下的 `settings.json`；Windows 通常是 `%APPDATA%/com.ssk-shandm.grabmeta/settings.json`。以运行时解析结果为准。默认下载目录来自系统 Downloads，不依赖工作目录。损坏或非法设置会回退默认值。旧配置没有 concurrentFragments 时自动补齐 8；有效范围为 1–16，UI 提供 1/4/8/16 路，保存时后端再次验证。

任务记录为本次运行内存状态，最多保留 200 条已结束任务；终端最多保留 2000 行。下载目录的文件不会因清空任务记录而删除。
