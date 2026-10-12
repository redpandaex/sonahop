use super::workers::WorkerMode;
use super::{AudioControl, AudioLane, AudioService, MicRequest, Phase};
use sonahop_audio::{MediaPacket, StreamKind, parse_stream_id, stream_id_text};
use sonahop_link::{LinkEvent, LinkRole};
impl AudioService {
    pub(super) async fn handle_event(&self, event: LinkEvent) {
        match event {
            LinkEvent::Connected { .. } => {}
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
                    kind == StreamKind::Listen && state.listeners.contains(&peer_id)
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
                let (stop_listen, stop_mic) = {
                    let mut state = self.shared.lock();
                    state.requests.retain(|r| r.peer_id != peer_id);
                    let was_listener = state.listeners.remove(&peer_id);
                    let listen = was_listener
                        && (state.role == LinkRole::Client || state.listeners.is_empty());
                    (
                        listen,
                        state.microphone.peer_id.as_deref() == Some(peer_id.as_str()),
                    )
                };
                for (kind, stop) in [
                    (StreamKind::Listen, stop_listen),
                    (StreamKind::Microphone, stop_mic),
                ] {
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
                if self.shared.lock().role == LinkRole::Host {
                    self.host_control(peer_id, control).await;
                } else {
                    self.client_control(peer_id, control).await;
                }
            }
        }
    }
    async fn host_control(&self, peer: String, control: AudioControl) {
        match control {
            AudioControl::Subscribe => {
                let offer = {
                    let mut state = self.shared.lock();
                    if state.microphone.peer_id.as_deref() == Some(peer.as_str()) {
                        drop(state);
                        let _ = self.shared.send(
                            &peer,
                            AudioControl::error(
                                StreamKind::Listen,
                                "同一台 Android 暂不能同时录音与收听",
                            ),
                        );
                        return;
                    }
                    state.listeners.insert(peer.clone());
                    if state.listen.phase == Phase::Active {
                        state.listen.stream_id
                    } else {
                        None
                    }
                };
                if let Some(id) = offer {
                    if self
                        .shared
                        .send(
                            &peer,
                            AudioControl::ListenOffer {
                                stream_id: stream_id_text(id),
                            },
                        )
                        .is_err()
                    {
                        self.shared.lock().listeners.remove(&peer);
                    }
                }
            }
            AudioControl::MicRequest => {
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
                    let removed = state.listeners.remove(&peer);
                    if state.microphone.peer_id.as_deref() != Some(peer.as_str())
                        && !state.requests.iter().any(|r| r.peer_id == peer)
                        && state.requests.len() < 2
                    {
                        state.requests.push(MicRequest {
                            peer_id: peer.clone(),
                            name,
                        });
                    }
                    removed && state.listeners.is_empty() && state.listen.phase == Phase::Active
                };
                if stop_listen {
                    self.stop_lane_inner(StreamKind::Listen, "最后一台收听设备切换为麦克风", false)
                        .await;
                }
            }
            AudioControl::Unsubscribe => {
                self.host_stop_peer(&peer, Some(AudioLane::Listen), "设备已停止收听")
                    .await
            }
            AudioControl::Stop { reason, lane } => self.host_stop_peer(&peer, lane, &reason).await,
            AudioControl::Error { message, lane } => {
                self.host_stop_peer(&peer, lane, &message).await
            }
            AudioControl::ListenOffer { .. } | AudioControl::MicGrant { .. } => {}
        }
    }
    async fn host_stop_peer(&self, peer: &str, direction: Option<AudioLane>, reason: &str) {
        let (stop_listen, stop_mic) = {
            let mut state = self.shared.lock();
            let may_listen = direction.is_none() || matches!(direction, Some(AudioLane::Listen));
            let may_mic = direction.is_none() || matches!(direction, Some(AudioLane::Microphone));
            let removed = may_listen && state.listeners.remove(peer);
            if may_mic {
                state.requests.retain(|r| r.peer_id != peer);
            }
            (
                removed && state.listeners.is_empty() && state.listen.phase == Phase::Active,
                may_mic && state.microphone.peer_id.as_deref() == Some(peer),
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
            AudioControl::ListenOffer { stream_id } => {
                let expected = {
                    let state = self.shared.lock();
                    state.listen.phase == Phase::Waiting && state.listeners.contains(&peer)
                };
                if !expected {
                    return;
                }
                let result = match parse_stream_id(&stream_id) {
                    Ok(id) => {
                        self.launch(WorkerMode::AndroidPlayback, StreamKind::Listen, id)
                            .await
                    }
                    Err(error) => Err(error),
                };
                if let Err(error) = result {
                    self.stop_lane_inner(StreamKind::Listen, &error, true).await;
                    self.shared.lock().listen.error = Some(error);
                }
            }
            AudioControl::MicGrant { stream_id } => {
                let expected = {
                    let state = self.shared.lock();
                    state.microphone.phase == Phase::Waiting
                        && state.microphone.peer_id.as_deref() == Some(peer.as_str())
                };
                if !expected {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::stop(StreamKind::Microphone, "手机已取消麦克风请求"),
                    );
                    return;
                }
                let result = match parse_stream_id(&stream_id) {
                    Ok(id) => {
                        self.launch(WorkerMode::AndroidCapture, StreamKind::Microphone, id)
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
            AudioControl::Stop { reason, lane } => {
                for kind in [StreamKind::Listen, StreamKind::Microphone] {
                    if lane.is_none() || lane.map(|lane| lane.kind()) == Some(kind) {
                        self.stop_lane_inner(kind, &reason, false).await;
                    }
                }
            }
            AudioControl::Error { message, lane } => {
                for kind in [StreamKind::Listen, StreamKind::Microphone] {
                    if lane.is_none() || lane.map(|lane| lane.kind()) == Some(kind) {
                        self.stop_lane_inner(kind, &message, true).await;
                        self.shared.lock().lane_mut(kind).error = Some(message.clone());
                    }
                }
            }
            AudioControl::Subscribe | AudioControl::Unsubscribe | AudioControl::MicRequest => {}
        }
    }
}
