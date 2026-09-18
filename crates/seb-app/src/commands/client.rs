use super::AppState;
use seb_core::device::{
    DeviceConfig, DeviceState, SimProfile, DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT,
};
use serde::Serialize;
use std::time::Duration;
use tauri::State;

/// 批量操作时逐台错开，避免网关同一瞬间收到一堆登录
const BATCH_GAP: Duration = Duration::from_millis(200);

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
pub struct ClientFrame {
    pub device_no: String,
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPoll {
    pub devices: Vec<DeviceState>,
    pub frames: Vec<ClientFrame>,
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

/// 设备清单跟着主配置一起落盘，重启后能恢复
async fn persist(state: &AppState) {
    let mut publisher = state.publisher.lock().await;
    let mut cfg = publisher.config().clone();
    cfg.sim_devices = state.devices.roster();
    let _ = seb_core::config::save(&cfg);
    publisher.set_config(cfg).await;
}

#[tauri::command]
pub fn client_devices(state: State<'_, AppState>) -> Vec<DeviceState> {
    state.devices.states()
}

#[tauri::command]
pub async fn client_update_device(
    state: State<'_, AppState>,
    config: DeviceConfig,
) -> Result<Vec<DeviceState>, String> {
    if config.device_no.trim().is_empty() {
        return Err("中控设备序列号不能为空".to_string());
    }
    state.devices.upsert(config);
    persist(&state).await;
    Ok(state.devices.states())
}

#[tauri::command]
pub async fn client_remove_device(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<Vec<DeviceState>, String> {
    if let Some(link) = state.devices.remove(&device_no) {
        link.disconnect().await;
    }
    persist(&state).await;
    Ok(state.devices.states())
}

#[tauri::command]
pub async fn client_connect(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<DeviceState, String> {
    let link = state.devices.get(&device_no)?;
    link.connect().await?;
    Ok(link.state())
}

#[tauri::command]
pub async fn client_disconnect(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<DeviceState, String> {
    let link = state.devices.get(&device_no)?;
    link.disconnect().await;
    Ok(link.state())
}

#[tauri::command]
pub async fn client_connect_all(state: State<'_, AppState>) -> Result<(), String> {
    for link in state.devices.list() {
        if link.connected() {
            continue;
        }
        if let Err(e) = link.connect().await {
            link.note(format!("连接失败: {e}"));
        }
        tokio::time::sleep(BATCH_GAP).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn client_disconnect_all(state: State<'_, AppState>) -> Result<(), String> {
    for link in state.devices.list() {
        link.disconnect().await;
    }
    Ok(())
}

#[tauri::command]
pub async fn client_send_location_all(state: State<'_, AppState>) -> Result<(), String> {
    for link in state.devices.list() {
        if !link.connected() {
            continue;
        }
        if let Err(e) = link.send_location().await {
            link.note(format!("上报定位失败: {e}"));
        }
        tokio::time::sleep(BATCH_GAP).await;
    }
    Ok(())
}

#[tauri::command]
pub fn client_poll(state: State<'_, AppState>) -> ClientPoll {
    let mut frames = Vec::new();
    for link in state.devices.list() {
        let device_no = link.device_no();
        for f in link.drain() {
            frames.push(ClientFrame {
                device_no: device_no.clone(),
                dir: f.dir,
                summary: f.summary,
                hex: f.hex,
            });
        }
    }
    ClientPoll {
        devices: state.devices.states(),
        frames,
    }
}

#[tauri::command]
pub async fn client_send_location(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<(), String> {
    state.devices.get(&device_no)?.send_location().await
}

#[tauri::command]
pub async fn client_send_bms(state: State<'_, AppState>, device_no: String) -> Result<(), String> {
    state.devices.get(&device_no)?.send_bms().await
}

#[tauri::command]
pub async fn client_send_alarm(
    state: State<'_, AppState>,
    device_no: String,
    alarm_type: u8,
    label: String,
) -> Result<(), String> {
    state
        .devices
        .get(&device_no)?
        .send_alarm(alarm_type, &label)
        .await
}

#[tauri::command]
pub async fn client_send_ping(state: State<'_, AppState>, device_no: String) -> Result<(), String> {
    state.devices.get(&device_no)?.send_ping().await
}

#[tauri::command]
pub async fn client_send_reply(
    state: State<'_, AppState>,
    device_no: String,
    msg_id: Option<String>,
    success: bool,
) -> Result<(), String> {
    state
        .devices
        .get(&device_no)?
        .send_reply(msg_id, success)
        .await
}

#[tauri::command]
pub async fn client_get_bike_nos(device_nos: Vec<String>) -> std::collections::HashMap<String, String> {
    let mut map = seb_core::db::batch_find_bike_nos_by_ecus(&seb_core::config::mysql(), &device_nos).await;
    for no in &device_nos {
        let no = no.trim();
        if !no.is_empty() && !map.contains_key(no) {
            let key = format!("{}{no}", seb_core::redis::DEVICE_SERIAL_NO_PREFIX);
            if let Ok(Some(bike_no)) = seb_core::redis::get(&key).await {
                let bike_no = bike_no.trim().to_string();
                if !bike_no.is_empty() {
                    map.insert(no.to_string(), bike_no);
                }
            }
        }
    }
    map
}
