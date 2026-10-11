# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 现在怎么使用

本地已经构建好的 Windows 界面可直接运行：

```powershell
.\target\release\sonahop-ui.exe
```

也可以在文件管理器中双击 `target/release/sonahop-ui.exe`。
Android 安装包位于 `artifacts/ui-preview/sonahop-android-debug.apk`，复制到手机或平板后安装即可。
这是 ARM64 调试版，约 19 MB；本轮按用户选择提供 APK，尚未安装到 Y700 真机。

**当前是界面预览。** Windows 可以选择和刷新真实播放设备、切换收听/麦克风模式和主题。
Android 提供电脑信息编辑、模式切换、音量偏好和设置页，手机与平板分别适配；外观跟随系统。
设置保存在各自设备本地。共享、连接和启动麦克风按钮尚未开放，调整滑块只保存偏好。
网络音频传输、独立音量的实际音频路径、手机麦克风服务仍待下一阶段实现。

## 首版目标

Windows 11 ↔ Android 16，两种方向分别运行：

- 收听模式：两台 Android 同时收听电脑的系统播放声音，接收设备独立控制音量，电脑静音时仍能收听。
- 麦克风模式：一台选定的 Android 采集麦克风并发送给电脑，通过已有 Windows 虚拟音频设备供应用使用；此时暂停收听。

Windows loopback 是电脑→手机的发送端，读取电脑正在播放的声音；手机→电脑会在 Android 端采集麦克风。
已有 CLI 保存 WAV 是诊断采集端的方式，不是网络传输格式。

## 开发界面

本轮环境为 Rust 1.99 / MSVC、Node 24.21、pnpm 11.25.0、Visual Studio C++ 工具链和 WebView2。
在仓库根目录运行：

```powershell
pnpm install --frozen-lockfile
pnpm dev
```

`pnpm dev` 启动 Windows Tauri 窗口。仅查看浏览器布局时运行：

```powershell
pnpm web:dev
```

打开 `http://localhost:1420/?platform=windows` 或 `http://localhost:1420/?platform=android`。
浏览器音源明确标注为示例；原生 Windows 窗口通过 Rust 枚举真实设备，读取失败会显示错误。

## 构建 Windows 与 Android

```powershell
pnpm desktop:build
```

Windows 产物是 `target/release/sonahop-ui.exe`，资源已嵌入程序，运行时不需要启动开发服务器。

Android 构建需要 JDK 21、Android SDK platform 37、NDK `30.0.16248370` 和 Rust target `aarch64-linux-android`。
设置 `JAVA_HOME`、`ANDROID_HOME`、`NDK_HOME` 后运行：

```powershell
pnpm android:build
```

产物在 `apps/sonahop-ui/src-tauri/gen/android/app/build/outputs/apk/arm64/debug/app-arm64-debug.apk`。
Android 工程已由 Tauri CLI 初始化；新工作区通常无需重复 `android:init`。
原生调试符号保留在 Cargo target，Gradle 会从安装包中移除这些符号，APK 仍保留调试属性。

Tauri CLI 在 Windows 上打包需要创建符号链接。本机未开启开发者模式，标准命令在 Rust 编译成功后因符号链接权限而停止。
本轮保留系统设置，复制刚编译的库并用 Gradle 打包。如果**唯一错误是上述符号链接权限**，可在根目录采用相同步骤：

```powershell
$androidProject = 'apps\sonahop-ui\src-tauri\gen\android'
$nativeLibrary = 'target\aarch64-linux-android\debug\libsonahop_ui_lib.so'
$jniDirectory = Join-Path $androidProject 'app\src\main\jniLibs\arm64-v8a'
New-Item -ItemType Directory -Path $jniDirectory -Force | Out-Null
Copy-Item -LiteralPath $nativeLibrary -Destination (Join-Path $jniDirectory 'libsonahop_ui_lib.so')
& (Join-Path $androidProject 'gradlew.bat') -p $androidProject :app:clean :app:assembleArm64Debug -x :app:rustBuildArm64Debug --console=plain
```

这些步骤要求前一步已成功编译**当前源码**；不要用旧库代替失败的 Rust 编译。
若设置了 `CARGO_TARGET_DIR`，`$nativeLibrary` 应使用该目录下的相同目标路径。
后续希望直接使用标准 CLI，可按 [Windows 官方开发者模式说明](https://learn.microsoft.com/windows/apps/get-started/enable-your-device-for-development)自行配置。
安装已经提供的 APK 不需要修改电脑的开发者模式。

## 采集诊断 CLI

```powershell
cargo run -p sonahop -- devices
cargo run -p sonahop -- devices --json
```

只枚举播放设备；`*` / `is_default` 标记系统默认设备，不使用 Windows 麦克风。
需要主动诊断系统播放声音时，在采集期间播放声音：

```powershell
cargo run -p sonahop -- capture --output artifacts/playback-new.wav --seconds 5
```

可以增加 `--device '<设备 id>'`。时长为 1～60 秒，默认 5 秒，已有文件不会覆盖。
WAV 为 float32，保留原设备采样率、声道数和已知扬声器布局。
Ctrl+C 停止采集并收尾文件，返回 130；读取错误也会尝试停止并收尾。
设备初始化失败可能留下空文件，重试请换文件名。
输出摘要包含写入帧数、峰值、不连续次数及窗口时长；没有音频帧与全零音频分别报告。
用户已手动播放歌曲验证默认 G24H2 loopback 采集成功；这不代表 Android 音频链路或音量独立已经通过。

## 检查与结构

```powershell
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

测试使用合成音频，不录制系统声音。
`artifacts/`、`target/`、`node_modules/` 和前端 `dist/` 均不入库。

- `crates/sonahop-core`：音频格式、采集接口、校验和有界 WAV 写入，不依赖 OS API 或 Tauri。
- `crates/sonahop-platform-windows`：Windows 端点枚举、WASAPI loopback 和原生资源生命周期。
- `apps/sonahop-cli`：采集诊断命令。
- `apps/sonahop-ui`：共用 React 界面、Tauri 只读桥接和生成的 Android 工程。

按 [界面阶段设计](docs/superpowers/specs/2026-10-10-desktop-android-ui-design.md)、[执行计划](docs/superpowers/plans/2026-10-10-desktop-android-ui.md)推进。
[本阶段验证记录](docs/superpowers/verification/2026-10-10-desktop-android-ui.md)区分浏览器、原生 Windows、APK 构建和未执行的 Android 真机测试。
旧 OpenSpec 保留为需求参考；Linux/macOS 会在有真实后端时接入。