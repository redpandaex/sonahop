# VB-CABLE 接入与同时收发 Implementation Plan

> 使用 Superpowers executing-plans 在当前 feat/lan-connection 分支收尾；用户已授权评估后直接接入、打包、提交推送。真机测试由用户执行。

**Goal:** 使用已安装的 VB-CABLE/Voicemeeter 或其他手动配置的虚拟音频线，把远端声音交给 Windows 录音应用，并完成已有同时收发改动。
**Architecture:** 复用 WASAPI 输出；前端区分普通播放和电脑麦克风目的，识别已安装的播放/录音端，不创建新驱动。Android 每个采集/播放方向独立会话，保留请求 ID 和主动授权。
**Tech Stack:** Rust/Tauri/React、WASAPI、Android AudioTrack/AudioRecord、Tauri opener、NSIS。
**Spec:** docs/superpowers/specs/2026-10-13-optional-devices-duplex-design.md

## Global Constraints
- 自研驱动明确暂停；不安装第三方驱动、不修改默认音频设备、签名策略、网络或启动配置。
- 虚拟设备只作为可选外部依赖，官网按钮只在用户点击时打开指定厂商页面。
- 保留固定 Opus、三帧队列、认证/加密、请求 ID 和 Windows 已有独立音频路径。
- 不运行自动测试、应用、实际音频或真机验证；执行最终编译打包、静态复核和 APK 签名检查。

## Tasks
- [x] 1. 接入设备识别、独立输出偏好和折叠引导。
  Files: src/lib/virtual-audio.ts、preferences.ts、use-workbench.ts、bridge.ts、components/audio-controls.tsx、virtual-audio-guide.tsx、App.css、capabilities/default.json。
  Interface: virtualAudioRoute/virtualAudioRoutes/virtualMicrophoneReady 使用现有 PlaybackDevice；Windows 主动选择入口，不默认选择实体扬声器。
  Expected: CABLE Input 对应 CABLE Output；已知录音端不可用时阻止启用；其他虚拟音频设备由用户显式高级选择。
- [x] 2. 复核并保留此前 Android/JNI 与 Rust 的同时收发实现。
  Files: AudioBridge.java、AudioService.java、LanPermissionPlugin.kt、android_audio.rs、mobile.rs、desktop.rs、audio/backend.rs、audio/mod.rs、audio/control.rs。
  Expected: 同一个 Android 客户端可收听并上传麦克风；单方向停止不终止另一方向，通知停止与断连接仍停止全部。
- [x] 3. 添加可选 NSIS 打包开关，更新使用说明，完成 Windows/Android/安装器编译和独立代码复核。
  Files: scripts/build-audio-mvp.ps1、README.md、AGENTS.md；artifacts/vb-cable 为忽略的交付目录。
  Expected: exe、ARM64 debug APK 和 NSIS 安装器来自当前源码；不捆绑或加载任何虚拟驱动，主程序单独安装。
- 4. 记录构建证据和用户验收步骤，提交并推送当前分支；实际提交状态记录在 artifacts/vb-cable/build-info.json。
  Expected: 编译/签名证据与真机功能证明明确区分；工作目录干净且远端分支包含提交。

## Review Focus
- 旧偏好迁移、设备拔除或改名、同名多端点、录音端未启用时不能误报可用。
- Windows host 批准麦克风与 client 接收到虚拟入口两条路径都传递显式设备 ID；普通播放偏好保持独立。
- Android 两方向的 JNI 会话、权限、前台服务、失败/取消/停止不能串扰。
- 官方链接限定指定 HTTPS 页面；安装器不能捆绑第三方驱动或更改系统默认音频。
- 包版本/签名/当前源码匹配，尚未进行的真机验收明确交给用户。
