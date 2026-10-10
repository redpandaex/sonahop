# Proposal

## Why

用户希望电脑和 Android 设备可以互相提供音频输入输出，并由一个共享 Rust 内核支撑后续平台扩展。当前仓库没有应用实现，需要先交付可在现有 Windows 11 电脑与两台 Android 16 真机上验证的首个 MVP，把模式和验收边界固定下来。

## What Changes

- 建立共享 Rust 内核、平台接口、Windows/Android 后端与 Tauri 应用入口，工程初始化和依赖管理优先使用官方 CLI。
- 提供“收听模式”：同一 Windows 电脑的系统声音可以同时传给两台已配对 Android；每个接收端独立缓冲和恢复。
- 提供“麦克风模式”：选择一台 Android 的麦克风，经网络送入 Windows 上已有虚拟音频设备，供会议或录音软件作为输入使用。
- 两种模式互斥。切换到麦克风模式时暂停所有收听流；恢复收听必须由用户明确操作；同一时刻仅一台 Android 持有麦克风发送权。
- 支持局域网配对、连接状态、停止与重新连接，以及 Android 后台音频服务、权限和设备变化处理。
- 使用固定编码参数、有界缓冲和基础诊断；提供 Windows 安装包、Android APK，以及可复现的构建和真机验收说明。

已由用户确认：Windows 11 为当前电脑；小米 17 Pro Max、联想 Y700 四代均使用 Android 16，能够通过 USB 部署和诊断；先兼容已有虚拟音频设备。

本次暂缓：同时收发、Android 应用内部声音采集、自研虚拟驱动、Linux/macOS 实际后端、iOS、精确多房间同步、动态码率、复杂纠错与回声消除、互联网穿透和专门 USB 音频传输。USB 可用不等于本次承诺 USB 音频功能。

## Capabilities

### New Capabilities

- `audio-sessions`：互斥模式、模式切换、麦克风所有权和可靠停止。
- `desktop-audio-sharing`：Windows 系统声音到两个独立 Android 接收端的实时播放。
- `phone-microphone-input`：选定 Android 的麦克风作为 Windows 软件可使用的音频输入。
- `device-pairing`：受用户控制的局域网配对、连接和重新连接。
- `android-background-audio`：权限、原生前台服务、锁屏运行与音频设备变化。
- `stream-health`：有界排队、基础故障诊断、格式一致性与可重复验收。

### Modified Capabilities

无。现有主规格目录没有任何能力规格，本次均为新增。

## Impact

- 新增应用代码位于后续通过 CLI 创建的 Cargo workspace、Windows/Android 平台后端及 Tauri UI；目前仅创建规划文档。
- Windows 音频拟通过 WASAPI，Android 音频拟通过 AAudio/AudioRecord 和原生服务接入共享核心；具体封装由编译与真机实验验证。
- 拟使用 Opus 编码和 QUIC Datagram 传输音频；控制消息走可靠通道。参数及库版本作为实施选择记录，不作为实测性能承诺。
- Windows 麦克风使用需要用户安装并选择已有虚拟音频设备；安装和系统设备选择在实施验收时由用户协助完成。
- 验收包含自动化协议/状态/缓冲检查，以及两台 Android 的播放、录音输入、后台运行、切换和断线恢复。只有取得证据后才能宣称功能或平台已支持。
