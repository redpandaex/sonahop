use crate::{
    audio::{AudioSnapshot, AudioSource},
    desktop::{DesktopAction, DesktopSessionSnapshot},
};
use sonahop_link::{ClientState, LinkRole, LinkSnapshot};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Idle,
    Searching,
    Busy,
    Active,
    Error,
}

#[derive(PartialEq, Eq)]
pub struct View {
    pub status: String,
    pub primary: &'static str,
    pub alternate: &'static str,
    pub tone: Tone,
    pub enabled: bool,
}

pub fn primary_action(link: &LinkSnapshot, audio: &AudioSnapshot) -> DesktopAction {
    match link.role {
        LinkRole::Host if link.running || audio.active || audio.waiting => {
            DesktopAction::StopSharing
        }
        LinkRole::Host => DesktopAction::StartSharing,
        LinkRole::Client
            if !link.peers.is_empty() || link.client_state == ClientState::Connecting =>
        {
            DesktopAction::Disconnect
        }
        LinkRole::Client => DesktopAction::ConnectDevices,
    }
}

pub fn present(
    link: &LinkSnapshot,
    audio: &AudioSnapshot,
    operation: &DesktopSessionSnapshot,
) -> View {
    let action = primary_action(link, audio);
    let primary = match action {
        DesktopAction::StartSharing => "开始共享系统声音",
        DesktopAction::StopSharing => "停止共享",
        DesktopAction::Disconnect => "断开连接",
        DesktopAction::ConnectDevices => "连接设备…",
    };
    let alternate = if link.role == LinkRole::Host {
        "收听其他设备…"
    } else {
        "共享系统声音"
    };
    let (mut status, mut tone) = if link.role == LinkRole::Host {
        if audio.listen.active || audio.listen.waiting {
            let source = if audio.send_source == AudioSource::Microphone {
                "麦克风"
            } else {
                "系统声音"
            };
            (
                format!("正在共享{source} · {} 台设备", link.peers.len()),
                Tone::Active,
            )
        } else if link.running {
            ("共享已就绪 · 等待音源".into(), Tone::Searching)
        } else {
            ("已停止共享".into(), Tone::Idle)
        }
    } else if !link.peers.is_empty() {
        let name = &link.peers[0].name;
        if audio.active {
            let use_name = if audio.listen.active {
                "正在收听"
            } else {
                "正在上传麦克风"
            };
            (format!("{use_name} · {name}"), Tone::Active)
        } else {
            (format!("已连接 · {name}"), Tone::Searching)
        }
    } else if link.client_state == ClientState::Connecting {
        ("正在连接设备…".into(), Tone::Busy)
    } else if link.running {
        let text = if operation.action == Some(DesktopAction::Disconnect) {
            "已断开 · 正在搜索设备"
        } else {
            "正在搜索设备"
        };
        (text.into(), Tone::Searching)
    } else {
        ("未连接设备".into(), Tone::Idle)
    };
    if let Some(error) = operation
        .error
        .as_ref()
        .or(link.last_error.as_ref())
        .or(audio.last_error.as_ref())
    {
        status = format!("操作失败：{error}");
        tone = Tone::Error;
    }
    if operation.busy {
        status = match operation.action {
            Some(DesktopAction::StartSharing) => "正在开启共享…",
            Some(DesktopAction::StopSharing) => "正在停止共享…",
            Some(DesktopAction::Disconnect) => "正在断开连接…",
            _ => "正在切换到收听…",
        }
        .into();
        tone = Tone::Busy;
    }
    View {
        status,
        primary,
        alternate,
        tone,
        enabled: !operation.busy,
    }
}
