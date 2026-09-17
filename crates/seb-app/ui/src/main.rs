mod actions;
mod api;
mod components;
mod pages;
mod state;

use components::LogPane;
use pages::{control::ControlPage, deploy::DeployPage, ecu::EcuPage};
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
        ctx.log_info("就绪。可先进行「一键接入」将车辆接入内网，随后进行中控与 ECU 调试");
    });

    view! {
        div(class=move || format!("app tint-{}", ctx.page.get().tint())) {
            // WKWebView 不认 -webkit-app-region，必须用 Tauri 自己的拖拽区标记
            header(class="header", data-tauri-drag-region="") {
                nav(class="nav") {
                    NavItem(page=Page::Deploy, label="一键接入")
                    NavItem(page=Page::Control, label="中控指令")
                    NavItem(page=Page::Ecu, label="中控配置")
                }
                div(class="pills") {
                    span(class="pill") {
                        span(class=if ctx.conn.get_clone().mq { "dot on" } else { "dot off" }) {}
                        span { (if ctx.conn.get_clone().mq { "MQ 正常" } else { "MQ 未连" }) }
                    }
                    span(class="pill") {
                        span(class=if ctx.conn.get_clone().mysql { "dot on" } else { "dot off" }) {}
                        span { (if ctx.conn.get_clone().mysql { "MySQL 正常" } else { "MySQL 未连" }) }
                    }
                    span(class="pill") {
                        span(class=if ctx.conn.get_clone().redis { "dot on" } else { "dot off" }) {}
                        span { (if ctx.conn.get_clone().redis { "Redis 正常" } else { "Redis 未连" }) }
                    }
                }
            }

            div(class="main") {
                div(class="content") {
                    (match ctx.page.get() {
                        Page::Deploy   => view! { DeployPage {} },
                        Page::Control  => view! { ControlPage {} },
                        Page::Ecu      => view! { EcuPage {} },
                    })
                }
            }

            LogPane {}
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
