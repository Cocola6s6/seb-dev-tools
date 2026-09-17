use super::AppState;
use seb_core::config;
use seb_core::AppConfig;
use tauri::State;

#[derive(serde::Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnState {
    pub mq: bool,
    pub mysql: bool,
    pub redis: bool,
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let publisher = state.publisher.lock().await;
    Ok(publisher.config().clone())
}

#[tauri::command]
pub async fn save_config(state: State<'_, AppState>, cfg: AppConfig) -> Result<(), String> {
    config::save(&cfg).map_err(|e| e.to_string())?;
    let mut publisher = state.publisher.lock().await;
    publisher.set_config(cfg).await;
    Ok(())
}

#[tauri::command]
pub async fn get_conn_state(state: State<'_, AppState>) -> Result<ConnState, String> {
    let publisher = state.publisher.lock().await;
    let mq = publisher.is_connected();
    drop(publisher);

    let mysql = seb_core::db::check_health(&seb_core::config::mysql()).await.unwrap_or(false);
    let redis = seb_core::redis::check_health().await.unwrap_or(false);

    Ok(ConnState { mq, mysql, redis })
}
