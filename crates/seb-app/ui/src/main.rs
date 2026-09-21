mod actions;
mod api;
mod components;
mod pages;
mod state;

use components::{
    GlobalToast, InfraPill, InstancePill, KnockEgg, LogPane, OnlinePills, ToolboxNavButton,
    WhatsNewNotice,
};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen::JsCast;
use pages::{
    battery::BatteryPage, client::ClientPage, control::ControlPage, deploy::DeployPage,
    ecu::EcuPage, settings::SettingsPage,
};
use state::{AppCtx, Page};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

fn main() {
    console_error_panic_hook::set_once();
    // 同一份 WASM 三个窗口共用，靠 hash 分流：
    // #dock 是桌面悬浮工具条，#egg 是工具条点鸡蛋时铺满屏幕的那层动画
    match web_sys::window()
        .and_then(|w| w.location().hash().ok())
        .unwrap_or_default()
        .as_str()
    {
        "#dock" => {
            mark_view("dock");
            sycamore::render(components::Dock);
        }
        "#egg" => {
            mark_view("egg");
            sycamore::render(components::EggWindow);
        }
        _ => sycamore::render(App),
    }
}

/// 这两个窗都是透明的，样式另走一套。html 也要标上：它自己有底色，
/// 只管 body 的话透明区会是一层白蒙版
fn mark_view(class: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    if let Some(html) = doc.document_element() {
        let _ = html.class_list().add_1(class);
    }
    if let Some(body) = doc.body() {
        let _ = body.class_list().add_1(class);
    }
}

#[component]
fn App() -> View {
    let ctx = AppCtx::new();
    provide_context(ctx);

    spawn_local(async move {
        match api::get_config().await {
            Ok(cfg) => {
                ctx.adopt_config(cfg);
                ctx.refresh_instance(true);
            }
            Err(e) => ctx.log_error(format!("【错误】读取配置失败: {e}")),
        }
        if let Ok(conn) = api::get_conn_state().await {
            ctx.conn.set(conn);
        }
        match api::list_ecu_params().await {
            Ok(list) => ctx.ecu_params.set(list),
            Err(e) => ctx.log_error(format!("【错误】加载 ECU 参数字典失败: {e}")),
        }
        match api::list_control_types().await {
            Ok(list) => ctx.control_types.set(list),
            Err(e) => ctx.log_error(format!("【错误】加载控制命令失败: {e}")),
        }
        match api::load_deploy_options().await {
            Ok(opts) => ctx.deploy_options.set(opts),
            Err(e) => ctx.log_warn(format!("【警告】从数据库加载配置选项失败，使用默认选项: {e}")),
        }
        if let Ok(d) = api::client_defaults().await {
            ctx.client.adopt_defaults(d);
        }
        match api::client_devices().await {
            Ok(list) if !list.is_empty() => {
                let first = list[0].config.device_no.clone();
                ctx.client.devices.set(list);
                ctx.client.select(&first);
            }
            // 还没有设备清单：把主配置里的中控序列号收编成第一台模拟设备
            Ok(_) => {
                let no = ctx.device_no.get_clone().trim().to_string();
                if !no.is_empty() {
                    let config = ctx
                        .client
                        .config(no.clone());
                    match api::client_update_device(config).await {
                        Ok(list) => {
                            ctx.client.devices.set(list);
                            ctx.client.select(&no);
                        }
                        Err(e) => ctx.log_error(format!("【错误】初始化模拟设备失败: {e}")),
                    }
                }
            }
            Err(e) => ctx.log_error(format!("【错误】读取模拟设备清单失败: {e}")),
        }
        if let Ok(d) = api::battery_defaults().await {
            ctx.battery.adopt_defaults(d);
        }
        match api::battery_devices().await {
            Ok(list) if !list.is_empty() => {
                let first = list[0].config.battery_no.clone();
                ctx.battery.devices.set(list);
                ctx.battery.select(&first);
            }
            Ok(_) => {
                let no = ctx.battery_no.get_clone().trim().to_string();
                let no = if no.is_empty() {
                    "CMAH030799497009".to_string()
                } else {
                    no
                };
                let config = ctx.battery.config(no.clone());
                if let Ok(list) = api::battery_update_device(config).await {
                    ctx.battery.devices.set(list);
                    ctx.battery.select(&no);
                }
            }
            Err(e) => ctx.log_error(format!("【错误】读取电池模拟设备清单失败: {e}")),
        }
        if let Ok(list) = api::list_alarm_types().await {
            if let Some(first) = list.first() {
                ctx.client.alarm_type.set(first.code.to_string());
            }
            ctx.client.alarm_types.set(list);
        }
        ctx.log_info("就绪。可先进行「一键接入」将车辆接入内网，随后进行中控、ECU 与电池调试");
    });

    setup_collapse_listener();

    spawn_local(async move {
        loop {
            TimeoutFuture::new(500).await;
            if let Ok(poll) = api::client_poll().await {
                // 内容没变就不要重置信号，否则设备列表每 500ms 重渲染一次
                if poll.devices != ctx.client.devices.get_clone() {
                    ctx.client.devices.set(poll.devices);
                }
                for f in poll.frames {
                    ctx.log_frame(&f.dir, &f.device_no, &f);
                }
            }
            if let Ok(logs) = api::take_ui_logs().await {
                for l in logs {
                    if l.level == "error" {
                        ctx.log_error(l.text);
                    } else {
                        ctx.log_info(l.text);
                    }
                }
            }
            if let Ok(bpoll) = api::battery_poll().await {
                if bpoll.devices != ctx.battery.devices.get_clone() {
                    ctx.battery.devices.set(bpoll.devices);
                }
                for f in bpoll.frames {
                    ctx.log_battery_frame(&f.dir, &f.battery_no, &f);
                }
            }
        }
    });

    // 三个依赖的连通性会中途变，得定期重新探一次，否则头上挂的一直是启动那一刻的状态
    spawn_local(async move {
        loop {
            TimeoutFuture::new(10_000).await;
            if let Ok(conn) = api::get_conn_state().await {
                if conn != ctx.conn.get_clone() {
                    ctx.conn.set(conn);
                }
            }
        }
    });

    let show_pills = move || ctx.page_devices().is_none();

    view! {
        div(class=move || format!("app tint-{}", ctx.page.get().tint())) {
            // WKWebView 不认 -webkit-app-region，必须用 Tauri 自己的拖拽区标记
            header(class="header", data-tauri-drag-region="") {
                nav(class="nav") {
                    ToolboxNavButton {}
                    div(class=move || format!("nav-track active-{}", ctx.page.get().index())) {
                        div(class="nav-indicator") {}
                        NavItem(page=Page::Deploy, label="一键接入")
                        NavItem(page=Page::Control, label="中控指令")
                        NavItem(page=Page::Ecu, label="中控配置")
                        NavItem(page=Page::Client, label="中控客户端")
                        NavItem(page=Page::Battery, label="电池客户端")
                    }
                }
                div(class="pills") {
                    InfraPill {}
                    (if show_pills() {
                        view! { InstancePill {} }
                    } else {
                        view! { OnlinePills {} }
                    })
                }
            }

            div(class="main") {
                // 五个页面常驻，靠显隐切换：卸载会销毁页面里的信号，
                // 正在飞的异步回来一读就 panic，表单填的内容也会丢
                div(class="content") {
                    div(class=show(ctx, Page::Deploy))  { DeployPage {} }
                    div(class=show(ctx, Page::Control)) { ControlPage {} }
                    div(class=show(ctx, Page::Ecu))     { EcuPage {} }
                    div(class=show(ctx, Page::Client))  { ClientPage {} }
                    div(class=show(ctx, Page::Battery)) { BatteryPage {} }
                    div(class=show_settings(ctx))       { SettingsPage {} }
                }
            }

            LogPane {}
            GlobalToast {}
            KnockEgg {}
            WhatsNewNotice {}
        }
    }
}

fn setup_collapse_listener() {
    let cb = wasm_bindgen::closure::Closure::<dyn FnMut(Option<f64>, Option<f64>)>::wrap(Box::new(
        move |opt_x: Option<f64>, opt_y: Option<f64>| {
            if let (Some(x), Some(y)) = (opt_x, opt_y) {
                set_fold_target(x, y);
            }
            set_body_class("collapsing", true);
            spawn_local(async move {
                TimeoutFuture::new(240).await;
                let _ = api::collapse_to_dock().await;
                // 窗口已经不可见了才摘掉这个类，否则会看到它原地弹回来
                set_body_class("collapsing", false);
            });
        },
    ));
    if let Some(w) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &w,
            &wasm_bindgen::JsValue::from_str("__triggerCollapse"),
            cb.as_ref().unchecked_ref(),
        );
        cb.forget();
    }
}

fn set_body_class(class: &str, on: bool) {
    let Some(body) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.body()) else {
        return;
    };
    let list = body.class_list();
    let _ = if on { list.add_1(class) } else { list.remove_1(class) };
}

/// 收起动画往哪儿飞：工具条中心相对主窗左上角的位置，交给 CSS 变量
#[allow(dead_code)]
fn set_fold_target(x: f64, y: f64) {
    let Some(html) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.document_element())
    else {
        return;
    };
    if let Some(el) = html.dyn_ref::<web_sys::HtmlElement>() {
        let _ = el.style().set_property("--fold-x", &format!("{x}px"));
        let _ = el.style().set_property("--fold-y", &format!("{y}px"));
    }
}

fn show(ctx: AppCtx, page: Page) -> impl Fn() -> &'static str {
    move || {
        if !ctx.is_settings.get() && ctx.page.get() == page {
            "page"
        } else {
            "page hidden"
        }
    }
}

fn show_settings(ctx: AppCtx) -> impl Fn() -> &'static str {
    move || {
        if ctx.is_settings.get() {
            "page"
        } else {
            "page hidden"
        }
    }
}

#[component(inline_props)]
fn NavItem(page: Page, label: &'static str) -> View {
    let ctx = use_context::<AppCtx>();
    view! {
        button(
            class=move || if ctx.page.get() == page { "nav-item active" } else { "nav-item" },
            on:click=move |_| ctx.page.set(page)
        ) { (label) }
    }
}
