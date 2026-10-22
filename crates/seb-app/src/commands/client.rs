use super::AppState;
use seb_core::device::{
    DeviceConfig, DeviceState, SimProfile, DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT,
};
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
pub struct ClientFrame {
    pub device_no: String,
    pub dir: String,
    pub summary: String,
    pub hex: String,
    pub fields: Vec<seb_core::semantic::Field>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPoll {
    pub devices: Vec<DeviceState>,
    pub frames: Vec<ClientFrame>,
}

#[tauri::command]
pub fn client_defaults() -> ClientDefaults {
    let c = seb_core::config::load().settings.client;
    let (host, port) = seb_core::config::split_endpoint(&c.default_inner_gw, DEFAULT_GATEWAY_HOST, DEFAULT_GATEWAY_PORT);

    let mut profile = SimProfile::default();
    if !c.default_coordinates.trim().is_empty() {
        profile.coordinates = c.default_coordinates.trim().to_string();
    }
    profile.soc = c.default_soc;
    profile.speed = c.default_speed as u16;
    profile.deflection_angle = c.default_deflection_angle as f64;

    ClientDefaults {
        host,
        port,
        soft_version: if c.default_soft_version.trim().is_empty() {
            seb_core::frame::DEFAULT_SOFT_VERSION.to_string()
        } else {
            c.default_soft_version
        },
        profile,
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
    mut config: DeviceConfig,
) -> Result<Vec<DeviceState>, String> {
    config.device_no = seb_core::normalize_ecu_no(&config.device_no);
    if config.device_no.is_empty() {
        return Err("中控设备序列号不能为空".to_string());
    }
    let (link, gateway_moved) = state.devices.upsert(config);
    // 链路还挂在旧网关上，留着会让人以为新环境已经连上了
    if gateway_moved && link.connected() {
        link.note("网关地址已变更，自动断开连接");
        link.disconnect().await;
    }
    persist(&state).await;
    Ok(state.devices.states())
}

#[tauri::command]
pub async fn client_rename_device(
    state: State<'_, AppState>,
    old_no: String,
    new_no: String,
) -> Result<Vec<DeviceState>, String> {
    let old_no = seb_core::normalize_ecu_no(&old_no);
    let new_no = seb_core::normalize_ecu_no(&new_no);
    if new_no.is_empty() {
        return Err("中控设备序列号不能为空".to_string());
    }
    if old_no == new_no {
        return Ok(state.devices.states());
    }
    let (link, _) = state.devices.rename(&old_no, &new_no)?;
    if link.connected() {
        link.note(format!("设备序列号已变更为 {new_no}，自动断开连接"));
        link.disconnect().await;
    }
    persist(&state).await;
    Ok(state.devices.states())
}

#[tauri::command]
pub async fn client_remove_device(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<Vec<DeviceState>, String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
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
    let device_no = seb_core::normalize_ecu_no(&device_no);
    let link = state.devices.get(&device_no)?;
    link.connect().await?;
    Ok(link.state())
}

#[tauri::command]
pub async fn client_disconnect(
    state: State<'_, AppState>,
    device_no: String,
) -> Result<DeviceState, String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
    let link = state.devices.get(&device_no)?;
    link.disconnect().await;
    Ok(link.state())
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
                fields: f.fields,
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
    let device_no = seb_core::normalize_ecu_no(&device_no);
    state.devices.get(&device_no)?.send_location().await
}

#[tauri::command]
pub async fn client_send_bms(state: State<'_, AppState>, device_no: String) -> Result<(), String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
    state.devices.get(&device_no)?.send_bms().await
}

#[tauri::command]
pub async fn client_send_alarm(
    state: State<'_, AppState>,
    device_no: String,
    alarm_type: u8,
    label: String,
) -> Result<(), String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
    state
        .devices
        .get(&device_no)?
        .send_alarm(alarm_type, &label)
        .await
}

#[tauri::command]
pub async fn client_send_ping(state: State<'_, AppState>, device_no: String) -> Result<(), String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
    state.devices.get(&device_no)?.send_ping().await
}

#[tauri::command]
pub async fn client_send_reply(
    state: State<'_, AppState>,
    device_no: String,
    msg_id: Option<String>,
    success: bool,
) -> Result<(), String> {
    let device_no = seb_core::normalize_ecu_no(&device_no);
    state
        .devices
        .get(&device_no)?
        .send_reply(msg_id, success)
        .await
}

#[tauri::command]
pub async fn client_get_bike_nos(
    state: State<'_, AppState>,
    device_nos: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    let cfg = seb_core::config::load();
    let client_cfg = &cfg.settings.client;
    let control_cfg = &cfg.settings.control;
    let mut map = std::collections::HashMap::new();

    let mut inner_ecus = Vec::new();
    let mut jsw_ecus = Vec::new();

    for orig in &device_nos {
        let norm = seb_core::normalize_ecu_no(orig);
        if norm.is_empty() {
            continue;
        }
        let gw = state
            .devices
            .get(&norm)
            .map(|l| {
                let c = l.config();
                format!("{}:{}", c.host, c.port)
            })
            .unwrap_or_else(|_| client_cfg.default_inner_gw.clone());

        if seb_core::config::is_inner_gateway(&gw, client_cfg) {
            inner_ecus.push(norm);
        } else {
            jsw_ecus.push(norm);
        }
    }

    // 内网环境使用 MySQL 直连
    if !inner_ecus.is_empty() {
        let inner_map = seb_core::db::batch_find_bike_nos_by_ecus(&control_cfg.mysql, &inner_ecus).await;
        map.extend(inner_map);
    }

    // 外网和正式环境使用 JSW 方式
    if !jsw_ecus.is_empty() {
        let jsw_map = seb_core::jsw::batch_find_bike_nos_by_ecus_with_cfg(&control_cfg.jsw, &jsw_ecus).await;
        map.extend(jsw_map);
    }

    for orig in &device_nos {
        let norm = seb_core::normalize_ecu_no(orig);
        if !norm.is_empty() {
            if let Some(bike) = map.get(&norm).cloned() {
                if orig != &norm {
                    map.insert(orig.clone(), bike);
                }
            } else {
                let key = format!("{}{norm}", seb_core::redis::DEVICE_SERIAL_NO_PREFIX);
                if let Ok(Some(bike_no)) = seb_core::redis::get(&key).await {
                    let bike_no = bike_no.trim().to_string();
                    if !bike_no.is_empty() {
                        map.insert(norm.clone(), bike_no.clone());
                        map.insert(orig.clone(), bike_no);
                    }
                }
            }
        }
    }
    Ok(map)
}

#[tauri::command]
pub async fn client_load_batteries() -> Vec<seb_core::db::BatteryOptionItem> {
    seb_core::db::load_battery_options(&seb_core::config::mysql()).await
}

#[tauri::command]
pub async fn client_resolve_bike(
    query: String,
    gateway: Option<String>,
) -> Result<Option<seb_core::db::BikeResolveInfo>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(None);
    }

    let cfg = seb_core::config::load();
    let client_cfg = &cfg.settings.client;
    let control_cfg = &cfg.settings.control;

    let target_gw = gateway
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(&client_cfg.default_inner_gw);

    if seb_core::config::is_inner_gateway(target_gw, client_cfg) {
        // 内网环境使用 MySQL 直连
        seb_core::db::resolve_bike_or_ecu(&control_cfg.mysql, query).await
    } else {
        // 外网与正式环境使用 JSW 方式
        seb_core::jsw::resolve_bike_or_ecu_with_cfg(&control_cfg.jsw, query).await
    }
}

