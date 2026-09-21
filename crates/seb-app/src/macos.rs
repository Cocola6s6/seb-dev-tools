use std::ffi::{c_char, c_void, CString};
use std::sync::OnceLock;
use tauri::{AppHandle, Manager, WebviewWindow};

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static HANDLER_INSTANCE: OnceLock<usize> = OnceLock::new();

extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_allocateClassPair(superclass: *mut c_void, name: *const c_char, extra_bytes: usize) -> *mut c_void;
    fn class_addMethod(cls: *mut c_void, name: *mut c_void, imp: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void), types: *const c_char) -> bool;
    fn objc_registerClassPair(cls: *mut c_void);
}

type MsgSendPtr = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
type MsgSendPtr1 = unsafe extern "C" fn(*mut c_void, *mut c_void, usize) -> *mut c_void;
type MsgSendVoid1 = unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void);

extern "C" {
    fn objc_msgSend();
}

unsafe extern "C" fn on_minimize_imp(_this: *mut c_void, _sel: *mut c_void, _sender: *mut c_void) {
    if let Some(app) = APP_HANDLE.get() {
        if let Some(win) = app.get_webview_window(crate::dock::MAIN_LABEL) {
            let (x, y) = crate::dock::anchor(app).unwrap_or((-60.0, 300.0));
            let js = format!("window.__triggerCollapse && window.__triggerCollapse({x:.1}, {y:.1});");
            let _ = win.eval(&js);
        }
    }
}

pub fn setup_traffic_lights(app: &AppHandle, win: &WebviewWindow) {
    let _ = APP_HANDLE.set(app.clone());

    let Ok(ns_ptr) = win.ns_window() else { return };
    if ns_ptr.is_null() {
        return;
    }

    unsafe {
        let handler_ptr = *HANDLER_INSTANCE.get_or_init(|| {
            let class_name = CString::new("SEBWindowMinimizeHandler").unwrap();
            let mut cls = objc_getClass(class_name.as_ptr());
            if cls.is_null() {
                let superclass = objc_getClass(CString::new("NSObject").unwrap().as_ptr());
                cls = objc_allocateClassPair(superclass, class_name.as_ptr(), 0);
                let types = CString::new("v@:@").unwrap();
                let sel_minimize = sel_registerName(CString::new("onMinimize:").unwrap().as_ptr());
                class_addMethod(cls, sel_minimize, on_minimize_imp, types.as_ptr());
                objc_registerClassPair(cls);
            }

            let msg_send_ptr: MsgSendPtr = std::mem::transmute(objc_msgSend as *const ());
            let sel_alloc = sel_registerName(CString::new("alloc").unwrap().as_ptr());
            let sel_init = sel_registerName(CString::new("init").unwrap().as_ptr());
            let allocated = msg_send_ptr(cls, sel_alloc);
            let instance = msg_send_ptr(allocated, sel_init);
            instance as usize
        }) as *mut c_void;

        let msg_send_ptr1: MsgSendPtr1 = std::mem::transmute(objc_msgSend as *const ());
        let msg_send_void1: MsgSendVoid1 = std::mem::transmute(objc_msgSend as *const ());

        let sel_btn = sel_registerName(CString::new("standardWindowButton:").unwrap().as_ptr());
        // 1 = NSWindowMiniaturizeButton
        let min_btn = msg_send_ptr1(ns_ptr, sel_btn, 1);
        if !min_btn.is_null() {
            let sel_set_target = sel_registerName(CString::new("setTarget:").unwrap().as_ptr());
            let sel_set_action = sel_registerName(CString::new("setAction:").unwrap().as_ptr());
            let sel_minimize = sel_registerName(CString::new("onMinimize:").unwrap().as_ptr());

            msg_send_void1(min_btn, sel_set_target, handler_ptr);
            msg_send_void1(min_btn, sel_set_action, sel_minimize);
        }
    }
}
