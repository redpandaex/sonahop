# Spec Delta

## Purpose

定义电脑与已连接 Android 设备之间的音频会话控制行为，让用户能够明确选择收听或麦克风模式、切换发送设备，并在停止、失败或连接变化时获得一致且可验证的状态。

## ADDED Requirements

### Requirement: Exclusive audio modes
系统 SHALL 提供收听、麦克风和停止三种可见状态；收听流与麦克风流 MUST 互斥，切换到麦克风模式时暂停所有收听流。

#### Scenario: Switch from listening to microphone
- **WHEN** 两台 Android 正在收听，用户为其中一台完成麦克风启动条件并切换到麦克风模式
- **THEN** 两台设备的收听流都停止输出，只有所选设备向电脑发送麦克风音频，电脑和手机显示一致的模式

#### Scenario: Return to listening
- **WHEN** 用户在麦克风模式中明确启动收听
- **THEN** 系统先停止麦克风发送与电脑输入流，再启动所选已连接设备的收听流

### Requirement: Single microphone sender
系统 SHALL 同时只允许一台已配对 Android 作为麦克风发送端，且 MUST 显示当前发送设备，拒绝未经切换流程的第二个发送者。

#### Scenario: A second phone requests microphone ownership
- **WHEN** 设备 A 正在提供麦克风，设备 B 请求成为发送端
- **THEN** 系统要求明确的设备切换操作，在切换完成前只接受 A 的音频

### Requirement: Predictable mode transition failure
系统 SHALL 在停止现有模式前检查新模式的前置条件；前置条件不满足时保持现有模式并说明原因，现有模式已停止后若启动失败则进入停止状态并说明故障。

#### Scenario: Virtual input is missing
- **WHEN** 收听模式运行中，用户启动麦克风模式，但电脑没有配置有效虚拟输入通路
- **THEN** 系统保持收听模式，提示需要配置虚拟设备，不显示麦克风正在工作

#### Scenario: New stream fails after the old stream stopped
- **WHEN** 旧模式已停止，而新音频流打开失败
- **THEN** 系统进入停止状态并显示错误，不留下旧模式或新模式的活动音频流

### Requirement: Explicit and repeatable stop
系统 SHALL 允许用户随时停止共享；重复停止操作 MUST 安全且结果一致，停止后不得继续采集或播放该会话的旧音频。

#### Scenario: Stop twice and start again
- **WHEN** 用户连续执行两次停止，然后启动新的会话
- **THEN** 已停止会话没有继续占用音频设备，新会话正常启动，迟到的旧音频包不会进入新会话
