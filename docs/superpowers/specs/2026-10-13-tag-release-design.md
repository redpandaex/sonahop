# Tag Release 设计

用户确认：v0.5.0 等语义版本 tag 指向 main 历史中的任意提交即可发布；不要求 main 最新提交。发布 Windows x64 免安装 exe、NSIS 安装器和固定正式签名的 Android ARM64 APK，附 SHA256SUMS 与提交/签名来源信息。

GitHub 的 branches/tags 过滤是分别匹配，不能表达“main 且 tag”。只监听版本 tag push，先在轻量 job 验证完整历史的 main 归属及应用版本；删除 tag、不属于 main 的 tag 均不构建或发布。分支上的 tag 可能启动校验 job，但不启动构建/发布 job。

后续构建 checkout 固定提交；发布前重新查询远端 tag 与 main，防止 tag 移动或历史重写导致资产对应错误。仅发布 job 有 contents:write 权限。所有平台成功、APK签名验证和摘要核对后才公开 Release；中途失败保持未公开草稿。已公开 Release 不被重试覆盖。

用户明确本轮不生成签名文件和 Secrets。workflow 使用 ANDROID_KEYSTORE_BASE64、ANDROID_KEYSTORE_PASSWORD、ANDROID_KEY_ALIAS、ANDROID_KEY_PASSWORD；缺失时早期报错，不以 debug APK 顶替。Android Release 编译使用已有保留 JNI 类的 consumer ProGuard 配置。

当前支持的平台范围与已交付程序一致。CI 不安装/发布虚拟音频驱动，不运行音频或设备测试。当前分支提交推送后，需要用户配置 Secrets、合并 main 并推送 tag 才会首次实际运行。
