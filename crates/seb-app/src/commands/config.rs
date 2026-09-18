use super::AppState;
use seb_core::config;
use seb_core::AppConfig;
use tauri::State;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConnState {
    pub mq: bool,
    pub mysql: bool,
    pub redis: bool,
    pub flink: seb_core::FlinkState,
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

    let mysql_cfg = seb_core::config::mysql();
    let (mysql_res, redis_res, flink) = tokio::join!(
        seb_core::db::check_health(&mysql_cfg),
        seb_core::redis::check_health(),
        seb_core::flink::check_flink_state(),
    );

    let mysql = mysql_res.unwrap_or(false);
    let redis = redis_res.unwrap_or(false);

    Ok(ConnState { mq, mysql, redis, flink })
}
