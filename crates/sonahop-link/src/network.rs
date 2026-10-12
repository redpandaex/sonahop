use crate::protocol::{self, Discovery, Frame, FrameReader, MAX_UDP, Payload, VERSION};
use crate::{
    CONTROL_PORT, ClientState, Command, DISCOVERY_PORT, DiscoveredHost, Inner, LinkEvent, LinkPeer,
    LinkRequest, LinkRole, MAX_MEDIA_BYTES, Outbound, channels, current, fail_service,
    finish_connection, publish_connected, publish_payload, spawn,
};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::watch;
use tokio::time::{Duration, MissedTickBehavior, interval, sleep_until, timeout};

const MAX_PENDING: usize = 4;
const MAX_PEERS: usize = 2;
const DISCOVERY_TTL: Duration = Duration::from_secs(10);
const APPROVAL_TTL: Duration = Duration::from_secs(60);

pub(crate) struct Sockets {
    udp: UdpSocket,
    listener: Option<TcpListener>,
}

pub(crate) async fn bind(role: LinkRole) -> Result<Sockets, String> {
    let port = if role == LinkRole::Host {
        DISCOVERY_PORT
    } else {
        0
    };
    let udp = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))
        .await
        .map_err(|error| format!("无法启动局域网发现：{error}"))?;
    if role == LinkRole::Client {
        udp.set_broadcast(true)
            .map_err(|error| format!("无法启用局域网广播：{error}"))?;
    }
    let listener = if role == LinkRole::Host {
        Some(
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CONTROL_PORT))
                .await
                .map_err(|error| format!("无法监听控制端口 {CONTROL_PORT}：{error}"))?,
        )
    } else {
        None
    };
    Ok(Sockets { udp, listener })
}

pub(crate) fn launch(
    inner: &Arc<Inner>,
    generation: u64,
    sockets: Sockets,
    cancel: watch::Receiver<bool>,
) {
    let Sockets { udp, listener } = sockets;
    if let Some(listener) = listener {
        let task_inner = inner.clone();
        let task_cancel = cancel.clone();
        spawn(inner, generation, async move {
            host_discovery(task_inner, generation, udp, task_cancel).await;
        });
        let task_inner = inner.clone();
        spawn(inner, generation, async move {
            accept_connections(task_inner, generation, listener, cancel).await;
        });
    } else {
        let task_inner = inner.clone();
        spawn(inner, generation, async move {
            client_discovery(task_inner, generation, udp, cancel).await;
        });
    }
}

async fn cancelled(mut receiver: watch::Receiver<bool>) {
    if *receiver.borrow() {
        return;
    }
    while receiver.changed().await.is_ok() {
        if *receiver.borrow_and_update() {
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

fn source_ip(address: SocketAddr) -> Option<Ipv4Addr> {
    match address {
        SocketAddr::V4(address) if protocol::local_ip(*address.ip()) => Some(*address.ip()),
        _ => None,
    }
}

async fn host_discovery(
    inner: Arc<Inner>,
    generation: u64,
    udp: UdpSocket,
    cancel: watch::Receiver<bool>,
) {
    let host_id = uuid::Uuid::new_v4().to_string();
    // Fixed receive storage also handles Windows oversized datagrams without a socket error.
    let mut buffer = [0; 65536];
    let mut rate_window = Instant::now();
    let mut replies = 0;
    loop {
        let received = tokio::select! {
            _ = cancelled(cancel.clone()) => return,
            received = udp.recv_from(&mut buffer) => received,
        };
        let (length, source) = match received {
            Ok(value) => value,
            Err(error) if transient_datagram_error(&error) => continue,
            Err(error) => {
                fail_service(&inner, generation, format!("局域网发现接收失败：{error}"));
                return;
            }
        };
        if length > MAX_UDP || source_ip(source).is_none() {
            continue;
        }
        let Ok(Discovery::Probe {
            version: VERSION,
            nonce,
        }) = serde_json::from_slice(&buffer[..length])
        else {
            continue;
        };
        if !protocol::valid_id(&nonce) {
            continue;
        }
        if rate_window.elapsed() >= Duration::from_secs(1) {
            rate_window = Instant::now();
            replies = 0;
        }
        if replies >= 32 {
            continue;
        }
        replies += 1;
        let name = {
            let state = inner.lock();
            if !current(&state, generation) {
                return;
            }
            state.snapshot.local_name.clone()
        };
        let reply = Discovery::Reply {
            version: VERSION,
            nonce,
            id: host_id.clone(),
            name,
            port: CONTROL_PORT,
        };
        if let Ok(payload) = serde_json::to_vec(&reply) {
            // A single unreachable probe sender must not take down the host service.
            tokio::select! {
                _ = cancelled(cancel.clone()) => return,
                _ = udp.send_to(&payload, source) => {},
            }
        }
    }
}

async fn client_discovery(
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
    let target = SocketAddrV4::new(Ipv4Addr::BROADCAST, DISCOVERY_PORT);
    let mut probes = interval(Duration::from_secs(3));
    let mut cleanup = interval(Duration::from_secs(1));
    probes.set_missed_tick_behavior(MissedTickBehavior::Skip);
    cleanup.set_missed_tick_behavior(MissedTickBehavior::Skip);
    // Fixed receive storage also handles Windows oversized datagrams without a socket error.
    let mut buffer = [0; 65536];
    loop {
        tokio::select! {
            _ = cancelled(cancel.clone()) => return,
            _ = probes.tick() => {
                if let Err(error) = udp.send_to(&probe, target).await {
                    fail_service(&inner, generation, format!("局域网广播失败，请检查网络或本地网络权限：{error}"));
                    return;
                }
            }
            _ = cleanup.tick() => {
                let mut state = inner.lock();
                if !current(&state, generation) { return; }
                state.discovered.retain(|_, (_, seen)| seen.elapsed() < DISCOVERY_TTL);
                update_discovered(&mut state);
            }
            received = udp.recv_from(&mut buffer) => {
                let (length, source) = match received {
                    Ok(value) => value,
                    Err(error) if transient_datagram_error(&error) => continue,
                    Err(error) => {
                        fail_service(&inner, generation, format!("局域网搜索接收失败：{error}"));
                        return;
                    }
                };
                let Some(address) = source_ip(source) else { continue; };
                if length > MAX_UDP { continue; }
                let Ok(Discovery::Reply { version: VERSION, nonce: response_nonce, id, name, port }) =
                    serde_json::from_slice(&buffer[..length]) else { continue; };
                if response_nonce != nonce || !protocol::valid_id(&id) || !protocol::valid_name(&name) || port == 0 {
                    continue;
                }
                let mut state = inner.lock();
                if !current(&state, generation) { return; }
                state.discovered.retain(|_, (_, seen)| seen.elapsed() < DISCOVERY_TTL);
                if state.discovered.len() < 16 || state.discovered.contains_key(&id) {
                    state.discovered.insert(id.clone(), (DiscoveredHost {
                        id, name, address: address.to_string(), port,
                    }, Instant::now()));
                    update_discovered(&mut state);
                }
            }
        }
    }
}

fn update_discovered(state: &mut crate::State) {
    let mut hosts: Vec<_> = state
        .discovered
        .values()
        .map(|(host, _)| host.clone())
        .collect();
    hosts.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    state.snapshot.discovered = hosts;
}

async fn accept_connections(
    inner: Arc<Inner>,
    generation: u64,
    listener: TcpListener,
    cancel: watch::Receiver<bool>,
) {
    // Bound online guesses globally and per source; no long-lived trust or unbounded IP map.
    let mut auth_window = Instant::now();
    let mut attempts = std::collections::HashMap::<Ipv4Addr, u8>::new();
    let mut total_attempts = 0_u8;
    loop {
        let accepted = tokio::select! {
            _ = cancelled(cancel.clone()) => return,
            accepted = listener.accept() => accepted,
        };
        let (stream, source) = match accepted {
            Ok(value) => value,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::Interrupted
                ) =>
            {
                continue;
            }
            Err(error) => {
                fail_service(&inner, generation, format!("控制连接监听失败：{error}"));
                return;
            }
        };
        let Some(address) = source_ip(source) else {
            continue;
        };
        if auth_window.elapsed() >= Duration::from_secs(60) {
            auth_window = Instant::now();
            attempts.clear();
            total_attempts = 0;
        }
        let source_attempts = attempts.get(&address).copied().unwrap_or(0);
        if total_attempts >= 20
            || source_attempts >= 5
            || (attempts.len() >= 32 && source_attempts == 0)
        {
            // Close without starting another cryptographic task; clients report the same generic auth failure.
            continue;
        }
        total_attempts += 1;
        attempts.insert(address, source_attempts + 1);
        let id = uuid::Uuid::new_v4().to_string();
        let (control, outbound) = channels();
        let (name, pin) = {
            let mut state = inner.lock();
            if !current(&state, generation) {
                return;
            }
            if state
                .controls
                .values()
                .filter(|control| !control.connected)
                .count()
                >= MAX_PENDING
            {
                continue;
            }
            let Some(pin) = state.snapshot.pairing_code.clone() else {
                return;
            };
            state.controls.insert(id.clone(), control);
            (state.snapshot.local_name.clone(), pin)
        };
        let task_inner = inner.clone();
        let task_cancel = cancel.clone();
        spawn(&inner, generation, async move {
            tokio::select! {
                _ = cancelled(task_cancel) => {},
                _ = host_session(task_inner.clone(), generation, &id, &name, pin, address, stream, outbound) => {},
            }
            finish_connection(&task_inner, generation, &id, None);
        });
    }
}

async fn encrypted_write<W: AsyncWrite + Unpin>(
    writer: &mut W,
    secure: &mut crate::crypto::SecureChannel,
    payload: &Payload,
) -> Result<(), String> {
    let frame = secure.seal(payload)?;
    protocol::write(writer, &frame).await
}

async fn host_handshake<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    id: &str,
    host_name: &str,
    pin: String,
    reader: &mut FrameReader<R>,
    writer: &mut W,
) -> Result<(crate::crypto::SecureChannel, String), String> {
    let hello = reader.read().await?;
    let Frame::Hello {
        version: VERSION,
        name,
        spake,
    } = hello
    else {
        return Err(crate::crypto::AUTH_ERROR.into());
    };
    if !protocol::valid_name(&name) {
        return Err(crate::crypto::AUTH_ERROR.into());
    }
    let client_message = crate::crypto::decode(&spake, 64)?;
    let (state, host_message) = crate::crypto::host_start(&pin);
    drop(pin);
    protocol::write(
        writer,
        &Frame::Auth {
            version: VERSION,
            id: id.to_owned(),
            host_name: host_name.to_owned(),
            spake: crate::crypto::encode(&host_message),
        },
    )
    .await?;
    let mut secure = crate::crypto::SecureChannel::establish(
        state,
        &client_message,
        &host_message,
        id,
        &name,
        host_name,
        true,
    )?;
    match secure.open(reader.read().await?)? {
        Payload::Confirm { id: confirmed_id } if confirmed_id == id => Ok((secure, name)),
        _ => Err(crate::crypto::AUTH_ERROR.into()),
    }
}

async fn host_session(
    inner: Arc<Inner>,
    generation: u64,
    id: &str,
    host_name: &str,
    pin: String,
    address: Ipv4Addr,
    stream: TcpStream,
    mut outbound: Outbound,
) -> Result<(), String> {
    configure_socket(&stream)?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = FrameReader::new(reader);
    let authentication = timeout(
        Duration::from_secs(10),
        host_handshake(id, host_name, pin, &mut reader, &mut writer),
    )
    .await;
    let (mut secure, name) = match authentication {
        Ok(Ok(authenticated)) => authenticated,
        _ => {
            let _ = protocol::write(
                &mut writer,
                &Frame::Rejected {
                    version: VERSION,
                    reason: crate::crypto::AUTH_ERROR.into(),
                },
            )
            .await;
            return Ok(());
        }
    };
    let verification_code = secure.verification_code.clone();
    let pending_since = tokio::time::Instant::now();
    {
        let mut state = inner.lock();
        if !current(&state, generation) {
            return Ok(());
        }
        let Some(control) = state.controls.get_mut(id) else {
            return Ok(());
        };
        control.approval_deadline = Some(Instant::now() + APPROVAL_TTL);
        state.snapshot.requests.push(LinkRequest {
            id: id.to_owned(),
            name: name.clone(),
            address: address.to_string(),
            verification_code: verification_code.clone(),
        });
    }
    encrypted_write(
        &mut writer,
        &mut secure,
        &Payload::Pending {
            id: id.to_owned(),
            host_name: host_name.to_owned(),
            verification_code,
        },
    )
    .await?;
    let approve = tokio::select! {
        _ = sleep_until(pending_since + APPROVAL_TTL) => {
            encrypted_write(&mut writer, &mut secure, &Payload::Rejected { reason: "连接请求已超过 60 秒".into() }).await?;
            return Ok(());
        }
        command = outbound.commands.recv() => match command {
            Some(Command::Answer(approve)) => approve,
            _ => return Ok(()),
        },
        frame = reader.read() => match secure.open(frame?)? {
            Payload::Bye => return Ok(()),
            _ => return Err("确认前收到了非预期控制帧".into()),
        },
    };
    let rejection = if pending_since.elapsed() >= APPROVAL_TTL {
        Some("连接请求已超过 60 秒")
    } else if !approve {
        Some("Windows 已拒绝连接请求")
    } else {
        None
    };
    if let Some(reason) = rejection {
        encrypted_write(
            &mut writer,
            &mut secure,
            &Payload::Rejected {
                reason: reason.into(),
            },
        )
        .await?;
        return Ok(());
    }
    let capacity = {
        let mut state = inner.lock();
        if !current(&state, generation) {
            return Ok(());
        }
        if state
            .controls
            .values()
            .filter(|control| control.connected)
            .count()
            >= MAX_PEERS
        {
            false
        } else if let Some(control) = state.controls.get_mut(id) {
            // Reserve capacity before the acceptance write; send APIs also require a published peer.
            control.connected = true;
            control.approval_deadline = None;
            true
        } else {
            return Ok(());
        }
    };
    if !capacity {
        encrypted_write(
            &mut writer,
            &mut secure,
            &Payload::Rejected {
                reason: "Windows 最多允许两台设备连接".into(),
            },
        )
        .await?;
        return Ok(());
    }
    encrypted_write(
        &mut writer,
        &mut secure,
        &Payload::Accepted {
            id: id.to_owned(),
            host_name: host_name.to_owned(),
        },
    )
    .await?;
    if !publish_connected(
        &inner,
        generation,
        LinkPeer {
            id: id.to_owned(),
            name,
            address: address.to_string(),
        },
    ) {
        return Ok(());
    }
    connected(
        &inner,
        generation,
        id,
        &mut reader,
        &mut writer,
        &mut secure,
        &mut outbound,
    )
    .await
}

pub(crate) async fn client_connection(
    inner: Arc<Inner>,
    generation: u64,
    id: String,
    name: String,
    address: SocketAddrV4,
    pin: String,
    outbound: Outbound,
    cancel: watch::Receiver<bool>,
) {
    let result = tokio::select! {
        _ = cancelled(cancel) => Ok(()),
        result = client_session(inner.clone(), generation, &id, &name, address, pin, outbound) => result,
    };
    finish_connection(&inner, generation, &id, result.err());
}

async fn client_session(
    inner: Arc<Inner>,
    generation: u64,
    id: &str,
    name: &str,
    address: SocketAddrV4,
    pin: String,
    mut outbound: Outbound,
) -> Result<(), String> {
    let stream = timeout(Duration::from_secs(5), TcpStream::connect(address))
        .await
        .map_err(|_| "连接 Windows 超时，请检查地址与防火墙")?
        .map_err(|error| format!("无法连接 Windows：{error}"))?;
    configure_socket(&stream)?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = FrameReader::new(reader);
    let (state, client_message) = crate::crypto::client_start(&pin);
    drop(pin);
    protocol::write(
        &mut writer,
        &Frame::Hello {
            version: VERSION,
            name: name.to_owned(),
            spake: crate::crypto::encode(&client_message),
        },
    )
    .await
    .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?;
    let response = timeout(Duration::from_secs(10), reader.read())
        .await
        .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?
        .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?;
    let (remote_id, host_name, host_message) = match response {
        Frame::Auth {
            version: VERSION,
            id,
            host_name,
            spake,
        } if protocol::valid_id(&id) && protocol::valid_name(&host_name) => {
            (id, host_name, crate::crypto::decode(&spake, 64)?)
        }
        _ => return Err(crate::crypto::AUTH_ERROR.into()),
    };
    let mut secure = crate::crypto::SecureChannel::establish(
        state,
        &client_message,
        &host_message,
        &remote_id,
        name,
        &host_name,
        false,
    )?;
    encrypted_write(
        &mut writer,
        &mut secure,
        &Payload::Confirm {
            id: remote_id.clone(),
        },
    )
    .await
    .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?;
    let pending_frame = timeout(Duration::from_secs(10), reader.read())
        .await
        .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?
        .map_err(|_| crate::crypto::AUTH_ERROR.to_owned())?;
    let pending = secure.open(pending_frame)?;
    let verification_code = match pending {
        Payload::Pending {
            id,
            host_name: pending_name,
            verification_code,
        } if id == remote_id
            && pending_name == host_name
            && verification_code == secure.verification_code =>
        {
            verification_code
        }
        _ => return Err(crate::crypto::AUTH_ERROR.into()),
    };
    {
        let mut state = inner.lock();
        if !current(&state, generation) {
            return Ok(());
        }
        state.snapshot.client_state = ClientState::AwaitingApproval;
        state.snapshot.verification_code = Some(verification_code);
    }
    let frame = timeout(APPROVAL_TTL + Duration::from_secs(1), reader.read())
        .await
        .map_err(|_| "Windows 确认已超时，请重新连接")??;
    match secure.open(frame)? {
        Payload::Accepted {
            id: accepted_id,
            host_name: accepted_name,
        } if accepted_id == remote_id && accepted_name == host_name => {}
        Payload::Rejected { reason } => return Err(display_reason(&reason)),
        _ => return Err("Windows 返回了非预期确认消息".into()),
    }
    if !publish_connected(
        &inner,
        generation,
        LinkPeer {
            id: id.to_owned(),
            name: host_name,
            address: address.ip().to_string(),
        },
    ) {
        return Ok(());
    }
    connected(
        &inner,
        generation,
        id,
        &mut reader,
        &mut writer,
        &mut secure,
        &mut outbound,
    )
    .await
}

fn display_reason(reason: &str) -> String {
    let reason: String = reason
        .chars()
        .filter(|character| !character.is_control())
        .take(128)
        .collect();
    if reason.is_empty() {
        "Windows 拒绝了连接请求".into()
    } else {
        reason
    }
}

async fn connected<R: AsyncRead + Unpin, W: AsyncWrite + Unpin>(
    inner: &Inner,
    generation: u64,
    id: &str,
    reader: &mut FrameReader<R>,
    writer: &mut W,
    secure: &mut crate::crypto::SecureChannel,
    outbound: &mut Outbound,
) -> Result<(), String> {
    let mut heartbeat = interval(Duration::from_secs(3));
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut last_received = Instant::now();
    let mut last_pong = None::<Instant>;
    let mut rate_window = Instant::now();
    let (mut controls, mut media, mut heartbeats) = (0_u16, 0_u16, 0_u16);
    loop {
        tokio::select! {
            biased;
            _ = sleep_until(tokio::time::Instant::from_std(last_received + Duration::from_secs(12))) => {
                return Err("控制连接心跳超时".into());
            }
            command = outbound.commands.recv() => match command {
                Some(Command::Disconnect) | None => {
                    let _ = encrypted_write(writer, secure, &Payload::Bye).await; return Ok(());
                }
                Some(Command::Answer(_)) => {},
            },
            _ = heartbeat.tick() => encrypted_write(writer, secure, &Payload::Ping).await?,
            control = outbound.controls.recv() => {
                let Some(payload) = control else { return Ok(()); };
                encrypted_write(writer, secure, &Payload::Control { payload }).await?;
            }
            frame = reader.read() => {
                let payload = secure.open(frame?)?;
                if rate_window.elapsed() >= Duration::from_secs(1) {
                    rate_window = Instant::now(); controls = 0; media = 0; heartbeats = 0;
                }
                if last_received.elapsed() >= Duration::from_millis(250) { last_received = Instant::now(); }
                match payload {
                    Payload::Ping => {
                        heartbeats += 1;
                        if heartbeats > 16 { return Err("心跳消息发送过于频繁".into()); }
                        if last_pong.is_none_or(|sent| sent.elapsed() >= Duration::from_millis(500)) {
                            encrypted_write(writer, secure, &Payload::Pong).await?; last_pong = Some(Instant::now());
                        }
                    }
                    Payload::Pong => {
                        heartbeats += 1;
                        if heartbeats > 16 { return Err("心跳消息发送过于频繁".into()); }
                    }
                    Payload::Bye => return Ok(()),
                    Payload::Control { payload } => {
                        controls += 1;
                        if controls > 32 { return Err("控制消息发送过于频繁".into()); }
                        protocol::validate_control(&payload)?;
                        publish_payload(inner, generation, id, LinkEvent::Control { peer_id: id.to_owned(), payload });
                    }
                    Payload::Media { payload } => {
                        media += 1;
                        if media > 128 { return Err("音频包发送过于频繁".into()); }
                        let payload = crate::crypto::decode(&payload, MAX_MEDIA_BYTES)?;
                        if payload.is_empty() { return Err("音频包为空".into()); }
                        publish_payload(inner, generation, id, LinkEvent::Media { peer_id: id.to_owned(), payload });
                    }
                    _ => return Err("非预期加密消息".into()),
                }
            }
            frame = outbound.media.recv() => {
                let Some(payload) = frame else { return Ok(()); };
                encrypted_write(writer, secure, &Payload::Media { payload: crate::crypto::encode(&payload) }).await?;
            }
        }
    }
}

fn configure_socket(stream: &TcpStream) -> Result<(), String> {
    stream
        .set_nodelay(true)
        .map_err(|error| error.to_string())?;
    let socket = socket2::SockRef::from(stream);
    socket
        .set_send_buffer_size(4096)
        .map_err(|error| format!("无法限制发送缓冲区：{error}"))?;
    socket
        .set_recv_buffer_size(8192)
        .map_err(|error| format!("无法限制接收缓冲区：{error}"))?;
    Ok(())
}
