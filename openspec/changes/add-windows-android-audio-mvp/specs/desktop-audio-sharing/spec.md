# Spec Delta

## Purpose

使用户能够把所选 Windows 输出设备正在播放的系统声音传给两台 Android，并保证每个接收端独立开始、停止和恢复，便于在现有手机和平板上验证电脑音频共享。

## ADDED Requirements

### Requirement: Selected desktop audio source
系统 SHALL 允许用户选择 Windows 输出设备作为系统声音来源，并将该设备实际播放的声音传到已授权接收端；音源不可用时 MUST 显示明确错误。

#### Scenario: Capture the selected output
- **WHEN** 用户在所选 Windows 输出设备播放可采集的测试音频并启动收听模式
- **THEN** 已加入该会话的 Android 播放对应音频，界面显示当前音源

#### Scenario: Selected output is unavailable
- **WHEN** 所选输出设备无法打开或已经移除
- **THEN** 系统显示音源错误并停止受影响的音频流，不持续显示正常播放

### Requirement: Two independent listeners
系统 SHALL 在收听模式同时向两台已配对 Android 提供同一音源；每个接收端 MUST 拥有独立连接、缓冲和状态。

#### Scenario: Two devices listen together
- **WHEN** 小米 17 Pro Max 和联想 Y700 四代均连接当前 Windows 11 电脑并加入收听模式
- **THEN** 两台设备同时播放同一电脑音源，电脑分别显示两台设备的连接和播放状态

#### Scenario: One listener loses its connection
- **WHEN** 设备 A 断线或无法及时消费音频，设备 B 的链路正常
- **THEN** 系统单独处理 A 的故障，B 持续播放，A 的积压不使 B 的排队时间增加

### Requirement: Playback through the Android output route
系统 SHALL 通过 Android 当前可用的系统音频输出路由播放接收到的声音，并在输出变化时恢复播放或显示需要重新启动的状态。

#### Scenario: Wired headphones replace the speaker
- **WHEN** Android 正在收听且用户连接有线耳机
- **THEN** 声音按 Android 的输出路由切换，应用正确显示流状态，不保留无法使用的旧音频设备

### Requirement: Silence and resumed audio preserve the timeline
系统 SHALL 正确处理音源静音或暂时无数据，并在声音恢复后继续播放当前音频，MUST 不重放静音前积压的旧数据。

#### Scenario: Pause and resume the desktop player
- **WHEN** 电脑播放器暂停后再次播放
- **THEN** 两台接收端恢复播放新的内容，时间戳连续性或会话重置得到正确处理，队列不会持续增长
