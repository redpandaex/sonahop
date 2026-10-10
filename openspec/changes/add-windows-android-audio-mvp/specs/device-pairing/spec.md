# Spec Delta

## Purpose

定义局域网设备如何在用户授权下建立和保留信任关系，并在连接中断后恢复控制通道，同时让配对、连通和音频工作状态分别可见，避免新连接被误认为已经取得共享权限。

## ADDED Requirements

### Requirement: User-controlled local pairing
系统 SHALL 提供用户可操作的局域网配对流程；未配对或身份验证失败的设备 MUST 无法接收电脑音频或提供电脑麦克风。

#### Scenario: Pair a new Android
- **WHEN** 用户在电脑开放配对并在 Android 输入连接信息，双方完成身份确认
- **THEN** 应用保存该设备的信任关系并允许用户选择共享模式

#### Scenario: An unknown device requests audio
- **WHEN** 未配对设备尝试加入音频会话
- **THEN** 系统拒绝音频访问，已有设备的共享状态不受影响

### Requirement: Authenticated encrypted transport
系统 MUST 对设备身份进行验证并加密音频和控制数据，重新连接时按保存的身份验证对端，不能仅凭设备显示名称或 IP 认定身份一致。

#### Scenario: Identity changes at the same address
- **WHEN** 已配对地址上的设备身份与保存记录不一致
- **THEN** 应用拒绝自动信任，并提示重新进行身份确认

### Requirement: Recover connections without reviving stopped capture
系统 SHALL 为每台设备显示连接中、已连接、断线和恢复状态；只有仍有效的共享意图和授权可以恢复音频，用户停止后的新连接 MUST 不自行启动采集。

#### Scenario: A listening device reconnects
- **WHEN** 设备 A 在有效收听会话中短暂断线后重新连接，电脑仍处于收听模式
- **THEN** A 可以恢复当前内容的收听，使用新的流标识和时间线，设备 B 的收听保持独立

#### Scenario: Reconnect after the user stopped
- **WHEN** 用户停止了共享，设备稍后重新建立控制连接
- **THEN** 应用显示已连接且音频停止，麦克风不自动开始采集

### Requirement: Clear connection and setup errors
系统 SHALL 区分地址不可达、身份确认失败、设备离线和音频前置条件缺失，提供可执行的检查提示，并允许用户取消连接尝试。

#### Scenario: A network prevents access
- **WHEN** 局域网隔离或防火墙使设备无法连接
- **THEN** 应用显示连接失败及地址、网络或防火墙检查提示，不自动修改系统防火墙或网络配置
