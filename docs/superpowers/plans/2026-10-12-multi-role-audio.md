# 跨平台服务器与播放器实现计划

> 按已授权范围直接执行；使用 Superpowers 流程和独立任务分工。最终设备验证由用户完成。

**Goal:** Android 与 Windows 都可提供服务器或播放器角色，音源与平台能力分离。
**Architecture:** Rust 连接角色可在停止状态切换，应用操作门控协调音频与网络生命周期；平台音频工厂选择系统/麦克风采集和本机播放，前端统一工作区。
**Spec:** docs/superpowers/specs/2026-10-12-multi-role-audio-design.md

## 任务与契约
- [x] Windows 后端：list_input_devices()->Result<Vec<OutputDevice>,AudioError>；MicrophoneCapture::new(Option<&str>)->Result<Self,AudioError> 实现 AudioCapture，向调用方提供 48kHz float32 stereo。仅修改 crates/sonahop-platform-windows。
- [x] Android 后端：保留 start_capture/start_playback，新增 start_system_capture；prepare_audio 的 playback 模式申请一次用户捕获授权。audio_capabilities()->Result<AudioCapabilities> 返回 playback_capture/microphone_capture。AudioRecord 播放采集为48kHz stereo PCM16，麦克风仍为mono；JNI队列三帧，并检查实际声道长度。仅修改 crates/tauri-plugin-sonahop-lan。
- [x] Rust 编排：LinkService.set_role；switch_connection_role(role)；所有 UI 变更命令通过 OperationGate。AudioService 音源 factory 与收听 sink 按平台能力选择，保留 Windows 远端麦克风路径。新增 capabilities 控制消息，ListenOffer 可选携带音源说明。
- [x] 前端：连接角色导航与统一服务器/播放器工作区。偏好增加 connectionRole/sendSource/inputDeviceId/outputDeviceId。AppInfo 增加 systemCaptureSupported；AudioSnapshot 增加 sendSource/remoteSource/remoteMicrophoneSupported。
- [x] 集成：版本0.3.0，最终打包至 artifacts/multi-role；检查签名、版本、差异，提交已授权源码。用户文件 .vscode/extensions.json 保留。

## 前端命令契约
- switch_connection_role({role:host|client})->LinkSnapshot
- start_listening({source?:system|microphone,inputDeviceId?:string,outputDeviceId?:string})->AudioSnapshot
- request_microphone({inputDeviceId?:string})->AudioSnapshot
- list_capture_devices()->与播放设备相同的 id/name/isDefault 数组
- get_app_info()->增加 systemCaptureSupported:boolean
- AudioSnapshot 增加 sendSource:system|microphone、remoteSource:system|microphone|null、remoteMicrophoneSupported:boolean|null；其余字段保留
- useAudio.startListen({source?,inputDeviceId?,outputDeviceId?,gain?}) 在 Android host 按 source 调用 prepareAudio(playback|microphone)，client 调用 prepareAudio(listen)。原有 microphone 申请仍保持可见用户操作与服务端批准。

## 检查重点
角色切换中旧任务/事件不会开启新录音；MediaProjection 撤销/拒绝以及单次 token 释放；手机音乐不因采集焦点被暂停；Windows 物理播放出口与虚拟麦克风入口分开；48kHz声道适配和边界；原有两条服务器音频方向、PIN/加密/日志保持。

## 流程裁定
用户要求按设计直接实现并由自己测试，项目 AGENTS 明确无需反复流程审批。因此文档作为可审查记录，直接执行已明确范围；不运行测试、应用、真实音频或安装驱动。平台后端任务在冻结接口后按独立文件职责并行，核心生命周期由主代理统一修改。

执行记录：两端源码已接入同一角色和音源契约。最终Windows release、Android ARM64 debug与前端编译成功；交付目录artifacts/multi-role。修复Tauri异步命令Result约束和Android PCM类型推断后打包；未运行测试或真实设备验证。Windows后端包含未运行的合成缓冲测试。

交付复核：最终Both构建返回0，Windows/Android版本0.3.0、APK versionCode3000、ARM64与v2签名通过，证书保持原debug证书。两端产物和build-info.json位于artifacts/multi-role。

独立审查修正：增加音频申请/订阅requestId，批准、offer、stop/error仅匹配当前操作；服务器批准命令也携带准确requestId，旧批准按钮不能接受新申请。独立只读复核确认A取消、B申请、旧GrantA/StopA及旧UI批准均受身份约束；没有运行测试或应用。新音频控制要求两端更新0.3.0。
