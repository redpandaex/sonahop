# 独立音频与自启实施计划

> 使用 Superpowers executing-plans 在当前分支顺序实施；用户已授权范围和交付，不重复流程确认，设备测试由用户进行。

**Goal:** 手机麦克风与平板收听可同时运行；系统音量代替播放滑块；默认关闭的 Windows 自启。
**Architecture:** listen/microphone 两条音频 Lane 独立持有 worker/epoch/stream；连接层维持既有策略。自启使用 Tauri 官方桌面插件。
**Spec:** docs/superpowers/specs/2026-10-12-independent-audio.md

## Constraints
- 不调整断线、TCP、心跳、配对策略。
- 不实际启用自启、不启动应用或音频，用户自行测试。
- Android 每台保持一个方向；Windows 一台麦克风与两台连接上限保留。

## Tasks
- [x] CLI 接入 Windows autostart，设置真实读取/用户开关注册，不自动 enable。
- [x] AudioService 拆分两方向状态、启动/停止/失败/断开和媒体路由。
- [x] 音频 worker 按方向校验 epoch，播放固定单位增益，麦克风增益保留。
- [x] 前端两方向独立控制，Android 播放量提示代替滑块，Windows 设置自启。
- [x] 构建两端新程序包，更新使用说明与未来手机角色规划。

## Review focus
停止共享不影响麦克风；停止/拒绝麦克风不影响平板。一个 peer 断开不停止其它 peer/方向。旧流包和旧 worker 回调不能覆盖新状态。新 APK 不继承旧播放衰减。自启缺省不修改启动项。
## Build evidence
2026-10-12 Windows release / Android ARM64 debug 构建退出 0，前端 tsc/Vite 通过；APK 签名验证退出 0。没有运行自动测试、启动应用、注册自启或操作音频/设备。
- Windows: artifacts/independent-audio/sonahop-windows.exe，11,535,360 bytes，SHA256 ED322621130159DD5917CBED257E37694A4F74A393B511F331B2C31FD2D5B479。
- Android: artifacts/independent-audio/sonahop-android-arm64-debug.apk，22,171,765 bytes，SHA256 C196A3934BE09D4AB95DDA4C67A9FAE320BF805D9748C63FDC8655845617F91E。

## Manual acceptance
更新电脑、手机、平板两端。平板收听电脑歌曲，手机申请麦克风，Windows 选择现有虚拟音频线后批准。确认平板继续播放，电脑应用可从虚拟录音端收到手机声音。分别停止共享/麦克风、断开单个设备，另一方向应保持。手机使用媒体音量键，旧播放滑块消失；麦克风增益保留。在 Windows 设置检查初始自启关闭，用户开启/关闭后状态与系统启动项一致。自启只打开应用，不开始音频。