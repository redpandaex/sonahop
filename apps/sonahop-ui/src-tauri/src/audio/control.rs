use super::workers::WorkerMode;
use super::{AudioControl, AudioLane, AudioService, MicRequest, Phase};
use sonahop_audio::{MediaPacket, StreamKind, parse_stream_id, stream_id_text};
use sonahop_link::{LinkEvent, LinkRole};

impl AudioService {
    pub(super) async fn handle_event(&self, event: LinkEvent) {
        match event {
            LinkEvent::Connected { peer_id } => {
                if !self
                    .shared
                    .link
                    .snapshot()
                    .peers
                    .iter()
                    .any(|p| p.id == peer_id)
                {
                    return;
                }
                if self.shared.lock().role == LinkRole::Host {
                    let _ = self.shared.send(
                        &peer_id,
                        AudioControl::Capabilities {
                            remote_microphone: cfg!(target_os = "windows"),
                            platform: std::env::consts::OS.into(),
                        },
                    );
                } else {
                    let mut state = self.shared.lock();
                    state.remote_microphone_supported = None;
                    state.remote_source = None;
                }
            }
            LinkEvent::Media { peer_id, payload } => {
                let Ok(packet) = MediaPacket::parse(&payload) else {
                    return;
                };
                let kind = packet.kind;
                let mut state = self.shared.lock();
                let expected = if state.role == LinkRole::Host {
                    kind == StreamKind::Microphone
                        && state.microphone.peer_id.as_deref() == Some(peer_id.as_str())
                } else {
                    kind == StreamKind::Listen && state.listeners.contains_key(&peer_id)
                };
                let lane = state.lane_mut(kind);
                if !expected
                    || lane.phase != Phase::Active
                    || lane.stream_id != Some(packet.stream_id)
                {
                    lane.dropped_packets += 1;
                    return;
                }
                if let Some(worker) = &lane.worker {
                    if worker.media.try_send(packet).is_ok() {
                        lane.received_packets += 1;
                    } else {
                        lane.dropped_packets += 1;
                    }
                }
            }
            LinkEvent::Disconnected { peer_id } => {
                let _lock = self.lifecycle.lock().await;
                let (listen, mic) = {
                    let mut state = self.shared.lock();
                    state.requests.retain(|r| r.peer_id != peer_id);
                    let removed = state.listeners.remove(&peer_id).is_some();
                    (
                        removed && (state.role == LinkRole::Client || state.listeners.is_empty()),
                        state.microphone.peer_id.as_deref() == Some(peer_id.as_str()),
                    )
                };
                for (kind, stop) in [(StreamKind::Listen, listen), (StreamKind::Microphone, mic)] {
                    if stop {
                        self.stop_lane_inner(kind, "设备连接已断开", false).await;
                        self.shared.lock().lane_mut(kind).error =
                            Some("设备连接已断开，请重新连接并手动开始".into());
                    }
                }
            }
            LinkEvent::Control { peer_id, payload } => {
                let Ok(control) = serde_json::from_value::<AudioControl>(payload) else {
                    return;
                };
                let _lock = self.lifecycle.lock().await;
                if !self
                    .shared
                    .link
                    .snapshot()
                    .peers
                    .iter()
                    .any(|p| p.id == peer_id)
                {
                    return;
                }
                let host = self.shared.lock().role == LinkRole::Host;
                if host {
                    self.host_control(peer_id, control).await;
                } else {
                    self.client_control(peer_id, control).await;
                }
            }
        }
    }

    async fn host_control(&self, peer: String, control: AudioControl) {
        match control {
            AudioControl::Subscribe { request_id } => {
                let Some(request) = request_id.filter(|r| parse_stream_id(r).is_ok()) else {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::error(
                            StreamKind::Listen,
                            "请将两端更新到 0.3.0 后开始音频",
                            None,
                        ),
                    );
                    return;
                };
                let offer = {
                    let mut state = self.shared.lock();
                    if state.microphone.peer_id.as_deref() == Some(peer.as_str()) {
                        drop(state);
                        let _ = self.shared.send(
                            &peer,
                            AudioControl::error(
                                StreamKind::Listen,
                                "同一个播放器一次运行一个音频方向",
                                Some(request),
                            ),
                        );
                        return;
                    }
                    state.listeners.insert(peer.clone(), request.clone());
                    if state.listen.phase == Phase::Active {
                        state.listen.stream_id.map(|id| (id, state.send_source))
                    } else {
                        None
                    }
                };
                if let Some((id, source)) = offer {
                    if self
                        .shared
                        .send(
                            &peer,
                            AudioControl::ListenOffer {
                                stream_id: stream_id_text(id),
                                source: Some(source),
                                request_id: Some(request),
                            },
                        )
                        .is_err()
                    {
                        self.shared.lock().listeners.remove(&peer);
                    }
                }
            }
            AudioControl::MicRequest { request_id } => {
                let Some(request) = request_id.filter(|r| parse_stream_id(r).is_ok()) else {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::error(
                            StreamKind::Microphone,
                            "请将两端更新到 0.3.0 后申请麦克风",
                            None,
                        ),
                    );
                    return;
                };
                if !cfg!(target_os = "windows") {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::error(
                            StreamKind::Microphone,
                            "该服务器不提供远端麦克风接收入口",
                            Some(request),
                        ),
                    );
                    return;
                }
                let name = self
                    .shared
                    .link
                    .snapshot()
                    .peers
                    .iter()
                    .find(|p| p.id == peer)
                    .map(|p| p.name.clone())
                    .unwrap_or_default();
                let stop_listen = {
                    let mut state = self.shared.lock();
                    let removed = state.listeners.remove(&peer).is_some();
                    if state.microphone.peer_id.as_deref() != Some(peer.as_str())
                        && state.requests.len() < 2
                    {
                        // A new request supersedes only the same peer's unapproved request.
                        state.requests.retain(|r| r.peer_id != peer);
                        state.requests.push(MicRequest {
                            peer_id: peer.clone(),
                            name,
                            request_id: request,
                        });
                    }
                    removed && state.listeners.is_empty() && state.listen.phase == Phase::Active
                };
                if stop_listen {
                    self.stop_lane_inner(StreamKind::Listen, "最后一台收听设备切换为麦克风", false)
                        .await;
                }
            }
            AudioControl::Unsubscribe { request_id } => {
                self.host_stop_peer(&peer, Some(AudioLane::Listen), "设备已停止收听", request_id)
                    .await
            }
            AudioControl::Stop {
                reason,
                lane,
                request_id,
            } => self.host_stop_peer(&peer, lane, &reason, request_id).await,
            AudioControl::Error {
                message,
                lane,
                request_id,
            } => self.host_stop_peer(&peer, lane, &message, request_id).await,
            AudioControl::ListenOffer { .. }
            | AudioControl::MicGrant { .. }
            | AudioControl::Capabilities { .. } => {}
        }
    }
    async fn host_stop_peer(
        &self,
        peer: &str,
        direction: Option<AudioLane>,
        reason: &str,
        request: Option<String>,
    ) {
        let Some(request) = request else {
            return;
        };
        let (stop_listen, stop_mic) = {
            let mut state = self.shared.lock();
            let may_listen = direction.is_none() || matches!(direction, Some(AudioLane::Listen));
            let may_mic = direction.is_none() || matches!(direction, Some(AudioLane::Microphone));
            let removed = may_listen
                && state.listeners.get(peer) == Some(&request)
                && state.listeners.remove(peer).is_some();
            if may_mic {
                state
                    .requests
                    .retain(|r| r.peer_id != peer || r.request_id != request);
            }
            (
                removed && state.listeners.is_empty() && state.listen.phase == Phase::Active,
                may_mic
                    && state.microphone.peer_id.as_deref() == Some(peer)
                    && state.microphone.request_id.as_ref() == Some(&request),
            )
        };
        if stop_listen {
            self.stop_lane_inner(StreamKind::Listen, reason, false)
                .await;
        }
        if stop_mic {
            self.stop_lane_inner(StreamKind::Microphone, reason, false)
                .await;
        }
    }
    async fn client_control(&self, peer: String, control: AudioControl) {
        match control {
            AudioControl::Capabilities {
                remote_microphone, ..
            } => self.shared.lock().remote_microphone_supported = Some(remote_microphone),
            AudioControl::ListenOffer {
                stream_id,
                source,
                request_id,
            } => {
                if request_id.is_none() {
                    let error = "音频协议需要两端更新到 0.3.0，请更新服务器".to_owned();
                    self.stop_lane_inner(StreamKind::Listen, &error, true).await;
                    self.shared.lock().listen.error = Some(error);
                    return;
                }
                let expected = {
                    let state = self.shared.lock();
                    state.listen.phase == Phase::Waiting
                        && state.listeners.contains_key(&peer)
                        && request_id.is_some()
                        && state.listen.request_id == request_id
                };
                if !expected {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::stop(StreamKind::Listen, "收听申请已结束", request_id),
                    );
                    return;
                }
                let result = match parse_stream_id(&stream_id) {
                    Ok(id) => {
                        let device = self.shared.lock().listen.output_device_id.clone();
                        self.shared.lock().remote_source = source;
                        self.launch(WorkerMode::Playback { device }, StreamKind::Listen, id)
                            .await
                    }
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    self.stop_lane_inner(StreamKind::Listen, &error, true).await;
                    self.shared.lock().listen.error = Some(error);
                }
            }
            AudioControl::MicGrant {
                stream_id,
                request_id,
            } => {
                if request_id.is_none() {
                    let error = "音频协议需要两端更新到 0.3.0，请更新服务器".to_owned();
                    self.stop_lane_inner(StreamKind::Microphone, &error, true)
                        .await;
                    self.shared.lock().microphone.error = Some(error);
                    return;
                }
                let expected = {
                    let state = self.shared.lock();
                    state.microphone.phase == Phase::Waiting
                        && state.microphone.peer_id.as_deref() == Some(peer.as_str())
                        && request_id.is_some()
                        && state.microphone.request_id == request_id
                };
                if !expected {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::stop(StreamKind::Microphone, "麦克风申请已结束", request_id),
                    );
                    return;
                }
                let result = match parse_stream_id(&stream_id) {
                    Ok(id) => {
                        let device = self.shared.lock().input_device_id.clone();
                        self.launch(
                            WorkerMode::Capture {
                                source: super::AudioSource::Microphone,
                                device,
                            },
                            StreamKind::Microphone,
                            id,
                        )
                        .await
                    }
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    self.stop_lane_inner(StreamKind::Microphone, &error, true)
                        .await;
                    self.shared.lock().microphone.error = Some(error);
                }
            }
            AudioControl::Stop {
                reason,
                lane,
                request_id,
            } => {
                self.client_stop_scoped(&peer, lane, &reason, request_id, false)
                    .await
            }
            AudioControl::Error {
                message,
                lane,
                request_id,
            } => {
                self.client_stop_scoped(&peer, lane, &message, request_id, true)
                    .await
            }
            AudioControl::Subscribe { .. }
            | AudioControl::Unsubscribe { .. }
            | AudioControl::MicRequest { .. } => {}
        }
    }
    async fn client_stop_scoped(
        &self,
        peer: &str,
        lane: Option<AudioLane>,
        reason: &str,
        request: Option<String>,
        error: bool,
    ) {
        let Some(request) = request else {
            return;
        };
        for kind in [StreamKind::Listen, StreamKind::Microphone] {
            let matches = {
                let state = self.shared.lock();
                let peer_matches = if kind == StreamKind::Listen {
                    state.listeners.contains_key(peer)
                } else {
                    state.microphone.peer_id.as_deref() == Some(peer)
                };
                (lane.is_none() || lane.map(|l| l.kind()) == Some(kind))
                    && peer_matches
                    && state.lane(kind).request_id.as_ref() == Some(&request)
            };
            if matches {
                self.stop_lane_inner(kind, reason, false).await;
                if error {
                    self.shared.lock().lane_mut(kind).error = Some(reason.into());
                }
            }
        }
    }
}
