use seb_core::battery::BatteryFleet;
use seb_core::device::DeviceFleet;
use seb_core::mq::PublishRecord;
use seb_core::{AppConfig, Publisher};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub mod battery;
pub mod client;
pub mod config;
pub mod deploy;
pub mod ecu;
pub mod hosts;
pub mod instance;
pub mod send;
pub mod terminal;
pub mod window;

pub struct AppState {
    pub publisher: Mutex<Publisher>,
    pub devices: DeviceFleet,
    pub batteries: BatteryFleet,
    /// 工具条自己没有日志区，它干的事记在这儿，由主窗轮询取走
    ui_logs: std::sync::Mutex<Vec<UiLog>>,
}

impl AppState {
    pub fn new(cfg: AppConfig) -> Self {
        let devices = DeviceFleet::default();
        devices.seed(cfg.sim_devices.clone());
        let batteries = BatteryFleet::default();
        batteries.seed(cfg.sim_batteries.clone());
        Self {
            publisher: Mutex::new(Publisher::new(cfg)),
            devices,
            batteries,
            ui_logs: std::sync::Mutex::new(Vec::new()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UiLog {
    pub text: String,
    pub level: String,
}

/// 主窗没开的时候日志先攒着，等它回来一起补上；攒太多就丢最早的
pub fn push_log(state: &AppState, level: &str, text: String) {
    if let Ok(mut logs) = state.ui_logs.lock() {
        if logs.len() >= 200 {
            logs.remove(0);
        }
        logs.push(UiLog { text, level: level.to_string() });
    }
}

#[tauri::command]
pub fn take_ui_logs(state: tauri::State<'_, AppState>) -> Result<Vec<UiLog>, String> {
    let mut logs = state.ui_logs.lock().map_err(|e| e.to_string())?;
    Ok(std::mem::take(&mut *logs))
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub action: String,
    pub exchange: String,
    pub routing_key: String,
    pub body: String,
    pub reconnected: bool,
}

impl SendResult {
    pub fn from_record(action: impl Into<String>, rec: PublishRecord) -> Self {
        Self {
            action: action.into(),
            exchange: rec.exchange,
            routing_key: rec.routing_key,
            body: rec.body,
            reconnected: rec.reconnected,
        }
    }
}

pub async fn publish(
    state: &AppState,
    kind: seb_core::exchange::Kind,
    action: impl Into<String>,
    payload: serde_json::Value,
) -> Result<SendResult, String> {
    let body = payload.to_string();
    let mut publisher = state.publisher.lock().await;
    let (exchange, routing_key) = {
        let cfg = publisher.config();
        let exchange = kind.resolve(&cfg.settings.control);
        let routing_key = seb_core::routing_key(&exchange, &cfg.instance);
        (exchange, routing_key)
    };
    publisher
        .publish(&exchange, &routing_key, &body)
        .await
        .map(|rec| SendResult::from_record(action, rec))
        .map_err(|e| e.to_string())
}

pub async fn device_no(state: &AppState) -> Result<String, String> {
    let publisher = state.publisher.lock().await;
    let no = seb_core::normalize_ecu_no(&publisher.config().device_no);
    if no.is_empty() {
        Err("中控设备序列号 (DeviceNo) 不能为空".to_string())
    } else {
        Ok(no)
    }
}
