use super::AppState;
use seb_core::device::{DeviceState, FrameLog, SimProfile, DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientDefaults {
    pub host: String,
    pub port: u16,
    pub soft_version: String,
    pub profile: SimProfile,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlarmType {
    pub name: String,
    pub code: u8,
    pub hex: String,
}

#[tauri::command]
pub fn list_alarm_types() -> Vec<AlarmType> {
    seb_core::ALARM_TYPES
        .iter()
        .map(|(name, code)| AlarmType {
            name: name.to_string(),
            code: *code,
            hex: format!("0x{code:02X}"),
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPoll {
    pub state: DeviceState,
    pub frames: Vec<FrameLog>,
}

#[tauri::command]
pub fn client_defaults() -> ClientDefaults {
    ClientDefaults {
        host: DEFAULT_GATEWAY_HOST.to_string(),
        port: DEFAULT_GATEWAY_PORT,
        soft_version: seb_core::frame::DEFAULT_SOFT_VERSION.to_string(),
        profile: SimProfile::default(),
    }
}

#[tauri::command]
pub async fn client_connect(
    state: State<'_, AppState>,
    host: String,
    port: u16,
    device_no: String,
    soft_version: String,
    heartbeat: bool,
) -> Result<DeviceState, String> {
    state
        .device
        .connect(&host, port, &device_no, &soft_version, heartbeat)
        .await?;
    Ok(state.device.state())
}

#[tauri::command]
pub async fn client_disconnect(state: State<'_, AppState>) -> Result<DeviceState, String> {
    state.device.disconnect().await;
    Ok(state.device.state())
}

#[tauri::command]
pub fn client_poll(state: State<'_, AppState>) -> ClientPoll {
    ClientPoll {
        state: state.device.state(),
        frames: state.device.drain(),
    }
}

#[tauri::command]
pub fn client_set_profile(state: State<'_, AppState>, profile: SimProfile, auto_reply: bool) {
    state.device.set_profile(profile);
    state.device.set_auto_reply(auto_reply);
}

#[tauri::command]
pub async fn client_send_location(state: State<'_, AppState>) -> Result<(), String> {
    state.device.send_location().await
}

#[tauri::command]
pub async fn client_send_bms(state: State<'_, AppState>) -> Result<(), String> {
    state.device.send_bms().await
}

#[tauri::command]
pub async fn client_send_alarm(
    state: State<'_, AppState>,
    alarm_type: u8,
    label: String,
) -> Result<(), String> {
    state.device.send_alarm(alarm_type, &label).await
}

#[tauri::command]
pub async fn client_send_ping(state: State<'_, AppState>) -> Result<(), String> {
    state.device.send_ping().await
}

#[tauri::command]
pub async fn client_send_reply(
    state: State<'_, AppState>,
    msg_id: Option<String>,
    success: bool,
) -> Result<(), String> {
    state.device.send_reply(msg_id, success).await
}
