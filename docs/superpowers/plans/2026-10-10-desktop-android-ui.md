# Windows and Android UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** 交付可启动的 Windows GUI 与 ARM64 Android 调试 APK，展示实际设备选择和双端音频控制界面。
**Architecture:** 共用 React/TypeScript 交互与偏好，两种平台布局；Tauri Rust 提供平台信息和后台线程中的 Windows 只读设备枚举。
**Tech Stack:** Tauri 2、React、TypeScript、pnpm workspace、Phosphor、Geist、自有 CSS；CLI 初始化与依赖管理。
**Spec:** docs/superpowers/specs/2026-10-10-desktop-android-ui-design.md

## Global Constraints
- 当前是界面阶段；网络、音频服务与麦克风授权后续实现，界面清楚标识预览状态。
- 只保存设置和枚举设备；不录音、播放测试音、修改系统音量/默认设备/防火墙。
- 手机独立音量为既定目标，首轮 Android 真机为联想 Y700 四代 / Android 16。
- 不覆盖用户未跟踪的 .vscode 文件，不参考 SoundPush。
- 不写验证生成模板或 CSS 常量的镜像测试；偏好边界先写失败测试，布局用浏览器及原生启动验证。

## Review Focus
- 原生设备枚举失败必须可见，不得使用示例冒充真机。
- 本地偏好损坏或非法音量必须安全恢复，不能得到 NaN 或越界值。
- 没有网络能力时不能显示假连接、假播放或请求麦克风权限。
- 窄屏、平板及最小桌面窗口不能出现横向溢出或不可达操作。
- Android 与 Windows 平台桥接、应用资源和构建路径必须能在合并后继续使用。

### Task 1: Tauri 工程与只读桥接
**Files:** package.json、pnpm-workspace.yaml、Cargo.toml、.gitignore、apps/sonahop-ui/。
**Interfaces:**
- Produces: UiAppInfo { platform: "windows" | "android" | "other", version: string, uiPreview: boolean }。
- Produces: UiPlaybackDevice { id: string, name: string, isDefault: boolean }。
- Produces: get_app_info() -> UiAppInfo；async list_playback_devices() -> Result<Vec<UiPlaybackDevice>, String>。
- [x] 用 pinned create-tauri-app CLI 创建 react-ts，pnpm 初始化根私有 workspace；通过 CLI 添加图标、字体及 Windows 路径依赖，移除无用 opener 插件。
- [x] 手工连接 Cargo/pnpm workspace 并配置应用名 SonaHop、identifier io.github.redpandaex.sonahop 和最小窗口；锁文件由 CLI 生成。
- [x] 实现只读桥接，Windows 枚举使用 spawn_blocking 保持 COM 所有权；Android 不调用 Windows 枚举。
- [x] 运行 pnpm 前端 build 与 cargo check -p sonahop-ui。Expected: 成功；没有开始音频流。提交本任务。

### Task 2: 双端布局与共享交互
**Files:** src/App.tsx、src/screens/、src/components/、src/lib/bridge.ts、src/lib/preferences.ts、src/lib/preferences.test.ts、src/App.css。
**Interfaces:**
- Consumes: Task 1 平台信息和设备 DTO。
- Produces: Preferences { version: 1, mode: "listen" | "microphone", volume: number, theme: "light" | "dark" | "system", sourceId: string, computerName: string, computerAddress: string }。
- Produces: readPreferences(raw: string | null): Preferences，序列化/保存失败不会使界面崩溃。
- [x] 先写偏好测试并观察 RED：null/损坏 JSON 恢复默认；非法模式/主题拒绝；音量非有限值恢复 70、有效数值限制 0..100；未知版本恢复默认；保存/读回选择不变。
- [x] 实现偏好与桥接、真实错误和浏览器示例标识；运行 Node 原生测试。Expected: 边界通过。
- [x] 完成暖白/深色桌面、跟随系统主题的手机/平板布局；模式互斥、真实来源选择/刷新、独立音量设置、电脑信息编辑和主题可用。
- [x] 用 Playwright CLI 检查交互、键盘、窄屏无溢出、深浅主题、设备错误与不能开始假共享；截图查看后修正实际问题。
- [x] 运行 pnpm test / typecheck / build。Expected: 成功。提交本任务。

### Task 3: Windows 与 Android 原生界面
**Files:** 品牌 SVG、src-tauri/icons/、src-tauri/gen/android/、构建说明。
**Interfaces:** 使用 Task 1/2 实际应用，不添加音频业务。
- [x] 用 tauri icon CLI 从代码矢量标记生成图标；tauri android init CLI 创建原生 Android 工程。
- [x] 用 tauri build --no-bundle 构建 Windows GUI，并实际启动核对设备枚举和界面。
- [x] 用 tauri android build --debug --apk --target aarch64 编译；Windows 缺少符号链接权限时，复制产出的库并用 Gradle assembleArm64Debug 打包。Expected: 包名正确、仅 ARM64、调试签名。
- [x] Y700 连接时安装/启动/截图；没有设备则明确真机未测并保存 APK。未授权不卸载现有应用或丢弃数据。
- [x] 保存截图与忽略的构建产物；完整 Rust 测试和 Clippy 通过。提交本任务。

### Task 4: 最终验证、独立审查与交付
**Files:** README.md、docs/superpowers/verification/2026-10-10-desktop-android-ui.md、阶段计划。
**Interfaces:** 使用前三任务实际源码与构建文件。
- [x] 写根目录开发命令、直接启动 exe 与安装 APK 的步骤；明确当前可操作内容和预览边界。
- [x] 记录实际 Node/Rust 检查、浏览器视口截图、Windows 和 APK 构建、原生运行与真机状态。
- [x] 进行一次独立分支审查，修复重要问题；其余建议明确保留。
- [ ] 本地快进交付主目录，保留用户文件、必要产物及完整执行取舍；不推送 GitHub。
