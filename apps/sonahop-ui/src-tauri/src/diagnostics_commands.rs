use serde::Serialize;
use sonahop_link::LinkService;
use std::sync::Arc;
use tauri::State;
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    log_path: Option<String>,
    log_error: Option<String>,
    recent_log: String,
    version: &'static str,
}
#[tauri::command]
pub fn get_diagnostics(link: State<'_, Arc<LinkService>>) -> DiagnosticReport {
    DiagnosticReport {
        log_path: link.diagnostics_path(),
        log_error: link.diagnostics_error(),
        recent_log: link.diagnostic_log(),
        version: env!("CARGO_PKG_VERSION"),
    }
}
