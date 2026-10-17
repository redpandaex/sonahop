# GitHub Releases 发版

仅推送 v0.5.0 这类版本 tag 会启动校验。tag 的提交必须属于 main 历史，应用版本也必须一致；满足后构建并发布。普通分支 push、PR 和手动操作不会发布。

Windows x64 免安装 exe、NSIS 安装器、Android ARM64 正式 APK、SHA256SUMS.txt 和 release-manifest.json 会一起进入 GitHub Releases。虚拟音频驱动不包含在产物中。

## 一次性准备 Android 签名

已有正式 keystore 时直接使用并备份，后续版本始终保留同一把密钥。Android keystore 用于签 APK，无需把证书安装进 Windows。当前 Windows 产物不启用 Authenticode 签名，无需配置 Windows 证书。

尚无密钥时，可在项目根目录手动执行；以下操作本轮没有由代理执行：

    New-Item -ItemType Directory -Path .secrets/android -Force
    keytool -genkeypair -keystore .secrets/android/sonahop-release.jks -storetype JKS -alias sonahop -keyalg RSA -keysize 3072 -validity 10000 -dname "CN=SonaHop"

命令会询问 keystore 密码与条目密码；条目密码直接回车时与 keystore 密码相同。备份 keystore、两项密码与别名到你保管的位置。不要把私钥、密码或 Base64 内容提交到 Git 或发送到聊天；.secrets/ 与常见 keystore 扩展名均被 Git 忽略。

生成 Base64 后可直接复制到剪贴板，无需把内容显示在终端：

    Set-Clipboard -Value ([Convert]::ToBase64String([IO.File]::ReadAllBytes((Resolve-Path '.secrets/android/sonahop-release.jks').Path)))

在 GitHub 仓库 Settings → Secrets and variables → Actions → Repository secrets 添加：

| Secret | 值 |
| --- | --- |
| ANDROID_KEYSTORE_BASE64 | keystore 文件的 Base64，粘贴剪贴板内容 |
| ANDROID_KEYSTORE_PASSWORD | keystore 密码 |
| ANDROID_KEY_ALIAS | 条目别名；上述命令是 sonahop |
| ANDROID_KEY_PASSWORD | 条目密码；直接回车时填同一个密码 |

GITHUB_TOKEN 由 GitHub Actions 自动提供，无需手动添加。

Secrets 未配置时 workflow 会早期失败，不发布 Windows 或 debug APK。后续持续使用同一份 keystore，避免覆盖更新失败。当前本地测试 APK 使用另一把 debug 签名；首次安装新正式 APK 需要卸载旧 debug 包，其本地应用配置会清除。

[Android 签名文档](https://developer.android.com/studio/publish/app-signing)

## 每次发布

1. 把 workflow 与待发布代码合并到 main。
2. 确认下面三处应用版本相同，且与 tag 去掉 v 后一致：
   - apps/sonahop-ui/package.json
   - apps/sonahop-ui/src-tauri/Cargo.toml
   - apps/sonahop-ui/src-tauri/tauri.conf.json
3. 在待发布的 main 提交上创建并推送 tag。第一次当前版本示例：

    git switch main
    git pull --ff-only origin main
    git tag -a v0.5.3 -m "SonaHop 0.5.3"
    git push origin v0.5.3

4. 等 GitHub Actions 的 Release workflow 完成，再从 Releases 下载。当前支持三个二进制产物，所有平台成功后才公开发布。

带 prerelease 后缀的 tag（例如 v0.6.0-beta.1，应用版本也为 0.6.0-beta.1）会标记为预发布。版本不匹配会失败，不自动改版本或创建 tag。

## 构建与失败处理

CI 使用 Rust 1.99.0、Node 24、pnpm 11.25.0、JDK 21、Android SDK platforms;android-37.0、build-tools 36.0.0、NDK 30.0.16248370。所有 Action 固定提交 SHA。

现有构建脚本新增 AndroidRelease；普通本地 debug 打包参数保持不变：

    ./scripts/build-audio-mvp.ps1 -Platform Both -WindowsInstaller -AndroidRelease -OutputDirectory artifacts/release-build

它只生成 unsigned Release APK；签名脚本读取环境中的 ANDROID_KEYSTORE_PATH、ANDROID_KEYSTORE_PASSWORD、ANDROID_KEY_ALIAS、ANDROID_KEY_PASSWORD：

    ./scripts/release/sign-android.ps1 -InputApk artifacts/release-build/sonahop-android-arm64-release-unsigned.apk -OutputApk artifacts/release-build/sonahop-android-arm64-release.apk -Version 0.5.3

签名工具先拒绝 debuggable APK，再做 16KiB ZIP 对齐、v2/v3 签名和验证。签名密码只以环境变量名称传给工具，临时 keystore 不上传；CI 完成或失败都会清除临时文件。

Android 安装步骤显式使用 platform-tools，不安装已下架的旧 tools 包。workflow 的 JavaScript Actions 均固定到原生 Node 24 版本。

发布阶段核对 tag/main、产物来源和 SHA256，上传后、公开前再检查一次远端 tag 与 main；支持普通和附注 tag，远端 tag 被删除或移动时中止。上传失败时留在未公开草稿，可以在同一个 tag 的 workflow 上重试。已公开版本不覆盖、不自动删除；修复发布内容时应使用新版本 tag。

首个完整自动发布版本 [v0.5.3](https://github.com/redpandaex/sonahop/releases/tag/v0.5.3) 已于 2026-10-17 发布。Windows 与 Android 编译、固定 APK 签名验证、跨 job 产物传递和最终公开发布均通过；三个二进制的大小与 SHA256 已对照公开 Release 核对。音频和真机行为仍由用户验收。

[GitHub tag/branch 过滤规则](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushbranchestagsbranches-ignoretags-ignore)

失败 tag 指向当时的 workflow 与锁文件，点击 Re-run 不会采用 main 上的新修复。为修复后的提交递增补丁版本，版本配置一致后创建新 tag；不移动或覆盖旧 tag。

## pnpm 冻结安装报错

pnpm 11 在 devEngines.packageManager 配置下还会锁定 pnpm 自身。pnpm-lock.yaml 的 packageManagerDependencies 缺失时，即使应用依赖没有变化，CI 的 frozen-lockfile 安装仍会失败。

使用与 CI 相同的 pnpm 11.25.0，通过命令更新并验证锁文件：

    pnpm install --lockfile-only --no-frozen-lockfile
    pnpm install --frozen-lockfile

应保留 CI 的 frozen-lockfile，不手工填写包版本或 integrity。本地验证还需确认 pnpm 入口没有设置 pm_on_fail=ignore；此覆盖会跳过包管理器锁文件检查。发布前在无 node_modules 的目录验证冻结安装，并确认安装前后锁文件哈希不变。

Android 打包显式使用 --split-per-abi --target aarch64，让正常 Tauri 构建与 Windows 符号链接回退都生成 arm64 目录中的 APK。未指定 split-per-abi 时，Tauri 会输出 universal APK，与当前 ARM64 产物路径不一致。
