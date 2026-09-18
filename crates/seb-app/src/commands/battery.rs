use super::AppState;
use seb_core::battery::{
    BatteryConfig, BatteryState, DEFAULT_BATTERY_HOST, DEFAULT_BATTERY_NO, DEFAULT_BATTERY_PORT,
    DEFAULT_COORDINATES, DEFAULT_ICCID,
};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryDefaults {
    pub battery_no: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryFrame {
    pub battery_no: String,
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatteryPoll {
    pub devices: Vec<BatteryState>,
    pub frames: Vec<BatteryFrame>,
}

#[tauri::command]
pub fn battery_defaults() -> BatteryDefaults {
    BatteryDefaults {
        battery_no: DEFAULT_BATTERY_NO.to_string(),
        env: "内网".to_string(),
        host: DEFAULT_BATTERY_HOST.to_string(),
        port: DEFAULT_BATTERY_PORT,
        iccid: DEFAULT_ICCID.to_string(),
        coordinates: DEFAULT_COORDINATES.to_string(),
    }
}

async fn persist(state: &AppState) {
    let mut publisher = state.publisher.lock().await;
    let mut cfg = publisher.config().clone();
    cfg.sim_batteries = state.batteries.roster();
    let _ = seb_core::config::save(&cfg);
    publisher.set_config(cfg).await;
}

#[tauri::command]
pub fn battery_devices(state: State<'_, AppState>) -> Vec<BatteryState> {
    state.batteries.states()
}

#[tauri::command]
pub async fn battery_update_device(
    state: State<'_, AppState>,
    config: BatteryConfig,
) -> Result<Vec<BatteryState>, String> {
    if config.battery_no.trim().is_empty() {
        return Err("电池编号不能为空".to_string());
    }
    state.batteries.upsert(config);
    persist(&state).await;
    Ok(state.batteries.states())
}

#[tauri::command]
pub async fn battery_remove_device(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<Vec<BatteryState>, String> {
    if let Some(link) = state.batteries.remove(&battery_no) {
        link.disconnect().await;
    }
    persist(&state).await;
    Ok(state.batteries.states())
}

#[tauri::command]
pub async fn battery_connect(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<BatteryState, String> {
    let link = state.batteries.get(&battery_no)?;
    link.connect().await?;
    Ok(link.state())
}

#[tauri::command]
pub async fn battery_disconnect(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<BatteryState, String> {
    let link = state.batteries.get(&battery_no)?;
    link.disconnect().await;
    Ok(link.state())
}

#[tauri::command]
pub fn battery_poll(state: State<'_, AppState>) -> BatteryPoll {
    let mut frames = Vec::new();
    for link in state.batteries.list() {
        let battery_no = link.battery_no();
        for f in link.drain() {
            frames.push(BatteryFrame {
                battery_no: battery_no.clone(),
                dir: f.dir,
                summary: f.summary,
                hex: f.hex,
            });
        }
    }
    BatteryPoll {
        devices: state.batteries.states(),
        frames,
    }
}

#[tauri::command]
pub async fn battery_send_login(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_login().await
}

#[tauri::command]
pub async fn battery_send_location(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_location().await
}

#[tauri::command]
pub async fn battery_send_alarm(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_alarm().await
}

#[tauri::command]
pub async fn battery_send_runtime(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_runtime().await
}

#[tauri::command]
pub async fn battery_send_ping(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_ping().await
}

#[tauri::command]
pub async fn battery_send_logout(
    state: State<'_, AppState>,
    battery_no: String,
) -> Result<(), String> {
    state.batteries.get(&battery_no)?.send_logout().await
}
