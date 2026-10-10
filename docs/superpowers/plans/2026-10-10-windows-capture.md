# Windows Audio Capture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** 交付可枚举 Windows 播放设备、采集系统播放声音并生成有效调试 WAV 的 Rust CLI。

**Architecture:** 三个 workspace 成员：core 定义格式/采集边界和 WAV 写入；Windows 后端封装 WASAPI loopback；CLI 只装配、校验参数并控制采集生命周期。当前不创建 Tauri、网络或空 Android 后端。

**Tech Stack:** Rust 2024、wasapi 0.25、hound（测试读回）、clap、thiserror、serde/serde_json、ctrlc；依赖通过 Cargo CLI 添加。

**Spec:** `docs/superpowers/specs/2026-10-10-windows-capture-design.md`

**Execution:** 用户已要求重新生成 Superpowers 计划并按计划执行；本轮采用当前会话内直接执行，结束时做一次独立代码审查。已确认范围不重复征求许可，真实音频采集另待用户确认。

## Global Constraints
- CLI 工程和依赖管理优先使用官方命令；仅手动编辑 CLI 未覆盖的配置。
- 不参考 SoundPush；共享核心不依赖 Tauri/Windows API。
- 采集 Windows 播放端点，不能默认选 Windows 麦克风。
- 当前只实现第一阶段，最终双 Android 和单手机麦克风的互斥目标保持不变。
- WAV 输出显式启用且不覆盖已有文件；录音、构建产物及临时记录不提交。
- 合成数据验证不等于实际硬件录音通过。

## Review Focus
- 静音缓冲的原始内容不可信，按帧数生成零值。
- 不完整声道帧、错误缓冲长度和非有限值必须被拒绝。
- Ctrl+C、读超时及中途错误后必须停止 native 流并收尾 WAV。
- 已有输出文件拒绝发生在开始读取真实音频前。
- 无活跃音频时命令应按时结束，并明确没有取得音频数据。

### Task 1: Workspace 与可复用核心

**Files:** `Cargo.toml`、`crates/sonahop-core/`、`crates/sonahop-platform-windows/`、`apps/sonahop-cli/`、`.gitignore`。
**Interfaces:**
- Produces: `AudioFormat { sample_rate: u32, channels: u16, channel_mask: Option<u32> }`；`AudioError`。
- Produces: `CapturePacket { samples: Vec<f32>, position: u64, discontinuity: bool }`。
- Produces: `AudioCapture::format(&self) -> AudioFormat`、`read_packet(&mut self, timeout: Duration) -> Result<Option<CapturePacket>, AudioError>`、`stop(&mut self) -> Result<(), AudioError>`。
- Produces: `decode_f32_packet(format, bytes, frame_count, silent) -> Result<Vec<f32>, AudioError>`。
- Produces: `WavRecorder<W: Write + Seek>::new(writer, format, max_frames)`、`write_samples(&mut self, samples) -> Result<usize, AudioError>`、`finish(self) -> Result<RecordingSummary, AudioError>`。

- [x] 用 `cargo new --vcs none` 创建两个库和 CLI（CLI package 名称为 sonahop），通过 `cargo metadata` 确认 workspace。模板和配置用编译验证，不写镜像模板的测试。
- [x] 通过 `cargo add` 添加依赖和路径引用；移除生成的示例函数/测试，运行 `cargo check --workspace`。Expected: 成功，核心不链接 Windows API。
- [x] 先写核心行为测试并运行 RED：48k 双声道输入字节 `[0,0,0,63,0,0,0,191]` 应得到 `[0.5,-0.5]`；静音两帧应得到四个零；不完整帧/NaN/零格式均拒绝。
- [x] 实现格式、采集接口和解析，运行 `cargo test -p sonahop-core`。Expected: 以上行为通过。
- [x] 先写 WAV 测试并运行 RED：48k 双声道、最大两帧，输入六个样本只保存前四个；hound 读回采样率 48000、声道 2、float32、时长两帧和原始样本；未对齐输入不改变已写帧数。
- [x] 实现有界 WAV 写入及明确收尾，运行核心测试与 workspace check；记录结果并提交本任务。

### Task 2: Windows 播放设备与 loopback

**Files:** `crates/sonahop-platform-windows/src/lib.rs`、`src/windows.rs`及相关测试。
**Interfaces:**
- Consumes: Task 1 的 `AudioFormat`、`AudioCapture`、`CapturePacket`、`AudioError`。
- Produces: `OutputDevice { id: String, name: String, is_default: bool }`、`list_output_devices() -> Result<Vec<OutputDevice>, AudioError>`。
- Produces: `open_loopback(device_id: Option<&str>) -> Result<Box<dyn AudioCapture>, AudioError>`。

- [x] 先写后端格式/缓冲转换测试并运行 RED：44100Hz、六声道的 native 格式保留；silent/不连续标记和帧位置传入核心，过短缓冲不得读越界。
- [x] 实现 Render 设备枚举、默认设备与明确 ID 查找，以及 shared-mode loopback；请求与 native 采样率/声道一致的 float32，不能使用默认 Capture 端点。
- [x] 实现单线程 native 资源所有权、预分配有限缓冲、有限等待和幂等 stop/Drop；超时表示暂无数据，其他 native 错误明确回报。
- [x] 运行 `cargo test -p sonahop-platform-windows` 与全 workspace 测试，确认 Windows API 类型没有泄漏进 core；记录结果并提交本任务。

### Task 3: CLI 参数、安全输出与采集窗口

**Files:** `apps/sonahop-cli/src/main.rs`、`src/cli.rs`、`src/capture.rs`、`tests/cli.rs`。
**Interfaces:**
- Consumes: Task 1 的 WAV 写入与采集接口，Task 2 的设备列表与 loopback。
- Produces: `sonahop devices [--json]`。
- Produces: `sonahop capture --output PATH [--seconds 5] [--device ID]`，支持 1..60 秒。
- Produces: 结果摘要包括设备、格式、窗口时长、写入帧数、静音/不连续信息；失败返回非零退出码。

- [x] 先写 CLI 集成测试并运行 RED：0/61 秒、缺少输出参数返回失败；已有文件内容保持不变；帮助信息说明采集系统播放声音。
- [x] 实现 clap 子命令和参数范围；以 create_new 打开输出文件，在真实采集前拒绝已有路径。
- [x] 实现实际采集窗口和帧上限、Ctrl+C 标记、无数据结果及错误收尾；等待采用剩余窗口限制，stop 和 WAV finalize 在所有退出路径执行。
- [x] 对采集循环使用受控合成源验证：停止请求、读取失败、无数据截止和多余帧裁剪；运行 `cargo test --workspace`。Expected: 自动化测试不采集真实系统声音。
- [x] 运行 CLI 帮助和参数失败场景，记录输出并提交本任务。

### Task 4: 使用说明、验证与独立审查

**Files:** `README.md`、`docs/superpowers/verification/2026-10-10-windows-capture.md`、阶段计划和执行记录。
**Interfaces:** 使用前三任务的实际 CLI 和源码，不新增产品能力。

- [x] 写入真实启动命令、系统声音与手机麦克风的方向说明、阶段边界和 WAV 诊断用途；确认命令可以执行。
- [x] 运行 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 和 `cargo build --release --workspace`，逐项记录结果。
- [x] 在本机运行 `cargo run -p sonahop -- devices --json`，记录播放端点及默认选择；此步骤不录音。
- [x] 用户授权后才播放短测试音并采集 WAV，读取文件头、帧数和信号峰值；未授权则作为验收待办明确列出，不伪称通过。本轮用户选择不录，后续接入 Android 时再验证。
- [x] 进行一次独立分支审查；修复重要问题并通过对应测试，记录其余意见。
- [x] 汇总执行结果与实际限制，完成当前阶段的代码交付；未经请求不推送 GitHub 或分发安装包。

## Workflow follow-up
- 后续计划依次接入 Tauri/Android、单向网络与双接收端、手机麦克风及互斥模式。
