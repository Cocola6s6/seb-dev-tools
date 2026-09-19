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
use pages::{
    battery::BatteryPage, client::ClientPage, control::ControlPage, deploy::DeployPage,
    ecu::EcuPage, settings::SettingsPage,
};
use state::{AppCtx, Page};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

fn main() {
    console_error_panic_hook::set_once();
    sycamore::render(App);
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
