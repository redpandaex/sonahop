mod control;
mod workers;

use serde::{Deserialize, Serialize};
use sonahop_audio::{StreamKind, new_stream_id, stream_id_text};
use sonahop_link::{LinkRole, LinkService};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::AppHandle;
use tauri::async_runtime::JoinHandle;
use workers::{Worker, WorkerMode};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicRequest {
    pub peer_id: String,
    pub name: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSnapshot {
    pub role: LinkRole,
    pub mode: String,
    pub active: bool,
    pub waiting: bool,
    pub mic_peer_id: Option<String>,
    pub output_device_id: Option<String>,
    pub pending_mic_requests: Vec<MicRequest>,
    pub sent_packets: u64,
    pub received_packets: u64,
    pub dropped_packets: u64,
    pub gain: u8,
    pub last_error: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(super) enum AudioControl {
    Subscribe,
    Unsubscribe,
    MicRequest,
    ListenOffer { stream_id: String },
    MicGrant { stream_id: String },
    Stop { reason: String },
    Error { message: String },
}
struct State {
    pub snapshot: AudioSnapshot,
    pub listeners: HashSet<String>,
    pub stream_id: Option<[u8; 16]>,
    pub epoch: u64,
    pub worker: Option<Worker>,
}
struct Shared {
    pub state: Mutex<State>,
    pub link: Arc<LinkService>,
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub app: AppHandle,
}
impl Shared {
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn send(&self, peer: &str, message: AudioControl) -> Result<(), String> {
        self.link.send_control(
            peer,
            serde_json::to_value(message).map_err(|e| e.to_string())?,
        )
    }
    pub fn fail(&self, epoch: u64, error: String) {
        let peers = {
            let mut state = self.lock();
            if state.epoch != epoch {
                return;
            }
            if let Some(worker) = &state.worker {
                worker.cancel();
            }
            state.snapshot.active = false;
            state.snapshot.waiting = false;
            state.snapshot.mode = "idle".into();
            state.snapshot.last_error = Some(error.clone());
            let mut peers: Vec<_> = state.listeners.drain().collect();
            if let Some(peer) = state.snapshot.mic_peer_id.take() {
                peers.push(peer);
            }
            peers
        };
        for peer in peers {
            let _ = self.send(
                &peer,
                AudioControl::Stop {
                    reason: error.clone(),
                },
            );
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
                    snapshot: AudioSnapshot {
                        role,
                        mode: "idle".into(),
                        active: false,
                        waiting: false,
                        mic_peer_id: None,
                        output_device_id: None,
                        pending_mic_requests: Vec::new(),
                        sent_packets: 0,
                        received_packets: 0,
                        dropped_packets: 0,
                        gain: 100,
                        last_error: None,
                    },
                    listeners: HashSet::new(),
                    stream_id: None,
                    epoch: 0,
                    worker: None,
                }),
            }),
            lifecycle: tokio::sync::Mutex::new(()),
            events: Mutex::new(None),
        }
    }
    // Subscription starts before sockets can be started by a UI command. No audio is started.
    pub fn start_events(self: &Arc<Self>) {
        let mut task = self.events.lock().unwrap_or_else(|e| e.into_inner());
        if task.is_some() {
            return;
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
                            .await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }));
    }
    pub fn snapshot(&self) -> AudioSnapshot {
        self.shared.lock().snapshot.clone()
    }
    pub fn set_gain(&self, gain: u8) -> Result<AudioSnapshot, String> {
        if gain > 100 {
            return Err("音量范围应为 0–100".into());
        }
        self.shared.lock().snapshot.gain = gain;
        Ok(self.snapshot())
    }
    fn role(&self, role: LinkRole) -> Result<(), String> {
        if self.snapshot().role != role {
            Err("此操作不适用于当前平台角色".into())
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
            .ok_or("请先连接电脑".into())
    }
    pub async fn start_listening(&self) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        if !self.shared.link.snapshot().running {
            return Err("请先开启局域网连接".into());
        }
        if self.snapshot().mode == "listen" && self.snapshot().active {
            return Ok(self.snapshot());
        }
        let waiting_listeners = {
            let state = self.shared.lock();
            if state.snapshot.role == LinkRole::Host && state.snapshot.mode == "idle" {
                state.listeners.clone()
            } else {
                HashSet::new()
            }
        };
        self.stop_inner("切换到收听模式", waiting_listeners.is_empty())
            .await;
        if !waiting_listeners.is_empty() {
            self.shared.lock().listeners = waiting_listeners;
        }
        if self.snapshot().role == LinkRole::Host {
            let id = new_stream_id();
            self.launch(WorkerMode::WindowsCapture, StreamKind::Listen, id)
                .await?;
            let listeners = {
                let state = self.shared.lock();
                if !state.snapshot.active {
                    return Err(state
                        .snapshot
                        .last_error
                        .clone()
                        .unwrap_or_else(|| "音频启动已终止".into()));
                }
                state.listeners.iter().cloned().collect::<Vec<_>>()
            };
            for peer in listeners {
                self.shared.send(
                    &peer,
                    AudioControl::ListenOffer {
                        stream_id: stream_id_text(id),
                    },
                )?;
            }
        } else {
            let peer = self.host_peer()?;
            {
                let mut state = self.shared.lock();
                state.snapshot.mode = "listen".into();
                state.snapshot.waiting = true;
                state.listeners.insert(peer.clone());
            }
            if let Err(error) = self.shared.send(&peer, AudioControl::Subscribe) {
                self.stop_inner("连接不可用", false).await;
                return Err(error);
            }
        }
        Ok(self.snapshot())
    }
    pub async fn request_microphone(&self) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        self.role(LinkRole::Client)?;
        let peer = self.host_peer()?;
        self.stop_inner("切换到麦克风模式", true).await;
        {
            let mut state = self.shared.lock();
            state.snapshot.mode = "microphone".into();
            state.snapshot.waiting = true;
            state.snapshot.mic_peer_id = Some(peer.clone());
        }
        if let Err(error) = self.shared.send(&peer, AudioControl::MicRequest) {
            self.stop_inner("连接不可用", false).await;
            return Err(error);
        }
        Ok(self.snapshot())
    }
    pub async fn answer_microphone(
        &self,
        peer_id: String,
        approve: bool,
        output: Option<String>,
    ) -> Result<AudioSnapshot, String> {
        let _lock = self.lifecycle.lock().await;
        self.role(LinkRole::Host)?;
        if !self
            .shared
            .lock()
            .snapshot
            .pending_mic_requests
            .iter()
            .any(|p| p.peer_id == peer_id)
        {
            return Err("麦克风请求已结束".into());
        }
        if !approve {
            self.shared
                .lock()
                .snapshot
                .pending_mic_requests
                .retain(|r| r.peer_id != peer_id);
            self.shared.send(
                &peer_id,
                AudioControl::Error {
                    message: "电脑拒绝了麦克风请求".into(),
                },
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
        self.stop_inner("电脑切换为手机麦克风", true).await;
        let id = new_stream_id();
        if let Err(error) = self
            .launch(
                WorkerMode::WindowsPlayback {
                    device: device.clone(),
                },
                StreamKind::Microphone,
                id,
            )
            .await
        {
            let _ = self.shared.send(
                &peer_id,
                AudioControl::Error {
                    message: error.clone(),
                },
            );
            return Err(error);
        }
        {
            let mut state = self.shared.lock();
            if !state.snapshot.active {
                return Err(state
                    .snapshot
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "音频启动已终止".into()));
            }
            state.snapshot.mic_peer_id = Some(peer_id.clone());
            state.snapshot.output_device_id = Some(device);
            state
                .snapshot
                .pending_mic_requests
                .retain(|r| r.peer_id != peer_id);
        }
        if let Err(error) = self.shared.send(
            &peer_id,
            AudioControl::MicGrant {
                stream_id: stream_id_text(id),
            },
        ) {
            self.stop_inner("麦克风连接失败", false).await;
            return Err(error);
        }
        Ok(self.snapshot())
    }
    async fn launch(&self, mode: WorkerMode, kind: StreamKind, id: [u8; 16]) -> Result<(), String> {
        let (epoch, shared) = {
            let mut state = self.shared.lock();
            state.epoch = state.epoch.wrapping_add(1);
            state.stream_id = Some(id);
            state.snapshot.last_error = None;
            (state.epoch, self.shared.clone())
        };
        let worker = tauri::async_runtime::spawn_blocking(move || {
            workers::spawn(shared, epoch, mode, kind, id)
        })
        .await
        .map_err(|e| e.to_string())?;
        match worker {
            Ok(worker) => {
                let startup_error = {
                    let mut state = self.shared.lock();
                    if let Some(error) = state.snapshot.last_error.clone() {
                        Some(error)
                    } else {
                        state.worker = Some(worker);
                        state.snapshot.mode = if kind == StreamKind::Listen {
                            "listen"
                        } else {
                            "microphone"
                        }
                        .into();
                        state.snapshot.active = true;
                        state.snapshot.waiting = false;
                        return Ok(());
                    }
                };
                let _ = tauri::async_runtime::spawn_blocking(move || worker.join()).await;
                Err(startup_error.unwrap_or_else(|| "音频启动取消".into()))
            }
            Err(error) => {
                let mut state = self.shared.lock();
                state.stream_id = None;
                state.snapshot.last_error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub async fn stop(&self) -> AudioSnapshot {
        let _lock = self.lifecycle.lock().await;
        self.reject_pending("电脑已取消麦克风申请", None);
        self.stop_inner("音频已停止", true).await;
        self.snapshot()
    }
    fn reject_pending(&self, reason: &str, except: Option<&str>) {
        let pending = {
            let mut state = self.shared.lock();
            std::mem::take(&mut state.snapshot.pending_mic_requests)
        };
        for request in pending {
            if except != Some(request.peer_id.as_str()) {
                let _ = self.shared.send(
                    &request.peer_id,
                    AudioControl::Error {
                        message: reason.into(),
                    },
                );
            }
        }
    }
    async fn stop_with_error(&self, error: String) {
        let _lock = self.lifecycle.lock().await;
        self.stop_inner(&error, true).await;
        self.shared.lock().snapshot.last_error = Some(error);
    }
    async fn stop_inner(&self, reason: &str, notify: bool) {
        let (worker, peers) = {
            let mut state = self.shared.lock();
            state.epoch = state.epoch.wrapping_add(1);
            let worker = state.worker.take();
            if let Some(worker) = &worker {
                worker.cancel();
            }
            let mut peers: HashSet<String> = state.listeners.drain().collect();
            if let Some(peer) = state.snapshot.mic_peer_id.take() {
                peers.insert(peer);
            }
            state.snapshot.mode = "idle".into();
            state.snapshot.active = false;
            state.snapshot.waiting = false;
            state.snapshot.output_device_id = None;
            state.snapshot.last_error = None;
            state.stream_id = None;
            // Pending microphone requests are not hardware and remain until answered/cancelled.
            (worker, peers)
        };
        if notify {
            for peer in peers {
                let _ = self.shared.send(
                    &peer,
                    AudioControl::Stop {
                        reason: reason.into(),
                    },
                );
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
        if let Some(worker) = &self.shared.lock().worker {
            worker.cancel();
        }
    }
}
