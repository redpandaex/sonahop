use crate::interfaces::{self, LanInterface};
use crate::protocol::{self, Discovery, MAX_UDP, VERSION};
use crate::{CONTROL_PORT, DISCOVERY_PORT, DiscoveredHost, Inner, current, fail_service};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::{Duration, MissedTickBehavior, interval};

async fn cancelled(mut cancel: watch::Receiver<bool>) {
    if *cancel.borrow() {
        return;
    }
    while cancel.changed().await.is_ok() {
        if *cancel.borrow() {
            return;
        }
    }
}
fn transient_datagram_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::Interrupted
            | std::io::ErrorKind::WouldBlock
    )
}

fn publish_interfaces(inner: &Inner, generation: u64, interfaces: &[LanInterface]) {
    let addresses = interfaces
        .iter()
        .map(|i| i.ip.to_string())
        .collect::<Vec<_>>();
    {
        let mut state = inner.lock();
        if !current(&state, generation) {
            return;
        }
        state.snapshot.local_addresses = addresses.clone();
    }
    inner.diagnostics.log("discovery-interfaces",serde_json::json!({"addresses":addresses,"interfaces":interfaces.iter().map(|i|serde_json::json!({"name":i.name,"ip":i.ip,"broadcast":i.broadcast,"index":i.index})).collect::<Vec<_>>()}));
}
async fn sockets(inner: &Inner, generation: u64) -> Vec<(LanInterface, Arc<UdpSocket>)> {
    let interfaces = interfaces::local_interfaces();
    publish_interfaces(inner, generation, &interfaces);
    let mut sockets = Vec::new();
    for interface in interfaces {
        match interfaces::udp_for(&interface).await {
            Ok(socket) => sockets.push((interface, Arc::new(socket))),
            Err(error) => inner.diagnostics.log(
                "discovery-interface-failed",
                serde_json::json!({"ip":interface.ip,"reason":error.to_string()}),
            ),
        }
    }
    sockets
}
pub(crate) async fn host(
    inner: Arc<Inner>,
    generation: u64,
    udp: UdpSocket,
    cancel: watch::Receiver<bool>,
) {
    let id = uuid::Uuid::new_v4().to_string();
    let mut outbound = sockets(&inner, generation).await;
    let mut refresh = interval(Duration::from_secs(10));
    refresh.set_missed_tick_behavior(MissedTickBehavior::Skip);
    refresh.tick().await;
    let mut buffer = [0; 65536];
    let mut window = Instant::now();
    let mut count = 0_u16;
    let (mut probes, mut replies) = (0_u64, 0_u64);
    loop {
        tokio::select! {
            _=cancelled(cancel.clone())=>return,
            _=refresh.tick()=>{
                let latest=interfaces::local_interfaces();
                if latest!=outbound.iter().map(|(i,_)|i.clone()).collect::<Vec<_>>(){outbound=sockets(&inner,generation).await;}
                inner.diagnostics.log("discovery-statistics",serde_json::json!({"role":"host","probesReceived":probes,"repliesSent":replies}));
            },
            received=udp.recv_from(&mut buffer)=>{
                let (size,source)=match received {Ok(v)=>v,Err(e) if transient_datagram_error(&e)=>continue,Err(e)=>{inner.diagnostics.log("discovery-receiver-failed",serde_json::json!({"role":"host","reason":e.to_string()}));fail_service(&inner,generation,format!("局域网发现接收失败：{e}"));return;}};
                let SocketAddr::V4(source4)=source else{continue;};
                if size>MAX_UDP||!protocol::local_ip(*source4.ip()){continue;}
                let Ok(Discovery::Probe {version:VERSION,nonce})=serde_json::from_slice(&buffer[..size]) else{continue;};
                if !protocol::valid_id(&nonce){continue;}
                if window.elapsed()>=Duration::from_secs(1){window=Instant::now();count=0;}
                if count>=32{continue;}count+=1;probes+=1;
                let name={let state=inner.lock();if !current(&state,generation){return;}state.snapshot.local_name.clone()};
                let reply=Discovery::Reply {version:VERSION,nonce,id:id.clone(),name,port:CONTROL_PORT};
                let Ok(payload)=serde_json::to_vec(&reply)else{continue;};
                let socket=outbound.iter().filter(|(i,_)|interfaces::same_subnet(i,*source4.ip())).max_by_key(|(i,_)|u32::from(i.mask).count_ones()).map(|(_,s)|s.as_ref()).unwrap_or(&udp);
                let sent=tokio::select!{_=cancelled(cancel.clone())=>return,result=socket.send_to(&payload,source)=>result};
                if sent.is_ok(){replies+=1;}else{inner.diagnostics.log("discovery-reply-failed",serde_json::json!({"target":source.to_string(),"reason":sent.err().map(|e|e.to_string())}));}
            },
        }
    }
}
fn receivers(
    sockets: &[(LanInterface, Arc<UdpSocket>)],
    fallback: Arc<UdpSocket>,
    tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
) -> JoinSet<Result<(), String>> {
    let mut tasks = JoinSet::new();
    for socket in sockets
        .iter()
        .map(|(_, s)| s.clone())
        .chain(std::iter::once(fallback))
    {
        let tx = tx.clone();
        tasks.spawn(async move {
            let mut buffer = [0; 65536];
            loop {
                let (size, source) = match socket.recv_from(&mut buffer).await {
                    Ok(received) => received,
                    Err(error) if transient_datagram_error(&error) => continue,
                    Err(error) => return Err(format!("局域网发现接收失败：{error}")),
                };
                if size <= MAX_UDP {
                    let _ = tx.try_send((buffer[..size].to_vec(), source));
                }
            }
        });
    }
    tasks
}
pub(crate) async fn client(
    inner: Arc<Inner>,
    generation: u64,
    udp: UdpSocket,
    cancel: watch::Receiver<bool>,
) {
    let nonce = uuid::Uuid::new_v4().to_string();
    let Ok(probe) = serde_json::to_vec(&Discovery::Probe {
        version: VERSION,
        nonce: nonce.clone(),
    }) else {
        return;
    };
    let fallback = Arc::new(udp);
    let mut paths = sockets(&inner, generation).await;
    let (tx, mut rx) = mpsc::channel(64);
    let mut tasks = receivers(&paths, fallback.clone(), tx.clone());
    let mut probes = interval(Duration::from_secs(3));
    probes.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut refresh = interval(Duration::from_secs(10));
    refresh.set_missed_tick_behavior(MissedTickBehavior::Skip);
    refresh.tick().await;
    let (mut sent, mut replies) = (0_u64, 0_u64);
    loop {
        tokio::select! {
            _=cancelled(cancel.clone())=>return,
            completed=tasks.join_next()=>{
                let reason=match completed {
                    Some(Ok(Err(reason)))=>reason,
                    Some(Err(error))=>format!("局域网发现接收任务结束：{error}"),
                    _=>"局域网发现接收任务意外结束".to_owned(),
                };
                inner.diagnostics.log("discovery-receiver-failed",serde_json::json!({"role":"client","reason":reason}));
                fail_service(&inner,generation,reason);return;
            },
            _=refresh.tick()=>{
                let latest=interfaces::local_interfaces();
                if latest!=paths.iter().map(|(i,_)|i.clone()).collect::<Vec<_>>() {
                    tasks.abort_all();while tasks.join_next().await.is_some(){}
                    paths=sockets(&inner,generation).await;tasks=receivers(&paths,fallback.clone(),tx.clone());
                }
                {let mut state=inner.lock();if !current(&state,generation){return;}state.discovered.retain(|_,(_,seen)|seen.elapsed()<Duration::from_secs(10));update(&mut state);}
                inner.diagnostics.log("discovery-statistics",serde_json::json!({"role":"client","probesSent":sent,"repliesReceived":replies}));
            },
            _=probes.tick()=>{
                if paths.is_empty() {
                    let result=tokio::select!{_=cancelled(cancel.clone())=>return,result=fallback.send_to(&probe,SocketAddrV4::new(Ipv4Addr::BROADCAST,DISCOVERY_PORT))=>result};
                    if result.is_ok(){sent+=1;}else{inner.diagnostics.log("discovery-probe-failed",serde_json::json!({"target":Ipv4Addr::BROADCAST,"reason":result.err().map(|e|e.to_string())}));}
                }else{
                    for (interface,socket) in &paths {
                        let result=tokio::select!{_=cancelled(cancel.clone())=>return,result=socket.send_to(&probe,SocketAddrV4::new(interface.broadcast,DISCOVERY_PORT))=>result};
                        if result.is_ok(){sent+=1;}else{inner.diagnostics.log("discovery-probe-failed",serde_json::json!({"ip":interface.ip,"target":interface.broadcast,"reason":result.err().map(|e|e.to_string())}));}
                    }
                }
            },
            received=rx.recv()=>{
                let Some((bytes,source))=received else{return;};
                let SocketAddr::V4(source)=source else{continue;};
                if !protocol::local_ip(*source.ip()){continue;}
                let Ok(Discovery::Reply {version:VERSION,nonce:reply_nonce,id,name,port})=serde_json::from_slice(&bytes)else{continue;};
                if reply_nonce!=nonce||!protocol::valid_id(&id)||!protocol::valid_name(&name)||port==0{continue;}
                replies+=1;
                let mut state=inner.lock();if !current(&state,generation){return;}
                state.discovered.retain(|_,(_,seen)|seen.elapsed()<Duration::from_secs(10));
                if state.discovered.len()<16||state.discovered.contains_key(&id) {
                    state.discovered.insert(id.clone(),(DiscoveredHost {id,name,address:source.ip().to_string(),port},Instant::now()));update(&mut state);
                }
            },
        }
    }
}
fn update(state: &mut crate::State) {
    let mut hosts = state
        .discovered
        .values()
        .map(|(host, _)| host.clone())
        .collect::<Vec<_>>();
    hosts.sort_by(|a, b| a.name.cmp(&b.name).then(a.address.cmp(&b.address)));
    state.snapshot.discovered = hosts;
}
