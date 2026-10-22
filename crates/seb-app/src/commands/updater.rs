use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub body: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub percent: Option<f64>,
    pub stage: String, // "downloading" | "installing" | "ready" | "error"
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateCheckResult, String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater.check().await.map_err(|e| e.to_string())?;

    if let Some(u) = update {
        Ok(UpdateCheckResult {
            has_update: true,
            current_version: u.current_version.clone(),
            latest_version: u.version.clone(),
            body: u.body.clone(),
            date: u.date.map(|d| d.to_string()),
        })
    } else {
        Ok(UpdateCheckResult {
            has_update: false,
            current_version: app.package_info().version.to_string(),
            latest_version: app.package_info().version.to_string(),
            body: None,
            date: None,
        })
    }
}

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    let updater = app.updater().map_err(|e| e.to_string())?;
    let update = updater
        .check()
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "当前已是最新版本，无需更新".to_string())?;

    let downloaded = Arc::new(AtomicU64::new(0));
    let downloaded_clone = downloaded.clone();
    let app_handle = app.clone();

    update
        .download_and_install(
            move |chunk_length, content_length| {
                let current = downloaded_clone.fetch_add(chunk_length as u64, Ordering::Relaxed)
                    + chunk_length as u64;
                let percent = content_length.map(|total| (current as f64 / total as f64) * 100.0);
                let _ = app_handle.emit(
                    "updater:progress",
                    UpdateProgress {
                        downloaded_bytes: current,
                        total_bytes: content_length,
                        percent,
                        stage: "downloading".to_string(),
                    },
                );
            },
            {
                let app_handle = app.clone();
                move || {
                    let _ = app_handle.emit(
                        "updater:progress",
                        UpdateProgress {
                            downloaded_bytes: downloaded.load(Ordering::Relaxed),
                            total_bytes: None,
                            percent: Some(100.0),
                            stage: "installing".to_string(),
                        },
                    );
                }
            },
        )
        .await
        .map_err(|e| {
            let _ = app.emit(
                "updater:progress",
                UpdateProgress {
                    downloaded_bytes: 0,
                    total_bytes: None,
                    percent: None,
                    stage: "error".to_string(),
                },
            );
            e.to_string()
        })?;

    let _ = app.emit(
        "updater:progress",
        UpdateProgress {
            downloaded_bytes: 0,
            total_bytes: None,
            percent: Some(100.0),
            stage: "ready".to_string(),
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn restart_app(app: AppHandle) -> Result<(), String> {
    app.restart();
}
