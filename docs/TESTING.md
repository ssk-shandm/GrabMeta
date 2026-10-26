**简体中文** | [English](TESTING.en.md)

# 测试与验收

本文记录自动化检查、各次本地验证结果，以及当前不在验证范围内的项目。**自动化检查通过不等于安装、网络、站点或发行审核已通过**；所有未覆盖的项目统一列在[系统要求与未验收范围](#系统要求与未验收范围)。

## 当前状态（2026-10-26，v2.3.3）

当前版本为 **2.3.3**，最低支持 Windows 10 / 11。下表为最近一次执行结果。

| 检查 | 结果 |
| --- | --- |
| `npm test` | 55 / 55 通过 |
| `npm run build` | 通过 |
| `npm run lint:check` | 通过 |
| `npm run test:ui` | 通过 |
| `npm run licenses:inventory` | 548 个包，unresolved 为 0 |
| `npm run licenses:check` | 通过 |
| `npm run licenses:release` | 通过（发布待办已清空） |

Rust 与桌面烟测的最近记录见 [2026-10-08](#2026-10-08更新链路下载优化与隔离安装)，本次未重跑。

## 自动化命令

在 `frontend/` 目录执行：

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

- 前端测试覆盖任务事件、历史数量限制、设置初始化、协议注册、NSIS/资源配置、自动更新控制器、许可材料、项目自有 number-precision 替换和文档相对链接。
- Rust 测试覆盖 URL/参数验证、下载模式、元数据映射、UTF-8 流、JSON 大小限制、任务并发限制和更新安装器校验。
- `npm run tools:check` 只检查可选的本地开发工具，不代表安装包包含这些工具。
- 测试通过不代表目标网站一定可下载；站点、登录、地区限制和 yt-dlp 版本都会影响结果。

## 历史验证记录

### 2026-10-09：number-precision 项目自有替换

- 替换：`frontend/vendor/number-precision`（MIT，版本 1.6.1），通过 `$number-precision` override 替代上游 1.6.0。未复制上游源码，上游仅用于行为对照。
- 对照：与上游 1.6.0 比较 95,848 个样本。`round` 结果完全一致；`plus`、`times`、`divide` 的最大相对误差约 2e-14，属于浮点表示差异。除零时 Infinity / -Infinity / NaN 与上游一致。
- 单元测试：新增 `number-precision.test.mjs` 5 项，覆盖浮点误差、极小商、负数、四舍五入及无负零。
- 依赖与许可：`licenses:inventory` 为 543 个包、unresolved 为 0；`licenses:redistribution` 重新生成后 `licenses:check` 通过。
- 构建与界面：`npm run build`、`npm run lint:check`、`npm run test:ui` 通过。

### 2026-10-08：更新链路、下载优化与隔离安装

- **更新链路**：Release 检查使用原生 HTTPS 客户端，与安装包下载共享代理和系统根证书。解析器测试覆盖 x64 NSIS 选择、digest 校验和版本比较；digest 缺失或不匹配时不安装，校验发生在任务更新锁之前。
- **联网烟测**：读取 GitHub latest（v2.0.1）的 NSIS 资产，完整读取 115,571,498 bytes，MZ、Content-Length 和 SHA-256 均通过。烟测不写入或启动安装器。
- **下载优化**：媒体分片默认 8 路（可选 1 / 4 / 8 / 16）；工具大文件最多 4 路 Range。任意分段失败时立即丢弃其他写入；前端等取消 IPC 结束后才允许重试。受控 8 MiB 本地传输中，单路 851.69 ms、4 路 527.18 ms，只说明并发实现有效，不代表互联网速度提升。
- **新手教程烟测**：16 步高亮、三种窗口尺寸；截图比较允许最大 1 级通道的合成舍入差，实际遮暗仍会失败。
- **隔离安装**（非干净系统）：使用 `tauri.install-test.conf.json` 构建 `GrabMeta Install Test`，WebView2 模式为 `skip`。中文与空格路径静默安装、首次启动、配置隔离、更新 IPC 身份拒绝、同版本覆盖重装及卸载均通过。正式版安装文件、配置、注册表快照和 WebView2 注册表记录前后一致。
- **许可资源核验**：隔离包安装后，`npm run licenses:installed` 逐字节核验根 LICENSE 和 licenses/ 全部文件。初次结果为 802 个文件、4,213,143 bytes 一致；安装目录没有三个工具 EXE。证据位于忽略目录 `.ytdlp-gui-install-resources-*/result.json`，以最新一次为准。
- **其他**：JS 33 / 33 通过；Rust 22 项通过，1 项网络烟测按默认设置忽略；ESLint、cargo fmt 和 clippy 通过。

```powershell
npm run desktop:build:install-test
npm run test:install-resources
```

`test:install-resources` 在测试身份已有安装时拒绝覆盖，卸载前确认目标位于本次专属测试目录，不安装或卸载正式版和 WebView2。测试版卸载时使用 NSIS `_?=` 参数等待完成，目录内保留的卸载器是本地证据，不是正式版残留。

该隔离测试只证明“重新生成安装包并核查已安装的 LICENSE 与 licenses/ 资源”，不等于正式身份跨版本升级或干净系统验收。

### 2026-10-06：2.0.0 与 2.0.1 发布

- **2.0.1**：修复更新下载的证书信任问题，改用系统根证书并启用 Windows 系统代理，保持 HTTPS 校验。在本机复现 `InvalidCertificate(UnknownIssuer)` 后，修复版同一 URL 可完整读取。前端 20 / 20、Rust 12 / 12 通过；网络烟测单独运行通过。未验证 PAC、代理认证、离线及其他机器环境。
- **2.0.0**：本地构建 115,596,191 bytes，SHA-256 `de5fb0162348e6b7ed364cfd1be4a97ec3711607dae2bb1a5ad730666a8966a8`。前端 19 / 19、Rust 10 / 10 通过；`npm run test:desktop` 覆盖六种下载模式、解析、封面、字幕、中文与空格路径、并发限制、取消和退出清理。

## 更新下载网络烟测

常规 Rust 测试不访问互联网。可单独运行以下忽略测试，使用生产下载器相同的系统证书、代理和超时配置，完整读取 GitHub Release 安装包，检查 EXE 的 MZ 标识、Content-Length 及 GitHub 资产 SHA-256。不写入或启动安装器。

```powershell
# 在 frontend 目录执行；也可改为待验证的 GitHub Release 安装包 URL。
$release = Invoke-RestMethod 'https://api.github.com/repos/ssk-shandm/GrabMeta/releases/latest' -Headers @{ 'User-Agent' = 'grabmeta-test' }
$asset = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
if (-not $asset.digest.StartsWith('sha256:')) { throw 'Missing GitHub asset SHA-256' }
$env:YT_DLP_GUI_UPDATE_TEST_URL = $asset.browser_download_url
$env:YT_DLP_GUI_UPDATE_TEST_SHA256 = $asset.digest.Substring(7)
cargo test --manifest-path src-tauri/Cargo.toml github_release_download_network_smoke -- --ignored --nocapture
```

该测试会产生完整安装包的网络流量（约 110 MiB）。不代表 NSIS 覆盖安装或所有网络环境已验收。

## 界面与桌面烟测

**浏览器界面烟测**（`npm run test:ui`）：使用本机 Edge / Chrome 无头模式，独立启动 1421 端口的 Vite 服务，结束时关闭浏览器和服务。不访问真实视频站点，不创建下载任务；找不到浏览器时可通过 `UI_BROWSER_PATH` 指定 Chromium。覆盖封面状态、加载中防重复操作、路由切换后输入保留、关于页链接与 opener 调用分支、非白名单链接拒绝、减少动画偏好、任务数量徽标、三个工具配置弹窗，以及所有页面在 1280 / 960 / 640px 宽度下的水平溢出检查。通过不等于原生系统浏览器已实际启动。

**真实桌面烟测**（`npm run test:desktop`）：使用 release EXE 与随应用资源提供的下载工具，工作目录设为临时目录。通过 playwright-core 连接测试进程 WebView2 的临时 CDP 端口，正式配置不开放调试端口。覆盖真实 Vue 输入与分析操作、快速/指定格式/音视频组合下载、封面、简介、字幕、中文与空格输出目录、非 HTTP URL 拒绝、404 错误事件、支持网站列表、4 任务并发限制、取消、原生窗口关闭及子进程清理。媒体由本机 FFmpeg 生成并经 loopback HTTP 提供，不代表外部视频网站兼容性。

脚本会短暂打开应用窗口并修改设置，结束时尝试恢复。**不要与日常使用的应用实例同时运行**，两者共用配置目录。测试安装后的 EXE 可传入路径：

```powershell
node scripts/smoke-desktop.mjs 'C:\实际安装目录\grabmeta.exe'
```

## 桌面 / 安装后手动清单

以下项目已由维护者在本机逐项验证通过（2026-10-26）。

- [x] 初始不显示示例封面；关于页的版本、许可证和仓库链接能在系统浏览器打开。
- [x] 启动窗口正常，不依赖 Python、Chrome 或单独的 Node.js。
- [x] 首次目录为系统 Downloads；选择中文/空格目录并重启，设置仍保留。
- [x] 切换重试 3 / 5 / 10 / infinite，重启后仍保留。
- [x] 正常视频解析：封面、格式、字幕正确；失败或非法 URL 时加载停止。
- [x] 解析时修改输入，不应用陈旧结果、不允许下载错误链接。
- [x] 快速下载、指定格式、音视频组合分别成功，合并输出可播放。
- [x] 封面、简介、人工字幕分别生成正确文件；无字幕视频正常显示空列表。
- [x] 网站列表和工具错误正确输出到终端。
- [x] 多任务日志有 ID，4 个任务满额后第五个被拒绝。
- [x] 取消任务及关闭应用后，yt-dlp / FFmpeg 子进程不残留；取消的部分文件不被误标成功。
- [x] 日志里的 HTML-like 文本只是文本，不执行脚本。
- [x] NSIS 在标准用户中安装、覆盖安装和卸载正常；下载目录和设置的数据保留策略符合预期。
- [x] 已安装版本在带 WebView2 的 Windows 10 / 11 上完成联网引导与首次启动（最低系统要求见下）。
- [x] 用非开发路径启动安装后的 GUI，确认首次运行引导安装工具；安装后从主程序目录下的 bin/ 加载三个工具。
- [x] 在已安装版本的“关于”页手动检查更新：有新 Release 时显示版本，下载并启动 `*-setup.exe`，应用随后退出。
- [x] 自动更新关闭时启动应用不下载；手动检查仍可用；有活动任务时更新进入等待状态，任务结束后只安装一次。
- [x] 没有 `*-setup.exe`、使用开发版/便携版、GitHub API 失败或下载不完整时，界面显示错误且不替换当前程序。

## 系统要求与未验收范围

**最低支持 Windows 10 / 11。** 这两个系统自带 WebView2 Runtime，因此本项目不再为缺少 WebView2 的旧版 Windows 做主动适配，也不再保留该项验收待办。安装器仍保留 `downloadBootstrapper` 联网引导作为兜底；缺少运行时的机器若无法联网，不属于支持范围。

以下项目不属于当前验证范围，也不作为发布门禁：

- **旧版 Windows**：Windows 8.1 及更早系统、缺少 WebView2 Runtime 且无法联网的机器，均不受支持。
- **安装与升级**：正式产品身份下的跨版本覆盖升级、交互式安装向导、卸载时“删除应用数据”选项、UAC 与权限边界。
- **签名与提示**：安装包未签名，SmartScreen 表现与代码签名不在验证范围。
- **真实网络**：真实上游两种工具配置的完整下载、安装、中断与升级；YouTube、Bilibili 等站点的实际下载、登录或地区限制场景。
- **原生交互**：全部 UI 控件的手动操作、损坏配置回退、封装编码兼容性和播放效果。
- **进程退出**：强制结束或系统崩溃后的子进程残留；当前覆盖的是正常窗口关闭。
- **许可审核**：第三方依赖的完整许可与源码审核，见 [licenses/RELEASE-CHECKLIST.md](../licenses/RELEASE-CHECKLIST.md)。
