# SonaHop

以 Rust 为核心的跨设备音频项目，计划通过 Tauri 提供桌面和移动端界面。

首版目标是 Windows 11 ↔ Android 16，两种模式分别运行：

- **收听模式：** 两台 Android 同时收听电脑的系统播放声音。
- **麦克风模式：** 一台选定的 Android 采集手机麦克风并发送给电脑，通过已有的 Windows 虚拟音频设备供应用使用；此时暂停所有收听。

## 当前实现

第一阶段包含 Rust workspace、Windows 播放设备枚举和 WASAPI loopback 采集 CLI。
网络传输、Tauri 界面、Android 应用及手机麦克风尚未实现。

这里的 Windows 采集用于 **电脑 → 手机** 的发送端：读取电脑正在播放的声音。
**手机 → 电脑** 的麦克风功能会在 Android 端录音。
本阶段保存 WAV 只是独立验证采集端的诊断方式，不是录音产品功能。

## 开发和运行

在 Windows 上安装 Rust stable（MSVC 工具链）与 Visual Studio C++ Build Tools。
在仓库根目录运行：

```powershell
cargo run -p sonahop -- --help
cargo run -p sonahop -- devices
cargo run -p sonahop -- devices --json
```

`devices` 只枚举播放设备，不启动采集。`*` 或 JSON 的 `is_default` 标记默认设备。

需要诊断系统播放声音时，主动执行：

```powershell
cargo run -p sonahop -- capture --output artifacts/playback.wav --seconds 5
```

指定播放设备可增加 `--device '<devices 输出中的 id>'`。
时长限制为 1～60 秒，默认 5 秒；必须指定输出路径，已有文件会被拒绝。
WAV 采用 float32，保留所选设备的采样率、声道数及已知的扬声器布局。它会包含采集窗口内该设备正在播放的声音。

Ctrl+C 会停止采集、收尾 WAV，并返回退出码 130。
读取错误也会尝试停止设备和收尾已写入内容；没有取得任何音频帧时返回错误。
设备初始化失败可能留下空输出文件，重试时请使用新的文件名。
输出摘要包含设备、格式、写入帧数、峰值、不连续次数及实际窗口时长。
音频时长按写入帧数计算，可能短于窗口时长；全零样本与没有收到帧分别统计。

`artifacts/` 和 `target/` 已加入 Git 忽略列表。

## 验证和构建

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
.\target\release\sonahop.exe devices --json
```

测试使用合成音频，不会录制真实系统声音。
本机已验证播放设备枚举，**实际 loopback 声音采集仍待真机验证**；这不代表 Android 链路已通过。
验证细节见 [阶段记录](docs/superpowers/verification/2026-10-10-windows-capture.md)。

## 工程结构

- `crates/sonahop-core`：音频格式、采集接口、数据校验和有界 WAV 写入；不依赖 Windows API 或 Tauri。
- `crates/sonahop-platform-windows`：Windows Render 端点枚举、WASAPI loopback 和原生资源生命周期。
- `apps/sonahop-cli`：参数、设备选择、采集窗口及结果输出。

当前只实现 Windows 后端；Linux/macOS 会在有真实实现时接入。
依赖和项目初始化优先使用 Cargo CLI。

按 [第一阶段设计](docs/superpowers/specs/2026-10-10-windows-capture-design.md) 和 [执行计划](docs/superpowers/plans/2026-10-10-windows-capture.md) 推进。
旧 OpenSpec 保留为需求参考。后续接入 Tauri/Android、网络收听与双接收端，再完成手机麦克风及模式切换。
