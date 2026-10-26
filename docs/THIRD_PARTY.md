**简体中文** | [English](THIRD_PARTY.en.md)

# 第三方工具与许可

项目 MIT 许可只覆盖本项目代码，不覆盖 yt-dlp、FFmpeg、WebView2 等第三方组件。

## 下载工具

- yt-dlp：建议采用官方 Windows 发布 EXE。单文件构建含运行时与依赖，不能仅凭主项目许可推断整个二进制的分发条件。
- FFmpeg / ffprobe：应采用来源可追溯的 Windows x64 构建。运行 `-version`、`-L`、`-buildconf` 查看版本与构建选项。
- WebView2：使用 Microsoft 官方引导器/运行时与 Tauri 支持的分发模式。

本仓库不托管上述 EXE，GUI 安装包也不包含这些第三方二进制。用户首次启动后可在 GUI 中选择并从受支持的上游地址下载工具；第三方程序的许可证和使用条件由相应上游项目或构建供应方负责。

## 上游与下载策略

应用不再把 yt-dlp、FFmpeg 和 ffprobe 作为固定 EXE 写入安装包。首次启动时，用户选择“基础版”或“完整版”，应用通过 HTTPS 从配置的上游发布地址下载并安装到本机 `bin/`；“关于”页可以再次下载完整版。

- yt-dlp 上游：[yt-dlp/yt-dlp](https://github.com/yt-dlp/yt-dlp)。
- FFmpeg 上游：[ffmpeg.org](https://ffmpeg.org/)。
- Windows 二进制构建供应方及其许可材料必须随版本记录；具体地址、版本和构建选项应以实际下载结果为准。
- 基础版和完整版必须分别记录其构建类型、适用许可证、第三方通知和对应源码入口。

发布说明不列出安装包或工具的 SHA-256。哈希可以作为维护者本地排查和缓存校验手段，但不是用户必须阅读的发布信息，也不能替代来源、许可证或对应源码材料。

开发调试时仍可使用以下命令检查本地工具：

```powershell
../bin/yt-dlp.exe --version
../bin/ffmpeg.exe -version
../bin/ffmpeg.exe -L
../bin/ffmpeg.exe -buildconf
../bin/ffprobe.exe -version
```

## 随包说明

项目将根目录 `licenses/` 作为 Tauri resource 一起打包到安装目录，内容包括：

- 面向用户的 `NOTICE.txt`；
- 上游项目和构建供应方链接；
- 已收集的第三方许可证与来源说明。旧 yt-dlp / Gyan FFmpeg 材料明确标为历史工具记录，不代表运行时下载版本。

这些文件是来源和使用提示，不表示安装包包含 yt-dlp 或 FFmpeg，也不提供 yt-dlp / FFmpeg 的完整对应源码归档。GUI 的 MPL 精确版本源码另保存在 `licenses/gui/sources/`。

这不等于自动完成全部法律审核。如果更换下载供应方、工具版本或构建类型，必须重新更新版本、来源和许可证提示。应用下载并校验工具后，会把实际安装记录写入用户本机的 `bin/licenses/installed-tools.json`。

## GUI 依赖与 WebView2 通知

已补充精确版本的 MPL 源码归档、复合许可与 Unicode 通知，以及 Microsoft 官网 Runtime 原始条款，均随 `licenses/gui/` 分发。NSIS 许可页和“关于”页提供版权与条款告知；工具安装后仍可独立访问 GUI 通知。

number-precision 已替换为项目自有 MIT 实现（`frontend/vendor/number-precision`），不再依赖上游 1.6.0。上游 1.6.0 缺少完整版权通知，项目不凭 npm 作者字段补写，详见 [GUI 许可记录](../licenses/gui/README.md)。

## 发布检查

具体审核动作和当前缺口见 [许可审核清单](../licenses/RELEASE-CHECKLIST.md)。这是对交付材料与适用条件的核对，不是收费认证。

- 依赖组合条款和 WebView2 通知已记录在 `licenses/gui/`。
- 最低支持 Windows 10 / 11，两者自带 WebView2 Runtime；本项目不再为缺少运行时的旧版 Windows 做适配或验收。安装器保留 `downloadBootstrapper` 联网引导作为兜底。

在 `frontend/` 目录运行 `npm run licenses:check` 检查随包说明材料。工具下载成功不等于 GUI 代替上游承担第三方项目的许可证义务；发布时应保留清楚的上游链接和用户提示。
