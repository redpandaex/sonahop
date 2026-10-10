# Spec Delta

## Purpose

为实时音频提供明确的排队边界、输入格式检查和故障观测，让用户和开发者能够区分网络、音频处理及设备问题，并通过可重复测试确认实际支持范围和播放效果。

## ADDED Requirements

### Requirement: Bounded queued audio
系统 MUST 为音频排队设置有限容量及时间上限；持续拥塞时淘汰过期音频或停止故障流，MUST 不无限增加内存或延迟。

#### Scenario: A receiver cannot keep up
- **WHEN** 某接收端持续无法消费音频
- **THEN** 该流的排队保持在配置边界内，丢弃或故障计数可见，恢复时对齐当前内容

### Requirement: Consistent and validated stream format
系统 SHALL 在开始音频流前确认编码、采样率、声道和时间线信息，并拒绝无效、超限、重复或属于旧流的数据。

#### Scenario: The peer sends an unsupported format
- **WHEN** 对端请求不能处理的音频格式
- **THEN** 系统报告不支持的格式，不开始错误播放或导致进程崩溃

#### Scenario: Duplicate and old packets arrive
- **WHEN** 接收端收到重复序号或已停止流的音频包
- **THEN** 数据不被重复播放，也不会污染当前流的缓冲和时间线

### Requirement: Distinct diagnostic metrics
系统 SHALL 按设备和音频流提供网络 RTT、音频包序号缺口、迟到或丢弃计数、缓冲时长、处理耗时和播放断流计数；MUST 不将网络 RTT 或缓冲时长标为已测得的端到端音频延迟。

#### Scenario: Diagnose a playback interruption
- **WHEN** 用户查看发生故障的设备诊断
- **THEN** 能获得对应流的状态、错误和相关指标，并将故障与另一台设备的指标区分开

### Requirement: Diagnostics do not implicitly record audio
系统 SHALL 默认仅记录运行元数据，音频或麦克风内容的本地录制需要独立且明确的用户操作，密钥和完整凭据 MUST 不进入诊断导出。

#### Scenario: Export a diagnostic report
- **WHEN** 用户导出诊断信息
- **THEN** 报告包含版本、设备状态、配置、错误和计数，不包含原始音频或私钥

### Requirement: Physical acceptance evidence
系统交付说明 MUST 记录 Windows 构建结果及两台指定 Android 16 的实际验证结果，分别注明播放、麦克风输入、锁屏、模式切换和断线恢复是否通过；未验证项目不得标为通过。

#### Scenario: Only compilation has succeeded
- **WHEN** Windows 和 APK 已构建，但某台设备尚未完成真机验收
- **THEN** 交付说明保留该设备的未验证状态，不能宣称该机型已经通过完整功能验证
