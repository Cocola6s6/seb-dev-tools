use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub steps: Vec<String>,
    pub bike_existed: bool,
    pub queue_code: u64,
    pub endpoint: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BikeDetail {
    pub bike_no: String,
    pub ecu_no: String,
    pub battery_no: String,
    pub battery_pid: String,
    pub battery_type_id: i64,
    pub city_id: i64,
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub batch_no: String,
    pub motor_no: String,
    pub frame_no: String,
    pub has_helmet: bool,
    pub has_trunk: bool,
    pub road_status: String,
    pub business_status: String,
    pub online_status: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EcuParam {
    pub key: String,
    pub name: String,
    pub desc: String,
    pub default_value: Option<String>,
}

impl EcuParam {
    pub fn description(&self) -> String {
        let mut text = self.name.clone();
        if !self.desc.is_empty() && self.desc != "-" {
            text.push_str(&format!("，{}", self.desc));
        }
        if let Some(v) = &self.default_value {
            text.push_str(&format!(" [默认: {v}]"));
        }
        if text.is_empty() {
            self.key.clone()
        } else {
            text
        }
    }

    pub fn matches(&self, keyword: &str) -> bool {
        let kw = keyword.trim().to_uppercase();
        kw.is_empty() || self.key.to_uppercase().contains(&kw) || self.name.to_uppercase().contains(&kw)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ControlType {
    pub name: String,
    pub code: u16,
    pub hex: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BorrowOptions {
    pub open_helmet_lock: bool,
    pub open_trunk_lock: bool,
    pub helmet_taken: bool,
    pub helmet_worn: bool,
    pub trunk_lock_close: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SendResult {
    pub action: String,
    pub exchange: String,
    pub routing_key: String,
    pub body: String,
    pub reconnected: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlinkJobStatus {
    pub name: String,
    pub running: bool,
    pub state: String,
    pub taskmanagers: u32,
    pub slots_total: u32,
    pub slots_available: u32,
    pub tasks_running: u32,
    pub tasks_total: u32,
    pub dashboard_url: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FlinkState {
    pub high: FlinkJobStatus,
    pub iot: FlinkJobStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnState {
    pub mq: bool,
    pub mysql: bool,
    pub redis: bool,
    pub flink: FlinkState,
}
