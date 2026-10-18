mod backend;
mod control;
mod workers;
use serde::{Deserialize, Serialize};
use sonahop_audio::{StreamKind, new_stream_id, stream_id_text};
use sonahop_link::{LinkRole, LinkService};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use workers::{Worker, WorkerMode};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioSource {
    System,
    Microphone,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioLane {
    Listen,
    Microphone,
}
impl AudioLane {
    fn kind(self) -> StreamKind {
        match self {
            Self::Listen => StreamKind::Listen,
            Self::Microphone => StreamKind::Microphone,
        }
    }
}
impl From<StreamKind> for AudioLane {
    fn from(kind: StreamKind) -> Self {
        if kind == StreamKind::Listen {
            Self::Listen
        } else {
            Self::Microphone
        }
    }
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicRequest {
    pub peer_id: String,
    pub name: String,
    pub request_id: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaneSnapshot {
    pub peak_level: f32,
    pub active: bool,
    pub waiting: bool,
    pub last_error: Option<String>,
    pub sent_packets: u64,
    pub received_packets: u64,
    pub dropped_packets: u64,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSnapshot {
    pub role: LinkRole,
    pub send_source: AudioSource,
    pub remote_source: Option<AudioSource>,
    pub remote_microphone_supported: Option<bool>,
    pub mode: String,
    pub active: bool,
    pub waiting: bool,
    pub mic_peer_id: Option<String>,
    pub output_device_id: Option<String>,
    pub pending_mic_requests: Vec<MicRequest>,
    pub gain: u8,
    pub last_error: Option<String>,
    pub sent_packets: u64,
    pub received_packets: u64,
    pub dropped_packets: u64,
    pub listen: LaneSnapshot,
    pub microphone: LaneSnapshot,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum AudioControl {
    Subscribe {
        #[serde(default)]
        request_id: Option<String>,
    },
    Unsubscribe {
        #[serde(default)]
        request_id: Option<String>,
    },
    MicRequest {
        #[serde(default)]
        request_id: Option<String>,
    },
    Capabilities {
        remote_microphone: bool,
        platform: String,
    },
    ListenOffer {
        stream_id: String,
        #[serde(default)]
        source: Option<AudioSource>,
        #[serde(default)]
        request_id: Option<String>,
    },
    ListenPaused {
        request_id: String,
    },
    MicGrant {
        stream_id: String,
        #[serde(default)]
        request_id: Option<String>,
    },
    Stop {
        reason: String,
        #[serde(default)]
        lane: Option<AudioLane>,
        #[serde(default)]
        request_id: Option<String>,
    },
    Error {
        message: String,
        #[serde(default)]
        lane: Option<AudioLane>,
        #[serde(default)]
        request_id: Option<String>,
    },
}
impl AudioControl {
    fn stop(kind: StreamKind, reason: impl Into<String>, request_id: Option<String>) -> Self {
        Self::Stop {
            reason: reason.into(),
            lane: Some(kind.into()),
            request_id,
        }
    }
    fn error(kind: StreamKind, message: impl Into<String>, request_id: Option<String>) -> Self {
        Self::Error {
            message: message.into(),
            lane: Some(kind.into()),
            request_id,
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Waiting,
    Active,
}
struct LaneState {
    peak_level: f32,
    phase: Phase,
    stream_id: Option<[u8; 16]>,
    request_id: Option<String>,
    epoch: u64,
    worker: Option<Worker>,
    peer_id: Option<String>,
    output_device_id: Option<String>,
    error: Option<String>,
    sent_packets: u64,
    received_packets: u64,
    dropped_packets: u64,
}
impl LaneState {
    fn new() -> Self {
        Self {
            peak_level: 0.0,
            phase: Phase::Idle,
            stream_id: None,
            request_id: None,
            epoch: 0,
            worker: None,
            peer_id: None,
            output_device_id: None,
            error: None,
            sent_packets: 0,
            received_packets: 0,
            dropped_packets: 0,
        }
    }
    fn snapshot(&self) -> LaneSnapshot {
        LaneSnapshot {
            peak_level: self.peak_level,
            active: self.phase == Phase::Active,
            waiting: self.phase == Phase::Waiting,
            last_error: self.error.clone(),
            sent_packets: self.sent_packets,
            received_packets: self.received_packets,
            dropped_packets: self.dropped_packets,
        }
    }
}
struct State {
    role: LinkRole,
    send_source: AudioSource,
    input_device_id: Option<String>,
    remote_source: Option<AudioSource>,
    remote_microphone_supported: Option<bool>,
    listen: LaneState,
    microphone: LaneState,
    listeners: HashMap<String, String>,
    requests: Vec<MicRequest>,
    gain: u8,
}
impl State {
    fn lane(&self, kind: StreamKind) -> &LaneState {
        if kind == StreamKind::Listen {
            &self.listen
        } else {
            &self.microphone
        }
    }
    fn lane_mut(&mut self, kind: StreamKind) -> &mut LaneState {
        if kind == StreamKind::Listen {
            &mut self.listen
        } else {
            &mut self.microphone
        }
    }
    fn peers_for(&self, kind: StreamKind) -> Vec<String> {
        if kind == StreamKind::Listen {
            self.listeners.keys().cloned().collect()
        } else {
            self.microphone.peer_id.iter().cloned().collect()
        }
    }
    fn request_for(&self, kind: StreamKind, peer: &str) -> Option<String> {
        if kind == StreamKind::Listen {
            self.listeners.get(peer).cloned()
        } else {
            self.microphone.request_id.clone()
        }
    }
    fn snapshot(&self) -> AudioSnapshot {
        let listen = self.listen.snapshot();
        let microphone = self.microphone.snapshot();
        let listening = listen.active || listen.waiting;
        let mic = microphone.active || microphone.waiting;
        AudioSnapshot {
            role: self.role,
            send_source: self.send_source,
            remote_source: self.remote_source,
            remote_microphone_supported: self.remote_microphone_supported,
            mode: match (listening, mic) {
                (true, true) => "duplex",
                (true, false) => "listen",
                (false, true) => "microphone",
                _ => "idle",
            }
            .into(),
            active: listen.active || microphone.active,
            waiting: listen.waiting || microphone.waiting,
            mic_peer_id: self.microphone.peer_id.clone(),
            output_device_id: self.microphone.output_device_id.clone(),
            pending_mic_requests: self.requests.clone(),
            gain: self.gain,
            last_error: listen.last_error.clone().or(microphone.last_error.clone()),
            sent_packets: listen.sent_packets + microphone.sent_packets,
            received_packets: listen.received_packets + microphone.received_packets,
            dropped_packets: listen.dropped_packets + microphone.dropped_packets,
            listen,
            microphone,
        }
    }
}
struct Shared {
    state: Mutex<State>,
    link: Arc<LinkService>,
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    app: AppHandle,
}
impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    fn send(&self, peer: &str, message: AudioControl) -> Result<(), String> {
        self.link.send_control(
            peer,
            serde_json::to_value(message).map_err(|e| e.to_string())?,
        )
    }
    fn fail(&self, kind: StreamKind, epoch: u64, error: String) {
        let peers = {
            let mut state = self.lock();
            if state.lane(kind).epoch != epoch {
                return;
            }
            let peers = state
                .peers_for(kind)
                .into_iter()
                .map(|peer| {
                    let request = state.request_for(kind, &peer);
                    (peer, request)
                })
                .collect::<Vec<_>>();
            let lane = state.lane_mut(kind);
            if let Some(worker) = &lane.worker {
                worker.cancel();
            }
            lane.peak_level = 0.0;
            lane.phase = Phase::Idle;
            lane.stream_id = None;
            lane.error = Some(error.clone());
            lane.peer_id = None;
            lane.output_device_id = None;
            if kind == StreamKind::Listen {
                state.listeners.clear();
            }
            peers
        };
        self.link.log_event("audio-failed", serde_json::json!({"lane":if kind==StreamKind::Listen{"listen"}else{"microphone"},"epoch":epoch,"reason":error}));
        for (peer, request) in peers {
            let _ = self.send(&peer, AudioControl::stop(kind, error.clone(), request));
        }
    }
}
pub struct AudioService {
    shared: Arc<Shared>,
    lifecycle: tokio::sync::Mutex<()>,
    events: Mutex<Option<JoinHandle<()>>>,
}
impl AudioService {
    pub fn new(app: AppHandle, link: Arc<LinkService>, role: LinkRole) -> Self {
        Self {
            shared: Arc::new(Shared {
                app,
                link,
                state: Mutex::new(State {
                    role,
                    send_source: AudioSource::System,
                    input_device_id: None,
                    remote_source: None,
                    remote_microphone_supported: None,
                    listen: LaneState::new(),
                    microphone: LaneState::new(),
                    listeners: HashMap::new(),
                    requests: Vec::new(),
                    gain: 100,
                }),
            }),
            lifecycle: tokio::sync::Mutex::new(()),
            events: Mutex::new(None),
        }
    }
    pub fn start_events(self: &Arc<Self>) {
        let mut task = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if task.is_some() {
            return;
        }
        #[cfg(target_os = "android")]
        {
            let service = Arc::downgrade(self);
            tauri_plugin_sonahop_lan::android_audio::set_notification_stop_handler(move || {
                let Some(service) = service.upgrade() else {
                    return false;
                };
                tauri::async_runtime::spawn(async move {
                    service.stop().await;
                });
                true
            });
        }
        let mut receiver = self.shared.link.subscribe();
        let weak = Arc::downgrade(self);
        *task = Some(tauri::async_runtime::spawn(async move {
            loop {
                let event = receiver.recv().await;
                let Some(service) = weak.upgrade() else {
                    break;
                };
                match event {
                    Ok(event) => service.handle_event(event).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        service
                            .stop_with_error("音频事件队列过载，请重新开始".into())
                            .await
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }
    pub fn snapshot(&self) -> AudioSnapshot {
        self.shared.lock().snapshot()
    }
    pub fn set_gain(&self, gain: u8) -> Result<AudioSnapshot, String> {
        if gain > 100 {
            return Err("麦克风增益范围应为 0–100".into());
        }
        self.shared.lock().gain = gain;
        Ok(self.snapshot())
    }
    fn role(&self, role: LinkRole) -> Result<(), String> {
        if self.shared.lock().role != role {
            Err("此操作不适用于当前连接角色".into())
        } else {
            Ok(())
        }
    }
    fn host_peer(&self) -> Result<String, String> {
        self.shared
            .link
            .snapshot()
            .peers
            .first()
            .map(|p| p.id.clone())
            .ok_or("请先连接服务器".into())
    }
    pub async fn start_listening(
        &self,
        source: Option<AudioSource>,
        input: Option<String>,
        output: Option<String>,
    ) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        if !self.shared.link.snapshot().running {
            return Err("请先开启局域网连接".into());
        }
        let role = self.shared.lock().role;
        let selected = source.unwrap_or(AudioSource::System);
        if self.shared.lock().listen.phase == Phase::Active {
            if role == LinkRole::Host && self.shared.lock().send_source != selected {
                return Err("请先停止共享，再切换音源".into());
            }
            return Ok(self.snapshot());
        }
        if role == LinkRole::Host {
            {
                let mut state = self.shared.lock();
                state.send_source = selected;
                state.input_device_id = input.filter(|v| !v.trim().is_empty());
            }
            let device = self.shared.lock().input_device_id.clone();
            let id = new_stream_id();
            self.launch(
                WorkerMode::Capture {
                    source: selected,
                    device,
                },
                StreamKind::Listen,
                id,
            )
            .await?;
            let peers = self.shared.lock().peers_for(StreamKind::Listen);
            for peer in peers {
                let request_id = self.shared.lock().listeners.get(&peer).cloned();
                if self
                    .shared
                    .send(
                        &peer,
                        AudioControl::ListenOffer {
                            stream_id: stream_id_text(id),
                            source: Some(selected),
                            request_id,
                        },
                    )
                    .is_err()
                {
                    self.shared.lock().listeners.remove(&peer);
                }
            }
        } else {
            let peer = self.host_peer()?;
            self.stop_lane_inner(StreamKind::Listen, "重新开始收听", true)
                .await;
            {
                let mut state = self.shared.lock();
                state.listen.phase = Phase::Waiting;
                state.listen.request_id = Some(stream_id_text(new_stream_id()));
                state.listen.output_device_id = output.filter(|v| !v.trim().is_empty());
                let request = state
                    .listen
                    .request_id
                    .clone()
                    .expect("new subscription has an ID");
                state.listeners.insert(peer.clone(), request);
            }
            let request_id = self.shared.lock().listen.request_id.clone();
            if let Err(error) = self
                .shared
                .send(&peer, AudioControl::Subscribe { request_id })
            {
                self.stop_lane_inner(StreamKind::Listen, "连接不可用", false)
                    .await;
                return Err(error);
            }
        }
        Ok(self.snapshot())
    }
    pub async fn request_microphone(&self, input: Option<String>) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        self.role(LinkRole::Client)?;
        if self.shared.lock().remote_microphone_supported == Some(false) {
            return Err("该服务器不支持接收远端麦克风".into());
        }
        let peer = self.host_peer()?;
        self.stop_lane_inner(StreamKind::Microphone, "重新申请麦克风", true)
            .await;
        {
            let mut state = self.shared.lock();
            state.microphone.phase = Phase::Waiting;
            state.microphone.request_id = Some(stream_id_text(new_stream_id()));
            state.input_device_id = input.filter(|v| !v.trim().is_empty());
            state.microphone.peer_id = Some(peer.clone());
        }
        let request_id = self.shared.lock().microphone.request_id.clone();
        if let Err(error) = self
            .shared
            .send(&peer, AudioControl::MicRequest { request_id })
        {
            self.stop_lane_inner(StreamKind::Microphone, "连接不可用", false)
                .await;
            return Err(error);
        }
        Ok(self.snapshot())
    }
    pub async fn answer_microphone(
        &self,
        peer_id: String,
        request_id: String,
        approve: bool,
        output: Option<String>,
    ) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        self.role(LinkRole::Host)?;
        if !cfg!(target_os = "windows") {
            return Err("该服务器没有远端麦克风接收后端".into());
        }
        if !self
            .shared
            .lock()
            .requests
            .iter()
            .any(|r| r.peer_id == peer_id && r.request_id == request_id)
        {
            return Err("麦克风请求已结束".into());
        }
        if !approve {
            self.shared.lock().requests.retain(|r| r.peer_id != peer_id);
            self.shared.send(
                &peer_id,
                AudioControl::error(
                    StreamKind::Microphone,
                    "服务器拒绝了麦克风请求",
                    Some(request_id.clone()),
                ),
            )?;
            return Ok(self.snapshot());
        }
        let device = output
            .filter(|s| !s.trim().is_empty())
            .ok_or("请先选择已安装的虚拟音频设备输出端点")?;
        if !self
            .shared
            .link
            .snapshot()
            .peers
            .iter()
            .any(|p| p.id == peer_id)
        {
            return Err("手机已断开".into());
        }
        self.reject_pending("电脑已选择另一台手机作为麦克风", Some(&peer_id));
        self.stop_lane_inner(StreamKind::Microphone, "电脑切换麦克风设备", true)
            .await;
        // Listening subscriptions survive an independently approved microphone lane.
        {
            let mut state = self.shared.lock();
            state.microphone.peer_id = Some(peer_id.clone());
            state.microphone.request_id = Some(request_id.clone());
            state.microphone.output_device_id = Some(device.clone());
            state.microphone.phase = Phase::Waiting;
        }
        let id = new_stream_id();
        if let Err(error) = self
            .launch(
                WorkerMode::Playback {
                    device: Some(device),
                },
                StreamKind::Microphone,
                id,
            )
            .await
        {
            let _ = self.shared.send(
                &peer_id,
                AudioControl::error(
                    StreamKind::Microphone,
                    error.clone(),
                    Some(request_id.clone()),
                ),
            );
            return Err(error);
        }
        if let Err(error) = self.shared.send(
            &peer_id,
            AudioControl::MicGrant {
                stream_id: stream_id_text(id),
                request_id: Some(request_id.clone()),
            },
        ) {
            self.stop_lane_inner(StreamKind::Microphone, "麦克风连接失败", false)
                .await;
            return Err(error);
        }
        Ok(self.snapshot())
    }
    async fn launch(&self, mode: WorkerMode, kind: StreamKind, id: [u8; 16]) -> Result<(), String> {
        let (epoch, old) = {
            let mut state = self.shared.lock();
            let lane = state.lane_mut(kind);
            lane.epoch = lane.epoch.wrapping_add(1);
            let old = lane.worker.take();
            if let Some(worker) = &old {
                worker.cancel();
            }
            lane.error = None;
            lane.peak_level = 0.0;
            lane.stream_id = Some(id);
            lane.phase = Phase::Waiting;
            (lane.epoch, old)
        };
        if let Some(old) = old {
            let _ = tauri::async_runtime::spawn_blocking(move || old.join()).await;
        }
        let shared = self.shared.clone();
        let result = tauri::async_runtime::spawn_blocking(move || {
            workers::spawn(shared, epoch, mode, kind, id)
        })
        .await
        .map_err(|e| e.to_string());
        let result = match result {
            Ok(value) => value,
            Err(error) => Err(error),
        };
        match result {
            Ok(worker) => {
                let error = {
                    let mut state = self.shared.lock();
                    let lane = state.lane_mut(kind);
                    if let Some(error) = lane.error.clone() {
                        Some(error)
                    } else {
                        lane.worker = Some(worker);
                        lane.phase = Phase::Active;
                        return Ok(());
                    }
                };
                let _ = tauri::async_runtime::spawn_blocking(move || worker.join()).await;
                Err(error.unwrap_or_else(|| "音频启动取消".into()))
            }
            Err(error) => {
                let mut state = self.shared.lock();
                let lane = state.lane_mut(kind);
                lane.peak_level = 0.0;
                lane.phase = Phase::Idle;
                lane.stream_id = None;
                lane.peer_id = None;
                lane.output_device_id = None;
                lane.error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub async fn stop(&self) -> AudioSnapshot {
        let _lock = self.lifecycle.lock().await;
        self.reject_pending("电脑已取消麦克风申请", None);
        self.stop_lane_inner(StreamKind::Listen, "声音共享已停止", true)
            .await;
        self.stop_lane_inner(StreamKind::Microphone, "麦克风已停止", true)
            .await;
        #[cfg(target_os = "android")]
        {
            use tauri_plugin_sonahop_lan::SonahopLanExt;
            let app = self.shared.app.clone();
            let result =
                tauri::async_runtime::spawn_blocking(move || app.sonahop_lan().stop_audio()).await;
            match result {
                Ok(Ok(())) => {}
                other => {
                    self.shared.lock().listen.error =
                        Some(format!("Android 音频停止失败：{other:?}"))
                }
            }
        }
        self.snapshot()
    }
    pub async fn set_role(&self, role: LinkRole) -> Result<(), String> {
        let _lock = self.lifecycle.lock().await;
        let mut state = self.shared.lock();
        if state.listen.worker.is_some() || state.microphone.worker.is_some() {
            return Err("切换角色前请停止音频".into());
        }
        state.role = role;
        state.listen = LaneState::new();
        state.microphone = LaneState::new();
        state.listeners.clear();
        state.requests.clear();
        state.remote_source = None;
        state.remote_microphone_supported = None;
        state.input_device_id = None;
        state.send_source = AudioSource::System;
        Ok(())
    }
    pub async fn stop_lane(&self, lane: AudioLane) -> AudioSnapshot {
        let _lock = self.lifecycle.lock().await;
        let kind = lane.kind();
        if kind == StreamKind::Microphone {
            self.reject_pending("电脑已取消麦克风申请", None);
        }
        if kind == StreamKind::Listen && self.shared.lock().role == LinkRole::Host {
            self.pause_listening().await;
        } else {
            self.stop_lane_inner(
                kind,
                if kind == StreamKind::Listen {
                    "收听已停止"
                } else {
                    "麦克风已停止"
                },
                true,
            )
            .await;
        }
        self.snapshot()
    }
    // A host pause releases capture without ending the listening subscription.
    // Receivers keep the existing sink/service so a background resume needs no new start.
    // Full stop, disconnect and notification stop still use stop_lane_inner.
    async fn pause_listening(&self) {
        let (worker, subscribers) = {
            let mut state = self.shared.lock();
            let host = state.role == LinkRole::Host;
            let subscribers = if host {
                state
                    .listeners
                    .iter()
                    .map(|(peer, request)| (peer.clone(), request.clone()))
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let lane = &mut state.listen;
            let worker = if host {
                lane.epoch = lane.epoch.wrapping_add(1);
                let worker = lane.worker.take();
                if let Some(worker) = &worker {
                    worker.cancel();
                }
                worker
            } else {
                None
            };
            lane.peak_level = 0.0;
            lane.phase = if host { Phase::Idle } else { Phase::Waiting };
            lane.stream_id = None;
            lane.error = None;
            (worker, subscribers)
        };
        for (peer, request_id) in subscribers {
            if self
                .shared
                .send(&peer, AudioControl::ListenPaused { request_id })
                .is_err()
            {
                self.shared.lock().listeners.remove(&peer);
            }
        }
        if let Some(worker) = worker {
            let _ = tauri::async_runtime::spawn_blocking(move || worker.join()).await;
        }
    }
    fn reject_pending(&self, reason: &str, except: Option<&str>) {
        let pending = {
            let mut state = self.shared.lock();
            std::mem::take(&mut state.requests)
        };
        for request in pending {
            if except != Some(request.peer_id.as_str()) {
                let _ = self.shared.send(
                    &request.peer_id,
                    AudioControl::error(StreamKind::Microphone, reason, Some(request.request_id)),
                );
            }
        }
    }
    async fn stop_with_error(&self, error: String) {
        let _lock = self.lifecycle.lock().await;
        self.stop_lane_inner(StreamKind::Listen, &error, true).await;
        self.stop_lane_inner(StreamKind::Microphone, &error, true)
            .await;
        let mut state = self.shared.lock();
        state.listen.error = Some(error.clone());
        state.microphone.error = Some(error);
    }
    async fn stop_lane_inner(&self, kind: StreamKind, reason: &str, notify: bool) {
        let (worker, peers) = {
            let mut state = self.shared.lock();
            let peers = state
                .peers_for(kind)
                .into_iter()
                .map(|peer| {
                    let request = state.request_for(kind, &peer);
                    (peer, request)
                })
                .collect::<Vec<_>>();
            let lane = state.lane_mut(kind);
            lane.epoch = lane.epoch.wrapping_add(1);
            let worker = lane.worker.take();
            if let Some(worker) = &worker {
                worker.cancel();
            }
            lane.peak_level = 0.0;
            lane.phase = Phase::Idle;
            lane.stream_id = None;
            lane.request_id = None;
            lane.peer_id = None;
            lane.output_device_id = None;
            lane.error = None;
            if kind == StreamKind::Listen {
                state.listeners.clear();
                state.remote_source = None;
            }
            (worker, peers)
        };
        if notify {
            for (peer, request) in peers {
                let _ = self
                    .shared
                    .send(&peer, AudioControl::stop(kind, reason, request));
            }
        }
        if let Some(worker) = worker {
            let _ = tauri::async_runtime::spawn_blocking(move || worker.join()).await;
        }
    }
}
impl Drop for AudioService {
    fn drop(&mut self) {
        if let Some(task) = self
            .events
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            task.abort();
        }
        let state = self.shared.lock();
        for kind in [StreamKind::Listen, StreamKind::Microphone] {
            if let Some(worker) = &state.lane(kind).worker {
                worker.cancel();
            }
        }
    }
}
