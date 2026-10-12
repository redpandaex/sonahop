# 连接稳定性与诊断 — 2026-10-12

用户真机截图提供了明确错误：小米发送超时、Y700 心跳超时；当前 writer 在读写同一 select 循环中 await，250ms 写期限过于激进，主机又丢弃了退出错误。本轮排查/修复读写阻塞并记录可导出的诊断日志，保留配对加密和有界音频队列。不运行自动测试、应用或音频，用户复测。

- [x] 保持方向密钥/counter，拆分加密 reader/writer；收包在发包等待时持续处理。
- [x] 写期限改为2s，心跳20s；媒体排队超过120ms在加密之前丢弃，不中止半写帧。
- [x] 持久化有界日志：session结束原因、收发计数、写耗时、丢弃。异步专用线程写盘，不记录PIN/密钥/音频。
- [x] 设置显示日志位置与最近日志，Android也可通过界面复制。
- [x] 音频UI显示麦克风信号，区分录到信号与仅传送静音。
- [x] 调整ICO顺序，避免Tauri默认取16px首帧后放大；保留同一图标像素内容。
- [x] 构建0.2.0诊断包并更新说明。用户已选择保持麦克风与系统声音独立，不加入监听/混音。
执行记录：以 9a81d72 为本轮基线，直接在已有 feat/lan-connection 工作分支执行，保留用户文件。按用户约定不编写/运行自动测试、不启动应用或操作真实音频；最终编译与打包作为交付检查，实际听音与断线复测由用户完成。

接口检查：读写各自持有唯一方向密钥和现有 counter；媒体过期丢弃发生在 seal 之前；reader 原有分段状态保留取消安全。Rust LaneSnapshot 与 TS LaneSnapshot 同步增加 peakLevel；get_diagnostics 字段与设置组件一致。

交付证据：scripts/build-audio-mvp.ps1 -Platform Both -OutputDirectory artifacts/link-diagnostics 返回0，Windows release / Android ARM64 debug 成功。apksigner verify 返回0，证书保持原 debug 证书；APK与exe版本均0.2.0。ICO首条目256px；没有运行应用、测试或真实音频。

独立源码审查完成：未发现 Critical/Important 问题。两项 Minor 留待后续：日志写盘队列拥塞时可能丢失持久记录（内存最近记录仍保留）；主动停止/abort 不产生会话最终统计（已有 link-stop）。本轮不因此延长真机交付，不额外运行测试。
