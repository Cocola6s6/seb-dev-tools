use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

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
            coordinates: "108.38,22.77".into(),
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
    pub(super) fn new() -> Self {
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
