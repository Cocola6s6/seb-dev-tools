use seb_core::ecu::{self, EcuParam};
use serde::Serialize;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ControlType {
    pub name: String,
    pub code: u16,
    pub hex: String,
}

#[tauri::command]
pub fn list_ecu_params() -> Vec<EcuParam> {
    ecu::all().to_vec()
}

#[tauri::command]
pub fn list_control_types() -> Vec<ControlType> {
    seb_core::CONTROL_TYPES
        .iter()
        .map(|(name, code)| ControlType {
            name: name.to_string(),
            code: *code,
            hex: format!("0x{code:02X}"),
        })
        .collect()
}
