# 第三方许可材料

当前 Windows 安装包只分发 GUI、项目 `LICENSE` 和本目录的说明材料，**不内置 `yt-dlp.exe`、`ffmpeg.exe` 或 `ffprobe.exe`**。用户在首次启动后选择工具方案，应用再从受支持的上游下载工具。

## 本项目与当前安装包

- 根目录 `LICENSE`：yt-dlp GUI 自有代码的 MIT License。
- `NOTICE.txt`：当前安装包范围、工具获取方式及历史材料说明。
- `release-status.json`：当前 GUI 分发范围与待办状态；`historicalTools` 单独保留旧工具哈希记录，不表示当前随包分发。
- `RELEASE-CHECKLIST.md`：许可核对记录，以及重新分发旧工具时需要满足的前置条件。

安装包保留本目录和根目录 `LICENSE`。首次安装后不需要有三个工具 EXE 或 `bin/`；工具安装后才写入当前用户应用安装目录的 `bin/`。

## 历史 yt-dlp 工具记录

以下材料只对应维护时记录的官方 Windows EXE `2026.08.19`，不是所有运行时下载版本：

- `yt-dlp/UNLICENSE.txt`：该版本主项目的 Unlicense。
- `yt-dlp/THIRD_PARTY_LICENSES.txt`：该版本官方 PyInstaller EXE 的第三方组件许可汇总。
- `yt-dlp/SOURCE-INFO.md`：该版本来源、源码归档及待核验事项。
- `evidence/yt-dlp-SHA2-256SUMS.txt`：该版本官方校验和，供核对历史本地工具。

## 历史 FFmpeg / ffprobe 工具记录

- `ffmpeg/COPYING.LGPLv2.1.txt`、`ffmpeg/COPYING.GPLv2.txt`、`ffmpeg/COPYING.GPLv3.txt`：已收集的上游许可文本。
- `ffmpeg/SOURCE-INFO.md`：历史 Gyan Essentials 9.0.2 本地构建的来源、配置和待核验事项。

历史 Gyan 构建记录启用了 `--enable-gpl` 和 `--enable-version3`。这些记录不代表当前基础版/完整版下载结果；实际下载供应方、版本、配置和许可见工具目录里的 `licenses/installed-tools.json` 与 `licenses/`。

## 检查范围

在 frontend 目录运行 `npm run licenses:check` 检查说明材料及当前不捆绑工具的配置；`npm run licenses:release` 在当前 GUI 发布待办未完成时失败。当前 `pending` 与 `historicalPending` 均为空，两项检查均通过。

`historicalTools` 仍保留旧工具哈希与来源记录，用于溯源；它们不代表当前随包分发内容，也不构成再分发承诺。若将来重新随包分发工具二进制，必须先完成独立的许可与对应源码审核，并把条目写回 `historicalPending`，否则门禁不会拦截。

GUI 的 MPL 准确版本源码现直接保存在 `gui/sources/`，获取说明见 `gui/SOURCE-NOTICE.txt`；这些 GUI 源码不能替代 yt-dlp / FFmpeg 的完整对应源码。本目录不是法律意见或第三方工具书面源码承诺。