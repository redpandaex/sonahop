use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionResponse {
    pub granted: bool,
    pub device_name: String,
}
/// Backend capabilities, not a report of the user's runtime permission grants.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioCapabilities {
    pub playback_capture: bool,
    pub microphone_capture: bool,
}
