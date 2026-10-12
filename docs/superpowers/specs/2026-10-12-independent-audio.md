# 独立音频方向与 Windows 自启 — 2026-10-12

本轮用户要求先实现非断线功能，再由用户真机测试。范围：移除 Android 收听音量滑块，播放按系统音量键控制；Windows 登录自启默认关闭；Windows 系统声音共享与接收一台手机麦克风同时运行，允许另一台平板收听。每台 Android 仍只运行一种方向；手机服务端、手机应用声音采集、手机↔手机留待后续。局域网传输参数、断线和重连策略不修改。

设计：Tauri AudioService 用 listen / microphone 两个独立 LaneState，各自拥有 stream id、epoch、worker、phase、错误与统计。停止、失败或断开仅影响所属方向/设备；Windows 保留一台麦克风的限制。既有 Windows I/O 在各自专用 MTA 线程创建和释放，复用共享 Opus 与认证链路。控制 Stop/Error 带可选方向，兼容缺失字段但按发送设备范围处理；媒体按 kind+stream id 路由。

界面：Windows 两个方向都有独立启停，界面切换不终止音频；Android 切换方向停止该设备旧方向。播放使用单位增益，旧播放音量偏好不再影响声音；麦克风增益保留。设置显示实际 Windows 启动项状态，只有用户打开开关时才注册；自启只打开应用，不自动开启共享或麦克风。

交付：使用 CLI 接入 Tauri 官方 autostart 插件，生成新的 Windows exe / Android APK。只构建和检查包，不启动应用、不设置实际自启、不运行自动/浏览器/设备测试，不采集真实音频。保留用户 .vscode 改动。
## Future mobile roles
本轮仍按 Windows host / Android client 连接。未来把连接监听者/发起者与音频发布者/接收者解耦，通过设备能力声明和 stream id 路由音源到播放端或虚拟麦克风端。现有共享 Rust 编解码、认证传输以及独立 Lane 生命周期可复用；Android host、应用声音的 AudioPlaybackCapture/MediaProjection 与 Windows 本地扬声器接收需要另行实现。当前两个固定 Lane 不代表已经完成任意设备/任意音源的路由。
## Navigation naming
两端统一显示发送音频 / 接收音频，顺序为发送、接收。Windows 发送对应系统声音共享，接收对应手机麦克风；Android 发送对应本设备麦克风，接收对应电脑声音。页面标题/面包屑与标签一致，说明文字注明当前实际音源，避免让服务器/播放器名称混同连接监听角色。此命名修改不改变音频方向状态、既有偏好或启停规则。
命名版本构建：Windows release、Android ARM64 debug 与前端生产构建成功，APK 签名检查退出 0；未运行界面或设备测试。产物 artifacts/audio-direction-tabs。
