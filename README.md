# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前交付：0.4.1

Windows 和 Android 都可共享本机声音，也可收听另一台设备。一个共享设备最多连接两台收听设备；内部播放声音与本机麦克风分别选择，不混音。Windows 共享端还可独立接收一台远端麦克风到用户已有的虚拟音频线。

界面采用“共享 / 收听 / 设置”，Android 使用固定底部导航。打开应用后自动恢复上次角色并启动局域网发现，不自动配对、共享、收听或录音。切换角色先停止音频和连接，再启动新角色的发现；进入设置保留当前会话。常规联网开关已移除，启动失败时可重试。

## 最新安装包

- artifacts/ui-refinement/sonahop-windows.exe
- artifacts/ui-refinement/sonahop-android-arm64-debug.apk

电脑和两台 Android 可覆盖更新到 0.4.1。Android 为 ARM64 debug 包，保留原 debug 签名，versionCode 4001；历史 artifacts/discovery-ui 是 0.4.0，artifacts/multi-role 是 0.3.0。

新 Windows exe 路径可能触发新的防火墙提示，请允许在本地网络通信；也可先关闭旧程序，再将新版替换到原先允许的 exe 路径。不要同时运行两个版本。

## 小米共享，电脑和平板一起听

1. 小米打开“共享”，等待自动发现启动；连接区显示本机 IPv4 地址、端口 32993 和六位 PIN。
2. 电脑和 Y700 打开“收听”，填小米的 PIN，选择发现的小米；在小米核对并接受两台设备。
3. 小米选择“应用声音”并点击开始共享，允许录音权限；在 Android 系统授权窗口选择单个应用或整个屏幕。选择单个应用时，下一步由系统页面选择应用。SonaHop 不再显示另一套范围/应用列表。
4. 电脑和 Y700 各点击开始收听，小米切到所选音乐/视频应用播放。采集只转发音频，不创建屏幕录像；目标应用必须允许音频采集。[Android 采集规则](https://developer.android.com/media/platform/av-capture)
5. 改为本机麦克风时先停止共享，再选择麦克风开始；收听端若已停止，需要重新点击收听。麦克风啸叫尚未处理，验证这一音源时优先使用耳机，避免扬声器声音重新进入麦克风。

范围授权及应用选择交给 Android 系统页面。[系统授权文档](https://developer.android.com/media/grow/media-projection) 系统的单应用选项描述画面范围；SonaHop 已按用户要求移除额外 UID 音频筛选，实际音频捕获仍由 Android 采集规则决定，不宣称额外的单应用音频隔离。

## Windows 共享和远端麦克风

Windows 的内部播放声音仍采用进程音频采集，排除 SonaHop 自身，与本地扬声器音量独立。共享本机麦克风需要已有录音端点；没有录音设备时禁用该音源，可连接设备后刷新。

手机提供 Windows 会议麦克风仍走独立路径：手机在“收听”页连接 Windows 共享端，切换到上传麦克风并主动申请；Windows 展开“提供给电脑应用的麦克风”，选择用户已安装的虚拟音频线播放端（如 CABLE Input）并批准，会议应用再选择对应录音端。Realtek/HDMI 扬声器不能变为录音设备。手机麦克风不会自动混入系统共享。

Windows 收听端可选择 SonaHop 的播放出口，空值使用默认设备，不修改系统默认音频设备。手机和平板使用系统媒体音量键；播放增益固定为 1，麦克风增益单独保留。

## 发现、授权与诊断

发现按运行中的本地 IPv4 接口绑定 socket，发送各子网广播并接收回复；Windows 同子网 TCP 连接也选择对应接口。接口变化会刷新发现任务；暂时的 UDP 重置继续接收，致命接收错误会显示失败与重试。以上只调整 SonaHop socket，不改 VPN、系统路由或防火墙。

若自动搜索不到，在共享端复制本机地址，在收听端填地址和 PIN 连接。多网卡设备可能列出多个地址，请优先使用与另一台设备处于同一 Wi-Fi 网段的地址。用户已验证 0.4.0 的 Windows/Android 可互联；0.4.1 保留这些网络逻辑。按最新反馈保留全部内网地址，不增加 WLAN 过滤。

Android 内部播放采集需要 Android 10+ 和每次新的系统授权。音频服务持有前台通知，不夺取被采集应用的音频焦点；授权撤销、通知停止、服务结束或错误均终止采集，不自动重启麦克风。

连接使用六位 PIN/SPAKE2 与 AES-GCM，TCP 读写独立。连续写阻塞两秒或二十秒没有数据结束连接；媒体队列最多三帧，排队超过 120ms 在加密前丢弃。固定 Opus 48kHz/20ms：收听 stereo 96kbps、远端麦克风 mono 48kbps。包数和信号强度来自实际数据，端到端延迟尚未测量。

出错后打开“设置 → 连接诊断 → 读取最近日志 → 复制日志”。本次运行保留最近 256 条，本地日志约 1MiB 加一份备份；日志包含发现接口、探测/回复计数和连接结束原因，不记录 PIN、密钥或音频正文。

Windows 开机自启默认关闭；用户开启后登录时打开应用，并遵循自动发现设置，音频仍需手动启动。注册项使用当前 exe 路径，移动程序后需重新设置。

## 构建

本机工具：Rust MSVC、Visual Studio C++、CMake/Ninja、JDK21、SDK platform37、NDK30.0.16248370、Rust target aarch64-linux-android。

```powershell
pnpm install --frozen-lockfile
$env:JAVA_HOME = 'D:\ProgramFiles\java\jdk-21.0.12.1+1'
$env:ANDROID_HOME = 'D:\Android\Sdk'
$env:NDK_HOME = 'D:\Android\Sdk\ndk\30.0.16248370'
./scripts/build-audio-mvp.ps1 -Platform Both -OutputDirectory artifacts/ui-refinement
```

脚本只编译打包，不启动应用或录音。Tauri 先生成插件依赖并编译当前 Rust，只有明确的 Windows 符号链接权限错误才使用同一次成功编译的 .so 交给 Gradle。交付记录在 artifacts/ui-refinement/build-info.json，构建日志在同目录 build.log 和 android-build.log。

开发原生界面：pnpm dev；浏览器布局预览：pnpm web:dev，地址 http://localhost:1420/?platform=windows 或 ?platform=android。浏览器预览不提供真实连接或音频。

前端采用 Biome、Tailwind4 和 shadcn/ui，源码位于 apps/sonahop-ui/src。测试与应用/音频验证由用户执行，编译和签名检查不代表真机功能已验证。

## 诊断 CLI 与后续

cargo run -p sonahop -- devices --json 仅列 Windows 播放设备。原有 capture --output artifacts/playback-new.wav --seconds 5 是用户手动的系统端点采集诊断，须在采集期间播放声音；不会覆盖已有文件，WAV 保持 float32。

当前计划：docs/superpowers/plans/2026-10-12-ui-refinement.md。角色分离设计：docs/superpowers/specs/2026-10-12-multi-role-audio-design.md。

Windows 自带虚拟播放/录音驱动、同机同时作为共享和收听端、音源混音、专用 USB 音频和麦克风回授抑制仍为后续范围。虚拟设备方案：docs/superpowers/specs/2026-10-12-virtual-audio-devices.md。
