mod app;
mod battery;
mod client;
mod dto;
mod settings;
mod ui;
mod util;

pub use app::{AppCtx, DeployOptionItem, DeployOptions};
pub use battery::{BatteryConfig, BatteryCtx, BatteryDefaults, BatteryPoll, BatteryState};
pub use client::{
    AlarmType, ClientCtx, ClientDefaults, ClientPoll, DeviceConfig, DeviceState,
};
pub use dto::{
    BikeDetail, BorrowOptions, ConnState, ControlType, DeployResult, EcuParam, SendResult,
};
pub use settings::{
    AppConfig, BatteryGlobalSettings, BatteryPayloadSettings, ClientGlobalSettings,
    ClientPayloadSettings, ControlGlobalSettings, DeployDefaults, GlobalSettings,
    TripleClickAction,
};
pub use ui::{LogEntry, LogLevel, Page, ToolboxMode};
pub use util::{
    elide_middle, host_env_tag, host_label, inner_host, normalize_ecu_no, qr_url,
    DEFAULT_BATTERY_QR, DEFAULT_BIKE_QR,
};
