# 局域网发现与连接

用户希望当前 Windows 和 Y700 能找到并连接对方；本阶段交付真实控制连接，暂无音频。采用 Rust 共用连接库，Tauri 只组合平台入口，React 展示真实状态。

用户按最新约定自行运行测试/构建/真机验证。本轮不运行这些检查，也不改 Windows 防火墙或 Android 系统设置，不采集声音。

## 流程
- Windows 用户主动启用发现，绑定 UDP 32992 和 TCP 32993，显示本机名、连接请求及已连接设备；停止即关闭任务/连接。
- Android 用户主动搜索，通过随机 UDP 端口广播探测，Windows向源地址单播回应；客户端按随机nonce过滤回应，10秒未刷新即移除，列表最多16台。
- Android 点击发现项或手动输入局域网 IPv4:port 发起 TCP；Windows显示名称、IP和6位核对编号，手机显示相同编号，Windows确认后才进入connected。待确认60秒，最多4个等待请求、2个控制连接。
- 连接保持真实的3秒心跳、12秒超时；拒绝、过期、断开、停止和错误均由后台状态反映，浏览器不得伪造连接。
- 本阶段控制帧不承载音频、文件或敏感数据，仅用于受控局域网的显式连接。没有持久信任或自动重连。音频接入前必须补认证/加密设计；当前不宣称安全配对。
- 控制帧JSON采用4字节大端长度前缀，长度上限4096；UDP上限1024、名称上限64字符、请求有超时和容量限制。只接受私有/链路本地IPv4（开发允许loopback），不接受DNS、全零、多播/广播目标；网络探测本身不做子网扫描。

## 平台
Android 16默认LAN由INTERNET授权，Android17/target37要求ACCESS_LOCAL_NETWORK；原生小插件只在系统API>=37时申请LAN权限并正确反馈拒绝，在旧系统不申请无关权限。Android16开发者自行开启兼容限制时，返回权限/网络错误，指引在设备设置处理，不自动改兼容开关。
依据：https://developer.android.com/privacy-and-security/local-network-permission
客户端只发UDP广播、接收单播，没有多播接收锁或后台音频服务。行为在Y700上仍需用户验证。

## 接口
LinkSnapshot { role:"host"|"client", running:boolean, localName:string, port:number, discovered:DiscoveredHost[], requests:LinkRequest[], peers:LinkPeer[], clientState:"idle"|"discovering"|"connecting"|"awaitingApproval"|"connected", verificationCode:string|null, lastError:string|null }
DiscoveredHost { id:string, name:string, address:string, port:number }
LinkRequest { id:string, name:string, address:string, verificationCode:string }
LinkPeer { id:string, name:string, address:string }

Tauri commands: get_link_snapshot(); start_link(name:string); stop_link(); connect_link(address:string,port:number); answer_link_request(requestId:string,approve:boolean); disconnect_link_peer(peerId:string)。状态通过get_link_snapshot每秒读取，启动/连接只由用户动作触发。

Rust库：LinkRole::Host|Client；LinkService::new(role); snapshot(&self)->LinkSnapshot; async start(&self,name:String)->Result<LinkSnapshot,String>; async stop(&self); async connect(&self,address:String,port:u16)->Result<(),String>; answer_request(&self,id:&str,approve:bool)->Result<(),String>; disconnect_peer(&self,id:&str)->Result<(),String>。

UI保留音量/模式偏好与音频按钮禁用；更改为连接阶段说明，已连接只代表控制链路，不代表正在收听。原有手机地址保存字段用于手动连接，Windows刷新播放设备与发现服务互不混淆。