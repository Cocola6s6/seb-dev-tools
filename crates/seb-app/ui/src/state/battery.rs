use serde::{Deserialize, Serialize};
use sycamore::prelude::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct BatteryConfig {
    pub battery_no: String,
    pub host: String,
    pub port: u16,
    pub iccid: String,
    pub coordinates: String,
    pub heartbeat: bool,
}

impl Default for BatteryConfig {
    fn default() -> Self {
        Self {
            battery_no: "CMAH030799497009".to_string(),
            host: "10.12.55.31".to_string(),
            port: 32402,
            iccid: "89860409081870640660".to_string(),
            coordinates: "108.375256,22.767133".to_string(),
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
    #[serde(default)]
    pub fields: Vec<crate::state::client::Field>,
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
    pub(super) fn new() -> Self {
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
        BatteryConfig {
            battery_no,
            host,
            port,
            iccid: self.iccid.get_clone().trim().to_string(),
            coordinates: self.coordinates.get_clone().trim().to_string(),
            heartbeat: self.heartbeat.get(),
        }
    }
}
