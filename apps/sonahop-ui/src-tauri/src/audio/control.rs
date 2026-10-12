use super::workers::WorkerMode;
use super::{AudioControl, AudioService, MicRequest};
use sonahop_audio::{MediaPacket, StreamKind, parse_stream_id, stream_id_text};
use sonahop_link::{LinkEvent, LinkRole};

impl AudioService {
    pub(super) async fn handle_event(&self, event: LinkEvent) {
        match event {
            LinkEvent::Connected { .. } => {}
            LinkEvent::Media { peer_id, payload } => {
                let Ok(packet) = MediaPacket::parse(&payload) else {
                    self.shared.lock().snapshot.dropped_packets += 1;
                    return;
                };
                let mut state = self.shared.lock();
                let expected = if state.snapshot.role == LinkRole::Host {
                    state.snapshot.mode == "microphone"
                        && state.snapshot.mic_peer_id.as_deref() == Some(peer_id.as_str())
                        && packet.kind == StreamKind::Microphone
                } else {
                    state.snapshot.mode == "listen"
                        && state.listeners.contains(&peer_id)
                        && packet.kind == StreamKind::Listen
                };
                if !state.snapshot.active || !expected || state.stream_id != Some(packet.stream_id)
                {
                    state.snapshot.dropped_packets += 1;
                    return;
                }
                if let Some(worker) = &state.worker {
                    if worker.media.try_send(packet).is_ok() {
                        state.snapshot.received_packets += 1;
                    } else {
                        state.snapshot.dropped_packets += 1;
                    }
                }
            }
            LinkEvent::Disconnected { peer_id } => {
                let _lock = self.lifecycle.lock().await;
                let must_stop = {
                    let mut state = self.shared.lock();
                    state
                        .snapshot
                        .pending_mic_requests
                        .retain(|r| r.peer_id != peer_id);
                    let was_listener = state.listeners.remove(&peer_id);
                    let was_microphone =
                        state.snapshot.mic_peer_id.as_deref() == Some(peer_id.as_str());
                    was_microphone
                        || (state.snapshot.role == LinkRole::Client && was_listener)
                        || (was_listener
                            && state.snapshot.mode == "listen"
                            && state.snapshot.active
                            && state.listeners.is_empty())
                };
                if must_stop {
                    self.stop_inner("设备连接已断开", true).await;
                    self.shared.lock().snapshot.last_error =
                        Some("设备连接已断开，请重新连接并手动开始".into());
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
                if self.snapshot().role == LinkRole::Host {
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
                    if state.snapshot.mode == "microphone" {
                        drop(state);
                        let _ = self.shared.send(
                            &peer,
                            AudioControl::Error {
                                message: "电脑正在使用手机麦克风，请停止后再收听".into(),
                            },
                        );
                        return;
                    }
                    state.listeners.insert(peer.clone());
                    if state.snapshot.mode == "listen" && state.snapshot.active {
                        state.stream_id
                    } else {
                        None
                    }
                };
                if let Some(id) = offer {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::ListenOffer {
                            stream_id: stream_id_text(id),
                        },
                    );
                }
            }
            AudioControl::Unsubscribe => {
                let stop = {
                    let mut state = self.shared.lock();
                    state.listeners.remove(&peer);
                    state.snapshot.mode == "listen"
                        && state.snapshot.active
                        && state.listeners.is_empty()
                };
                if stop {
                    self.stop_inner("最后一台收听设备已停止", false).await;
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
                let mut state = self.shared.lock();
                state.listeners.remove(&peer);
                if state.snapshot.mic_peer_id.as_deref() == Some(peer.as_str()) {
                    return;
                }
                if !state
                    .snapshot
                    .pending_mic_requests
                    .iter()
                    .any(|r| r.peer_id == peer)
                    && state.snapshot.pending_mic_requests.len() < 2
                {
                    state.snapshot.pending_mic_requests.push(MicRequest {
                        peer_id: peer,
                        name,
                    });
                }
            }
            AudioControl::Stop { reason } | AudioControl::Error { message: reason } => {
                let stop = {
                    let mut state = self.shared.lock();
                    let was_listener = state.listeners.remove(&peer);
                    state
                        .snapshot
                        .pending_mic_requests
                        .retain(|r| r.peer_id != peer);
                    state.snapshot.mic_peer_id.as_deref() == Some(peer.as_str())
                        || (was_listener
                            && state.snapshot.mode == "listen"
                            && state.snapshot.active
                            && state.listeners.is_empty())
                };
                if stop {
                    self.stop_inner(&reason, true).await;
                }
            }
            AudioControl::ListenOffer { .. } | AudioControl::MicGrant { .. } => {}
        }
    }
    async fn client_control(&self, peer: String, control: AudioControl) {
        match control {
            AudioControl::ListenOffer { stream_id } => {
                let expected = {
                    let state = self.shared.lock();
                    state.snapshot.mode == "listen"
                        && state.snapshot.waiting
                        && state.listeners.contains(&peer)
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
                match result {
                    Ok(()) => {
                        let mut state = self.shared.lock();
                        if state.snapshot.last_error.is_none() {
                            state.snapshot.waiting = false;
                        }
                    }
                    Err(error) => {
                        self.stop_inner(&error, true).await;
                        self.shared.lock().snapshot.last_error = Some(error);
                    }
                }
            }
            AudioControl::MicGrant { stream_id } => {
                let expected = {
                    let state = self.shared.lock();
                    state.snapshot.mode == "microphone"
                        && state.snapshot.waiting
                        && state.snapshot.mic_peer_id.as_deref() == Some(peer.as_str())
                };
                if !expected {
                    let _ = self.shared.send(
                        &peer,
                        AudioControl::Stop {
                            reason: "手机已取消麦克风请求".into(),
                        },
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
                match result {
                    Ok(()) => {
                        let mut state = self.shared.lock();
                        if state.snapshot.last_error.is_none() {
                            state.snapshot.waiting = false;
                        }
                    }
                    Err(error) => {
                        self.stop_inner(&error, true).await;
                        self.shared.lock().snapshot.last_error = Some(error);
                    }
                }
            }
            AudioControl::Stop { reason } => {
                if self.snapshot().mode != "idle" {
                    self.stop_inner(&reason, false).await;
                    self.shared.lock().snapshot.last_error = Some(reason);
                }
            }
            AudioControl::Error { message } => {
                self.stop_inner(&message, true).await;
                self.shared.lock().snapshot.last_error = Some(message);
            }
            AudioControl::Subscribe | AudioControl::Unsubscribe | AudioControl::MicRequest => {}
        }
    }
}
