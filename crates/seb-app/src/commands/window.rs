use crate::dock::{self, MAIN_LABEL};
use tauri::{AppHandle, Manager};

#[tauri::command]
pub async fn collapse_to_dock(app: AppHandle) -> Result<(), String> {
    dock::show_dock(&app);
    match app.get_webview_window(MAIN_LABEL) {
        Some(win) => win.hide().map_err(|e| e.to_string()),
        None => Err("找不到主窗口".to_string()),
    }
}

#[tauri::command]
pub async fn show_main(app: AppHandle) -> Result<(), String> {
    dock::show_main(&app);
    Ok(())
}

#[tauri::command]
pub async fn dock_expand(app: AppHandle, expanded: bool) -> Result<String, String> {
    Ok(dock::expand(&app, expanded))
}

#[tauri::command]
pub async fn dock_drag(app: AppHandle) -> Result<(), String> {
    dock::start_drag(&app)
}

#[tauri::command]
pub async fn dock_anchor(app: AppHandle) -> Result<(f64, f64), String> {
    dock::anchor(&app).ok_or_else(|| "拿不到工具条位置".to_string())
}

#[tauri::command]
pub async fn set_dock_qr_selection(bike_no: String, battery_no: String) -> Result<(), String> {
    dock::set_qr_selection(bike_no, battery_no);
    Ok(())
}

#[tauri::command]
pub async fn get_dock_qr_selection() -> Result<dock::DockQrSelection, String> {
    Ok(dock::get_qr_selection())
}

#[tauri::command]
pub async fn dock_egg(app: AppHandle, on: bool) -> Result<(), String> {
    if on {
        dock::show_egg(&app)
    } else {
        dock::hide_egg(&app)
    }
}

#[tauri::command]
pub async fn hide_dock(app: AppHandle) -> Result<(), String> {
    dock::hide_dock(&app)
}
