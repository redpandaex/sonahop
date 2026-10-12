//! PIN-authenticated, encrypted SonaHop v2 LAN control and media transport.
mod crypto;
mod diagnostics;
mod network;
mod protocol;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;
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
    AwaitingApproval,
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
pub struct LinkRequest {
    pub id: String,
    pub name: String,
    pub address: String,
    pub verification_code: String,
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
    pub port: u16,
    pub discovered: Vec<DiscoveredHost>,
    pub requests: Vec<LinkRequest>,
    pub peers: Vec<LinkPeer>,
    pub client_state: ClientState,
    pub verification_code: Option<String>,
    pub pairing_code: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug)]
pub enum LinkEvent {
    Connected { peer_id: String },
    Disconnected { peer_id: String },
    Control { peer_id: String, payload: Value },
    Media { peer_id: String, payload: Vec<u8> },
}

pub(crate) enum Command {
    Answer(bool),
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
    pub answered: bool,
    pub approval_deadline: Option<Instant>,
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
            answered: false,
            approval_deadline: None,
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
fn shutdown(state: &mut State, events: &broadcast::Sender<LinkEvent>) -> Vec<JoinHandle<()>> {
    state.generation = state.generation.wrapping_add(1);
    if let Some(cancel) = state.cancel.take() {
        let _ = cancel.send(true);
    }
    for peer in &state.snapshot.peers {
        let _ = events.send(LinkEvent::Disconnected {
            peer_id: peer.id.clone(),
        });
    }
    state.snapshot.running = false;
    state.snapshot.discovered.clear();
    state.snapshot.requests.clear();
    state.snapshot.peers.clear();
    state.snapshot.client_state = ClientState::Idle;
    state.snapshot.verification_code = None;
    state.snapshot.pairing_code = None;
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
        state.tasks = shutdown(&mut state, &inner.events);
        state.snapshot.last_error = Some(error);
    }
}
pub(crate) fn finish_connection(inner: &Inner, generation: u64, id: &str, error: Option<String>) {
    let mut state = inner.lock();
    if !current(&state, generation) {
        return;
    }
    let connected = state.snapshot.peers.iter().any(|peer| peer.id == id);
    state.controls.remove(id);
    state.snapshot.requests.retain(|request| request.id != id);
    state.snapshot.peers.retain(|peer| peer.id != id);
    if connected {
        let _ = inner.events.send(LinkEvent::Disconnected {
            peer_id: id.to_owned(),
        });
    }
    if let Some(message) = &error {
        state.snapshot.last_error = Some(message.clone());
    }
    inner.diagnostics.log(
        "connection-ended",
        serde_json::json!({"peerId": id, "connected": connected, "reason": error}),
    );
    if state.snapshot.role == LinkRole::Client {
        state.snapshot.client_state = ClientState::Discovering;
        state.snapshot.verification_code = None;
        state.snapshot.last_error = error;
    }
}
pub(crate) fn publish_connected(inner: &Inner, generation: u64, peer: LinkPeer) -> bool {
    let mut state = inner.lock();
    if !current(&state, generation) {
        return false;
    }
    let Some(control) = state.controls.get_mut(&peer.id) else {
        return false;
    };
    control.connected = true;
    control.approval_deadline = None;
    state
        .snapshot
        .requests
        .retain(|request| request.id != peer.id);
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
                        port: CONTROL_PORT,
                        discovered: Vec::new(),
                        requests: Vec::new(),
                        peers: Vec::new(),
                        client_state: ClientState::Idle,
                        verification_code: None,
                        pairing_code: None,
                        last_error: None,
                    },
                    generation: 0,
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
        snapshot.requests.retain(|request| {
            state.controls.get(&request.id).is_some_and(|control| {
                control
                    .approval_deadline
                    .is_none_or(|deadline| Instant::now() < deadline)
            })
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
        let pairing_code = if role == LinkRole::Host {
            Some(crypto::generate_pin()?)
        } else {
            None
        };
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
            state.snapshot.running = true;
            state.snapshot.local_name = name;
            state.snapshot.pairing_code = pairing_code;
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
    pub async fn stop(&self) {
        let _lifecycle = self.inner.lifecycle.lock().await;
        self.log_event("link-stop", serde_json::json!({}));
        {
            let mut state = self.inner.lock();
            state.snapshot.last_error = None;
            state.tasks = shutdown(&mut state, &self.inner.events);
        }
        drain_tasks(&self.inner).await;
    }
    pub async fn connect(
        &self,
        address: String,
        port: u16,
        pairing_code: String,
    ) -> Result<(), String> {
        let _lifecycle = self.inner.lifecycle.lock().await;
        let address = protocol::local_target(&address, port)?;
        if !crypto::valid_pin(&pairing_code) {
            return Err("请输入 Windows 显示的 6 位 PIN".into());
        }
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
            if !state.controls.is_empty() {
                return Err("请先断开当前连接或等待请求结束".into());
            }
            let cancel = state.cancel.as_ref().ok_or("发现服务已停止")?.subscribe();
            state.controls.insert(id.clone(), control);
            state.snapshot.client_state = ClientState::Connecting;
            state.snapshot.verification_code = None;
            state.snapshot.last_error = None;
            (state.generation, state.snapshot.local_name.clone(), cancel)
        };
        let inner = self.inner.clone();
        spawn(&self.inner, generation, async move {
            network::client_connection(
                inner,
                generation,
                id,
                name,
                address,
                pairing_code,
                outbound,
                cancel,
            )
            .await;
        });
        Ok(())
    }
    pub fn answer_request(&self, id: &str, approve: bool) -> Result<(), String> {
        let mut state = self.inner.lock();
        if state.snapshot.role != LinkRole::Host || !state.snapshot.running {
            return Err("主机发现服务未运行".into());
        }
        if !state
            .snapshot
            .requests
            .iter()
            .any(|request| request.id == id)
        {
            return Err("连接请求已结束".into());
        }
        let control = state.controls.get_mut(id).ok_or("连接请求已结束")?;
        if control
            .approval_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err("连接请求已超过 60 秒".into());
        }
        if control.answered {
            return Err("连接请求已处理".into());
        }
        control
            .sender
            .try_send(Command::Answer(approve))
            .map_err(|_| "连接请求已结束")?;
        control.answered = true;
        Ok(())
    }
    pub fn disconnect_peer(&self, id: &str) -> Result<(), String> {
        let state = self.inner.lock();
        if !state.snapshot.peers.iter().any(|peer| peer.id == id) {
            return Err("设备已断开".into());
        }
        state
            .controls
            .get(id)
            .ok_or("设备已断开")?
            .sender
            .try_send(Command::Disconnect)
            .map_err(|_| "设备已断开".to_owned())
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
        shutdown(&mut self.inner.lock(), &self.inner.events);
    }
}
