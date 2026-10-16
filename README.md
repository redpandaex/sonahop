# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前交付：0.5.0

Windows 和 Android 都可共享本机声音，也可收听另一台设备。一个共享设备最多连接两台收听设备；内部播放声音与本机麦克风分别选择，不混音。Android 客户端可在收听 Windows 的同时上传麦克风，两条方向可分别停止。Windows 共享端还可独立接收一台远端麦克风到用户已有的虚拟音频线。

界面采用“共享 / 收听 / 设置”，Android 使用固定底部导航。打开应用后自动恢复上次角色并启动局域网发现，不自动配对、共享、收听或录音。切换角色先停止音频和连接，再启动新角色的发现；进入设置保留当前会话。常规联网开关已移除，启动失败时可重试。

## 最新安装包

- artifacts/vb-cable/sonahop-windows.exe
- artifacts/vb-cable/sonahop-windows-setup.exe（独立主程序安装器，不包含虚拟驱动）
- artifacts/vb-cable/sonahop-android-arm64-debug.apk

本版 Windows/Android 版本为 0.5.0。Android 为 ARM64 debug 包，保留原 debug 签名，versionCode 5000；历史 address-polish 是 0.4.2、ui-refinement 是 0.4.1。安装器按当前用户安装主程序，虚拟设备由用户按需单独安装。

新 Windows exe 路径可能触发新的防火墙提示，请允许在本地网络通信；也可先关闭旧程序，再将新版替换到原先允许的 exe 路径。不要同时运行两个版本。

## 小米共享，电脑和平板一起听

1. 小米打开“共享”，等待自动发现启动；连接区显示本机 IPv4 地址和六位 PIN。
2. 电脑和 Y700 打开“收听”，填小米的 PIN，选择发现的小米；在小米核对并接受两台设备。
3. 小米选择“应用声音”并点击开始共享，允许录音权限；在 Android 系统授权窗口选择单个应用或整个屏幕。选择单个应用时，下一步由系统页面选择应用。SonaHop 不再显示另一套范围/应用列表。
4. 电脑和 Y700 各点击开始收听，小米切到所选音乐/视频应用播放。采集只转发音频，不创建屏幕录像；目标应用必须允许音频采集。[Android 采集规则](https://developer.android.com/media/platform/av-capture)
5. 改为本机麦克风时先停止共享，再选择麦克风开始；收听端若已停止，需要重新点击收听。麦克风啸叫尚未处理，验证这一音源时优先使用耳机，避免扬声器声音重新进入麦克风。

范围授权及应用选择交给 Android 系统页面。[系统授权文档](https://developer.android.com/media/grow/media-projection) 系统的单应用选项描述画面范围；SonaHop 已按用户要求移除额外 UID 音频筛选，实际音频捕获仍由 Android 采集规则决定，不宣称额外的单应用音频隔离。

## Windows 共享和远端麦克风

Windows 的内部播放声音仍采用进程音频采集，排除 SonaHop 自身，与本地扬声器音量独立。共享本机麦克风需要已有录音端点；没有录音设备时禁用该音源，可连接设备后刷新。

手机提供 Windows 会议麦克风仍走独立路径：手机在“收听”页连接 Windows 共享端，切换到上传麦克风并主动申请；Windows 展开“提供给电脑应用的麦克风”，选择用户已安装的虚拟音频线播放端（如 CABLE Input）并批准，会议应用再选择对应录音端。Realtek/HDMI 扬声器不能变为录音设备。手机麦克风不会自动混入系统共享。

Windows 收听端可选择 SonaHop 的音频输出设备，空值使用默认设备，不修改系统默认音频设备。手机和平板使用系统媒体音量键；播放增益固定为 1，麦克风增益单独保留。

## 手机共享麦克风，Windows 接收给会议应用

1. 手机进入“共享”，选择“本机麦克风”，主动开始并授权。
2. Windows 进入“收听”，使用手机的 IP/PIN 连接，在手机确认连接。
3. Windows 接收用途切换为“电脑麦克风”，选择虚拟音频线播放端（如 CABLE Input），再点击开始接收。界面显示会议软件应选择的对应录音端。
4. 在会议或录音应用里选择该虚拟音频线的对应录音端。此路径沿用现有接收输出能力，不安装驱动或更改 Windows 默认设备。

这个角色组合参考 [AudioRelay 手机麦克风流程](https://audiorelay.net/docs/windows/use-your-phone-as-a-mic-for-windows)：手机共享麦克风，Windows 接收到虚拟麦克风，业务应用使用其录音端。SonaHop 使用用户已有的虚拟音频线，并提供专门的接收用途选择；自带虚拟麦克风驱动已暂停。

## 按需接入 VB-CABLE / Voicemeeter

VB-Audio 是厂商，VB-CABLE 是它提供的虚拟音频线，Voicemeeter 是带虚拟设备的混音器。接入复用 Windows WASAPI，不需要专用 SDK，也不依赖 AudioRelay 的私有驱动接口。

1. 需要电脑麦克风时，从 [VB-CABLE 官网](https://vb-audio.com/Cable/) 下载并按厂商说明安装，完成后重启 Windows。主程序不自动安装驱动或调整启动策略。
2. 在 SonaHop 刷新设备，明确选择 CABLE Input；在会议/录音软件选择 CABLE Output。安装 CABLE A/B/C/D 时同样选择同一条线的两端。
3. 使用 [Voicemeeter](https://vb-audio.com/Voicemeeter/) 时，需要运行混音器，选择其输入端并启用相应 B 总线，录音软件使用对应输出。这里只接入音频端点，不自动操作混音器。

推荐列表按播放/录音端名称识别；已知录音端未启用时阻止启动。同名、改名或其他驱动可通过“使用其他虚拟设备”手动选择播放端及录音端，选择分别按设备 ID 保存。手动模式由用户确认两端属于同一条音频线；缺失或已禁用的录音端不能启用。设备识别是界面提示，无法仅凭名字保证任意驱动内部路由。

普通播放和电脑麦克风的输出选择分别保存，不修改 Windows 默认设备，不默认选择实体扬声器作为麦克风入口。若两条独立虚拟路由都需要虚拟音频线，应使用不同的线，避免信号串入同一条线。

VB-CABLE/Voicemeeter 是外部可选组件，采用厂商的捐赠授权；没有被复制或捆绑到 SonaHop。具体使用和商业授权以 [厂商条款](https://vb-audio.com/Services/licensing.htm) 为准。

## 用户验收

1. VB-CABLE 安装后刷新：能选择 CABLE Input，能看到 CABLE Output 提示；会议软件使用该录音端，对手机说话检查信号。
2. 分别验证 Windows 共享端批准远端麦克风、Android 共享麦克风到 Windows 收听端两种角色组合。
3. 同一台 Android 连接 Windows，共享端批准麦克风后同时收听/上传；分别停止一条方向，确认另一条继续。在一条运行、另一条等待时点击通知“停止全部”，再批准旧申请，确认不会重新开始采集。麦克风测试优先使用耳机。
4. 重开应用检查两种输出选择独立保存；禁用录音端后刷新应显示不可用；主程序安装/卸载不安装或删除第三方驱动。

## 发现、授权与诊断

发现按运行中的本地 IPv4 接口绑定 socket，发送各子网广播并接收回复；Windows 同子网 TCP 连接也选择对应接口。接口变化会刷新发现任务；暂时的 UDP 重置继续接收，致命接收错误会显示失败与重试。以上只调整 SonaHop socket，不改 VPN、系统路由或防火墙。

若自动搜索不到，在共享端复制本机 IP，在收听端填写 IP 和 PIN 连接。多网卡设备可能列出多个地址，请优先使用与另一台设备处于同一 Wi-Fi 网段的地址。用户已验证 0.4.0 的 Windows/Android 可互联；0.4.1 保留这些网络逻辑。Windows 地址区仅显示 Wi-Fi 接口地址，Android 保留原地址列表。界面、复制及手动填写只使用 IP；旧保存或粘贴的 IP:端口 会转为 IP，内部仍使用固定控制端口。发现与连接继续使用完整接口列表。

Android 内部播放采集需要 Android 10+ 和每次新的系统授权。音频服务持有前台通知，不夺取被采集应用的音频焦点；授权撤销、通知停止、服务结束或错误均终止采集，不自动重启麦克风。

连接使用六位 PIN/SPAKE2 与 AES-GCM，TCP 读写独立。连续写阻塞两秒或二十秒没有数据结束连接；媒体队列最多三帧，排队超过 120ms 在加密前丢弃。固定 Opus 48kHz/20ms：收听 stereo 96kbps、远端麦克风 mono 48kbps。包数和信号强度来自实际数据，端到端延迟尚未测量。

出错后打开“设置 → 连接诊断 → 读取最近日志 → 复制日志”。本次运行保留最近 256 条，本地日志约 1MiB 加一份备份；日志包含发现接口、探测/回复计数和连接结束原因，不记录 PIN、密钥或音频正文。

Windows 开机自启默认关闭；用户开启后登录时打开应用，并遵循自动发现设置，音频仍需手动启动。注册项使用当前 exe 路径，移动程序后需重新设置。

## 自动发布

发布流程见 [GitHub Releases 发版说明](docs/releases.md)。只有 main 历史提交上的版本 tag 才构建发布，Android 需要固定签名 Secrets；普通 push 不发布。当前分支尚未合并 main，也未创建发布 tag。

## 构建

本机工具：Rust MSVC、Visual Studio C++、CMake/Ninja、JDK21、SDK platform37、NDK30.0.16248370、Rust target aarch64-linux-android。

```powershell
pnpm install --frozen-lockfile
$env:JAVA_HOME = 'D:\ProgramFiles\java\jdk-21.0.12.1+1'
$env:ANDROID_HOME = 'D:\Android\Sdk'
$env:NDK_HOME = 'D:\Android\Sdk\ndk\30.0.16248370'
./scripts/build-audio-mvp.ps1 -Platform Both -WindowsInstaller -OutputDirectory artifacts/vb-cable
```

脚本只编译打包，不启动应用或录音。Tauri 先生成插件依赖并编译当前 Rust，只有明确的 Windows 符号链接权限错误才使用同一次成功编译的 .so 交给 Gradle。交付记录在 artifacts/vb-cable/build-info.json，构建日志在同目录 build.log 和 android-build.log。

开发原生界面：pnpm dev；浏览器布局预览：pnpm web:dev，地址 http://localhost:1420/?platform=windows 或 ?platform=android。浏览器预览不提供真实连接或音频。

前端采用 Biome、Tailwind4 和 shadcn/ui，源码位于 apps/sonahop-ui/src。测试与应用/音频验证由用户执行，编译和签名检查不代表真机功能已验证。

## 诊断 CLI 与后续

cargo run -p sonahop -- devices --json 仅列 Windows 播放设备。原有 capture --output artifacts/playback-new.wav --seconds 5 是用户手动的系统端点采集诊断，须在采集期间播放声音；不会覆盖已有文件，WAV 保持 float32。

当前计划：docs/superpowers/plans/2026-10-13-optional-devices-duplex.md。角色分离设计：docs/superpowers/specs/2026-10-12-multi-role-audio-design.md。

自研 Windows 虚拟播放/录音驱动暂停；同机同时作为共享和收听角色、音源混音、专用 USB 音频和麦克风回授抑制仍为后续范围。同一 Android 客户端收听和上传麦克风的改动已纳入本版，需按上面的验收步骤实测。虚拟设备方案：docs/superpowers/specs/2026-10-12-virtual-audio-devices.md。
