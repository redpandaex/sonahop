# Windows / Android UI 阶段验证

完成日期：2026-10-11。设计/计划文件沿用启动阶段的 2026-10-10 文件名。
本阶段按用户最新要求先交付界面，暂未实现网络、收听、麦克风服务或音量独立的音频路径。

## 已执行

| 范围 | 实际结果 |
| --- | --- |
| 前端 | pnpm typecheck、7 个 Node 偏好边界测试和生产 build 通过；测试先观察 RED 再实现 GREEN |
| Rust | fmt、Clippy workspace/all-targets/-D warnings 通过；30 个原有测试全部通过，GUI 无新增 Rust 单元测试 |
| 浏览器 | Playwright CLI 检查 320×740、360×800、800×1280、880×660、1280×850，无横向溢出；手机底部控件滚动后不被导航遮挡 |
| 交互 | 两模式切换、电脑表单保存、音量键盘调整、重载恢复、设备选择/刷新、Windows 深浅/系统主题及 Android 跟随系统主题 |
| 失败路径 | 模拟原生枚举失败，错误可见、来源禁用且不回退到示例；当前所有音频启动动作均不可用 |
| Windows 原生 | tauri build --no-bundle 成功；实际启动 release GUI，Tauri IPC 返回 G24H2 默认输出和 Realtek Digital Output；选择、刷新、模式和原生 setTheme 调用通过 |
| Android | Tauri CLI 成功编译 ARM64 Rust 库；本机符号链接权限不足，用复制库 + 官方 Gradle assembleArm64Debug 完成打包 |
| APK 校验 | apksigner verify 成功；io.github.redpandaex.sonahop / 0.1.0 / versionCode 1000；debuggable；仅 arm64-v8a；不声明 RECORD_AUDIO |

浏览器截图与原生截图已查看。Android 截图是浏览器布局预览，不是手机真机截图。
Windows 测试使用临时 WebView2 本机调试端口检查当前应用，交付时恢复普通启动方式。
没有录制声音、播放测试音、改变系统默认播放设备/音量/防火墙或开启 Windows 开发者模式。

## 本地产物

- Windows：target/release/sonahop-ui.exe。
- Android：artifacts/ui-preview/sonahop-android-debug.apk，19,182,160 字节（约 19 MB）。
- APK SHA256：066CEC13100A650DBD31B91E2FEFC75C3730C5F9EB395355358B621886551BE3。
- APK 内 Rust 库 SHA256：3D301A5A4F50AB8CB74728CB388230A54B639DF0192B8DF991B48588C1901906，与 Gradle stripped_native_libs 中的当前输出一致。
- 完整本地 APK 元数据：artifacts/ui-preview/apk-evidence.json。
- 截图：artifacts/ui-preview/windows-native.png、windows-native-microphone.png、windows-native-settings.png、android-tablet.png、android-tablet-dark.png、android-phone-top.png、android-phone-controls.png。

Android 原始库保留全部调试符号；Gradle 对安装包去除符号。初始 APK 约 132 MB，清理旧增量打包空间后为上述约 19 MB。
完整 Rust 库只依赖 Android 系统 libandroid/libdl/liblog/libm/libc，未遗漏额外第三方 .so。
Android 构建环境为 JDK 21、SDK 37、NDK 30.0.16248370，Gradle 9.6.1；生成模板的 Java/Kotlin/Gradle 弃用警告保留，未修改上游生成代码来掩盖警告。

## 未执行与后续

用户选择本轮先提供 APK，之后连接 Y700。未声称 Android 原生启动、状态栏/导航栏、系统返回、旋转、键盘和两台真机布局通过。
手机麦克风、后台服务、网络传输、同时双接收端及电脑静音时远端仍播放均未实现，本轮预览说明与禁用按钮体现此边界。
下一音频阶段以联想 Y700 四代 / Android 16 为首轮真机，接入后再验证电脑静音时仍能收听。

## 执行取舍

按以下 ledger 裁定执行，保留顺序和判断失误的代价：

1. 用户优先 Windows/Android GUI，交付真实 Windows 枚举与明确预览状态；代价：音频传输留到下一阶段。
2. 重命名后原生 worktree API 无法识别仓库，用忽略目录中的 Git worktree，保留用户 .vscode；代价：需要手动清理该 worktree。
3. design-taste-frontend 排除产品界面，改用 minimalist-ui；代价：视觉 token 需要时可再调整。
4. Tauri Android 不支持 setTheme，Android 跟随系统，Windows 提供三种主题；代价：未来自选 Android 主题需要原生系统栏桥接。
5. 本机缺少符号链接权限，保留 OS 设置，复制已成功编译的库并用 Gradle 打包；代价：后续标准 CLI 需要开发者模式，或按 README 的复制/打包步骤执行，并避免使用旧库。

辅助任务完成脚本因静默 fmt 输出返回失败，最终采用有输出的实际 APK 校验作为 Task 3 完成检查；Windows PowerShell 5 的模块环境不兼容，改用现有 PowerShell 7 运行该本地校验。

## 独立审查与主目录交付

一次独立只读审查覆盖 dc4a24b..f3ff150。最初 gpt-6-astra 因额度不足没有开始审查，改由 gpt-6.1-sol high 完成唯一实际审查。
结论：0 Critical、0 Important，可按 UI 预览范围交付。仅文档收尾发生在该审查之后。

两项轻微待办保留：

1. Android 原生 Manifest 仍引用生成工程中的模板启动图标，后续同步 icons/android 品牌资源。
2. 电脑信息表单在存储不可用时仍先显示已保存，虽然另有失败提示；后续区分本次应用与持久保存。

对审查者按范围留到后续的每项行为作出以下裁定：

- 网络发现、配对、传输和双接收端后续做；当前连接/开始动作禁用。代价：下一阶段必须真正实现并验证。
- 电脑静音独立收听与实际增益后续验收；界面仅保存偏好并说明音频接入后生效。代价：采集或路由可能需要调整。
- 麦克风权限、服务与虚拟音频后续接入；当前无录音权限或活动状态。代价：下一原生阶段必须补权限和后台生命周期。
- 电脑地址仅保存，无网络校验；尚不执行连接。代价：连接动作上线前必须校验端点。
- Android 原生启动、系统栏、返回、旋转、键盘按用户选择延期；截图明确为浏览器预览。代价：音频接入前可能需要真机修正。
- iOS/Linux/macOS/其他 Android ABI 不声称支持。代价：扩平台前补真实后端与构建验证。
- 正式安装器、商店与发布签名后续做；当前交付本地 EXE 和调试 APK。代价：后续需要发布阶段。
- 沿用已授权的主目录交付方式，本地快进到 main，不推送。代价：若需要撤销，可回退本地提交范围。

Windows EXE 为 8,870,912 字节；Android Cargo fingerprint 确认 custom-protocol/compression，调试 APK 使用打包资源。
已本地快进至 D:\code\sonahop 的 main。重新安装依赖后，主目录的 typecheck、7 个 Node 测试、前端/Windows release 构建、fmt 和 30 个 Rust 测试通过。
离线安装缺少注册表策略元数据，改为正常 frozen-lockfile 安装，供应链策略检查通过，锁文件未改变。
Windows GUI 已由主目录 exe 普通启动，进程窗口标题为 SonaHop，无临时 WebView2 调试参数或 9223 监听。
用户未跟踪的 .vscode 文件哈希与交付前一致；原 playback.wav 和旧阶段产物保留。没有推送 GitHub。
本地产物、截图和完整检查日志保留在 artifacts/ui-preview；临时 worktree 清理不影响这些文件。

后续更新：用户已在 Windows 和 Y700 打开应用。上面的 APK 哈希是初次交付记录；当前图标修正包、资源验证及规划简化见 [2026-10-11 修正记录](2026-10-11-ui-fixes.md)。
