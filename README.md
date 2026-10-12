# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前交付：0.3.0

连接角色与操作系统分开：Windows 和 Android 都可选择服务器或播放器。服务器可分别选择内部播放声音或本机麦克风，最多让两台播放器同时收听；两种音源不混音。Windows 服务器还保留接收一台远端麦克风到已有虚拟音频线的功能。

Windows release、Android ARM64 debug 和前端生产构建成功，APK签名/版本检查见本轮交付记录。没有启动应用、运行自动测试或真实音频/设备验证；新场景由用户复测。此前用户已确认0.2.0麦克风有信号，暂未再次断线。

## 最新安装包

- `artifacts/multi-role/sonahop-windows.exe`
- `artifacts/multi-role/sonahop-android-arm64-debug.apk`

电脑和两台 Android 都更新到0.3.0，音频申请采用新请求编号，不能混用旧版音频控制。旧ui-preview/audio-mvp/ui-polish/link-diagnostics目录保留历史包，避免混用。Android包维持原debug签名，版本号升级为3000。

## Android作为服务器，电脑和平板一起听

1. 小米打开“服务器”，在连接区开启服务器，显示六位PIN。
2. 电脑和Y700切到“播放器”，搜索小米，填小米显示的PIN并发起连接；在小米核对并接受两台播放器。
3. 小米选择“内部播放声音”并开始共享，允许Android的录音和音频共享授权。电脑和Y700各点击开始收听。
4. 小米切到音乐/视频应用播放。SonaHop使用前台音频服务继续采集允许共享的应用声音，不夺取该应用的音频焦点。部分应用禁止被采集，可能只收到静音。[Android采集规则](https://developer.android.com/media/platform/av-capture)
5. 测试麦克风音源时先停止共享、选择本机麦克风再开始；播放器若已停止，需要手动再次开始收听。

Windows播放器可选择本应用的扬声器/耳机出口；空值使用默认播放设备，不修改系统默认设备。手机/平板使用系统媒体音量键，播放增益固定为1。

## Windows作为服务器

选择“内部播放声音”保留原来的Windows进程采集，排除SonaHop自身，与本地扬声器音量独立。选择“本机麦克风”使用已有录音设备；没有录音端点时选项禁用，连接麦克风后可刷新。

手机提供Windows会议麦克风仍走独立路径：手机以播放器角色连接Windows服务器，选择上传本机麦克风、主动申请；Windows选择用户已安装的虚拟音频线播放端（如CABLE Input）并批准，会议/录音应用再选择对应录音端。物理Realtek/HDMI播放设备不能变成Windows录音设备。麦克风不会自动混入系统共享。

## 角色、权限和诊断

界面统一为“服务器 / 播放器 / 设置”。保存连接角色，但应用重启不自动联网、共享或录音。切换角色先停止所有音频、断开连接、释放旧任务和捕获授权；切换设置页保留会话。共享时音源不可更换，停止后再选择。

Android内部播放采集需要Android10+与每次新的系统授权；只建立音频流，没有屏幕录像文件。授权撤销、通知停止、服务结束或错误均终止当前采集，不自动重启麦克风。

连接仍使用6位PIN/SPAKE2与AES-GCM，TCP读写独立；连续写阻塞2秒或20秒没有数据结束连接。媒体队列最多三帧，排队超过120ms在加密前丢弃。固定Opus48kHz/20ms：收听stereo96kbps、远端麦克风mono48kbps。播放时显示真实包数和信号强度；尚未测量端到端延迟。

断线后两端打开“设置 → 连接诊断 → 读取最近日志 → 复制日志”。界面提供本次运行最近256条；本地文件约1MiB，保留一份备份，显示实际文件路径。不记录PIN、密钥或音频正文。读写统计和结束原因用于用户真机复测。

Windows自启默认关闭，用户手动启用后只在登录时打开应用，仍需主动开启连接和音频。注册项使用当前exe位置，移动exe后需重新设置。

## 构建

本机工具：Rust MSVC、Visual Studio C++、CMake/Ninja、JDK21、SDK platform37、NDK30.0.16248370、Rust target aarch64-linux-android。

```powershell
pnpm install --frozen-lockfile
$env:JAVA_HOME = 'D:\ProgramFiles\java\jdk-21.0.12.1+1'
$env:ANDROID_HOME = 'D:\Android\Sdk'
$env:NDK_HOME = 'D:\Android\Sdk\ndk\30.0.16248370'
./scripts/build-audio-mvp.ps1 -Platform Both -OutputDirectory artifacts/multi-role
```

脚本只编译打包，不启动应用或录音。Tauri先生成插件依赖并编译当前Rust，只有明确的Windows符号链接权限错误才使用同一次成功编译的.so交给Gradle。构建日志在artifacts/multi-role/build.log和android-build.log。

开发原生界面：`pnpm dev`；浏览器预览：`pnpm web:dev`，访问http://localhost:1420/?platform=windows或?platform=android。浏览器只显示布局，不能连接、采集或播放。

前端采用Biome、Tailwind4和shadcn/ui，源码在apps/sonahop-ui/src；`pnpm format`格式化。测试按用户约定手动执行。

## 诊断CLI与后续

`cargo run -p sonahop -- devices --json`仅列Windows播放设备。原有`capture --output artifacts/playback-new.wav --seconds 5`仍是用户手动的系统端点采集诊断，须在采集期间主动播放声音；不会覆盖已有文件，WAV保持float32格式。

设计文档：docs/superpowers/specs/2026-10-12-multi-role-audio-design.md；实现计划：docs/superpowers/plans/2026-10-12-multi-role-audio.md。

Windows自带虚拟播放/录音驱动、同机同时扮演服务器和播放器、手机音源混音、专用USB音频仍为后续范围；虚拟设备方案在docs/superpowers/specs/2026-10-12-virtual-audio-devices.md。
