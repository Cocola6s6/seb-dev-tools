use crate::state::{
    AppConfig, BorrowOptions, ConnState, ControlType, DeployOptions, DeployResult, EcuParam,
    SendResult,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen(inline_js = r###"
export function tauri_invoke(cmd, args) {
    return window.__TAURI_INTERNALS__.invoke(cmd, args);
}

export function copy_text(text) {
    return navigator.clipboard.writeText(text);
}
"###)]
extern "C" {
    #[wasm_bindgen(catch)]
    fn tauri_invoke(cmd: &str, args: JsValue) -> Result<js_sys::Promise, JsValue>;
    #[wasm_bindgen(catch)]
    fn copy_text(text: &str) -> Result<js_sys::Promise, JsValue>;
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

pub async fn bike_lookup(bike_no: &str) -> Result<String, String> {
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
