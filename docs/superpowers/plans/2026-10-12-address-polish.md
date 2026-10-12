# 地址精简与手机麦克风流程参考 — 2026-10-12

用户澄清：Windows地址区只显示Wi-Fi；Android继续显示原来各地址。显示、复制、手动填写和保存只使用IP，传输仍使用固定控制端口。阅读AudioRelay手机麦克风文档并对照现有实现，不安装驱动、不更改Windows默认设备、不新增自带虚拟设备。

- [x] Windows按GetIfEntry2/IEEE80211实际接口类型筛选地址展示；Android保留原列表，所有发现/同子网连接路径仍使用全部适用接口。
- [x] 移除界面端口文本；纯IPv4解析采用固定端口，保存地址验证，旧保存/粘贴IP:端口迁移为IP；空地址可清除保存。
- [x] Windows接收出口采用中性的音频输出名称，避免选择虚拟音频线时仍称扬声器；文档列明手机共享麦克风→Windows接收→虚拟音频线→业务应用录音端的现有路径。
- [x] 根据新增截图统一输入框聚焦样式：移除全局outline与shadcn ring叠加，采用细边框和一层浅色提示，保留键盘及错误状态。
- [x] 编译打包0.4.2 Windows release和Android ARM64 debug；签名及版本检查，真机与应用测试由用户执行。

参考：[AudioRelay手机作为Windows麦克风](https://audiorelay.net/docs/windows/use-your-phone-as-a-mic-for-windows)，手机服务器选麦克风，Windows播放器选Mic输出，业务应用选Virtual Mic。值得后续借鉴：将接收后输出目的明确区分本机播放与供其他应用使用；当前继续使用已安装的虚拟音频线，不声称SonaHop已提供自带Virtual Mic驱动。

已有源码路径：Windows收听页outputDeviceId交给Playback后端，AudioOutput使用显式选择的播放设备。选择虚拟音频线播放端即可将接收音频交给其对应录音端；具体虚拟设备/业务应用效果由用户验证。原Windows共享端批准远端麦克风路径仍保留。

交付证据：最终Windows release、前端TypeScript/Vite、Android ARM64 debug打包命令退出0。exe版本0.4.2，APK versionName0.4.2/versionCode4002、arm64-v8a，v2签名与原debug证书一致。地址/焦点源码限定只读复核未发现Critical/Important。

只读Windows接口元数据显示WLAN(index10)类型71且Up，WSL/Hyper-V接口类型6；未运行新应用来确认界面。未运行自动测试、网络探测、真机或音频验证，也未更改系统网络/音频配置。

用户验收：Windows只出现Wi-Fi地址、Android原地址保留；地址显示/复制不带端口，输入IP+PIN可连接，旧保存/粘贴地址能恢复为IP；PIN/IP键盘与鼠标聚焦为单层淡色提示；已有虚拟音频线环境下自行验证手机共享麦克风→Windows收听输出→业务应用录音端。
