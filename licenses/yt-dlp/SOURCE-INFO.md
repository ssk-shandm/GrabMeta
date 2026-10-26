# yt-dlp 源码和第三方组件获取说明

> **历史工具记录；当前 GUI 安装包不包含此 EXE。** 本文是历史本地开发工具的溯源记录，不是源码提供承诺。工具本身由用户在首次启动后从 yt-dlp 官方 GitHub Release 下载，来源见 `bin/licenses/installed-tools.json`。

历史本地开发目录记录的 `yt-dlp.exe` 使用官方 yt-dlp Windows 发布版本 `2026.08.19`，SHA-256 为：

```text
66674953fe251b89f4d08c5f0e35e0728679bd67ab3d7d05c0562af101dd3e7a
```

## yt-dlp 源码

- 官方发布页：<https://github.com/yt-dlp/yt-dlp/releases/tag/2026.08.19>
- 对应源代码归档：<https://github.com/yt-dlp/yt-dlp/releases/download/2026.08.19/yt-dlp.tar.gz>
- 源代码仓库标签：<https://github.com/yt-dlp/yt-dlp/tree/2026.08.19>
- yt-dlp 本身的许可证：`UNLICENSE.txt`

源代码归档的官方 SHA-256 记录位于 `../evidence/yt-dlp-SHA2-256SUMS.txt`。

## PyInstaller 第三方组件

官方 Windows PyInstaller 可执行文件还包含 Python 运行时和其他第三方组件。该版本已收集的许可证文本保留在同目录 `THIRD_PARTY_LICENSES.txt`，不表示当前安装包分发该 EXE。其中说明：组件源代码可从原始项目取得；若无法取得，可联系 yt-dlp 维护者：<maintainers@yt-dlp.org>。

本文件和上述官方发布材料只对应 `2026.08.19`。更换 yt-dlp 后应重新下载对应版本的 `LICENSE`、`THIRD_PARTY_LICENSES.txt`、校验和和源码说明。
