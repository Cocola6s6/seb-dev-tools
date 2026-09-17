use seb_core::mq::PublishRecord;
use seb_core::{AppConfig, Publisher};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

pub mod config;
pub mod deploy;
pub mod ecu;
pub mod instance;
pub mod send;

pub struct AppState {
    pub publisher: Mutex<Publisher>,
}

impl AppState {
    pub fn new(cfg: AppConfig) -> Self {
        Self {
            publisher: Mutex::new(Publisher::new(cfg)),
        }
    }

    #[allow(dead_code)]
    pub async fn config(&self) -> AppConfig {
        self.publisher.lock().await.config().clone()
    }
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
    exchange: &str,
    action: impl Into<String>,
    payload: serde_json::Value,
) -> Result<SendResult, String> {
    let body = payload.to_string();
    let mut publisher = state.publisher.lock().await;
    let routing_key = seb_core::routing_key(exchange, &publisher.config().instance);
    publisher
        .publish(exchange, &routing_key, &body)
        .await
        .map(|rec| SendResult::from_record(action, rec))
        .map_err(|e| e.to_string())
}

pub async fn device_no(state: &AppState) -> Result<String, String> {
    let publisher = state.publisher.lock().await;
    let no = publisher.config().device_no.trim().to_string();
    if no.is_empty() {
        Err("中控设备序列号 (DeviceNo) 不能为空".to_string())
    } else {
        Ok(no)
    }
}
