# Spec Delta

## Purpose

确保 Android 用户能够在明确授权和可见共享状态下使用音频功能，并在界面后台化、锁屏、权限变化及系统音频中断时保持可理解的行为，使用两台指定 Android 16 真机取得验证证据。

## ADDED Requirements

### Requirement: Explicit microphone permission and start
系统 MUST 在取得录音权限并由用户从 Android 可见界面确认启动后开始麦克风采集；权限拒绝或撤销时不采集，并显示权限状态。

#### Scenario: Permission is denied
- **WHEN** 用户拒绝录音权限
- **THEN** 应用不启动麦克风采集，显示允许录音的操作提示，不误报流正在运行

#### Scenario: Permission is revoked during capture
- **WHEN** 用户撤销正在使用的录音权限
- **THEN** 应用停止采集并同步电脑端状态，不继续保存或发送未获授权的音频

### Requirement: Background audio session ownership
系统 SHALL 使用符合 Android 要求的后台音频运行机制，使已经由用户启动的播放或录音会话在界面退到后台或锁屏后继续工作，并提供可见状态与停止入口。

#### Scenario: Lock the screen while listening
- **WHEN** 任一指定 Android 16 设备正在收听，用户锁屏并保持网络正常
- **THEN** 音频继续播放，用户仍可通过系统提供的运行状态入口停止该会话

#### Scenario: Lock the screen while supplying a microphone
- **WHEN** 指定设备已在可见界面完成授权并启动麦克风，随后锁屏
- **THEN** 电脑继续收到该设备的有效输入，系统运行状态保持可见

### Requirement: UI reopening preserves the actual session
系统 SHALL 在界面重新打开时查询并显示实际后台会话状态，MUST 不因界面重建重复创建录音或播放会话。

#### Scenario: Reopen an active app
- **WHEN** 用户在播放或录音期间离开并重新打开界面
- **THEN** 界面显示实际活动模式，只有一条对应音频流，停止按钮可以结束该流

### Requirement: Visible interruption and controlled recovery
系统 SHALL 处理音频设备变化、系统音频中断和后台服务终止；无法继续时说明原因并同步电脑端，录音重新开始 MUST 遵守用户授权及系统启动要求。

#### Scenario: The native session is terminated
- **WHEN** Android 系统终止音频服务或使录音设备不可用
- **THEN** 电脑不再显示该设备正常采集，应用恢复可见界面后显示实际停止或中断状态，并要求符合条件的重新启动
