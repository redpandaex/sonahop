# Windows 采集阶段验证记录

日期：2026-10-10。

## 验证范围

本阶段实现 Windows 系统播放声音采集端及独立诊断 CLI。
手机麦克风应在 Android 端采集；本阶段不包含 Android、网络传输或虚拟音频设备接入。

本机环境：Windows 11 Pro、Rust/Cargo 1.99.0、stable-x86_64-pc-windows-msvc。
验证仅涉及合成数据和播放设备枚举，没有启动真实音频采集，也没有录制 Windows 麦克风。

## 自动化证据

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 通过 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 通过 |
| `cargo test --workspace` | 30 个测试通过，0 失败 |
| `cargo build --release --workspace` | 通过；生成 target/release/sonahop.exe |

30 个测试分别为：

- core 音频数据测试 8 个：格式、帧对齐、静音、无效样本及有界解码。
- WAV 写入测试 6 个：float32 文件头与样本读回、帧上限和异常数据。
- Windows 原生缓冲辅助测试 7 个：格式、空静音指针、容量校验及复制所有权。
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

用户选择本轮先不录音，后续接入 Android 时再验证，因此本轮不执行播放测试音及 capture 命令。
后续需验证：

1. 在授权的测试音窗口采集，读回 WAV 头、采样率、声道、帧数和非零信号峰值。
2. 真机 Ctrl+C、无活跃播放、设备断开与不同播放端点的实际行为。
3. Android 接收与手机麦克风功能在后续阶段分别验收。

当前不宣称 WASAPI 实际音频读出、端到端时延或 Android 双机功能通过。

## 独立审查


已完成一次独立分支审查，原审查结论为 With fixes；随后完成一轮修复并验证，未重复派遣审查。

**已修复的重要问题：** Windows 请求格式和 WAV 文件头曾按声道数量推断默认布局，丢失侧环绕等原生位置。现在 AudioFormat 带有可选 channel_mask，Windows 初始化传递原生掩码，WAV 明确写入同一掩码。未知布局写零，不推断扬声器位置。

新增 3 个回归测试，均观察到断言失败后通过：未知布局不被推断、六/八声道 WAV 布局及样本读回、Windows 实际初始化格式构造函数保留布局。
hound 不提供自定义写入掩码；生产代码使用有限的 float32 WAVEFORMATEXTENSIBLE 写入，hound 仅作为独立读回测试依赖，由 Cargo CLI 调整。
字段含义依据 [Microsoft WAVEFORMATEXTENSIBLE](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ksmedia/ns-ksmedia-waveformatextensible)。

**暂缓的非阻塞意见：**

1. 共享事件模式仍请求 100 ms 缓冲，文档契约与依赖实践存在差异；后续真机采集确认兼容性。[Initialize 文档](https://learn.microsoft.com/en-us/windows/win32/api/audioclient/nf-audioclient-iaudioclient-initialize)。
2. 正常完成后若 stop 与 WAV finalize 同时失败，两项清理都会执行，但目前只报告 stop 错误。后续补充错误聚合与失败 writer 验证。

无 Critical 遗留问题。原 Important 已修复并通过回归；实际 Windows 声音读出和端到端功能仍按用户选择留待后续验收。
