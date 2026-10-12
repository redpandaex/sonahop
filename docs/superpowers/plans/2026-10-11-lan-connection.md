# LAN Discovery and Connection Implementation Plan

> 执行采用轻量并行分工；用户自行运行测试与构建。现有授权为提交当前阶段后直接继续实现，不再重复过程确认。

**Goal:** Windows 与 Android 可主动发现、请求连接、确认和断开，真实心跳状态，无音频。
**Architecture:** sonahop-link 共用Rust套接字/有界协议，Tauri适配原生命令与Android LAN许可，React共享连接状态。
**Spec:** docs/superpowers/specs/2026-10-11-lan-connection.md

- [ ] 连接核心：协议/状态类型、UDP广播探测及单播回应、过期清理、长度界限；TCP待确认/接受/拒绝、双peer上限、心跳与停止生命周期。
- [ ] Tauri与Android：原生LAN权限小插件，API37授权/拒绝；只组合已有库；6个命令与平台角色；销毁时停止。
- [ ] 界面：Windows服务开关/请求确认/设备列表；Android搜索/发现列表/手动IPv4连接/核对编号/断开；原生错误明确显示，浏览器无假网络。
- [ ] 收尾：保留小阶段说明和用户验证步骤，静态阅读接口/状态一致性；不运行测试、编译、真实网络或真机操作。

Review focus: 超大/未知版本控制帧、停用后迟到状态与旧连接清理、Windows拒绝/超时及容量、Android许可拒绝/广播隔离、手机设置页切换与组件卸载不得伪造/泄漏连接。