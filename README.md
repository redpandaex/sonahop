# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前交付状态

当前源码支持两台 Android 与 Windows 配对；Windows 可同时共享系统声音、接收一台手机麦克风。手机/平板一次运行一个方向，因此可用一台手机供麦克风、另一台平板收听。接收播放使用系统媒体音量键，麦克风增益保留。

本轮 Windows release / Android ARM64 debug 与前端生产构建成功，APK 签名检查通过。未启动应用、录音/播放、运行自动测试或设备验证，也没有在本机实际注册自启。此前用户确认基本收听已可用；新增混合收发和启动设置待用户复测。

## 最新测试包

两端导航统一为发送音频 / 接收音频。Windows 发送系统声音、接收手机麦克风；Android 发送麦克风、接收电脑声音。标题说明实际音源。

同时更新电脑及两台 Android：

- `artifacts/ui-polish/sonahop-windows.exe`
- `artifacts/ui-polish/sonahop-android-arm64-debug.apk`

旧 audio-mvp / ui-preview 包保留为历史版本。最新包包含 Android 前台状态和 Windows 采集参数重复释放的修复，以及独立音频方向和 Windows 自启设置。

1. 平板配对后开始收听，Windows 开始共享并播放歌曲。
2. 手机配对、切换麦克风并申请；Windows 选择已安装的虚拟音频线播放入口并批准。
3. 检查平板仍播放，电脑应用选择该虚拟线的录音出口后能收到手机说话。
4. Windows 两个控制区分别停止共享、停止麦克风，检查另一方向保持；切换电脑界面也不停止音频。
5. Android 使用系统媒体音量键；Windows 设置中自启初始关闭，可由用户手动切换。

自启只在用户登录 Windows 后打开应用，不自动联网、共享声音或录音。启动项记录当前程序位置，先将 exe 放在固定位置再启用。关闭启动开关会移除该应用的注册项。应用初始化不调用 enable/disable，保留用户已经选择的状态。

此次保持原有网络超时、配对与重连策略；偶发断线按用户要求留待后续。手机作为服务器、手机↔手机和同一 Android 同时录放尚未实现。
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

Android 脚本先由 Tauri 生成当前原生插件依赖并编译 Rust。只有明确的 Windows 符号链接权限错误且 Rust 已成功完成时，才复制当前 `.so` 后调用 Gradle 打包；其他编译错误直接停止。不会复用一次失败编译留下的旧预览包。构建日志位于 `artifacts/audio-mvp/android-build.log`。本轮独立音频包最后构建日志见 `artifacts/independent-audio/build.log`；脚本可通过 `-OutputDirectory` 指定仓库内产物目录。

## 音频路径

电脑→手机使用 Windows 进程级系统声音捕获，排除 SonaHop 本身；与原 CLI 的端点 loopback 诊断路径分开。Android 使用 AudioTrack 播放，播放固定单位增益，使用设备系统媒体音量，不修改电脑音量。

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