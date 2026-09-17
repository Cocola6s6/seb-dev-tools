#[tauri::command]
pub async fn instance_lookup(device_no: String) -> Result<Option<String>, String> {
    seb_core::redis::instance_of(&device_no).await
}
