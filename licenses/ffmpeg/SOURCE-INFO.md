# FFmpeg / ffprobe 历史本地构建记录（Gyan 9.0.2，非当前下载源）

> **历史工具记录；当前 GUI 安装包不包含这些 EXE。** 本文只记录早期本地开发目录中的 Gyan 构建，用于溯源，**不代表应用实际下载的 FFmpeg**。

## 当前实际下载源

应用从 `BtbN/FFmpeg-Builds`（Windows 二进制构建供应方）下载，按用户选择取 `ffmpeg-master-latest-win64-lgpl.zip`（基础版）或 `-gpl.zip`（完整版）。实际版本、来源与校验记录写在安装目录的 `bin/licenses/installed-tools.json` 与 `bin/licenses/ffmpeg/` 中。上游获取资源包是本项目的设计方式，安装包不内置任何 FFmpeg 二进制。

## 历史本地构建（Gyan）

历史本地开发目录记录的 `ffmpeg.exe` 和 `ffprobe.exe` 是 Gyan FFmpeg Windows x64 Essentials static build。

### 对应构建

- 构建版本：FFmpeg 9.0.2
- 构建日期：2026-09-19
- 构建标识：`9.0.2-essentials_build-www.gyan.dev`
- 构建归档：`ffmpeg-9.0.2-essentials_build.zip`
- 构建归档官方下载地址：<https://github.com/GyanD/codexffmpeg/releases/download/9.0.2/ffmpeg-9.0.2-essentials_build.zip>
- 构建归档 SHA-256：`60f467265b1e312373dbcd92200c2618a74850f98d3d078e94296bb3fa2047ba`
- Gyan 构建页：<https://www.gyan.dev/ffmpeg/builds/>

## 对应源代码

Gyan 在 9.0.2 发布记录中指向以下 FFmpeg 源代码提交：

- FFmpeg 源代码提交：<https://github.com/FFmpeg/FFmpeg/commit/946fcce07b>
- FFmpeg 源代码仓库：<https://github.com/FFmpeg/FFmpeg>

如无法访问网页，可在 FFmpeg 仓库中检出该提交：

```text
git clone https://github.com/FFmpeg/FFmpeg.git
git -C FFmpeg checkout 946fcce07b
```

## 本构建配置

本机 `ffmpeg.exe -buildconf` 报告的历史构建配置如下。`ffprobe.exe` 自报相同构建标识。

```text
--enable-gpl
--enable-version3
--enable-static
--disable-w32threads
--disable-autodetect
--enable-cairo
--enable-fontconfig
--enable-iconv
--enable-gnutls
--enable-libxml2
--enable-gmp
--enable-bzlib
--enable-lzma
--enable-zlib
--enable-libsrt
--enable-libssh
--enable-libzmq
--enable-avisynth
--enable-sdl2
--enable-libwebp
--enable-libx264
--enable-libx265
--enable-libxvid
--enable-libaom
--enable-libopenjpeg
--enable-libvpx
--enable-mediafoundation
--enable-libass
--enable-libfreetype
--enable-libfribidi
--enable-libharfbuzz
--enable-libvidstab
--enable-libvmaf
--enable-libzimg
--enable-amf
--enable-cuda-llvm
--enable-cuvid
--enable-dxva2
--enable-d3d11va
--enable-d3d12va
--enable-ffnvcodec
--enable-libvpl
--enable-nvdec
--enable-nvenc
--enable-vaapi
--enable-openal
--enable-libgme
--enable-libopenmpt
--enable-libopencore-amrwb
--enable-libmp3lame
--enable-libtheora
--enable-libvo-amrwbenc
--enable-libgsm
--enable-libopencore-amrnb
--enable-libopus
--enable-libspeex
--enable-libvorbis
--enable-librubberband
```

> 注：上面的列表按历史本地工具输出记录。若将来改为随包分发二进制，必须重新运行 `ffmpeg.exe -buildconf` 并更新本文件；若构建供应方对上游源代码、第三方库或构建脚本做了修改，还须同时提供对应修改后的源码和构建材料。

## 许可

该历史构建启用了 GPL 组件和 version3 组件。本目录保留 `COPYING.LGPLv2.1.txt`、`COPYING.GPLv2.txt` 和 `COPYING.GPLv3.txt` 作为参考材料。当前版本不随包分发 FFmpeg，用户下载的工具由上游供应方承担相应许可与源码义务；实际许可以 `bin/licenses/` 中记录的实际构建为准。
