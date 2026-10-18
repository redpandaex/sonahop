# SonaHop

以 Rust 为核心、Tauri 提供 Windows / Android 界面的跨设备音频项目。

## 当前源码：一键连接与默认共享（待发布）

Windows 和 Android 都能共享本机声音，也能收听另一台设备；每个共享设备最多连接两台收听设备。当前源码使用无 PIN 的 v3 加密连接，取消连接审批，收听端点击“连接”后自动收听，点击“断开”结束连接和音频。

**两端必须一起更新。** 已发布的 [v0.5.3](https://github.com/redpandaex/sonahop/releases/tag/v0.5.3) 使用旧连接协议，不包含本节的新交互，不能与当前 v3 源码互连。2026-10-18 已生成本地验证产物，位于 artifacts/ui-simplified，版本号仍为 0.5.3、连接协议为 v3；尚未发布新版本。

## 共享与收听

1. 共享设备打开“共享”，自动开启发现并准备系统/应用声音。Android 每次启动内部声音采集仍需确认系统授权，可在系统页面选择单个应用或整个屏幕。
2. 另一台设备打开“收听”，选择输出设备，再点击发现的共享设备“连接”；不填 PIN，也不需要共享端再接受。
3. 收听自动开始；音源未就绪时显示等待。点击设备卡片的“断开”即停止本端音频并断开。
4. 共享端可暂停、恢复或切换音源，收听端保留订阅并等待新音源，不用重新连接；最后一台收听设备退出不会停止共享音源。进入设置保留当前会话，切换共享/收听角色会结束旧会话。
5. 麦克风只在明确选择后启用，打开应用或重连不会自动恢复麦克风录制。系统停止、权限撤销或采集错误后也不会自动反复请求权限。

Android 内部音频采集需要 Android 10+，目标应用必须允许音频被采集。SonaHop 只转发音频，不保存屏幕录像；系统的单应用选择不代表 SonaHop 添加了额外 UID 音频筛选。[Android 采集规则](https://developer.android.com/media/platform/av-capture)

## 手机作为电脑麦克风

1. 手机进入“共享”，点击“麦克风”音源并允许录音。
2. Windows 进入“收听”，选择“电脑麦克风”用途，以及用户已安装虚拟音频线的播放端，再连接手机。
3. 在会议或录音应用中选择对应录音端。例如 SonaHop 写入 VB-CABLE 的 CABLE Input，会议应用读取 CABLE Output。SonaHop 不安装驱动，也不修改系统默认设备。
4. 已识别的 VB-CABLE/Voicemeeter 入口会提示对应录音端；重复名称或手动配置需明确选择，避免把声卡扬声器误当成麦克风入口。

这与 [AudioRelay 的手机麦克风流程](https://audiorelay.net/docs/windows/use-your-phone-as-a-mic-for-windows) 对应：手机提供麦克风音源，电脑接收到虚拟麦克风路径。已有其他虚拟设备也可通过高级配置明确选择，不依赖 AudioRelay 的私有接口。

Windows 共享系统声音时，仍可独立接收一台手机上传的麦克风；手机也可同时收听和上传麦克风。此高级方向保留独立申请/批准，不会因连接而开始录音。手机麦克风与系统共享不混音；麦克风监听/回放啸叫处理仍未实现。

## 音量、布局和状态

接收音量使用设备自己的系统音量键。Windows 系统音源使用进程音频采集、排除 SonaHop 自身，与本地扬声器音量独立；麦克风增益可单独调整。

普通页面只有一个正文滚动区域。Windows 展开高级麦克风面板仍由主内容区滚动；Android 页头和底部导航留在正文外，系统栏与刘海边距由原生容器处理。下拉弹层可以独立滚动。

状态依据真实连接和音频状态显示：灰色停止，蓝色可发现/搜索，黄色连接/等待，绿色已连接/活动，红色错误。文字状态同时保留，不只依赖颜色。

## 网络与诊断

同一局域网中可发现并直接连接共享设备；也可复制共享端 IP 手动连接。Windows 显示 Wi-Fi 地址，Android 保留可用地址列表。输入和复制只使用 IP，内部控制端口固定；粘贴旧格式 IP:端口 时会提取 IP。共享设备名称和地址不再存入连接偏好，也不保留最近连接；手动 IP 只在当前输入中使用，连接成功后清空。

v3 使用每连接临时 X25519、HKDF-SHA256 和双向 AES-256-GCM，保留计数器、转录绑定、握手/人数上限与速率限制。无 PIN 模式不验证设备身份，同网络中的设备均可接入；传输加密不等于对主动中间人的身份验证。

TCP 读写独立，发送队列有界；连续写阻塞两秒或二十秒无数据时结束连接，排队超过 120ms 的媒体在加密前丢弃。固定 Opus 48kHz/20ms：收听 stereo 96kbps，远端麦克风 mono 48kbps。信号和包数来自实际数据，端到端延迟尚未测量。

出错后打开“设置 → 连接诊断 → 读取最近日志 → 复制日志”。日志保留最近 256 条，本地约 1MiB 加一份备份，不记录密钥或音频正文。

Windows 开机自启默认关闭。启用后打开应用并恢复角色；处于共享角色时会准备系统声音，麦克风仍需明确选择。程序移动后需重新设置自启路径。

## 手动验收

源码检查后，2026-10-18 已按用户要求完成本地 Windows EXE/NSIS 和 Android ARM64 Release 打包；APK 使用现有正式证书，v2/v3 签名及产物哈希已验证，证书与此前 v0.5.3 Release 一致。交付记录在 artifacts/ui-simplified/build-info.json，校验值在同目录 SHA256SUMS.txt。未运行自动化测试、浏览器交互、应用或真机音频。两端使用本地同一新版后检查：
- 收听页不再有保存设备模块；手动 IP 从空白开始，成功连接后清空，重启不会回填；主题和音频设备偏好保留。
- 连接无需 PIN，自动收听，断开后停止，重新连接继续播放。
- 已连接时暂停/恢复、切换共享音源，收听自动恢复并保留原输出设备；安卓锁屏或切后台、暂停超过 12 秒后也检查这一点；连接途中快速断开能回到发现列表。
- 两台收听上限以及第三台拒绝；手机麦克风需显式授权。
- Windows 展开详情只有一个主滚动条。
- Android 横竖屏、刘海、键盘和底部导航下的安全区与滚动。
- 浅深色下各状态文字、圆点颜色与实际状态一致。

## 2026-10-18 验收反馈与修正

用户报告基础互联、暂停恢复、双设备、同时收发、后台运行及主要界面检查通过。音源切换功能通过，手动录音端布局需调整。麦克风噪声场景已澄清为“手机共享麦克风 → PC 本地播放”，并未启用电脑虚拟麦克风用途；用户明确选择 Type-C 耳机后确认人声正常、刺耳声基本消失，本轮不改动录音源或音频算法。

本地最新包位于 artifacts/ui-simplified：主动停止原因在设备清理前登记，正常本端/对端关闭不再写成音频错误，真正的传输失败仍留在诊断；正在握手的连接也有最多两秒发送关闭消息的宽限。高级录音端增加独立分组、间距和分割线。

本轮优先回归：两个方向分别点击断开及连接途中取消，检查不长期卡住、不把正常停止报成网络/音频异常；展开“使用其他虚拟设备”检查录音端标签与按钮分离。修正后的真机结果仍待用户确认。

## 自动发布

发布流程见 [GitHub Releases 发版说明](docs/releases.md)。只有 main 历史提交上的版本 tag 才构建发布，Android 需要固定签名 Secrets；普通 push 不发布。失败版本的重试和新 tag 步骤见发版说明。

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

当前计划：[一键连接与视口调整](docs/superpowers/plans/2026-10-17-one-click-audio-viewport.md)。已完成的可选虚拟设备阶段：docs/superpowers/plans/2026-10-13-optional-devices-duplex.md。角色分离设计：docs/superpowers/specs/2026-10-12-multi-role-audio-design.md。

自研 Windows 虚拟播放/录音驱动暂停；同机同时作为共享和收听角色、音源混音、专用 USB 音频和麦克风回授抑制仍为后续范围。同一 Android 客户端收听和上传麦克风的改动已纳入本版，需按上面的验收步骤实测。虚拟设备方案：docs/superpowers/specs/2026-10-12-virtual-audio-devices.md。
