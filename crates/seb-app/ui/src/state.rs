use crate::api;
use gloo_timers::future::TimeoutFuture;
use serde::{Deserialize, Serialize};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// 中控指令、中控配置这些只打内网后台，所以要认得出设备连的是哪套网关
pub const INNER_HOST: &str = "bike-seb-inner-test.costrip.cn";

pub fn host_label(host: &str) -> String {
    if host.contains("bike-seb-inner-test") {
        "内网".to_string()
    } else if host.contains("bike-seb-test") {
        "外网".to_string()
    } else if host.contains("bike-seb.costrip.cn") {
        "正式".to_string()
    } else if host.is_empty() {
        "未配置".to_string()
    } else {
        host.to_string()
    }
}

pub fn host_env_tag(host: &str) -> (&'static str, &'static str) {
    if host.contains("bike-seb-inner-test") || host.contains("10.12.55.31") {
        ("内网", "badge-env-inner")
    } else if host.contains("bike-seb-test") || host.contains("140.143.180.28") {
        ("外网", "badge-env-test")
    } else if host.contains("bike-seb.costrip.cn") || host.contains("140.143.214.51") {
        ("正式", "badge-env-prod")
    } else if host.is_empty() {
        ("未配置", "badge-env-none")
    } else {
        ("自定义", "badge-env-custom")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DeployDefaults {
    pub bike_type_id: i64,
    pub supplier_id: i64,
    pub dealer_id: i64,
    pub device_company_id: i64,
    pub battery_type_id: i64,
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
            battery_type_id: 0,
            has_helmet: true,
            has_trunk: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct AppConfig {
    pub instance: String,
    pub device_no: String,
    pub bike_no: String,
    pub battery_no: String,
    pub city_id: i64,
    pub deploy: DeployDefaults,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            instance: "0".into(),
            device_no: String::new(),
            bike_no: String::new(),
            battery_no: String::new(),
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
    pub tint: &'static str,
    /// 收发报文才有：Some("up") 上行、Some("down") 下行
    pub dir: Option<&'static str>,
    /// 模拟设备产生的日志才有
    pub device: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Deploy,
    Control,
    Ecu,
    Client,
    Battery,
}

impl Page {
    pub fn index(self) -> usize {
        match self {
            Page::Deploy => 0,
            Page::Control => 1,
            Page::Ecu => 2,
            Page::Client => 3,
            Page::Battery => 4,
        }
    }

    pub fn tint(self) -> &'static str {
        match self {
            Page::Deploy => "deploy",
            Page::Control => "control",
            Page::Ecu => "ecu",
            Page::Client => "client",
            Page::Battery => "battery",
        }
    }

    pub fn is_client(self) -> bool {
        matches!(self, Page::Client | Page::Battery)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct SimProfile {
    pub coordinates: String,
    pub vehicle_state: u8,
    pub motion: bool,
    pub soc: u8,
    pub speed: u16,
    pub helmet_lock_unlocked: bool,
    pub helmet_present: bool,
    pub trunk_latch: bool,
    pub acc_on: bool,
    pub deflection_angle: f64,
    pub battery_no: String,
    pub reply_success: bool,
    pub reply_with_location: bool,
}

impl Default for SimProfile {
    fn default() -> Self {
        Self {
            coordinates: "116.29721053978871,40.05213174125153".into(),
            vehicle_state: 0,
            motion: false,
            soc: 80,
            speed: 0,
            helmet_lock_unlocked: false,
            helmet_present: true,
            trunk_latch: true,
            acc_on: true,
            deflection_angle: 5.0,
            battery_no: String::new(),
            reply_success: true,
            reply_with_location: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DeviceConfig {
    pub device_no: String,
    pub host: String,
    pub port: u16,
    pub soft_version: String,
    pub heartbeat: bool,
    pub auto_reply: bool,
    pub profile: SimProfile,
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            device_no: String::new(),
            host: String::new(),
            port: 32405,
            soft_version: String::new(),
            heartbeat: true,
            auto_reply: true,
            profile: SimProfile::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct DeviceState {
    pub connected: bool,
    pub endpoint: String,
    pub last_msg_id: Option<String>,
    pub config: DeviceConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FrameLog {
    pub device_no: String,
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AlarmType {
    pub name: String,
    pub code: u8,
    pub hex: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientDefaults {
    pub host: String,
    pub port: u16,
    pub soft_version: String,
    pub profile: SimProfile,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientPoll {
    pub devices: Vec<DeviceState>,
    pub frames: Vec<FrameLog>,
}

/// 客户端页面的表单状态放在全局，切页后回来不会丢。
#[derive(Clone, Copy)]
pub struct ClientCtx {
    pub gateway: Signal<String>,
    pub soft_version: Signal<String>,
    pub heartbeat: Signal<bool>,
    pub auto_reply: Signal<bool>,
    pub reply_success: Signal<bool>,
    pub reply_with_location: Signal<bool>,
    pub coordinates: Signal<String>,
    pub vehicle_state: Signal<String>,
    pub motion: Signal<bool>,
    pub soc: Signal<String>,
    pub speed: Signal<String>,
    pub helmet_lock_unlocked: Signal<bool>,
    pub helmet_present: Signal<bool>,
    pub trunk_latch: Signal<bool>,
    pub acc_on: Signal<bool>,
    pub deflection_angle: Signal<String>,
    pub battery_no: Signal<String>,
    pub battery_options: Signal<Vec<crate::api::BatteryOptionItem>>,
    pub alarm_type: Signal<String>,
    pub alarm_types: Signal<Vec<AlarmType>>,
    pub devices: Signal<Vec<DeviceState>>,
    pub selected: Signal<String>,
    pub selected_set: Signal<Vec<String>>,
    pub bike_map: Signal<std::collections::HashMap<String, String>>,
    /// 表单当前归属的设备：切设备时先置空，避免把上一台的值写进新设备
    pub owner: Signal<String>,
    pub device_no: Signal<String>,
}

impl ClientCtx {
    fn new() -> Self {
        let d = SimProfile::default();
        Self {
            gateway: create_signal(String::new()),
            soft_version: create_signal(String::new()),
            heartbeat: create_signal(true),
            auto_reply: create_signal(true),
            reply_success: create_signal(d.reply_success),
            reply_with_location: create_signal(d.reply_with_location),
            coordinates: create_signal(d.coordinates.clone()),
            vehicle_state: create_signal(d.vehicle_state.to_string()),
            motion: create_signal(d.motion),
            soc: create_signal(d.soc.to_string()),
            speed: create_signal(d.speed.to_string()),
            helmet_lock_unlocked: create_signal(d.helmet_lock_unlocked),
            helmet_present: create_signal(d.helmet_present),
            trunk_latch: create_signal(d.trunk_latch),
            acc_on: create_signal(d.acc_on),
            deflection_angle: create_signal(d.deflection_angle.to_string()),
            battery_no: create_signal(d.battery_no.clone()),
            battery_options: create_signal(Vec::new()),
            alarm_type: create_signal(String::new()),
            alarm_types: create_signal(Vec::new()),
            devices: create_signal(Vec::new()),
            selected: create_signal(String::new()),
            selected_set: create_signal(Vec::new()),
            bike_map: create_signal(std::collections::HashMap::new()),
            owner: create_signal(String::new()),
            device_no: create_signal(String::new()),
        }
    }

    pub fn adopt_defaults(&self, d: ClientDefaults) {
        self.gateway.set(format!("{}:{}", d.host, d.port));
        self.soft_version.set(d.soft_version);
    }

    pub fn current(&self) -> Option<DeviceState> {
        let no = self.selected.get_clone();
        self.devices.get_clone().into_iter().find(|d| d.config.device_no == no)
    }

    pub fn split_gateway(&self) -> (String, u16) {
        let text = self.gateway.get_clone();
        let (host, port) = text.trim().rsplit_once(':').unwrap_or((text.trim(), ""));
        (host.trim().to_string(), port.trim().parse().unwrap_or(0))
    }

    pub fn config(&self, device_no: String) -> DeviceConfig {
        let (host, port) = self.split_gateway();
        DeviceConfig {
            device_no,
            host,
            port,
            soft_version: self.soft_version.get_clone(),
            heartbeat: self.heartbeat.get(),
            auto_reply: self.auto_reply.get(),
            profile: self.profile(),
        }
    }

    pub fn is_selected(&self, device_no: &str) -> bool {
        let set = self.selected_set.get_clone();
        if set.is_empty() {
            self.selected.get_clone() == device_no
        } else {
            set.iter().any(|s| s == device_no)
        }
    }

    pub fn selected_list(&self) -> Vec<String> {
        let set = self.selected_set.get_clone();
        if set.is_empty() {
            let cur = self.selected.get_clone();
            if cur.is_empty() {
                vec![]
            } else {
                vec![cur]
            }
        } else {
            set
        }
    }

    pub fn load_device(&self, device_no: &str) {
        let found = self
            .devices
            .get_clone()
            .into_iter()
            .find(|d| d.config.device_no == device_no);
        self.owner.set(String::new());
        self.selected.set(device_no.to_string());
        self.device_no.set(device_no.to_string());
        if let Some(st) = found {
            let c = st.config;
            self.gateway.set(format!("{}:{}", c.host, c.port));
            self.soft_version.set(c.soft_version);
            self.heartbeat.set(c.heartbeat);
            self.auto_reply.set(c.auto_reply);
            let p = c.profile;
            self.reply_success.set(p.reply_success);
            self.reply_with_location.set(p.reply_with_location);
            self.coordinates.set(p.coordinates);
            self.vehicle_state.set(p.vehicle_state.to_string());
            self.motion.set(p.motion);
            self.soc.set(p.soc.to_string());
            self.speed.set(p.speed.to_string());
            self.helmet_lock_unlocked.set(p.helmet_lock_unlocked);
            self.helmet_present.set(p.helmet_present);
            self.trunk_latch.set(p.trunk_latch);
            self.acc_on.set(p.acc_on);
            self.deflection_angle.set(p.deflection_angle.to_string());
            self.battery_no.set(p.battery_no);
        }
        self.owner.set(device_no.to_string());
    }

    /// 切换设备：单选
    pub fn select(&self, device_no: &str) {
        self.selected_set.set(if device_no.is_empty() {
            vec![]
        } else {
            vec![device_no.to_string()]
        });
        self.load_device(device_no);
    }

    /// Cmd / Ctrl 切换选中
    pub fn toggle_select(&self, device_no: &str) {
        let mut set = self.selected_set.get_clone();
        if set.is_empty() {
            let cur = self.selected.get_clone();
            if !cur.is_empty() && cur != device_no {
                set.push(cur);
            }
        }
        if let Some(pos) = set.iter().position(|x| x == device_no) {
            set.remove(pos);
            if let Some(last) = set.last().cloned() {
                self.load_device(&last);
            }
        } else {
            set.push(device_no.to_string());
            self.load_device(device_no);
        }
        self.selected_set.set(set);
    }

    /// Shift 连选
    pub fn range_select(&self, target_no: &str) {
        let devs = self.devices.get_clone();
        let cur_no = self.selected.get_clone();
        let start_idx = devs.iter().position(|d| d.config.device_no == cur_no).unwrap_or(0);
        let end_idx = devs.iter().position(|d| d.config.device_no == target_no).unwrap_or(0);
        let (min_i, max_i) = if start_idx <= end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        let set: Vec<String> = devs[min_i..=max_i]
            .iter()
            .map(|d| d.config.device_no.clone())
            .collect();
        self.selected_set.set(set);
        self.load_device(target_no);
    }

    pub fn profile(&self) -> SimProfile {
        SimProfile {
            coordinates: self.coordinates.get_clone().trim().to_string(),
            vehicle_state: self.vehicle_state.get_clone().parse().unwrap_or(0),
            motion: self.motion.get(),
            soc: self.soc.get_clone().trim().parse().unwrap_or(80),
            speed: self.speed.get_clone().trim().parse().unwrap_or(0),
            helmet_lock_unlocked: self.helmet_lock_unlocked.get(),
            helmet_present: self.helmet_present.get(),
            trunk_latch: self.trunk_latch.get(),
            acc_on: self.acc_on.get(),
            deflection_angle: self.deflection_angle.get_clone().trim().parse().unwrap_or(5.0),
            battery_no: self.battery_no.get_clone().trim().to_string(),
            reply_success: self.reply_success.get(),
            reply_with_location: self.reply_with_location.get(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryConfig {
    pub battery_no: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
    pub hw_major_version: u8,
    pub hw_minor_version: u8,
    pub hw_rev_version: u8,
    pub sw_major_version: u8,
    pub sw_minor_version: u8,
    pub sw_rev_version: u8,
    pub heartbeat: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            battery_no: "CMAH030799497009".to_string(),
            env: "内网".to_string(),
            host: "10.12.55.31".to_string(),
            port: 32402,
            iccid: "89860409081870640660".to_string(),
            coordinates: "116.302928,40.054926".to_string(),
            hw_major_version: 2,
            hw_minor_version: 1,
            hw_rev_version: 3,
            sw_major_version: 3,
            sw_minor_version: 2,
            sw_rev_version: 1,
            heartbeat: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryState {
    pub connected: bool,
    pub endpoint: String,
    pub config: BatteryConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryDefaults {
    pub battery_no: String,
    pub env: String,
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryFrame {
    pub battery_no: String,
    pub dir: String,
    pub summary: String,
    pub hex: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryPoll {
    pub devices: Vec<BatteryState>,
    pub frames: Vec<BatteryFrame>,
}

#[derive(Clone, Copy)]
pub struct BatteryCtx {
    pub gateway: Signal<String>,
    pub heartbeat: Signal<bool>,
    pub coordinates: Signal<String>,
    pub iccid: Signal<String>,
    pub devices: Signal<Vec<BatteryState>>,
    pub selected: Signal<String>,
    pub selected_set: Signal<Vec<String>>,
    pub owner: Signal<String>,
    pub battery_no: Signal<String>,
}

impl BatteryCtx {
    fn new() -> Self {
        let d = BatteryConfig::default();
        Self {
            gateway: create_signal(format!("{}:{}", d.host, d.port)),
            heartbeat: create_signal(d.heartbeat),
            coordinates: create_signal(d.coordinates),
            iccid: create_signal(d.iccid),
            devices: create_signal(Vec::new()),
            selected: create_signal(d.battery_no.clone()),
            selected_set: create_signal(Vec::new()),
            owner: create_signal(String::new()),
            battery_no: create_signal(d.battery_no),
        }
    }

    pub fn adopt_defaults(&self, d: BatteryDefaults) {
        self.battery_no.set(d.battery_no);
        self.gateway.set(format!("{}:{}", d.host, d.port));
        self.iccid.set(d.iccid);
        self.coordinates.set(d.coordinates);
    }

    pub fn split_gateway(&self) -> (String, u16) {
        let text = self.gateway.get_clone();
        let (host, port) = text.trim().rsplit_once(':').unwrap_or((text.trim(), ""));
        (host.trim().to_string(), port.trim().parse().unwrap_or(0))
    }

    pub fn is_selected(&self, battery_no: &str) -> bool {
        let set = self.selected_set.get_clone();
        if set.is_empty() {
            self.selected.get_clone() == battery_no
        } else {
            set.iter().any(|s| s == battery_no)
        }
    }

    pub fn selected_list(&self) -> Vec<String> {
        let set = self.selected_set.get_clone();
        if set.is_empty() {
            let cur = self.selected.get_clone();
            if cur.is_empty() {
                vec![]
            } else {
                vec![cur]
            }
        } else {
            set
        }
    }

    pub fn load_device(&self, battery_no: &str) {
        let found = self
            .devices
            .get_clone()
            .into_iter()
            .find(|d| d.config.battery_no == battery_no);
        self.owner.set(String::new());
        self.selected.set(battery_no.to_string());
        self.battery_no.set(battery_no.to_string());
        if let Some(st) = found {
            let c = st.config;
            self.gateway.set(format!("{}:{}", c.host, c.port));
            self.iccid.set(c.iccid);
            self.coordinates.set(c.coordinates);
            self.heartbeat.set(c.heartbeat);
        }
        self.owner.set(battery_no.to_string());
    }

    pub fn select(&self, battery_no: &str) {
        self.selected_set.set(if battery_no.is_empty() {
            vec![]
        } else {
            vec![battery_no.to_string()]
        });
        self.load_device(battery_no);
    }

    pub fn toggle_select(&self, battery_no: &str) {
        let mut set = self.selected_set.get_clone();
        if set.is_empty() {
            let cur = self.selected.get_clone();
            if !cur.is_empty() && cur != battery_no {
                set.push(cur);
            }
        }
        if let Some(pos) = set.iter().position(|x| x == battery_no) {
            set.remove(pos);
            if let Some(last) = set.last().cloned() {
                self.load_device(&last);
            }
        } else {
            set.push(battery_no.to_string());
            self.load_device(battery_no);
        }
        self.selected_set.set(set);
    }

    pub fn range_select(&self, target_no: &str) {
        let devs = self.devices.get_clone();
        let cur_no = self.selected.get_clone();
        let start_idx = devs.iter().position(|d| d.config.battery_no == cur_no).unwrap_or(0);
        let end_idx = devs.iter().position(|d| d.config.battery_no == target_no).unwrap_or(0);
        let (min_i, max_i) = if start_idx <= end_idx {
            (start_idx, end_idx)
        } else {
            (end_idx, start_idx)
        };
        let set: Vec<String> = devs[min_i..=max_i]
            .iter()
            .map(|d| d.config.battery_no.clone())
            .collect();
        self.selected_set.set(set);
        self.load_device(target_no);
    }

    #[allow(dead_code)]
    pub fn current(&self) -> Option<BatteryState> {
        let no = self.selected.get_clone();
        self.devices.get_clone().into_iter().find(|d| d.config.battery_no == no)
    }

    pub fn config(&self, battery_no: String) -> BatteryConfig {
        let (host, port) = self.split_gateway();
        let env = match host.as_str() {
            "10.12.55.31" => "内网",
            "140.143.180.28" => "外网",
            "140.143.214.51" => "正式",
            _ => "自定义",
        }
        .to_string();
        BatteryConfig {
            battery_no,
            env,
            host,
            port,
            iccid: self.iccid.get_clone().trim().to_string(),
            coordinates: self.coordinates.get_clone().trim().to_string(),
            hw_major_version: 2,
            hw_minor_version: 1,
            hw_rev_version: 3,
            sw_major_version: 3,
            sw_minor_version: 2,
            sw_rev_version: 1,
            heartbeat: self.heartbeat.get(),
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
    pub battery_types: Vec<DeployOptionItem>,
}

#[derive(Clone, Copy)]
pub struct AppCtx {
    pub page: Signal<Page>,
    pub client: ClientCtx,
    pub battery: BatteryCtx,
    pub cfg: Signal<AppConfig>,
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
}

const MAX_LOGS: usize = 500;

impl AppCtx {
    pub fn new() -> Self {
        Self {
            page: create_signal(Page::Deploy),
            client: ClientCtx::new(),
            battery: BatteryCtx::new(),
            cfg: create_signal(AppConfig::default()),
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
        cfg
    }

    pub fn city_id_value(&self) -> i64 {
        self.city_id.get_clone().trim().parse::<i64>().unwrap_or(0)
    }

    pub fn adopt_config(&self, cfg: AppConfig) {
        self.device_no.set(cfg.device_no.clone());
        self.bike_no.set(cfg.bike_no.clone());
        self.battery_no.set(cfg.battery_no.clone());
        self.instance.set(cfg.instance.clone());
        self.city_id.set(cfg.city_id.to_string());
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

pub fn now_hms() -> String {
    let d = js_sys::Date::new_0();
    format!(
        "{:02}:{:02}:{:02}",
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}
