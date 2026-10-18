use crate::{
    audio::{AudioService, AudioSource},
    desktop::{DesktopAction, DesktopSessionSnapshot, DesktopSettings},
    session_commands::OperationGate,
    tray_presentation::{self, Tone, View},
};
use sonahop_link::{LinkRole, LinkService};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tauri::{
    AppHandle, CloseRequestApi, Emitter, Manager,
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub const TRAY_ID: &str = "sonahop-tray";
const SESSION_EVENT: &str = "sonahop://desktop-session";

pub struct TrayController {
    operation: Mutex<DesktopSessionSnapshot>,
    exiting: AtomicBool,
    status: MenuItem<tauri::Wry>,
    primary: MenuItem<tauri::Wry>,
    alternate: MenuItem<tauri::Wry>,
    last_view: Mutex<Option<View>>,
}

impl TrayController {
    pub fn snapshot(&self) -> DesktopSessionSnapshot {
        self.operation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn clear_error(&self) {
        self.operation
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .error = None;
    }

    pub fn is_busy(&self) -> bool {
        self.snapshot().busy
    }
}

fn log_error(app: &AppHandle, action: &str, error: impl std::fmt::Display) {
    app.state::<Arc<LinkService>>()
        .log_event(action, serde_json::json!({ "reason": error.to_string() }));
}

fn show_window(app: &AppHandle, page: Option<&str>) {
    if let Some(window) = app.get_webview_window("main") {
        let navigation = match page {
            Some("settings") => window.eval("window.location.hash = 'settings'"),
            Some("workspace") => window.eval("window.location.hash = 'workspace'"),
            _ => Ok(()),
        };
        if let Err(error) = navigation
            .and_then(|_| window.show())
            .and_then(|_| window.unminimize())
            .and_then(|_| window.set_focus())
        {
            log_error(app, "tray-restore-failed", error);
        }
    }
}

// Composited in memory as a native status badge; no generated image assets are stored.
fn status_icon(base: &Image<'_>, tone: Tone) -> Image<'static> {
    let mut rgba = base.rgba().to_vec();
    let (width, height) = (base.width(), base.height());
    let radius = (width.min(height) as f32 * 0.23).max(2.0);
    let (cx, cy) = (width as f32 - radius - 1.0, height as f32 - radius - 1.0);
    let color = match tone {
        Tone::Idle => [100, 116, 139, 255],
        Tone::Searching => [37, 99, 235, 255],
        Tone::Busy => [217, 119, 6, 255],
        Tone::Active => [22, 163, 74, 255],
        Tone::Error => [220, 38, 38, 255],
    };
    for y in 0..height {
        for x in 0..width {
            let distance = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            if distance <= radius {
                let offset = ((y * width + x) * 4) as usize;
                let pixel = if distance > radius - 1.3 {
                    [255, 255, 255, 255]
                } else {
                    color
                };
                rgba[offset..offset + 4].copy_from_slice(&pixel);
            }
        }
    }
    Image::new_owned(rgba, width, height)
}

// All native menu/image updates run on the UI thread, outside OperationGate.
fn refresh_now(app: &AppHandle) {
    let Some(controller) = app.try_state::<Arc<TrayController>>() else {
        return;
    };
    if controller.exiting.load(Ordering::Acquire) {
        return;
    }
    let link = app.state::<Arc<LinkService>>().snapshot();
    let audio = app.state::<Arc<AudioService>>().snapshot();
    let view = tray_presentation::present(&link, &audio, &controller.snapshot());
    {
        let last = controller
            .last_view
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if last.as_ref() == Some(&view) {
            return;
        }
    }
    let update = || -> tauri::Result<()> {
        controller.status.set_text(&view.status)?;
        controller.primary.set_text(view.primary)?;
        controller.primary.set_enabled(view.enabled)?;
        controller.alternate.set_text(view.alternate)?;
        controller.alternate.set_enabled(view.enabled)?;
        if let Some(tray) = app.tray_by_id(TRAY_ID) {
            let tooltip: String = format!("SonaHop\n{}", view.status)
                .chars()
                .take(120)
                .collect();
            tray.set_tooltip(Some(tooltip))?;
            if let Some(base) = app.default_window_icon() {
                tray.set_icon(Some(status_icon(base, view.tone)))?;
            }
        }
        Ok(())
    };
    match update() {
        Ok(()) => {
            *controller
                .last_view
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(view)
        }
        Err(error) => log_error(app, "tray-update-failed", error),
    }
}

fn schedule_refresh(app: &AppHandle) {
    let handle = app.clone();
    if let Err(error) = app.run_on_main_thread(move || refresh_now(&handle)) {
        log_error(app, "tray-update-dispatch-failed", error);
    }
}

fn check_running(controller: &TrayController) -> Result<(), String> {
    if controller.exiting.load(Ordering::Acquire) {
        Err("程序正在退出".into())
    } else {
        Ok(())
    }
}

async fn perform_action(
    app: &AppHandle,
    action: DesktopAction,
    controller: &TrayController,
) -> Result<(), String> {
    let gate = app.state::<OperationGate>();
    let _guard = gate.0.lock().await;
    check_running(controller)?;
    let link = app.state::<Arc<LinkService>>();
    let audio = app.state::<Arc<AudioService>>();
    let name = {
        let existing = link.snapshot().local_name;
        if existing.is_empty() {
            std::env::var("COMPUTERNAME").unwrap_or_else(|_| "SonaHop 设备".into())
        } else {
            existing
        }
    };
    link.begin_stop();
    let stopped = audio.stop().await;
    link.stop().await;
    if let Some(error) = stopped.last_error {
        return Err(error);
    }
    check_running(controller)?;
    let role = match action {
        DesktopAction::StartSharing | DesktopAction::StopSharing => LinkRole::Host,
        DesktopAction::ConnectDevices | DesktopAction::Disconnect => LinkRole::Client,
    };
    link.set_role(role).await?;
    audio.set_role(role).await?;
    if action != DesktopAction::StopSharing {
        check_running(controller)?;
        link.start(name).await?;
    }
    if action == DesktopAction::StartSharing {
        check_running(controller)?;
        if let Err(error) = audio
            .start_listening(Some(AudioSource::System), None, None)
            .await
        {
            link.begin_stop();
            audio.stop().await;
            link.stop().await;
            return Err(error);
        }
    }
    Ok(())
}

fn begin_action(app: &AppHandle, action: DesktopAction) {
    let controller = app.state::<Arc<TrayController>>().inner().clone();
    let revision;
    {
        let mut operation = controller
            .operation
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if operation.busy || controller.exiting.load(Ordering::Acquire) {
            return;
        }
        revision = app.state::<Arc<AudioService>>().invalidate_pending_starts();
        *operation = DesktopSessionSnapshot {
            revision,
            action: Some(action),
            busy: true,
            role: Some(app.state::<Arc<LinkService>>().snapshot().role),
            error: None,
        };
    }
    // Stop admissions immediately, even while an older user command is draining.
    app.state::<Arc<LinkService>>().begin_stop();
    let _ = app.emit(SESSION_EVENT, controller.snapshot());
    refresh_now(app);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = perform_action(&handle, action, &controller).await;
        if controller.exiting.load(Ordering::Acquire) {
            return;
        }
        {
            let mut operation = controller
                .operation
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if operation.revision != revision {
                return;
            }
            operation.busy = false;
            operation.role = Some(handle.state::<Arc<LinkService>>().snapshot().role);
            operation.error = result.err();
        }
        let snapshot = controller.snapshot();
        if let Some(error) = snapshot.error.as_ref() {
            log_error(&handle, "tray-action-failed", error);
        }
        let _ = handle.emit(SESSION_EVENT, snapshot.clone());
        schedule_refresh(&handle);
        if snapshot.error.is_some() || action == DesktopAction::ConnectDevices {
            let handle_copy = handle.clone();
            let page = if action == DesktopAction::ConnectDevices {
                Some("workspace")
            } else {
                None
            };
            let _ = handle.run_on_main_thread(move || show_window(&handle_copy, page));
        }
    });
}

pub fn install(app: &AppHandle) -> tauri::Result<()> {
    let status = MenuItem::with_id(app, "tray-status", "准备中…", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "tray-open", "打开主窗口", true, None::<&str>)?;
    let primary = MenuItem::with_id(app, "tray-primary", "开始共享系统声音", true, None::<&str>)?;
    let alternate = MenuItem::with_id(app, "tray-alternate", "收听其他设备…", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "tray-settings", "设置", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "tray-quit", "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let last_separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &open,
            &separator,
            &primary,
            &alternate,
            &last_separator,
            &settings,
            &quit,
        ],
    )?;
    let icon = app.default_window_icon().ok_or_else(|| {
        tauri::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "缺少托盘图标",
        ))
    })?;
    let controller = Arc::new(TrayController {
        operation: Mutex::new(DesktopSessionSnapshot::default()),
        exiting: AtomicBool::new(false),
        status,
        primary,
        alternate,
        last_view: Mutex::new(None),
    });
    app.manage(controller.clone());
    TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("SonaHop")
        .icon(icon.clone())
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray-open" => show_window(app, None),
            "tray-settings" => show_window(app, Some("settings")),
            "tray-quit" => app.exit(0),
            "tray-primary" => {
                let link = app.state::<Arc<LinkService>>().snapshot();
                let audio = app.state::<Arc<AudioService>>().snapshot();
                begin_action(app, tray_presentation::primary_action(&link, &audio));
            }
            "tray-alternate" => {
                let role = app.state::<Arc<LinkService>>().snapshot().role;
                begin_action(
                    app,
                    if role == LinkRole::Host {
                        DesktopAction::ConnectDevices
                    } else {
                        DesktopAction::StartSharing
                    },
                );
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_window(tray.app_handle(), None);
            }
        })
        .build(app)?;
    refresh_now(app);
    let weak = Arc::downgrade(&controller);
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            let Some(controller) = weak.upgrade() else {
                break;
            };
            if controller.exiting.load(Ordering::Acquire) {
                break;
            }
            schedule_refresh(&handle);
        }
    });
    Ok(())
}

pub fn begin_exit(app: &AppHandle) {
    if let Some(controller) = app.try_state::<Arc<TrayController>>() {
        controller.exiting.store(true, Ordering::Release);
    }
}

pub fn close_requested(app: &AppHandle, label: &str, api: &CloseRequestApi) {
    if label != "main"
        || app.tray_by_id(TRAY_ID).is_none()
        || !app.state::<DesktopSettings>().closes_to_tray()
    {
        return;
    }
    if let Some(window) = app.get_webview_window(label) {
        match window.hide() {
            Ok(()) => api.prevent_close(),
            Err(error) => log_error(app, "tray-hide-failed", error),
        }
    }
}
