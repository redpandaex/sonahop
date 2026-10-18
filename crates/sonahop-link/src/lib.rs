//! Anonymous X25519-encrypted SonaHop v3 LAN control and media transport.
mod crypto;
mod diagnostics;
mod discovery;
mod interfaces;
mod network;
mod protocol;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, mpsc, watch};
use tokio::task::JoinHandle;

pub const DISCOVERY_PORT: u16 = 32992;
pub const CONTROL_PORT: u16 = 32993;
pub const MAX_MEDIA_BYTES: usize = 1400;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkRole {
    Host,
    Client,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientState {
    Idle,
    Discovering,
    Connecting,
    Connected,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredHost {
    pub id: String,
    pub name: String,
    pub address: String,
    pub port: u16,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPeer {
    pub id: String,
    pub name: String,
    pub address: String,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkSnapshot {
    pub role: LinkRole,
    pub running: bool,
    pub local_name: String,
    pub local_addresses: Vec<String>,
    pub port: u16,
    pub discovered: Vec<DiscoveredHost>,
    pub peers: Vec<LinkPeer>,
    pub client_state: ClientState,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", content = "message", rename_all = "camelCase")]
pub enum DisconnectReason {
    Local,
    Remote,
    Failed(String),
}

#[derive(Clone, Debug)]
pub enum LinkEvent {
    Connected {
        peer_id: String,
    },
    Disconnected {
        peer_id: String,
        reason: DisconnectReason,
    },
    Control {
        peer_id: String,
        payload: Value,
    },
    Media {
        peer_id: String,
        payload: Vec<u8>,
    },
}

pub(crate) enum Command {
    Disconnect,
}
pub(crate) struct MediaEnvelope {
    pub payload: Vec<u8>,
    pub queued_at: Instant,
}
pub(crate) struct Control {
    pub sender: mpsc::Sender<Command>,
    pub controls: mpsc::Sender<Value>,
    pub media: mpsc::Sender<MediaEnvelope>,
    pub connected: bool,
    pub local_stop_requested: bool,
    pub remote_stop_received: bool,
}
pub(crate) struct Outbound {
    pub commands: mpsc::Receiver<Command>,
    pub controls: mpsc::Receiver<Value>,
    pub media: mpsc::Receiver<MediaEnvelope>,
}
pub(crate) fn channels() -> (Control, Outbound) {
    let (sender, commands) = mpsc::channel(2);
    let (controls_tx, controls) = mpsc::channel(16);
    let (media_tx, media) = mpsc::channel(3);
    (
        Control {
            sender,
            controls: controls_tx,
            media: media_tx,
            connected: false,
            local_stop_requested: false,
            remote_stop_received: false,
        },
        Outbound {
            commands,
            controls,
            media,
        },
    )
}
pub(crate) struct State {
    pub snapshot: LinkSnapshot,
    pub generation: u64,
    pub stopping: bool,
    pub discovered: HashMap<String, (DiscoveredHost, Instant)>,
    pub controls: HashMap<String, Control>,
    pub tasks: Vec<JoinHandle<()>>,
    pub cancel: Option<watch::Sender<bool>>,
}
pub(crate) struct Inner {
    pub state: Mutex<State>,
    pub events: broadcast::Sender<LinkEvent>,
    pub diagnostics: diagnostics::Diagnostics,
    lifecycle: tokio::sync::Mutex<()>,
}
impl Inner {
    pub fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
pub(crate) fn current(state: &State, generation: u64) -> bool {
    state.generation == generation && state.snapshot.running
}
pub(crate) fn spawn(
    inner: &Arc<Inner>,
    generation: u64,
    task: impl std::future::Future<Output = ()> + Send + 'static,
) {
    let mut state = inner.lock();
    if current(&state, generation) {
        state.tasks.retain(|task| !task.is_finished());
        state.tasks.push(tokio::spawn(task));
    }
}
fn shutdown(
    state: &mut State,
    events: &broadcast::Sender<LinkEvent>,
    reason: DisconnectReason,
) -> Vec<JoinHandle<()>> {
    state.generation = state.generation.wrapping_add(1);
    if let Some(cancel) = state.cancel.take() {
        let _ = cancel.send(true);
    }
    for peer in &state.snapshot.peers {
        let _ = events.send(LinkEvent::Disconnected {
            peer_id: peer.id.clone(),
            reason: reason.clone(),
        });
    }
    state.snapshot.running = false;
    state.stopping = false;
    state.snapshot.discovered.clear();
    state.snapshot.peers.clear();
    state.snapshot.client_state = ClientState::Idle;
    state.discovered.clear();
    state.controls.clear();
    let tasks = std::mem::take(&mut state.tasks);
    for task in &tasks {
        task.abort();
    }
    tasks
}
// Preserve aborted handles when a start/stop future is itself cancelled during teardown.
struct TaskDrain {
    inner: Arc<Inner>,
    generation: u64,
    tasks: Vec<JoinHandle<()>>,
}
impl Drop for TaskDrain {
    fn drop(&mut self) {
        let mut state = self.inner.lock();
        for task in self.tasks.drain(..) {
            if task.is_finished() {
                continue;
            }
            task.abort();
            if state.generation == self.generation && !state.snapshot.running {
                state.tasks.push(task);
            }
        }
    }
}
async fn drain_tasks(inner: &Arc<Inner>) {
    let (generation, tasks) = {
        let mut state = inner.lock();
        (state.generation, std::mem::take(&mut state.tasks))
    };
    let mut drain = TaskDrain {
        inner: inner.clone(),
        generation,
        tasks,
    };
    for task in &mut drain.tasks {
        task.abort();
        let _ = task.await;
    }
}
pub(crate) fn fail_service(inner: &Inner, generation: u64, error: String) {
    let mut state = inner.lock();
    if current(&state, generation) {
        inner.diagnostics.log(
            "link-failed",
            serde_json::json!({"generation":generation,"reason":error}),
        );
        state.tasks = shutdown(
            &mut state,
            &inner.events,
            DisconnectReason::Failed(error.clone()),
        );
        state.snapshot.last_error = Some(error);
    }
}
pub(crate) fn finish_connection(inner: &Inner, generation: u64, id: &str, error: Option<String>) {
    let mut state = inner.lock();
    if !current(&state, generation) {
        return;
    }
    let Some(control) = state.controls.remove(id) else {
        return;
    };
    let connected = state.snapshot.peers.iter().any(|peer| peer.id == id);
    let reason = if control.local_stop_requested {
        Some(DisconnectReason::Local)
    } else if control.remote_stop_received {
        Some(DisconnectReason::Remote)
    } else {
        error
            .clone()
            .map(DisconnectReason::Failed)
            .or_else(|| connected.then(|| DisconnectReason::Failed("连接意外结束".into())))
    };
    state.snapshot.peers.retain(|peer| peer.id != id);
    if connected {
        if let Some(reason) = &reason {
            let _ = inner.events.send(LinkEvent::Disconnected {
                peer_id: id.to_owned(),
                reason: reason.clone(),
            });
        }
    }
    let user_error = match &reason {
        Some(DisconnectReason::Failed(message)) => Some(message.clone()),
        _ => None,
    };
    // Unpublished host handshakes belong in diagnostics, not the host's session error.
    if connected {
        if let Some(message) = &user_error {
            state.snapshot.last_error = Some(message.clone());
        }
    }
    inner.diagnostics.log(
        "connection-ended",
        serde_json::json!({"peerId":id,"generation":generation,"connected":connected,"reason":reason,"transportError":error}),
    );
    if state.snapshot.role == LinkRole::Client {
        state.snapshot.client_state = ClientState::Discovering;
        state.snapshot.last_error = user_error;
    }
}
pub(crate) fn publish_connected(inner: &Inner, generation: u64, peer: LinkPeer) -> bool {
    let mut state = inner.lock();
    if !current(&state, generation) || state.stopping {
        return false;
    }
    let Some(control) = state.controls.get_mut(&peer.id) else {
        return false;
    };
    if control.local_stop_requested {
        return false;
    }
    control.connected = true;
    if state.snapshot.role == LinkRole::Client {
        state.snapshot.client_state = ClientState::Connected;
    }
    let id = peer.id.clone();
    state.snapshot.peers.push(peer);
    let _ = inner.events.send(LinkEvent::Connected { peer_id: id });
    true
}
pub(crate) fn publish_payload(inner: &Inner, generation: u64, id: &str, event: LinkEvent) {
    let state = inner.lock();
    if current(&state, generation) && state.snapshot.peers.iter().any(|peer| peer.id == id) {
        let _ = inner.events.send(event);
    }
}

/// Construction starts no tasks. Async methods must run on the application's Tokio runtime.
pub struct LinkService {
    inner: Arc<Inner>,
}
impl LinkService {
    pub fn new(role: LinkRole) -> Self {
        let (events, _) = broadcast::channel(128);
        Self {
            inner: Arc::new(Inner {
                diagnostics: diagnostics::Diagnostics::default(),
                lifecycle: tokio::sync::Mutex::new(()),
                events,
                state: Mutex::new(State {
                    snapshot: LinkSnapshot {
                        role,
                        running: false,
                        local_name: String::new(),
                        local_addresses: Vec::new(),
                        port: CONTROL_PORT,
                        discovered: Vec::new(),
                        peers: Vec::new(),
                        client_state: ClientState::Idle,
                        last_error: None,
                    },
                    generation: 0,
                    stopping: false,
                    discovered: HashMap::new(),
                    controls: HashMap::new(),
                    tasks: Vec::new(),
                    cancel: None,
                }),
            }),
        }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<LinkEvent> {
        self.inner.events.subscribe()
    }
    pub fn configure_logging(&self, path: std::path::PathBuf) -> Result<(), String> {
        self.inner.diagnostics.configure(path)
    }
    pub fn diagnostics_path(&self) -> Option<String> {
        self.inner.diagnostics.path()
    }
    pub fn diagnostics_error(&self) -> Option<String> {
        self.inner.diagnostics.error()
    }
    pub fn diagnostic_log(&self) -> String {
        self.inner.diagnostics.recent()
    }
    pub fn log_event(&self, event: &str, fields: Value) {
        self.inner.diagnostics.log(event, fields);
    }
    pub fn snapshot(&self) -> LinkSnapshot {
        let state = self.inner.lock();
        let mut snapshot = state.snapshot.clone();
        snapshot.discovered.retain(|host| {
            state
                .discovered
                .get(&host.id)
                .is_some_and(|(_, seen)| seen.elapsed() < std::time::Duration::from_secs(10))
        });
        snapshot
    }
    pub async fn set_role(&self, role: LinkRole) -> Result<LinkSnapshot, String> {
        let _lifecycle = self.inner.lifecycle.lock().await;
        {
            let mut state = self.inner.lock();
            if state.snapshot.running || !state.controls.is_empty() {
                return Err("切换角色前请停止当前连接".into());
            }
            state.snapshot.role = role;
            state.snapshot.client_state = ClientState::Idle;
            state.snapshot.last_error = None;
            state.snapshot.discovered.clear();
            state.discovered.clear();
        }
        self.log_event("role-changed", serde_json::json!({"role":role}));
        Ok(self.snapshot())
    }
    pub async fn start(&self, name: String) -> Result<LinkSnapshot, String> {
        let _lifecycle = self.inner.lifecycle.lock().await;
        let name = name.trim().to_owned();
        if !protocol::valid_name(&name) {
            return Err("设备名称应为 1–64 个字符，且不包含控制字符".into());
        }
        let running = self.inner.lock().snapshot.running;
        if running {
            return Ok(self.snapshot());
        }
        drain_tasks(&self.inner).await;
        let role = self.inner.lock().snapshot.role;
        let sockets = match network::bind(role).await {
            Ok(sockets) => sockets,
            Err(error) => {
                self.inner.lock().snapshot.last_error = Some(error.clone());
                return Err(error);
            }
        };
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let generation = {
            let mut state = self.inner.lock();
            state.generation = state.generation.wrapping_add(1);
            state.stopping = false;
            state.snapshot.running = true;
            state.snapshot.local_name = name;
            state.snapshot.last_error = None;
            state.snapshot.client_state = if role == LinkRole::Client {
                ClientState::Discovering
            } else {
                ClientState::Idle
            };
            state.cancel = Some(cancel_tx);
            state.generation
        };
        self.log_event(
            "link-start",
            serde_json::json!({"role":role,"port":CONTROL_PORT}),
        );
        network::launch(&self.inner, generation, sockets, cancel_rx);
        Ok(self.snapshot())
    }
    /// Record the local intent before callers wait for audio/device cleanup.
    pub fn begin_stop(&self) {
        let mut state = self.inner.lock();
        let first_request = !state.stopping;
        state.stopping = true;
        for control in state.controls.values_mut() {
            if !control.local_stop_requested {
                control.local_stop_requested = true;
                let _ = control.sender.try_send(Command::Disconnect);
            }
        }
        if first_request {
            self.inner.diagnostics.log(
                "link-stop-requested",
                serde_json::json!({"generation":state.generation,"connections":state.controls.len()}),
            );
        }
    }
    pub async fn stop(&self) {
        let _lifecycle = self.inner.lifecycle.lock().await;
        self.begin_stop();
        self.log_event("link-stop", serde_json::json!({}));
        let grace = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                // A client may still be reading Accepted while the host has published it.
                // Let pending handshakes reach the stopping check and send Bye as well.
                if self.inner.lock().controls.is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await;
        if grace.is_err() {
            self.log_event(
                "link-stop-grace-expired",
                serde_json::json!({"timeoutMs":2000}),
            );
        }
        {
            let mut state = self.inner.lock();
            state.snapshot.last_error = None;
            state.tasks = shutdown(&mut state, &self.inner.events, DisconnectReason::Local);
        }
        drain_tasks(&self.inner).await;
    }
    pub async fn connect(&self, address: String, port: u16) -> Result<(), String> {
        let _lifecycle = self.inner.lifecycle.lock().await;
        let address = protocol::local_target(&address, port)?;
        let id = uuid::Uuid::new_v4().to_string();
        let (control, outbound) = channels();
        let (generation, name, cancel) = {
            let mut state = self.inner.lock();
            if state.snapshot.role != LinkRole::Client {
                return Err("只有客户端可主动连接".into());
            }
            if !state.snapshot.running {
                return Err("请先启动局域网搜索".into());
            }
            if state.stopping {
                return Err("连接正在停止，请稍后重试".into());
            }
            if !state.controls.is_empty() {
                return Err("请先断开当前连接或等待连接结束".into());
            }
            let cancel = state.cancel.as_ref().ok_or("发现服务已停止")?.subscribe();
            state.controls.insert(id.clone(), control);
            state.snapshot.client_state = ClientState::Connecting;
            state.snapshot.last_error = None;
            (state.generation, state.snapshot.local_name.clone(), cancel)
        };
        let inner = self.inner.clone();
        spawn(&self.inner, generation, async move {
            network::client_connection(inner, generation, id, name, address, outbound, cancel)
                .await;
        });
        Ok(())
    }
    pub fn disconnect_peer(&self, id: &str) -> Result<(), String> {
        let mut state = self.inner.lock();
        let Some(control) = state.controls.get_mut(id) else {
            return Ok(());
        };
        if control.local_stop_requested {
            return Ok(());
        }
        // Persist the user's intent before a concurrently ready read/write can finish.
        control.local_stop_requested = true;
        let _ = control.sender.try_send(Command::Disconnect);
        self.inner.diagnostics.log(
            "peer-stop-requested",
            serde_json::json!({"peerId":id,"generation":state.generation}),
        );
        Ok(())
    }
    pub fn send_control(&self, peer_id: &str, payload: Value) -> Result<(), String> {
        protocol::validate_control(&payload)?;
        let state = self.inner.lock();
        if !state.snapshot.running || !state.snapshot.peers.iter().any(|peer| peer.id == peer_id) {
            return Err("控制设备未连接".into());
        }
        let control = state
            .controls
            .get(peer_id)
            .filter(|control| control.connected)
            .ok_or("控制设备未连接")?;
        control
            .controls
            .try_send(payload)
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => "控制发送队列已满".into(),
                mpsc::error::TrySendError::Closed(_) => "控制设备已断开".into(),
            })
    }
    pub fn send_media(&self, peer_id: &str, payload: Vec<u8>) -> Result<(), String> {
        if payload.is_empty() || payload.len() > MAX_MEDIA_BYTES {
            return Err("音频包长度应为 1–1400 字节".into());
        }
        let state = self.inner.lock();
        if !state.snapshot.running || !state.snapshot.peers.iter().any(|peer| peer.id == peer_id) {
            return Err("音频设备未连接".into());
        }
        let control = state
            .controls
            .get(peer_id)
            .filter(|control| control.connected)
            .ok_or("音频设备未连接")?;
        control
            .media
            .try_send(MediaEnvelope {
                payload,
                queued_at: Instant::now(),
            })
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => "音频发送队列已满，本帧已丢弃".into(),
                mpsc::error::TrySendError::Closed(_) => "音频设备已断开".into(),
            })
    }
}
impl Drop for LinkService {
    fn drop(&mut self) {
        shutdown(
            &mut self.inner.lock(),
            &self.inner.events,
            DisconnectReason::Local,
        );
    }
}
