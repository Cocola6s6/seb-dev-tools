mod actions;
mod api;
mod components;
mod pages;
mod state;

use components::{LogPane, QrNavButton};
use gloo_timers::future::TimeoutFuture;
use pages::{client::ClientPage, control::ControlPage, deploy::DeployPage, ecu::EcuPage};
use state::{AppCtx, FrameLog, Page};
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
        if let Ok(list) = api::list_alarm_types().await {
            if let Some(first) = list.first() {
                ctx.client.alarm_type.set(first.code.to_string());
            }
            ctx.client.alarm_types.set(list);
        }
        ctx.log_info("就绪。可先进行「一键接入」将车辆接入内网，随后进行中控与 ECU 调试");
    });

    spawn_local(async move {
        loop {
            TimeoutFuture::new(500).await;
            if let Ok(poll) = api::client_poll().await {
                // 后台开关和界面不一致，说明上一次同步没送到，补一次
                if poll.state.auto_reply != ctx.client.auto_reply.get() {
                    let profile = ctx.client.profile(ctx.battery_no.get_clone().trim().to_string());
                    let auto_reply = ctx.client.auto_reply.get();
                    let _ = api::client_set_profile(profile, auto_reply).await;
                }
                ctx.client.state.set(poll.state);
                for f in poll.frames {
                    ctx.log_frame(&f.dir, frame_line(&f));
                }
            }
        }
    });

    let show_pills = move || ctx.page.get() != Page::Client;

    view! {
        div(class=move || format!("app tint-{}", ctx.page.get().tint())) {
            // WKWebView 不认 -webkit-app-region，必须用 Tauri 自己的拖拽区标记
            header(class="header", data-tauri-drag-region="") {
                nav(class="nav") {
                    QrNavButton {}
                    div(class=move || format!("nav-track active-{}", ctx.page.get().index())) {
                        div(class="nav-indicator") {}
                        NavItem(page=Page::Deploy, label="一键接入")
                        NavItem(page=Page::Control, label="中控指令")
                        NavItem(page=Page::Ecu, label="中控配置")
                        NavItem(page=Page::Client, label="中控客户端")
                    }
                }
                (if show_pills() {
                    view! {
                        div(class="pills") {
                            span(class="pill") { span(class="dot on") {} span { "已连接：内网" } }
                            span(class="pill") {
                                span(class=move || if ctx.conn.get_clone().mysql { "dot on" } else { "dot off" }) {}
                                span { (move || if ctx.conn.get_clone().mysql { "MySQL 正常" } else { "MySQL 未连" }) }
                            }
                            span(class="pill") {
                                span(class=move || if ctx.conn.get_clone().redis { "dot on" } else { "dot off" }) {}
                                span { (move || if ctx.conn.get_clone().redis { "Redis 正常" } else { "Redis 未连" }) }
                            }
                            span(class="pill") {
                                span(class=move || if ctx.conn.get_clone().mq { "dot on" } else { "dot off" }) {}
                                span { (move || if ctx.conn.get_clone().mq { "MQ 正常" } else { "MQ 未连" }) }
                            }
                        }
                    }
                } else {
                    view! {
                        div(class="pills") {
                            span(class="pill") {
                                span(class=move || if ctx.client.state.get_clone().connected { "dot on" } else { "dot off" }) {}
                                span { (move || {
                                    let st = ctx.client.state.get_clone();
                                    if st.connected { format!("已连接 {}", st.endpoint) } else { "未连接".to_string() }
                                }) }
                            }
                        }
                    }
                })
            }

            div(class="main") {
                div(class="content") {
                    (match ctx.page.get() {
                        Page::Deploy   => view! { DeployPage {} },
                        Page::Control  => view! { ControlPage {} },
                        Page::Ecu      => view! { EcuPage {} },
                        Page::Client   => view! { ClientPage {} },
                    })
                }
            }

            LogPane {}
        }
    }
}

fn frame_line(f: &FrameLog) -> String {
    if f.hex.is_empty() {
        f.summary.clone()
    } else {
        format!("{} | {}", f.summary, f.hex)
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
