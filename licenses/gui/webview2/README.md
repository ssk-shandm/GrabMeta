# WebView2 来源、通知和安装方式（2026-10-08）

## SDK 静态 Loader（与 Runtime 分开）

Rust 绑定 webview2-com-sys 0.39.1，发布源码提交 `edc2caf886175ccaebe86078c9cfe1ae2a187328` 的 update-bindings 指定 Microsoft.Web.WebView2 **1.0.3800.47**。实际 x64 WebView2LoaderStatic.lib SHA-256 `89c6b872783b8f6c3cedbff618adb42082d455c615453ae10cfc753f1e8f25d8`，与 Microsoft 官方 NuGet 对应文件一致。原 SDK `LICENSE.txt`、`NOTICE.txt`、nuspec 保留于本目录，不用 Rust 绑定的 MIT 替代。

- https://api.nuget.org/v3-flatcontainer/microsoft.web.webview2/1.0.3800.47/microsoft.web.webview2.1.0.3800.47.nupkg
- https://github.com/wravery/webview2-rs/blob/edc2caf886175ccaebe86078c9cfe1ae2a187328/crates/update-bindings/src/main.rs

## Evergreen Runtime

正式 NSIS 使用 `downloadBootstrapper` + `silent: true`。缺失 Runtime 时由安装器从 Microsoft 获取引导程序并安装；存在满足要求的 Runtime 则复用。不是 embedBootstrapper、offlineInstaller 或 Fixed Version，不承诺缺失运行时且无网络时可以安装。隔离资源测试继续 `skip`，不操作本机共享运行时。卸载 GUI 不卸载共享 Runtime。

Microsoft 下载页分别以以下官方端点提供分发/最终用户流程条款；已保留原始 JSON、未改动 HTML，以及供 NSIS 使用的纯文本。转换仅处理 HTML 排版和实体，原始材料 SHA-256 记录在 `../redistribution-materials.json`。

- 分发者：https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us
- 最终用户：https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us&consumer=true

条款原文要求的 Microsoft 数据收集及 SmartScreen 告知已纳入 `../INSTALLER-NOTICE.txt`，该文件被 Tauri `bundle.licenseFile` 选为 NSIS 许可页。保留 Microsoft 隐私声明与 SmartScreen 说明入口，不把独立 Runtime 标成 MIT。“关于”页也有离线 GUI 通知入口。

`silent: true` 只表示 Runtime 引导程序的参数；交互 NSIS 安装仍展示许可页。GUI 的 `/S`、`/P` 或自动更新部署可能绕过交互页，部署者必须另行履行适当告知/同意义务，不能推定用户已经接受。分发者本身对 Microsoft 原始分发条款的承诺也不是通过保存文件就自动履行。

## 支持范围（2026-10-26 决策）

最低支持 Windows 10 / 11，两者自带 Evergreen Runtime，因此本项目**不再为缺少 WebView2 的旧版 Windows 做主动适配，也不再保留该项验收待办**。安装器保留 `downloadBootstrapper` 联网引导作为兜底；缺少运行时且无法联网的机器不属于支持范围。

本机已有 Runtime，不卸载它以制造测试环境；隔离 `skip` 资源测试不覆盖正式安装路径。该项按产品决策关闭，不是验收通过。

官方依据：

- https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- https://developer.microsoft.com/en-us/microsoft-edge/webview2/
- https://v2.tauri.app/distribute/windows-installer/
