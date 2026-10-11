# 前端工具链接入

2026-10-11。用户要求引入格式化/CSS 工具并选择组件库，同时明确由自己运行测试与验证。

通过 CLI 安装 Biome 2.5.15、Tailwind CSS 4.3.3 和 @tailwindcss/vite；Biome init 生成配置。
shadcn CLI 初始化 radix-nova，生成 Button、Input、Badge 和 cn 工具；组件已用于当前桌面/移动页面。
新增 @ 路径别名、Tailwind Vite 插件、data-theme 深色变体和共用调色变量。为避免和 shadcn 的 muted 背景 token 冲突，原文本色 token 改为 text-muted。
Biome 设置两空格、100 列、Tailwind 指令解析，限制前端范围；根脚本提供 format、format:check、lint、check、check:fix。
执行 biome format --write 处理 22 文件（20 文件格式化），这是源码修改；未运行 lint/check/typecheck/test/build、浏览器或真机验证。

给用户的验证步骤：
1. pnpm check / pnpm build，反馈完整报错。
2. pnpm web:dev，打开 http://localhost:1420/?platform=windows 和 ?platform=android。
3. 查看浅色/深色、模式、输入保存、音量滑块和窄屏布局。设备发现与音频当前仍不可用。

现有 exe/APK 不包含这次工具链/组件改动；打包与运行验证待用户反馈或明确授权。
源码改动保留在工作区，便于查看。项目旧规格工具及生成技能已删除；后续只用轻量阶段文档。