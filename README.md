# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前交付状态

首版音频源码已经接入：局域网发现、输入连接码、电脑批准、加密传输、Windows 系统声音采集、Android 收听与后台麦克风、Windows 既有虚拟音频设备输出、独立手机增益和真实状态界面。

**本次已生成当前 Windows release exe 和 Android ARM64 debug APK，前端、双端 Rust 与 Android Java/Kotlin 编译通过；未运行自动测试、启动应用、播放/录音或真机验证。** 请使用 `artifacts/audio-mvp` 新产物，同时更新两端；`artifacts/ui-preview` 仍是旧界面版本，旧协议客户端不能连接新版本。

范围为 Windows 11 和 Android 16。收听最多两台设备；麦克风一次一台，并暂停全部收听。完整双工、自带驱动、专用 USB 音频、Linux/macOS/iOS 后续实施。

## 当前真机修复包

首次用户测试发现两个问题，已分别修正并编译：Android 前台状态误判，以及 Windows 系统采集参数被重复释放导致 0xc0000374 退出。

现阶段复测请使用 `artifacts/audio-mvp/sonahop-windows-heap-fix.exe` 和 `sonahop-android-foreground-fix.apk`。关掉旧 Windows 程序，重新开启联网/配对，再验证共享。修复根因和构建证据在对应的 2026-10-12 Superpowers 修复记录中；尚未由 agent 运行音频或真机复测。

## 构建当前版本

本机需要 JDK 21、SDK platform 37、NDK `30.0.16248370`、Visual Studio C++ 与 CMake/Ninja，以及 Rust Android target `aarch64-linux-android`。

仓库根目录运行：

```powershell
pnpm install --frozen-lockfile
pnpm mvp:build
```

单独构建可用 `pnpm mvp:windows` 或 `pnpm mvp:android`。如果当前终端没有 Android 环境变量，本机对应位置为：

```powershell
$env:JAVA_HOME = 'D:\ProgramFiles\java\jdk-21.0.12.1+1'
$env:ANDROID_HOME = 'D:\Android\Sdk'
$env:NDK_HOME = 'D:\Android\Sdk\ndk\30.0.16248370'
pnpm mvp:build
```

脚本只在调用时构建，不启动应用、播放或录音。它从 Cargo 的锁定依赖取得 Opus 源码，分别构建 Windows / ARM64 Android 静态库，然后使用 Tauri CLI 构建应用。只修改该脚本进程环境；结束时还原环境变量。

构建成功后产物：

- `artifacts/audio-mvp/sonahop-windows.exe`
- `artifacts/audio-mvp/sonahop-android-arm64-debug.apk`

Android 脚本先由 Tauri 生成当前原生插件依赖并编译 Rust。只有明确的 Windows 符号链接权限错误且 Rust 已成功完成时，才复制当前 `.so` 后调用 Gradle 打包；其他编译错误直接停止。不会复用一次失败编译留下的旧预览包。构建日志位于 `artifacts/audio-mvp/android-build.log`。本轮脚本已成功执行；最后构建日志见 `artifacts/audio-mvp/final-build.log`，Android 完整日志见上述文件。

## 明早先这样测试

1. 电脑和 Y700 连接同一个 Wi-Fi / 局域网，启动刚构建的两端。Windows 点击开启局域网；如 Windows 弹出防火墙授权，允许专用网络。不会由应用自动修改防火墙。
2. Android 点击搜索，填写电脑显示的六位连接码后连接；Windows 核对设备名称和双方显示的验证码后批准。找不到时填电脑实际 LAN IPv4 地址；端口为 TCP `32993`，发现使用 UDP `32992`，不需要重启应用。
3. Windows 点击开始共享；Android 点击开始收听。电脑播放歌曲，检查 Android 是否听到，并调节手机音量。再把电脑本地扬声器静音，检查手机声音仍继续。
4. 在第二台 Android 重复连接、批准、收听；分别改变两台设备增益，检查互不影响。停止一台，另一台应继续；断开最后一台时应释放电脑采集。
5. 麦克风测试前准备已经安装的虚拟音频线。Android 切换麦克风并主动申请、允许录音；Windows 选择虚拟线的播放入口（例如 CABLE Input）并批准。电脑录音/会议软件选择对应录音出口，检查手机说话是否进入软件；两台设备收听此时应暂停。
6. 检查手机锁屏/切换其他应用后的持续播放或录音，以及停止按钮、通知栏停止、断开网络后的硬件释放。Android 麦克风不得自动重新启动；重新连接后需再次主动申请。

若列表只有显示器和 Realtek 物理设备，表示当前没有可选的虚拟音频入口；先完成电脑→手机，再准备麦克风测试。项目不会安装驱动，也不会自动选择物理扬声器作为麦克风路径。

## 音频路径

电脑→手机使用 Windows 进程级系统声音捕获，排除 SonaHop 本身；与原 CLI 的端点 loopback 诊断路径分开。Android 使用 AudioTrack 播放，手机增益在 Rust 解码后应用，不修改电脑音量。

手机→电脑使用 Android AudioRecord（mono 48 kHz PCM16）与原生 foreground service，经 JNI 交给 Rust Opus 编码后传输；电脑解码并写入用户指定的现有虚拟音频 render 端点。

传输采用固定 Opus / 48 kHz / 20 ms：收听 stereo 96 kbps、麦克风 mono 48 kbps。SPAKE2 协商密钥，AES-GCM 加密控制和媒体；还需电脑明确批准。媒体发送/接收队列最多三帧、TCP 缓冲区有界、写入超时断开；当前先使用 TCP，后续可根据真机网络结果优化传输。界面显示包数和丢弃数；不宣称尚未测量的实际端到端延迟。

## 开发界面

正常开发仍可使用 `pnpm dev`，需要 CMake 在 PATH 或配置本机的 `OPUS_LIB_DIR` 静态库安装根目录。仅浏览器布局运行 `pnpm web:dev`，打开 `http://localhost:1420/?platform=windows` 或 `?platform=android`；浏览器不能调用原生连接或音频。

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

## 前端工具与组件

前端采用 Biome 2.5.15、Tailwind CSS 4.3.3（Vite 插件）和 shadcn/ui 的 Radix 组件。
已生成并接入 Button、Input、Badge；组件源码位于 apps/sonahop-ui/src/components/ui，可按项目需求修改。
Tailwind 与原有布局 CSS 共用暖白/深色主题变量；主题仍使用根元素 data-theme。

```powershell
pnpm format       # 格式化前端源码
pnpm format:check # 只检查格式
pnpm lint         # 静态规则检查
pnpm check        # 格式、规则、导入检查
pnpm check:fix    # 应用安全修正
```

新增组件使用官方 CLI，例如：

```powershell
pnpm dlx shadcn@latest add dialog -c apps/sonahop-ui
```

Biome 配置位于 apps/sonahop-ui/biome.json，启用 Tailwind 指令解析，不处理 Rust/Android 生成工程。
前端扩展推荐包含 Biome 和 Tailwind CSS IntelliSense；用户根目录的 VS Code 设置保持现有内容。

本轮为交付安装包运行了前端生产构建（包含 tsc）、Windows release 与 Android debug 构建，并核对 APK 签名/权限/JNI 符号。没有运行自动测试、应用、浏览器交互或真机验证；额外静态规则检查可由用户运行 `pnpm check`。
浏览器打开 Windows / Android 预览链接，检查两种主题、模式切换、电脑信息编辑和音量滑块。
旧 ui-preview APK 仅包含此前图标修正版；直接安装本轮 audio-mvp 产物，修改源码后再使用 mvp 构建脚本。
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
Linux/macOS 会在有真实后端时接入。