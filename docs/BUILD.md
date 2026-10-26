**简体中文** | [English](BUILD.en.md)

# Windows x64 / NSIS 构建

## 依赖

开发机需 Node.js、Rust MSVC、C++ Build Tools / Windows SDK、WebView2。第一次 Rust 编译和 NSIS 打包需要网络下载 crate、Tauri 打包工具及运行时引导文件，耗时可能较长。

## 外部工具

安装包只包含 GUI 本体，不捆绑 `yt-dlp.exe`、`ffmpeg.exe` 或 `ffprobe.exe`。用户首次启动后选择基础版或完整版，应用通过 HTTPS 从受支持的上游下载并安装到本机 `bin/`。

因此，干净克隆可以直接构建不含第三方 EXE 的安装包。`npm run tools:check` 仅用于检查本地开发目录中的工具（若存在），不代表安装包包含这些工具。下载来源、供应方和用户提示见 [THIRD_PARTY.md](THIRD_PARTY.md)。

## 构建命令

```powershell
cd frontend
npm ci
npm run build
npm test
npm run test:rust
npm run desktop:build
```

产物在 `src-tauri/target/release/bundle/nsis/`。release 主 EXE 在 `src-tauri/target/release/` 中。NSIS 安装包部署主 EXE、`LICENSE` 和 `licenses/`，不部署工具 `bin/`。发布时使用完整安装包，不只分发主 EXE。

## 隔离安装测试构建

开发机已安装正式版时，不得为测试而卸载正式版或 WebView2。使用独立配置：

```powershell
cd frontend
npm run desktop:build:install-test
```

该命令合并 `src-tauri/tauri.install-test.conf.json`，不修改正式版 `tauri.conf.json`：

| 项目 | 测试值 |
| --- | --- |
| 产品名 / NSIS 注册表子项 | `GrabMeta Install Test` |
| 应用标识 / 配置目录名 | `com.ssk-shandm.grabmeta.install-test` |
| 主 EXE | `grabmeta-install-test.exe` |
| 开始菜单目录 | `GrabMeta Install Test` |
| WebView2 安装模式 | `skip`，不安装、卸载或升级现有运行时 |

安装到新建的独立测试目录（可含中文和空格），不要选择正式安装目录。先检查生成的 `target/release/nsis/x64/installer.nsi`：产品名、主 EXE、应用标识必须都是测试值；`INSTALLWEBVIEW2MODE` 与 `MINIMUMWEBVIEW2VERSION` 应为空。生成模板将配置中的 `skip` 表示为空模式。

测试版继承正式版资源，但不允许下载或启动正式版更新安装器。使用 `/S` 做静默安装；NSIS `/D=绝对测试目录` 参数放在最后。卸载前核对测试版注册表 `InstallLocation`，只运行该目录的 `uninstall.exe`。不要运行正式版安装器来测试“升级”。

安装前后保存正式版安装文件、配置文件及相关注册表快照，比较它们与 WebView2 记录，确认无变化。测试构建和正式构建共用 Rust target 目录；下一次发布必须重新执行 `npm run desktop:build`，不要把测试 EXE 或测试安装包发布为正式版。

隔离测试只能验证已有 WebView2 环境下的安装、启动、同版本重装及卸载，不代表干净系统、缺失 WebView2、离线引导器或正式版跨版本升级通过。结果见 [TESTING.md](TESTING.md)。

隔离配置的 `silent: null` 是 JSON merge patch：将类型改为 `skip` 时删除正式配置的 `silent` 字段，避免生成 `skip + silent` 这一不符合 schema 的组合，不影响正式版的联网策略。

## 当前安装配置

- 产品名称：`GrabMeta`；窗口标题为中文版名称。
- 应用标识：`com.ssk-shandm.grabmeta`。2.3.3 起由 `com.ssk-shandm.ytdlp-gui` 改名，与产品名统一；应用启动时会把旧配置目录的 `settings.json` 自动迁移一次，升级用户的下载目录与偏好不会丢失。
- NSIS：当前用户安装，简体中文/英文安装界面，可选择语言。
- 安装包只含 GUI、项目 LICENSE 和文档。第三方工具在首次启动后下载到当前用户的应用工具目录（`bin/`），主 EXE 不内嵌 FFmpeg。
- WebView2：`downloadBootstrapper`。缺少运行时时需要联网，不是完全离线安装器。
- 自动更新由应用层实现：桌面端查询项目 GitHub Releases API，筛选版本号和文件名以 `-setup.exe` 结尾的 NSIS 资产，下载后启动安装器。发布时上传 `npm run desktop:build` 生成的 `*-setup.exe` 即可；当前不读取 `latest.json`、`.sig` 或 Tauri updater endpoint。

## 应用图标

图标原始图稿为根目录 `视频下载工具图标设计.png`，生成源为 `frontend/app-icon.png`。Windows EXE、安装器和卸载器读取 `frontend/src-tauri/icons/icon.ico`，包含 16 / 32 / 48 / 256px 等尺寸；打包配置同时引用 `32x32.png`、`128x128.png`、`128x128@2x.png`，另保留通用 `icon.png`。重新生成方式见 [开发指南](DEVELOPMENT.md#修改代码)。

替换图标后需运行 `npm run desktop:build` 重新构建；已有 EXE 和安装包不会自动变化。

## 离线部署选项

如需离线部署，在 Windows 构建配置中将 `bundle.windows.webviewInstallMode.type` 改为 `offlineInstaller`，然后重新构建，并在无网络、无 WebView2 的 Windows 测试机上验收。离线 WebView2 运行时会增大安装包；FFmpeg 仍不内置，首次下载工具需要网络。

## 安装包体积

安装包不包含第三方工具，体积主要由 GUI、Tauri/WebView2 安装引导器和许可证材料决定。首次下载工具的体积随上游版本及用户选择的基础版/完整版变化。

## 发布前

1. 在非开发机或独立 Windows 用户环境安装，验证路径含中文与空格、标准用户权限、首次启动。
2. 完成 [TESTING.md](TESTING.md) 中的下载、合并、取消和退出验收。
3. 验证覆盖安装、卸载、WebView2 缺失和离线场景，确认不会删除用户下载文件。
4. 审核第三方工具的来源、许可证说明和下载校验策略。
5. **可选**：有预算时通过 Tauri Windows 签名配置接入代码签名证书。公益项目可以先发布未签名包，不把证书费用作为构建或自动更新的硬性前置条件。未签名包可能出现 SmartScreen 提示，应在发布说明中如实说明，并保留 GitHub Release 资产的 SHA-256 元数据供更新器校验；不要求把哈希写进面向用户的发布正文。
6. 同步三个版本号、更新日志和构建验证记录，再发布。

仅有成功编译或本地 NSIS 产物，不能证明覆盖安装、卸载、签名和所有站点下载都通过测试。

## 发布更新包

1. 在 `frontend/` 更新 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的版本号，并同步 `docs/CHANGELOG.md`。
2. 运行 `npm run desktop:build`，构建不包含第三方 EXE 的 GUI 安装包。
3. 从 `src-tauri/target/release/bundle/nsis/` 取出以 `-setup.exe` 结尾的安装包，并完成测试清单。
4. 先创建对应标签的草稿 Release，核对安装包名称、大小和说明后再公开发布。自动更新通过 GitHub 的 latest Release API 检查正式版本；预发布版本不应作为正式更新目标。
5. 如实记录代码签名（未签名时也要明确写明）、第三方许可材料和覆盖安装验收状态；发布不代表这些检查自动通过。本项目更新器不要求付费代码签名。未签名安装包仍可能被 SmartScreen、Smart App Control 或组织策略提示或阻止执行，不能保证所有 Windows 环境都可直接安装。仅上传主 EXE 的 Release 不满足自动更新要求。

自动更新只接受本项目 GitHub Release 的 Windows x64 NSIS 安装包，并要求 GitHub API 提供有效的 `sha256:` 资产 digest。下载大小和 SHA-256 校验失败、digest 缺失或格式错误时，均停止自动安装，不降级为无校验执行；旧资产若缺少 digest，改由用户在发布页核实后手动安装。哈希校验用于完整性检查，不等同于发布者签名。

自动更新只适用于已安装的当前用户 NSIS 版本。开发版、便携版或运行目录中没有 `uninstall.exe` 的程序会拒绝自动安装，避免误替换开发文件。

### Release 内容格式

Release 说明统一使用 [发布说明模板](RELEASE_TEMPLATE.md)，只保留面向用户的以下四部分：

1. **更新内容**：新增功能、体验优化和问题修复。
2. **下载与安装**：安装包选择、安装或升级方式。
3. **使用须知**：平台限制、签名、WebView2 和必要的安全提示。
4. **文件信息**：真实安装包名称、平台和大小。

公开 Release 不包含“验证”“测试结果”“验收记录”等章节，也不附测试数量、构建、lint、Clippy 或烟测通过列表。内部验证记录在 [TESTING.md](TESTING.md)，不复制到发布正文。模板占位符必须替换为本次实际信息，不能把未包含在安装包中的源码变更写成已发布功能。

仅调整已发布 Release 文案时，同步对应的本地发布说明；保持标签、版本号、发布时间和安装包资产不变。

### Release 说明编码

发布说明按 [发布说明模板](RELEASE_TEMPLATE.md) 编写，并直接维护在 GitHub Release 页面。使用 GitHub API 时，将 JSON 显式转换为 UTF-8 字节，并设置 `Content-Type: application/json; charset=utf-8`；不要依赖 Windows PowerShell 的默认字符串编码。

## 许可材料

`npm run licenses:check` 检查项目自身及随包说明材料。打包配置会将项目 `licenses/` 目录复制到安装目录，因此安装包包含：

```text
LICENSE
licenses/
  NOTICE.txt
  README.md
  yt-dlp/
  ffmpeg/
```

发布时应使用完整 NSIS 安装包。用户安装工具后，本机才会生成 `bin/`。更换下载上游或构建类型后，先更新 [THIRD_PARTY.md](THIRD_PARTY.md) 和 `licenses/` 中的说明，再运行检查和构建命令。

项目不随安装包分发第三方工具二进制；`licenses/` 仅提供上游来源、用户提示和项目依赖说明，不替代各上游项目的许可证。正式发布前仍应运行 `npm run licenses:check`，并确认下载来源和用户界面说明准确。

## GUI 版权、源码与 WebView2 安装前告知

生产分发维持 WebView2 `downloadBootstrapper`，明确静默运行时安装。NSIS 交互安装通过 `bundle.licenseFile` 显示中文告知、GUI MIT 原文及 Microsoft 官方最终用户条款。共享 Runtime 不随 GUI 卸载。无网络且缺失 Runtime 不属于可安装范围。


`licenses/gui/sources/` 随资源交付准确版本的 MPL 和复合许可组件源码；`SOURCE-NOTICE.txt` 说明解压与源码获取方式。更新依赖或通知材料后，依次运行 `npm run licenses:inventory`（有 unresolved 时非零退出）和 `npm run licenses:redistribution`，再运行 `licenses:check` 与 `licenses:release`。number-precision 已由项目自有 MIT 实现替换，详见 [GUI 许可记录](../licenses/gui/README.md)。最低支持 Windows 10 / 11，两者自带 WebView2 Runtime；不再为缺少运行时的旧版系统做适配，该项待办已按产品决策关闭。
