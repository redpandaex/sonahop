# Android 音频前台状态修复 — 2026-10-12

用户已连上电脑，在可见界面点击开始收听，prepareAudio 拒绝：Audio permission must be requested from a visible user action。

根因：前台判断使用初始化时 hasWindowFocus() 的缓存，仅靠插件 onResume/onPause 更新。Tauri 2.11.6 PluginManager.load 不重放当前生命周期，晚注册时缓存 false；错误发生在系统权限申请之前，不能说明用户权限不足。

修复：在现有 UI 线程上直接读取 Activity 的 LifecycleOwner.currentState ≥ RESUMED；继续检查 finishing/destroyed/decorView.isShown。删除前台缓存和插件 resume/pause 覆写；权限请求与原生录放启动都使用同一真实状态检查。后台麦克风启动仍被拒绝，RECORD_AUDIO 与通知权限行为保持。

静态复核：Activity 是 AppCompatActivity/LifecycleOwner；已有 AppCompat 依赖提供 Lifecycle，不添加新依赖。仅重构建 Android APK，不运行自动测试或设备/录音/播放操作。用户更新 APK 后复验连接、开始收听与权限弹窗返回。
构建结果：Android Kotlin/Rust/Gradle 打包成功，APK 签名检查退出 0；修复包 sonahop-android-foreground-fix.apk，22105981 bytes，SHA256 44EFEF6C882B51AC45D2B6D042DA19A6E0506FB9868703963E192F616DAEF8F8。尚未真机复测。电脑端协议和程序不变，本次仅更新 Android。
