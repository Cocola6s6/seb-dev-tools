use super::battery::BatteryCtx;
use super::client::ClientCtx;
use super::dto::{ConnState, ControlType, DeployResult, EcuParam, SendResult};
use super::settings::{AppConfig, GlobalSettings};
use super::ui::{LogEntry, LogLevel, Page, ToolboxMode};
use super::util::now_hms;
use crate::api;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

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
    pub battery_types: Vec<DeployOptionItem>,
}

#[derive(Clone, Copy)]
pub struct AppCtx {
    pub page: Signal<Page>,
    pub is_settings: Signal<bool>,
    pub toolbox_mode: Signal<ToolboxMode>,
    /// 正在敲鸡蛋。顶栏 .nav 有 backdrop-filter，固定定位的遮罩只能挂在根上
    pub egg: Signal<bool>,
    pub client: ClientCtx,
    pub battery: BatteryCtx,
    pub cfg: Signal<AppConfig>,
    pub global_settings: Signal<GlobalSettings>,
    pub device_no: Signal<String>,
    pub bike_no: Signal<String>,
    pub battery_no: Signal<String>,
    pub instance: Signal<String>,
    pub city_id: Signal<String>,
    pub conn: Signal<ConnState>,
    pub logs: Signal<Vec<LogEntry>>,
    pub ecu_params: Signal<Vec<EcuParam>>,
    pub control_types: Signal<Vec<ControlType>>,
    pub deploy_options: Signal<DeployOptions>,
    pub toast_msg: Signal<Option<String>>,
    pub show_whats_new: Signal<bool>,
}

const MAX_LOGS: usize = 500;

impl AppCtx {
    pub fn new() -> Self {
        Self {
            page: create_signal(Page::Deploy),
            is_settings: create_signal(false),
            toolbox_mode: create_signal(ToolboxMode::QrCode),
            egg: create_signal(false),
            client: ClientCtx::new(),
            battery: BatteryCtx::new(),
            cfg: create_signal(AppConfig::default()),
            global_settings: create_signal(GlobalSettings::default()),
            device_no: create_signal(String::new()),
            bike_no: create_signal(String::new()),
            battery_no: create_signal(String::new()),
            instance: create_signal("0".to_string()),
            city_id: create_signal("0".to_string()),
            conn: create_signal(ConnState::default()),
            logs: create_signal(Vec::new()),
            ecu_params: create_signal(Vec::new()),
            control_types: create_signal(Vec::new()),
            deploy_options: create_signal(DeployOptions::default()),
            toast_msg: create_signal(None),
            show_whats_new: create_signal(false),
        }
    }

    /// 当前功能页自己的模拟设备：(网关地址, 是否在线)。没有模拟客户端的页面返回 None
    pub fn page_devices(&self) -> Option<Vec<(String, bool)>> {
        match self.page.get() {
            Page::Client => Some(
                self.client
                    .devices
                    .get_clone()
                    .into_iter()
                    .map(|d| (d.config.host, d.connected))
                    .collect(),
            ),
            Page::Battery => Some(
                self.battery
                    .devices
                    .get_clone()
                    .into_iter()
                    .map(|d| (d.config.host, d.connected))
                    .collect(),
            ),
            _ => None,
        }
    }

    pub fn toast(&self, msg: impl Into<String>) {
        let msg = msg.into();
        self.toast_msg.set(Some(msg));
        let toast_sig = self.toast_msg.clone();
        spawn_local(async move {
            TimeoutFuture::new(1600).await;
            toast_sig.set(None);
        });
    }

    pub fn current_config(&self) -> AppConfig {
        let mut cfg = self.cfg.get_clone();
        cfg.device_no = self.device_no.get_clone().trim().to_string();
        cfg.bike_no = self.bike_no.get_clone().trim().to_string();
        cfg.battery_no = self.battery_no.get_clone().trim().to_string();
        cfg.instance = self.instance.get_clone().trim().to_string();
        cfg.city_id = self.city_id_value();
        cfg.settings = self.global_settings.get_clone();
        cfg
    }

    pub fn city_id_value(&self) -> i64 {
        self.city_id.get_clone().trim().parse::<i64>().unwrap_or(0)
    }

    pub fn adopt_config(&self, cfg: AppConfig) {
        let current_version = env!("CARGO_PKG_VERSION");
        if cfg.last_seen_version != current_version {
            self.show_whats_new.set(true);
        }
        self.device_no.set(cfg.device_no.clone());
        self.bike_no.set(cfg.bike_no.clone());
        self.battery_no.set(cfg.battery_no.clone());
        self.instance.set(cfg.instance.clone());
        self.city_id.set(cfg.city_id.to_string());
        self.global_settings.set(cfg.settings.clone());
        self.cfg.set(cfg);
    }

    pub fn log(&self, text: impl Into<String>, level: LogLevel) {
        self.log_tinted(text, level, self.page.get().tint(), None, String::new());
    }

    /// 客户端的收发固定用客户端配色：指令常常是在「中控指令」页发出的，但回包属于客户端
    pub fn log_client(&self, text: impl Into<String>, level: LogLevel) {
        let device = self.client.selected.get_clone();
        self.log_tinted(text, level, Page::Client.tint(), None, device);
    }

    /// 电池客户端的收发固定用电池客户端配色
    pub fn log_battery(&self, text: impl Into<String>, level: LogLevel) {
        let device = self.battery.selected.get_clone();
        self.log_tinted(text, level, Page::Battery.tint(), None, device);
    }

    pub fn log_battery_frame(&self, dir: &str, battery_no: &str, text: impl Into<String>) {
        let dir = match dir {
            "up" => Some("up"),
            "down" => Some("down"),
            _ => None,
        };
        self.log_tinted(text, LogLevel::Info, Page::Battery.tint(), dir, battery_no.to_string());
    }

    /// 报文日志：上行还是下行、是哪台设备，都要一眼能分出来
    pub fn log_frame(&self, dir: &str, device: &str, text: impl Into<String>) {
        let dir = match dir {
            "up" => Some("up"),
            "down" => Some("down"),
            _ => None,
        };
        self.log_tinted(text, LogLevel::Info, Page::Client.tint(), dir, device.to_string());
    }

    fn log_tinted(
        &self,
        text: impl Into<String>,
        level: LogLevel,
        tint: &'static str,
        dir: Option<&'static str>,
        device: String,
    ) {
        let entry = LogEntry {
            ts: now_hms(),
            text: text.into(),
            level,
            tint,
            dir,
            device,
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
