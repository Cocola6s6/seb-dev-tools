use std::ffi::c_void;
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewWindow};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

const WM_SYSCOMMAND: u32 = 0x0112;
const SC_MINIMIZE: usize = 0xF020;

#[link(name = "comctl32")]
extern "system" {
    fn SetWindowSubclass(
        hwnd: *mut c_void,
        pfn_subclass: unsafe extern "system" fn(
            *mut c_void,
            u32,
            usize,
            isize,
            usize,
            usize,
        ) -> isize,
        uid_subclass: usize,
        dw_ref_data: usize,
    ) -> i32;

    fn DefSubclassProc(
        hwnd: *mut c_void,
        msg: u32,
        wparam: usize,
        lparam: isize,
    ) -> isize;
}

unsafe extern "system" fn subclass_proc(
    hwnd: *mut c_void,
    msg: u32,
    wparam: usize,
    lparam: isize,
    _uid_subclass: usize,
    _dw_ref_data: usize,
) -> isize {
    if msg == WM_SYSCOMMAND && (wparam & 0xFFF0) == SC_MINIMIZE {
        if let Some(app) = APP_HANDLE.get() {
            if let Some(win) = app.get_webview_window(crate::dock::MAIN_LABEL) {
                let (x, y) = crate::dock::anchor(app).unwrap_or((-60.0, 300.0));
                let js = format!("window.__triggerCollapse && window.__triggerCollapse({x:.1}, {y:.1});");
                let _ = win.eval(&js);
                return 0;
            }
        }
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

pub fn setup_window_minimize(app: &AppHandle, win: &WebviewWindow) {
    let _ = APP_HANDLE.set(app.clone());
    let Ok(hwnd) = win.hwnd() else { return };
    let raw_hwnd = hwnd.0 as *mut c_void;
    if raw_hwnd.is_null() {
        return;
    }
    unsafe {
        SetWindowSubclass(raw_hwnd, subclass_proc, 1001, 0);
    }
}
