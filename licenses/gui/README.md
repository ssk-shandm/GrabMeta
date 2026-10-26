# GUI 许可与源码交付记录（2026-10-08，未发布）

## 已落地

- 保留根 MIT 原文及 `Copyright (c) 2025 ssk-shandm`，未擅自改动版权所有者或年份。Tauri 资源映射保留根 `LICENSE` 和 `licenses/`。
- `dependency-inventory.json` 与锁文件一致，`packages/`、`supplemental/` 保留各包原始通知；`review.json` 逐项记录 OR 选择、AND 累加及开发依赖范围。无法从 metadata 判定的 `memorystream@0.3.1` 以实际发布包 MIT LICENSE 为依据。
- `unicode-ident` 采用 **MIT AND Unicode-3.0**；`brotli` 的 BSD-3-Clause AND MIT、`dpi` 的 Apache-2.0 AND MIT、`ring` 的 Apache-2.0 AND ISC 及其包内其他通知均保留。没有把组合许可统一标成 MIT。ring 使用自己的 ISC / Apache / vendored 许可，不因 BoringSSL 来源就套用旧 OpenSSL 许可。
- Windows Cargo 图中所有 MPL-2.0 包（cssparser、cssparser-macros、dtoa-short、option-ext、selectors）的准确版本 `.crate` 源码**直接随安装资源交付**，不是只放一个可能失效的链接。已与 Cargo.lock checksum 匹配，并将归档所有文件和实际安装的 crate 逐字节核对，无本地修改。详见 `SOURCE-NOTICE.txt` 与 `redistribution-materials.json`。
- 为复合许可/文件级署名额外保留 brotli、dpi、ring、unicode-ident 完整准确版本源码。原始源码中的版权信息没有被伪造或删减。
- lightningcss / Windows npm binding 是 Vite 的构建依赖，不作为构建工具 EXE 部署到 GUI 安装目录；保守通知清单继续保留它们。将来分发构建工具本身时必须单独交付相应源码并重审。
- Microsoft WebView2 静态 SDK Loader 与独立 Evergreen Runtime 条款分开保存，见 `webview2/`。
- 新安装器使用 `INSTALLER-NOTICE.txt` 作为 NSIS 许可页，包含中文分发/隐私告知、未经修改的 GUI MIT 原文及 Microsoft 官方最终用户条款全文。“关于”页另有固定的 GUI 通知目录入口，不被已安装工具的 `bin/licenses/` 遮蔽。

## number-precision：已由项目自有实现替换

`@arco-design/web-vue@2.58.0` 运行时依赖的上游 `number-precision@1.6.0` 只声明 `MIT`，没有完整版权/授权文本。上游 1.1.4 至 1.6.0 全部标签及提交历史均无 LICENSE 文件，不能凭 npm 的 author 字段补写版权人或年份。

当前版本改为 `frontend/vendor/number-precision`（版本 `1.6.1`，私有包，不发布到 npm）。它以 BigInt 实现 Arco 实际调用的 `times`、`plus`、`divide`、`round` 与 `enableBoundaryChecking`，许可证为项目根 `LICENSE` 的副本，通过 `frontend/package.json` 的 `overrides` 接入。依赖清单中该包记为 `npm:number-precision@1.6.1`，`unresolved` 为空。

验证：与上游 1.6.0 对 95,848 个样本比对，`round` 完全一致；`times`、`plus`、`divide` 的差异仅为末位浮点误差，最大相对误差约 2e-14。已有 `frontend/tests/number-precision.test.mjs` 固定关键用例，并通过 `npm test`、`npm run build` 与 `npm run test:ui`。

实现未复制上游源码，上游 1.6.0 仅用于行为对照测试。若将来改回上游包，须重新处理其版权通知缺口。

## WebView2 分发与支持范围

生产配置维持 `downloadBootstrapper`，明确 `silent: true`（运行时引导程序静默，不等于 GUI 安装器不显示许可页）。已有满足要求的共享 Runtime 会复用；缺失时从 Microsoft 下载并安装。GUI 不内置 Evergreen 完整离线包或 Fixed Version Runtime；卸载 GUI 不删除共享运行时。网络失败或权限问题不得宣称可离线安装。

Microsoft 官方下载页使用的 EULA API 已取回，原始 JSON、HTML 与仅转换排版/HTML 实体的纯文本分别保留。`runtime-distributor-terms.*` 是供发布者核对的分发条款，`runtime-consumer-terms.*` 是官网最终用户下载流程使用的条款；它们**不是 SDK Loader 的 BSD 通知**。安装前提示包含 Microsoft Defender SmartScreen 及 Microsoft 隐私入口，并明确第三方条款不改变 GUI MIT。

自动/静默/被动安装不会让用户看见交互许可页；部署者须另行预先告知并取得适当同意，不能以“安装成功”作为同意证明。最低支持 Windows 10 / 11，两者自带 Evergreen Runtime，因此无 Runtime 的旧版 Windows 与断网安装不属于支持范围，该项验收按产品决策关闭（不是验收通过）。已存在 Runtime 的本机启动与隔离 `skip` 测试只覆盖资源核验，不覆盖上述不支持场景。

## 官方依据及可复现命令

- Mozilla FAQ Q8 / Q10 / Q16：https://www.mozilla.org/en-US/MPL/2.0/FAQ/
- Microsoft 部署文档：https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution
- Tauri Windows 安装方式：https://v2.tauri.app/distribute/windows-installer/
- Microsoft 官网分发条款：https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us
- Microsoft 官网最终用户条款：https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us&consumer=true

在 frontend 执行：

```powershell
npm run licenses:inventory       # 未解决通知存在时非零退出；仍生成清单
npm run licenses:redistribution  # 精确源码核验、官网条款、安装告知和材料哈希
npm run licenses:check           # 材料/哈希/锁文件/分发配置检查
npm run licenses:release         # 当前通过；待办非空时会阻止发布
```

重新生成依赖清单后须重新运行 `licenses:redistribution`；该脚本不捏造 number-precision 通知，也不自动解除法律/安装验收待办。构建和资源核验记录见仓库 `docs/TESTING.md`；旧安装包不会因本次源码更改自动更新。
