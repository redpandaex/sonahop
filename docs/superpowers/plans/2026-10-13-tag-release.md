# Tag Release Implementation Plan

> Superpowers executing-plans；用户已确认 main 历史提交上的版本 tag 才能发布，固定 Android 正式签名，密钥与 GitHub Secrets 稍后自行配置。当前分支只提交推送 workflow，不合并 main、不创建 tag、不实际发布。

**Goal:** 在版本 tag 推送后验证 main 归属，构建 Windows exe/NSIS 与 ARM64 Release APK，固定签名并验证后一次发布完整 GitHub Release。
**Architecture:** validate → build → publish 三个 job；复用已有 PowerShell 构建脚本，Node 24 执行独立类型检查的 TypeScript 发布工具。
**Spec:** docs/superpowers/specs/2026-10-13-tag-release-design.md

## Constraints
- 普通 push、PR、手动 dispatch 均不发布；不把 branches/main 和 tags/v* 写成误导性的 AND 条件。
- tag 必须属于 main 历史、版本匹配三份应用配置；构建固定在验证后的提交，发布前再次核对远端 tag 与 main。
- 仅发布当前支持的 Windows x64 与 Android ARM64；虚拟音频驱动仍为独立外部组件。
- 四项 Android Secrets 缺失立即失败，不生成或回退 debug 签名；本轮不生成/上传真实密钥。
- 不运行自动测试或真机/音频；做 workflow 静态检查、脚本类型检查和必要 Release 编译，实际签名待用户配置。

## Tasks
- [x] 1. 新建 tag 校验、产物摘要和发布工具，设置 Node 脚本独立类型检查。
- [x] 2. 已有构建脚本支持 unsigned Android Release，添加只读取环境签名参数的 APK 签名/校验工具。
- [x] 3. workflow 固定工具/Action 版本，main/tag gate、Secrets fail-fast、完整产物发布和草稿重试。
- [x] 4. 写固定签名配置/发版说明，完成静态复核与 Android Release 编译。

交付方式：提交并推送当前分支；不合并 main、不创建发布 tag。推送结果记录在本地交付 ledger。

## Verification (2026-10-16)
- actionlint、独立 TypeScript 类型检查、Biome、PowerShell 语法及 Git diff 检查通过。
- Android ARM64 unsigned Release 构建成功，0.5.0 / versionCode 5000，非 debuggable；APK 中 JNI 导出与保留的桥接类静态检查通过。
- 独立代码复核发现上传期间 tag 移动的窗口：已在公开前再次检查显式远端 tag/main，并校验 tag 仍存在。
- 正式签名、GitHub runner 首次完整构建与真机行为待用户配置密钥及测试，未运行自动化测试。

## Review Focus
- tag 在 main 历史/不在 main/删除/版本不符/构建期间移动时只能发布正确提交。
- 普通 push 和 PR 不触发发布；构建失败或 Secret 缺失不能产生部分公开 Release。
- 私钥和密码不入 Git、日志或 Release 资产；APK 必须非 debuggable 且固定签名验证成功。
- 同 tag 重试只能覆盖未公开草稿，不能覆盖已公开 Release。
- CI 冷启动工具链/SDK/Opus/Gradle路径真实可用，Release JNI 不被 R8 混淆破坏。

## CI follow-up (2026-10-16)
- Evidence: first v0.5.0 run failed in setup-android before compilation: `Failed to find package 'tools'`, sdkmanager exit 1. Node 20 deprecation is a separate Action-runtime warning.
- Replace setup-android with pinned v4.0.4 (Node 24), pin command-line tools 15859902, and explicitly install platform-tools only. Silence full license-text output.
- Also replace the pinned pnpm/upload/download Actions that declare Node 20 with verified Node 24 releases; keep existing inputs, artifact layout and main/tag gates.
- Static checks: actionlint, release-script type checking and source diff; no runtime/device tests. Verify each pinned Action manifest declares Node 24.
- Owner explicitly approved upgrading package.json, Cargo.toml, tauri.conf.json and Cargo.lock to 0.5.1, committing and pushing. Keep v0.5.0 unchanged; the owner will create the new release tag.
- Verification completed: actionlint, release TypeScript check, package formatting, version consistency and offline locked Cargo metadata passed; all 7 pinned Action manifests declare Node 24. Full hosted CI awaits a new tag; no automated tests or device/audio checks ran.
