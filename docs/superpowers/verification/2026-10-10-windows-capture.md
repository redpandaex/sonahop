# Windows 采集阶段验证记录

日期：2026-10-10。

## 验证范围

本阶段实现 Windows 系统播放声音采集端及独立诊断 CLI。
手机麦克风应在 Android 端采集；本阶段不包含 Android、网络传输或虚拟音频设备接入。

本机环境：Windows 11 Pro、Rust/Cargo 1.98.1、stable-x86_64-pc-windows-msvc。
验证仅涉及合成数据和播放设备枚举，没有启动真实音频采集，也没有录制 Windows 麦克风。

## 自动化证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| `cargo test --workspace` | 27 个测试通过，0 失败 |
| `cargo build --release --workspace` | 通过；生成 target/release/sonahop.exe |

27 个测试分别为：

- core 音频数据测试 8 个：格式、帧对齐、静音、无效样本及有界解码。
- WAV 写入测试 4 个：float32 文件头与样本读回、帧上限和异常数据。
- Windows 原生缓冲辅助测试 6 个：格式、空静音指针、容量校验及复制所有权。
- CLI 采集生命周期测试 5 个：裁剪、取消、读取失败、无数据截止及停止失败时 WAV 收尾。
- CLI 集成测试 4 个：时长范围、输出参数、已有文件保护及帮助信息。

功能测试均经历失败到通过的实现过程。
合成源的取消测试验证停止标记处理；实际控制台 Ctrl+C 和驱动行为仍需真机采集验证。

## 本机设备枚举

`cargo run -p sonahop -- devices --json` 成功返回两个 Render 端点：

| 设备 | 默认 |
| --- | --- |
| Realtek Digital Output (Realtek(R) Audio) | 否 |
| G24H2 (NVIDIA High Definition Audio) | 是 |

release 可执行文件的 devices --json 也成功返回同样结果。
设备 ID 来自当前机器，不作为固定配置写入项目。
设备枚举不调用音频流 Start，不采集声音。
`capture --help` 已确认描述为 Windows 系统播放声音，明确不是 Windows 麦克风。

## 待验收

未获得真实系统声音采集授权，因此本轮不执行播放测试音及 capture 命令。
后续需验证：

1. 在授权的测试音窗口采集，读回 WAV 头、采样率、声道、帧数和非零信号峰值。
2. 真机 Ctrl+C、无活跃播放、设备断开与不同播放端点的实际行为。
3. Android 接收与手机麦克风功能在后续阶段分别验收。

当前不宣称 WASAPI 实际音频读出、端到端时延或 Android 双机功能通过。

## 独立审查

等待一次独立分支审查；最终结果与处理记录补充于此。
