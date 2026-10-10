# Spec Delta

## Purpose

让用户选定一台 Android 作为电脑的麦克风来源，并将收到的音频送入 Windows 已有虚拟音频通路，使录音或通信软件能够选择对应输入，同时清楚区分设备配置成功与实际输入验证。

## ADDED Requirements

### Requirement: Selected phone supplies the microphone
系统 SHALL 仅采集用户选定且完成启动授权的 Android 麦克风，并在麦克风模式持续传给已配对 Windows 电脑。

#### Scenario: Start an authorized microphone
- **WHEN** 用户选择设备 A，A 在可见界面确认启动且授予录音权限
- **THEN** 电脑接收 A 的麦克风音频，A 和电脑均显示当前发送设备，设备 B 不发送麦克风音频

### Requirement: Existing virtual audio input integration
系统 SHALL 允许用户配置已有 Windows 虚拟音频通路的播放入口，并说明外部软件应选择的录音出口；MUST 不把普通扬声器播放成功视为麦克风功能验收完成。

#### Scenario: Use the phone from a recording application
- **WHEN** 用户配置有效的虚拟音频入口，在录音或通信软件中选择其对应录音出口，并向所选手机说话
- **THEN** 外部软件获得可识别的语音输入，设备名称和实际使用的软件被记录在验收结果中

#### Scenario: Virtual device is not installed
- **WHEN** 电脑没有可用的虚拟音频通路
- **THEN** 应用说明配置前置条件，不启动麦克风传输，也不自动安装系统驱动

### Requirement: Reject audio from an unselected sender
系统 MUST 拒绝未持有当前麦克风发送权、已退出或属于旧会话的设备音频，即使该设备仍保持控制连接。

#### Scenario: The old phone sends a late packet
- **WHEN** 麦克风已由 A 切换到 B，而电脑收到 A 的旧音频包
- **THEN** 该音频不写入虚拟输入，B 的会话继续正常运行

### Requirement: Microphone interruption is visible
系统 SHALL 在手机断线、录音权限撤销或虚拟设备失效时终止受影响的麦克风流并显示原因，MUST 不持续输出过期语音或显示仍在正常采集。

#### Scenario: Selected phone disconnects
- **WHEN** 麦克风发送端离线
- **THEN** 电脑停止写入该流，显示发送端断线，另一台 Android 不会未经用户选择自动成为麦克风
