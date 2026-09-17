use super::{device_no, publish, AppState, SendResult};
use seb_core::payload::{self, BorrowOptions};
use seb_core::{ecu, exchange};
use tauri::State;

#[tauri::command]
pub async fn send_borrow(
    state: State<'_, AppState>,
    options: BorrowOptions,
) -> Result<SendResult, String> {
    let dev = device_no(&state).await?;
    let payload = payload::borrow(&dev, options);
    publish(&state, exchange::CONTROL, "借车", payload).await
}

#[tauri::command]
pub async fn send_control(
    state: State<'_, AppState>,
    control_command: u16,
    label: Option<String>,
) -> Result<SendResult, String> {
    let dev = device_no(&state).await?;
    let action = match label {
        Some(l) if !l.trim().is_empty() => format!("控制[{}]", l.trim()),
        _ => format!("控制[0x{control_command:02X}]"),
    };
    let payload = payload::control(&dev, control_command);
    publish(&state, exchange::CONTROL, action, payload).await
}

#[tauri::command]
pub async fn send_voice(state: State<'_, AppState>, voice_id: i64) -> Result<SendResult, String> {
    let dev = device_no(&state).await?;
    let payload = payload::voice(&dev, voice_id);
    publish(&state, exchange::VOICE, format!("语音({voice_id})"), payload).await
}

#[tauri::command]
pub async fn send_ecu_query(
    state: State<'_, AppState>,
    keys: Vec<String>,
) -> Result<SendResult, String> {
    if keys.is_empty() {
        return Err("请选择要查询的 ECU 参数项".to_string());
    }
    let dev = device_no(&state).await?;
    let payload = payload::ecu_query(&dev, &keys);
    let action = format!("ECU查询({})", keys.join(", "));
    publish(&state, exchange::QUERY, action, payload).await
}

#[tauri::command]
pub async fn send_ecu_set(
    state: State<'_, AppState>,
    entries: Vec<(String, String)>,
) -> Result<SendResult, String> {
    if entries.is_empty() {
        return Err("请选择要设置的 ECU 参数项".to_string());
    }
    let dev = device_no(&state).await?;
    let payload = payload::ecu_set(&dev, &entries);
    let action = format!(
        "ECU设置({})",
        entries
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    publish(&state, exchange::SET, action, payload).await
}

#[tauri::command]
pub async fn send_preset(state: State<'_, AppState>, preset: String) -> Result<SendResult, String> {
    let dev = device_no(&state).await?;
    match preset.as_str() {
        "helmet_enable" => {
            let payload = payload::ecu_set(&dev, ecu::PRESET_HELMET_ENABLE);
            publish(
                &state,
                exchange::SET,
                "一键下发[头盔开启佩戴全功能15]",
                payload,
            )
            .await
        }
        "helmet_disable" => {
            let payload = payload::ecu_set(&dev, ecu::PRESET_HELMET_DISABLE);
            publish(
                &state,
                exchange::SET,
                "一键下发[头盔恢复传统模式3]",
                payload,
            )
            .await
        }
        "helmet_query" => {
            let payload = payload::ecu_query(&dev, ecu::PRESET_HELMET_QUERY);
            publish(&state, exchange::QUERY, "一键查询[头盔全套配置]", payload).await
        }
        other => Err(format!("未知的一键场景: {other}")),
    }
}
