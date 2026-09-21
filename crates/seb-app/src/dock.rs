use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

const DOCK_LABEL: &str = "dock";
const EGG_LABEL: &str = "egg";
pub const MAIN_LABEL: &str = "main";

const STRIP_W: f64 = 76.0;
const CARD_W: f64 = 292.0;
const DOCK_H: f64 = 306.0;
const EDGE_GAP: f64 = 24.0;

fn expand_side() -> &'static Mutex<Option<&'static str>> {
    static SIDE: OnceLock<Mutex<Option<&'static str>>> = OnceLock::new();
    SIDE.get_or_init(|| Mutex::new(None))
}

fn pending_pos() -> &'static Mutex<Option<(i32, i32)>> {
    static POS: OnceLock<Mutex<Option<(i32, i32)>>> = OnceLock::new();
    POS.get_or_init(|| Mutex::new(None))
}

fn last_programmatic() -> &'static Mutex<std::time::Instant> {
    static AT: OnceLock<Mutex<std::time::Instant>> = OnceLock::new();
    AT.get_or_init(|| Mutex::new(std::time::Instant::now()))
}

fn touch_geometry() {
    if let Ok(mut at) = last_programmatic().lock() {
        *at = std::time::Instant::now();
    }
}

static DRAGGING: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DockQrSelection {
    pub bike_no: String,
    pub battery_no: String,
}

fn qr_selection() -> &'static Mutex<DockQrSelection> {
    static SELECTION: OnceLock<Mutex<DockQrSelection>> = OnceLock::new();
    SELECTION.get_or_init(|| Mutex::new(DockQrSelection::default()))
}

pub fn set_qr_selection(bike_no: String, battery_no: String) {
    if let Ok(mut selection) = qr_selection().lock() {
        selection.bike_no = bike_no;
        selection.battery_no = battery_no;
    }
}

pub fn get_qr_selection() -> DockQrSelection {
    qr_selection().lock().map(|selection| selection.clone()).unwrap_or_default()
}

pub fn start_drag(app: &AppHandle) -> Result<(), String> {
    let dock = app.get_webview_window(DOCK_LABEL).ok_or("找不到工具条")?;
    DRAGGING.store(true, Ordering::Relaxed);
    dock.start_dragging().map_err(|e| e.to_string())
}

fn monitor_at(app: &AppHandle, x: i32, y: i32) -> Option<tauri::Monitor> {
    app.available_monitors().ok()?.into_iter().find(|m| {
        let p = m.position();
        let s = m.size();
        x >= p.x && y >= p.y && x < p.x + s.width as i32 && y < p.y + s.height as i32
    })
}

pub fn anchor(app: &AppHandle) -> Option<(f64, f64)> {
    let dock = app.get_webview_window(DOCK_LABEL)?;
    let main = app.get_webview_window(MAIN_LABEL)?;
    let scale = main.scale_factor().unwrap_or(1.0);
    let dp = dock.outer_position().ok()?;
    let mp = main.outer_position().ok()?;
    let cx = dp.x as f64 + STRIP_W * scale / 2.0;
    let cy = dp.y as f64 + DOCK_H * scale / 2.0;
    Some(((cx - mp.x as f64) / scale, (cy - mp.y as f64) / scale))
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Copy)]
struct Placement {
    x: i32,
    y: i32,
}

fn placement_path() -> Option<std::path::PathBuf> {
    seb_core::config::config_path()
        .ok()
        .map(|p| p.with_file_name("dock.json"))
}

fn load_placement() -> Option<Placement> {
    let path = placement_path()?;
    let body = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&body).ok()
}

fn save_placement(x: i32, y: i32) {
    let Some(path) = placement_path() else { return };
    if let Ok(body) = serde_json::to_string(&Placement { x, y }) {
        let _ = std::fs::write(path, body);
    }
}

fn build_dock(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let dock = WebviewWindowBuilder::new(
        app,
        DOCK_LABEL,
        WebviewUrl::App("index.html#dock".into()),
    )
    .title("工具条")
    .inner_size(STRIP_W, DOCK_H)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible(false)
    .build()?;

    let _ = dock.set_visible_on_all_workspaces(true);

    match load_placement().filter(|p| monitor_at(app, p.x, p.y).is_some()) {
        Some(p) => {
            touch_geometry();
            let _ = dock.set_position(PhysicalPosition::new(p.x, p.y));
        }
        None => place_default(&dock),
    }
    Ok(dock)
}

pub fn setup(app: &AppHandle) -> tauri::Result<()> {
    build_dock(app)?;

    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        let taken = pending_pos().lock().ok().and_then(|mut p| p.take());
        if let Some((x, y)) = taken {
            save_placement(x, y);
        }
    });

    Ok(())
}

fn place_default(dock: &tauri::WebviewWindow) {
    let scale = dock.scale_factor().unwrap_or(1.0);
    let monitor = dock
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| dock.app_handle().primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let m_pos = monitor.position();
    let m_size = monitor.size();
    let h = DOCK_H * scale;
    let x = m_pos.x + (EDGE_GAP * scale) as i32;
    let y = m_pos.y + ((m_size.height as f64 - h) / 2.0) as i32;
    touch_geometry();
    let _ = dock.set_position(PhysicalPosition::new(x, y));
}

pub fn hide_dock(app: &AppHandle) -> Result<(), String> {
    collapse(app);
    let dock = app.get_webview_window(DOCK_LABEL).ok_or("找不到工具条")?;
    dock.hide().map_err(|e| e.to_string())
}

pub fn show_dock(app: &AppHandle) {
    match app.get_webview_window(DOCK_LABEL) {
        Some(dock) => {
            let offscreen = dock
                .outer_position()
                .ok()
                .map(|p| monitor_at(app, p.x, p.y).is_none())
                .unwrap_or(true);
            if offscreen {
                touch_geometry();
                place_default(&dock);
            }
            let _ = dock.show();
            let _ = dock.set_always_on_top(true);
            let _ = dock.eval("window.__refreshDockQr && window.__refreshDockQr();");
        }
        None => {
            let _ = build_dock(app);
        }
    }
}

fn collapse(app: &AppHandle) -> Option<()> {
    DRAGGING.store(false, Ordering::Relaxed);
    let dock = app.get_webview_window(DOCK_LABEL)?;
    let side = expand_side().lock().ok()?.take()?;
    let scale = dock.scale_factor().unwrap_or(1.0);
    let strip = (STRIP_W * scale).round();
    let card = (CARD_W * scale).round();
    let h = (DOCK_H * scale).round();
    let pos = dock.outer_position().ok()?;
    touch_geometry();
    let _ = dock.set_size(PhysicalSize::new(strip as u32, h as u32));
    if side == "left" {
        let _ = dock.set_position(PhysicalPosition::new(pos.x + card as i32, pos.y));
    }
    touch_geometry();
    Some(())
}

pub fn expand(app: &AppHandle, expanded: bool) -> String {
    DRAGGING.store(false, Ordering::Relaxed);
    let Some(dock) = app.get_webview_window(DOCK_LABEL) else {
        return "right".to_string();
    };
    let current = expand_side().lock().ok().and_then(|s| *s);

    if !expanded {
        collapse(app);
        return current.unwrap_or("right").to_string();
    }
    if let Some(side) = current {
        return side.to_string();
    }

    let scale = dock.scale_factor().unwrap_or(1.0);
    let strip = (STRIP_W * scale).round();
    let card = (CARD_W * scale).round();
    let h = (DOCK_H * scale).round();
    let pos = dock.outer_position().unwrap_or(PhysicalPosition::new(0, 0));

    let mut side = "right";
    if let Some(monitor) = monitor_at(app, pos.x, pos.y).or_else(|| dock.current_monitor().ok().flatten()) {
        let right_edge = monitor.position().x as f64 + monitor.size().width as f64;
        if pos.x as f64 + strip + card > right_edge {
            side = "left";
        }
    }

    if let Ok(mut slot) = expand_side().lock() {
        *slot = Some(side);
    };
    touch_geometry();
    if side == "left" {
        let _ = dock.set_position(PhysicalPosition::new(pos.x - card as i32, pos.y));
    }
    let _ = dock.set_size(PhysicalSize::new((strip + card) as u32, h as u32));
    touch_geometry();
    side.to_string()
}

pub fn show_egg(app: &AppHandle) -> Result<(), String> {
    if app.get_webview_window(EGG_LABEL).is_some() {
        return Ok(());
    }
    let screen = app
        .get_webview_window(DOCK_LABEL)
        .and_then(|d| d.current_monitor().ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let egg = WebviewWindowBuilder::new(app, EGG_LABEL, WebviewUrl::App("index.html#egg".into()))
        .title("鸡蛋")
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(m) = screen {
        let _ = egg.set_position(PhysicalPosition::new(m.position().x, m.position().y));
        let _ = egg.set_size(PhysicalSize::new(m.size().width, m.size().height));
    }
    let _ = egg.set_visible_on_all_workspaces(true);
    let _ = egg.show();
    let _ = egg.set_focus();
    Ok(())
}

pub fn hide_egg(app: &AppHandle) -> Result<(), String> {
    match app.get_webview_window(EGG_LABEL) {
        Some(egg) => egg.close().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

pub fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window(MAIN_LABEL) {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
    let _ = hide_dock(app);
}

pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    match (window.label(), event) {
        (DOCK_LABEL, WindowEvent::Moved(pos)) => {
            let expanded = expand_side().lock().ok().and_then(|s| *s).is_some();
            let settling = last_programmatic()
                .lock()
                .map(|at| at.elapsed() < std::time::Duration::from_millis(500))
                .unwrap_or(false);
            if !expanded && (DRAGGING.load(Ordering::Relaxed) || !settling) {
                if let Ok(mut slot) = pending_pos().lock() {
                    *slot = Some((pos.x, pos.y));
                }
            }
        }
        (DOCK_LABEL, WindowEvent::CloseRequested { api, .. }) => {
            api.prevent_close();
            let _ = window.hide();
        }
        (MAIN_LABEL, WindowEvent::CloseRequested { .. }) => {
            window.app_handle().exit(0);
        }
        _ => {}
    }
}
