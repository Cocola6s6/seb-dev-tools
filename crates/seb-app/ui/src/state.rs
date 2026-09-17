use crate::api;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployDefaults {
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub has_helmet: bool,
    pub has_trunk: bool,
}

impl Default for DeployDefaults {
    fn default() -> Self {
        Self {
            bike_type_id: 0,
            supplier_id: 0,
            dealer_id: 0,
            device_company_id: 0,
            has_helmet: true,
            has_trunk: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub instance: String,
    pub device_no: String,
    pub bike_no: String,
    pub city_id: i64,
    pub deploy: DeployDefaults,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            instance: "0".into(),
            device_no: String::new(),
            bike_no: String::new(),
            city_id: 0,
            deploy: DeployDefaults::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployResult {
    pub steps: Vec<String>,
    pub bike_existed: bool,
    pub queue_code: u64,
    pub endpoint: String,
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
pub struct ConnState {
    pub mq: bool,
    pub mysql: bool,
    pub redis: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn css(&self) -> &'static str {
        match self {
            LogLevel::Info => "log-info",
            LogLevel::Warn => "log-warn",
            LogLevel::Error => "log-error",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub ts: String,
    pub text: String,
    pub level: LogLevel,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Deploy,
    Control,
    Ecu,
}

impl Page {
    pub fn tint(self) -> &'static str {
        match self {
            Page::Deploy => "deploy",
            Page::Control => "control",
            Page::Ecu => "ecu",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployOptionItem {
    pub id: i64,
    pub label: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeployOptions {
    pub cities: Vec<DeployOptionItem>,
    pub bike_types: Vec<DeployOptionItem>,
    pub suppliers: Vec<DeployOptionItem>,
    pub dealers: Vec<DeployOptionItem>,
    pub device_companies: Vec<DeployOptionItem>,
}

#[derive(Clone, Copy)]
pub struct AppCtx {
    pub page: Signal<Page>,
    pub cfg: Signal<AppConfig>,
    pub device_no: Signal<String>,
    pub bike_no: Signal<String>,
    pub instance: Signal<String>,
    pub city_id: Signal<String>,
    pub conn: Signal<ConnState>,
    pub logs: Signal<Vec<LogEntry>>,
    pub ecu_params: Signal<Vec<EcuParam>>,
    pub control_types: Signal<Vec<ControlType>>,
    pub deploy_options: Signal<DeployOptions>,
}

const MAX_LOGS: usize = 500;

impl AppCtx {
    pub fn new() -> Self {
        Self {
            page: create_signal(Page::Deploy),
            cfg: create_signal(AppConfig::default()),
            device_no: create_signal(String::new()),
            bike_no: create_signal(String::new()),
            instance: create_signal("0".to_string()),
            city_id: create_signal("0".to_string()),
            conn: create_signal(ConnState::default()),
            logs: create_signal(Vec::new()),
            ecu_params: create_signal(Vec::new()),
            control_types: create_signal(Vec::new()),
            deploy_options: create_signal(DeployOptions::default()),
        }
    }

    pub fn current_config(&self) -> AppConfig {
        let mut cfg = self.cfg.get_clone();
        cfg.device_no = self.device_no.get_clone().trim().to_string();
        cfg.bike_no = self.bike_no.get_clone().trim().to_string();
        cfg.instance = self.instance.get_clone().trim().to_string();
        cfg.city_id = self.city_id_value();
        cfg
    }

    pub fn city_id_value(&self) -> i64 {
        self.city_id.get_clone().trim().parse::<i64>().unwrap_or(0)
    }

    pub fn adopt_config(&self, cfg: AppConfig) {
        self.device_no.set(cfg.device_no.clone());
        self.bike_no.set(cfg.bike_no.clone());
        self.instance.set(cfg.instance.clone());
        self.city_id.set(cfg.city_id.to_string());
        self.cfg.set(cfg);
    }

    pub fn log(&self, text: impl Into<String>, level: LogLevel) {
        let entry = LogEntry {
            ts: now_hms(),
            text: text.into(),
            level,
        };
        let mut list = self.logs.get_clone();
        list.push(entry);
        if list.len() > MAX_LOGS {
            let overflow = list.len() - MAX_LOGS;
            list.drain(0..overflow);
        }
        self.logs.set(list);
    }

    pub fn log_info(&self, text: impl Into<String>) {
        self.log(text, LogLevel::Info);
    }

    pub fn log_warn(&self, text: impl Into<String>) {
        self.log(text, LogLevel::Warn);
    }

    pub fn log_error(&self, text: impl Into<String>) {
        self.log(text, LogLevel::Error);
    }

    pub fn set_connected(&self, connected: bool) {
        let mut c = self.conn.get_clone();
        c.mq = connected;
        self.conn.set(c);
    }

    pub fn apply_result(&self, result: Result<SendResult, String>) {
        match result {
            Ok(r) => {
                if r.reconnected {
                    self.log_warn("RabbitMQ 连接已断开，已重连并重发");
                }
                self.log_info(format!("{}[RK:{}]: {}", r.action, r.routing_key, r.body));
                self.set_connected(true);
            }
            Err(e) => {
                self.log_error(format!("【错误】{e}"));
                self.set_connected(false);
            }
        }
    }

    pub fn apply_deploy(&self, result: Result<DeployResult, String>) {
        match result {
            Ok(r) => {
                self.log_info(format!(
                    "一键接入完成，车辆{}",
                    if r.bike_existed { "已存在，走更新" } else { "为新增" }
                ));
                for step in r.steps {
                    self.log_info(format!("  {step}"));
                }
            }
            Err(e) => self.log_error(format!("【错误】{e}")),
        }
    }

    pub fn refresh_instance(&self, quiet: bool) {
        let ctx = *self;
        let device_no = self.device_no.get_clone().trim().to_string();
        if device_no.is_empty() {
            if !quiet {
                ctx.log_warn("【警告】请先填写中控设备序列号 (DeviceNo)");
            }
            return;
        }
        spawn_local(async move {
            match api::instance_lookup(&device_no).await {
                Ok(Some(instance)) => {
                    let instance = instance.trim().to_string();
                    ctx.log_info(format!("中控 {device_no} 当前服务实例号: {instance}"));
                    ctx.instance.set(instance);
                }
                Ok(None) => ctx.log_warn(format!(
                    "未查到中控 {device_no} 的实例号（中控可能尚未上线），沿用当前值 {}",
                    ctx.instance.get_clone()
                )),
                Err(e) => ctx.log_error(format!("【错误】查询实例号失败: {e}")),
            }
        });
    }
}

pub fn now_hms() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}
