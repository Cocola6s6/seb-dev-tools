use crate::state::{
    AlarmType, AppConfig, BatteryConfig, BatteryDefaults, BatteryPoll, BatteryState, BikeDetail,
    BorrowOptions, ClientDefaults, ClientPoll, ConnState, ControlType, DeployOptions, DeployResult,
    DeviceConfig, DeviceState, EcuParam, SendResult,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r###"
export function tauri_invoke(cmd, args) {
    return window.__TAURI_INTERNALS__.invoke(cmd, args);
}

export function copy_text(text) {
    return navigator.clipboard.writeText(text);
}

export function js_init_map_picker(container_id, initial_coord, callback) {
    if (window.initMapPicker) {
        window.initMapPicker(container_id, initial_coord, callback);
    }
}

export function js_jump_map_coord(lng, lat) {
    if (window.jumpMapCoord) {
        window.jumpMapCoord(lng, lat);
    }
}

export function js_locate_current_position(callback) {
    if (window.locateCurrentPosition) {
        window.locateCurrentPosition(callback);
    }
}

export function js_start_log_resize(on_resize, on_end) {
    if (window.startLogResize) {
        window.startLogResize(on_resize, on_end);
    }
}

export function js_scroll_log_to_hit(index) {
    if (window.scrollLogToHit) {
        window.scrollLogToHit(index);
    }
}
"###)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn tauri_invoke(cmd: &str, args: JsValue) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch)]
    fn copy_text(text: &str) -> Result<js_sys::Promise, JsValue>;
    fn js_init_map_picker(container_id: &str, initial_coord: &str, callback: &js_sys::Function);
    fn js_jump_map_coord(lng: f64, lat: f64);
    fn js_locate_current_position(callback: &js_sys::Function);
    fn js_start_log_resize(on_resize: &js_sys::Function, on_end: &js_sys::Function);
    fn js_scroll_log_to_hit(index: u32);
}

#[derive(Serialize)]
struct Empty {}

fn js_err(e: JsValue) -> String {
    e.as_string().unwrap_or_else(|| format!("{e:?}"))
}

async fn invoke<A: Serialize, R: serde::de::DeserializeOwned>(
    cmd: &str,
    args: A,
) -> Result<R, String> {
    let args_js = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let promise = tauri_invoke(cmd, args_js).map_err(js_err)?;
    let result = JsFuture::from(promise).await.map_err(js_err)?;
    serde_wasm_bindgen::from_value(result).map_err(|e| format!("返回值解析失败: {e}"))
}

async fn invoke_void<A: Serialize>(cmd: &str, args: A) -> Result<(), String> {
    let args_js = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let promise = tauri_invoke(cmd, args_js).map_err(js_err)?;
    JsFuture::from(promise).await.map_err(js_err)?;
    Ok(())
}

pub async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    let promise = copy_text(text).map_err(js_err)?;
    JsFuture::from(promise).await.map_err(js_err)?;
    Ok(())
}

#[derive(Serialize)]
struct UrlArg<'a> {
    url: &'a str,
}

pub async fn open_external_url(url: &str) -> Result<(), String> {
    invoke_void("open_external_url", UrlArg { url }).await
}

pub async fn switch_hosts() -> Result<String, String> {
    invoke("switch_hosts", Empty {}).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EggArgs {
    on: bool,
}

pub async fn dock_egg(on: bool) -> Result<(), String> {
    invoke_void("dock_egg", EggArgs { on }).await
}

#[derive(Deserialize)]
pub struct UiLog {
    pub text: String,
    pub level: String,
}

pub async fn take_ui_logs() -> Result<Vec<UiLog>, String> {
    invoke("take_ui_logs", Empty {}).await
}

pub async fn hosts_current() -> Result<String, String> {
    invoke("hosts_current", Empty {}).await
}

pub async fn collapse_to_dock() -> Result<(), String> {
    invoke_void("collapse_to_dock", Empty {}).await
}

pub async fn dock_expand(expanded: bool) -> Result<String, String> {
    #[derive(Serialize)]
    struct A {
        expanded: bool,
    }
    invoke("dock_expand", A { expanded }).await
}

pub async fn hide_dock() -> Result<(), String> {
    invoke_void("hide_dock", Empty {}).await
}

pub async fn dock_drag() -> Result<(), String> {
    invoke_void("dock_drag", Empty {}).await
}

#[allow(dead_code)]
pub async fn dock_anchor() -> Result<(f64, f64), String> {
    invoke("dock_anchor", Empty {}).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DockQrSelectionArgs<'a> {
    bike_no: &'a str,
    battery_no: &'a str,
}

#[derive(Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct DockQrSelection {
    pub bike_no: String,
    pub battery_no: String,
}

pub async fn set_dock_qr_selection(bike_no: &str, battery_no: &str) -> Result<(), String> {
    invoke_void(
        "set_dock_qr_selection",
        DockQrSelectionArgs { bike_no, battery_no },
    )
    .await
}

pub async fn get_dock_qr_selection() -> Result<DockQrSelection, String> {
    invoke("get_dock_qr_selection", Empty {}).await
}

pub async fn show_main() -> Result<(), String> {
    invoke_void("show_main", Empty {}).await
}

pub async fn hosts_writable() -> Result<bool, String> {
    invoke("hosts_writable", Empty {}).await
}

pub async fn set_hosts_free(enable: bool) -> Result<(), String> {
    #[derive(Serialize)]
    struct A {
        enable: bool,
    }
    invoke_void("set_hosts_free", A { enable }).await
}

pub fn init_map_picker(container_id: &str, initial_coord: &str, on_pick: impl Fn(String) + 'static) {
    let cb = Closure::wrap(Box::new(move |coord: String| {
        on_pick(coord);
    }) as Box<dyn Fn(String)>);
    js_init_map_picker(container_id, initial_coord, cb.as_ref().unchecked_ref());
    cb.forget();
}

pub fn jump_map_coord(lng: f64, lat: f64) {
    js_jump_map_coord(lng, lat);
}

pub fn locate_current_position(on_success: impl Fn(String) + 'static) {
    let cb = Closure::wrap(Box::new(move |coord: String| {
        on_success(coord);
    }) as Box<dyn Fn(String)>);
    js_locate_current_position(cb.as_ref().unchecked_ref());
    cb.forget();
}

pub fn start_log_resize(on_resize: impl Fn(f64) + 'static, on_end: impl Fn() + 'static) {
    let cb_resize = Closure::wrap(Box::new(move |h: f64| {
        on_resize(h);
    }) as Box<dyn Fn(f64)>);
    let cb_end = Closure::wrap(Box::new(move || {
        on_end();
    }) as Box<dyn Fn()>);
    js_start_log_resize(cb_resize.as_ref().unchecked_ref(), cb_end.as_ref().unchecked_ref());
    cb_resize.forget();
    cb_end.forget();
}

pub fn scroll_log_to_hit(index: usize) {
    js_scroll_log_to_hit(index as u32);
}

pub async fn get_config() -> Result<AppConfig, String> {
    invoke("get_config", Empty {}).await
}

pub async fn load_deploy_options() -> Result<DeployOptions, String> {
    invoke("load_deploy_options", Empty {}).await
}

pub async fn save_config(cfg: &AppConfig) -> Result<(), String> {
    #[derive(Serialize)]
    struct A<'a> {
        cfg: &'a AppConfig,
    }
    invoke_void("save_config", A { cfg }).await
}

pub async fn get_conn_state() -> Result<ConnState, String> {
    invoke("get_conn_state", Empty {}).await
}

pub async fn instance_lookup(device_no: &str) -> Result<Option<String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        device_no: &'a str,
    }
    invoke("instance_lookup", A { device_no }).await
}

pub async fn bike_deploy(
    bike_no: &str,
    ecu_no: &str,
    battery_no: &str,
    battery_type_id: i64,
    battery_pid: &str,
    city_id: i64,
    bike_type_id: i64,
    supplier_id: i64,
    dealer_id: i64,
    device_company_id: i64,
    batch_no: &str,
    motor_no: &str,
    frame_no: &str,
    has_helmet: bool,
    has_trunk: bool,
) -> Result<DeployResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        bike_no: &'a str,
        ecu_no: &'a str,
        battery_no: &'a str,
        battery_type_id: i64,
        battery_pid: &'a str,
        city_id: i64,
        bike_type_id: i64,
        supplier_id: i64,
        dealer_id: i64,
        device_company_id: i64,
        batch_no: &'a str,
        motor_no: &'a str,
        frame_no: &'a str,
        has_helmet: bool,
        has_trunk: bool,
    }
    invoke(
        "bike_deploy",
        A {
            bike_no,
            ecu_no,
            battery_no,
            battery_type_id,
            battery_pid,
            city_id,
            bike_type_id,
            supplier_id,
            dealer_id,
            device_company_id,
            batch_no,
            motor_no,
            frame_no,
            has_helmet,
            has_trunk,
        },
    )
    .await
}

pub async fn bike_lookup(bike_no: &str) -> Result<Option<BikeDetail>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        bike_no: &'a str,
    }
    invoke("bike_lookup", A { bike_no }).await
}

pub async fn list_ecu_params() -> Result<Vec<EcuParam>, String> {
    invoke("list_ecu_params", Empty {}).await
}

pub async fn list_control_types() -> Result<Vec<ControlType>, String> {
    invoke("list_control_types", Empty {}).await
}

pub async fn send_borrow(options: BorrowOptions) -> Result<SendResult, String> {
    #[derive(Serialize)]
    struct A {
        options: BorrowOptions,
    }
    invoke("send_borrow", A { options }).await
}

pub async fn send_control(control_command: u16, label: &str) -> Result<SendResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A<'a> {
        control_command: u16,
        label: &'a str,
    }
    invoke(
        "send_control",
        A {
            control_command,
            label,
        },
    )
    .await
}

pub async fn send_voice(voice_id: i64) -> Result<SendResult, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A {
        voice_id: i64,
    }
    invoke("send_voice", A { voice_id }).await
}

pub async fn send_ecu_query(keys: Vec<String>) -> Result<SendResult, String> {
    #[derive(Serialize)]
    struct A {
        keys: Vec<String>,
    }
    invoke("send_ecu_query", A { keys }).await
}

pub async fn send_ecu_set(entries: Vec<(String, String)>) -> Result<SendResult, String> {
    #[derive(Serialize)]
    struct A {
        entries: Vec<(String, String)>,
    }
    invoke("send_ecu_set", A { entries }).await
}

pub async fn send_preset(preset: &str) -> Result<SendResult, String> {
    #[derive(Serialize)]
    struct A<'a> {
        preset: &'a str,
    }
    invoke("send_preset", A { preset }).await
}

pub async fn client_defaults() -> Result<ClientDefaults, String> {
    invoke("client_defaults", Empty {}).await
}

pub async fn list_alarm_types() -> Result<Vec<AlarmType>, String> {
    invoke("list_alarm_types", Empty {}).await
}

pub async fn client_devices() -> Result<Vec<DeviceState>, String> {
    invoke("client_devices", Empty {}).await
}

pub async fn client_update_device(config: DeviceConfig) -> Result<Vec<DeviceState>, String> {
    #[derive(Serialize)]
    struct A {
        config: DeviceConfig,
    }
    invoke("client_update_device", A { config }).await
}

pub async fn client_remove_device(device_no: &str) -> Result<Vec<DeviceState>, String> {
    invoke("client_remove_device", Device { device_no }).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Device<'a> {
    device_no: &'a str,
}

pub async fn client_connect(device_no: &str) -> Result<DeviceState, String> {
    invoke("client_connect", Device { device_no }).await
}

pub async fn client_disconnect(device_no: &str) -> Result<DeviceState, String> {
    invoke("client_disconnect", Device { device_no }).await
}

pub async fn client_poll() -> Result<ClientPoll, String> {
    invoke("client_poll", Empty {}).await
}

pub async fn client_send_location(device_no: String) -> Result<(), String> {
    invoke_void("client_send_location", Device { device_no: &device_no }).await
}

pub async fn client_send_bms(device_no: String) -> Result<(), String> {
    invoke_void("client_send_bms", Device { device_no: &device_no }).await
}

pub async fn client_send_ping(device_no: String) -> Result<(), String> {
    invoke_void("client_send_ping", Device { device_no: &device_no }).await
}

pub async fn client_send_alarm(device_no: String, alarm_type: u8, label: String) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A {
        device_no: String,
        alarm_type: u8,
        label: String,
    }
    invoke_void(
        "client_send_alarm",
        A {
            device_no,
            alarm_type,
            label,
        },
    )
    .await
}

pub async fn client_send_reply(device_no: String, success: bool) -> Result<(), String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A {
        device_no: String,
        msg_id: Option<String>,
        success: bool,
    }
    invoke_void(
        "client_send_reply",
        A {
            device_no,
            msg_id: None,
            success,
        },
    )
    .await
}

pub async fn open_terminal_log(
    device_no: Option<String>,
    bike_no: Option<String>,
) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A {
        device_no: Option<String>,
        bike_no: Option<String>,
    }
    invoke("open_terminal_log", A { device_no, bike_no }).await
}

pub async fn client_get_bike_nos(
    device_nos: Vec<String>,
) -> Result<std::collections::HashMap<String, String>, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct A {
        device_nos: Vec<String>,
    }
    invoke("client_get_bike_nos", A { device_nos }).await
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryOptionItem {
    pub battery_no: String,
    pub bound_bike_no: Option<String>,
}

pub async fn client_load_batteries() -> Result<Vec<BatteryOptionItem>, String> {
    invoke("client_load_batteries", Empty {}).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatteryArg<'a> {
    battery_no: &'a str,
}

pub async fn battery_defaults() -> Result<BatteryDefaults, String> {
    invoke("battery_defaults", Empty {}).await
}

pub async fn battery_devices() -> Result<Vec<BatteryState>, String> {
    invoke("battery_devices", Empty {}).await
}

pub async fn battery_update_device(config: BatteryConfig) -> Result<Vec<BatteryState>, String> {
    #[derive(Serialize)]
    struct A {
        config: BatteryConfig,
    }
    invoke("battery_update_device", A { config }).await
}

pub async fn battery_remove_device(battery_no: &str) -> Result<Vec<BatteryState>, String> {
    invoke("battery_remove_device", BatteryArg { battery_no }).await
}

pub async fn battery_connect(battery_no: &str) -> Result<BatteryState, String> {
    invoke("battery_connect", BatteryArg { battery_no }).await
}

pub async fn battery_disconnect(battery_no: &str) -> Result<BatteryState, String> {
    invoke("battery_disconnect", BatteryArg { battery_no }).await
}

pub async fn battery_poll() -> Result<BatteryPoll, String> {
    invoke("battery_poll", Empty {}).await
}

#[allow(dead_code)]
pub async fn battery_send_login(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_login", BatteryArg { battery_no: &battery_no }).await
}

pub async fn battery_send_location(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_location", BatteryArg { battery_no: &battery_no }).await
}

pub async fn battery_send_alarm(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_alarm", BatteryArg { battery_no: &battery_no }).await
}

pub async fn battery_send_runtime(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_runtime", BatteryArg { battery_no: &battery_no }).await
}

pub async fn battery_send_ping(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_ping", BatteryArg { battery_no: &battery_no }).await
}

#[allow(dead_code)]
pub async fn battery_send_logout(battery_no: String) -> Result<(), String> {
    invoke_void("battery_send_logout", BatteryArg { battery_no: &battery_no }).await
}
