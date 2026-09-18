mod actions;
mod api;
mod components;
mod pages;
mod state;

use components::{LogPane, QrNavButton};
use gloo_timers::future::TimeoutFuture;
use pages::{client::ClientPage, control::ControlPage, deploy::DeployPage, ecu::EcuPage};
use state::{host_label, AppCtx, FrameLog, Page};
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
                        .config(no.clone(), ctx.battery_no.get_clone().trim().to_string());
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
                // 内容没变就不要重置信号，否则设备列表每 500ms 重渲染一次
                if poll.devices != ctx.client.devices.get_clone() {
                    ctx.client.devices.set(poll.devices);
                }
                for f in poll.frames {
                    ctx.log_frame(&f.dir, &f.device_no, frame_line(&f));
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

    let show_pills = move || ctx.page.get() != Page::Client;
    // 可能同时连不同环境，按网关地址分组，别把两个网关的在线数混在一起
    let groups = create_memo(move || {
        let mut groups: Vec<(String, usize, usize)> = Vec::new();
        for d in ctx.client.devices.get_clone() {
            let label = host_label(&d.config.host);
            match groups.iter_mut().find(|g| g.0 == label) {
                Some(g) => {
                    g.1 += usize::from(d.connected);
                    g.2 += 1;
                }
                None => groups.push((label, usize::from(d.connected), 1)),
            }
        }
        groups
    });

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
                            span(class="pill") {
                                (conn_icon(ctx, "MySQL", |c| c.mysql, ICON_DB))
                                (conn_icon(ctx, "Redis", |c| c.redis, ICON_CACHE))
                                (conn_icon(ctx, "MQ", |c| c.mq, ICON_MQ))
                                span { "内网" }
                            }
                        }
                    }
                } else {
                    view! {
                        div(class="pills") {
                            Indexed(
                                list=groups,
                                view=move |(label, online, total): (String, usize, usize)| {
                                    view! {
                                        span(class="pill") {
                                            span(class=if online > 0 { "dot on" } else { "dot off" }) {}
                                            span { (format!("{label} 在线 {online}/{total}")) }
                                        }
                                    }
                                }
                            )
                        }
                    }
                })
            }

            div(class="main") {
                // 四个页面常驻，靠显隐切换：卸载会销毁页面里的信号，
                // 正在飞的异步回来一读就 panic，表单填的内容也会丢
                div(class="content") {
                    div(class=show(ctx, Page::Deploy))  { DeployPage {} }
                    div(class=show(ctx, Page::Control)) { ControlPage {} }
                    div(class=show(ctx, Page::Ecu))     { EcuPage {} }
                    div(class=show(ctx, Page::Client))  { ClientPage {} }
                }
            }

            LogPane {}
        }
    }
}

const ICON_DB: &str = "M8 1.6c2.8 0 5 .8 5 1.8s-2.2 1.8-5 1.8-5-.8-5-1.8 2.2-1.8 5-1.8zM3 3.4v9.2c0 1 2.2 1.8 5 1.8s5-.8 5-1.8V3.4M3 8c0 1 2.2 1.8 5 1.8s5-.8 5-1.8";
const ICON_CACHE: &str = "M8.8 1.5L3.8 8.6H7.6L7.2 14.5L12.2 7.4H8.4L8.8 1.5z";
const ICON_MQ: &str = "M2 3.5h12v9H2zM2 4l6 4.2L14 4";

fn conn_icon(ctx: AppCtx, label: &'static str, pick: fn(&state::ConnState) -> bool, d: &'static str) -> View {
    let ok = move || pick(&ctx.conn.get_clone());
    view! {
        div(class="btn-with-tip") {
            span(class=move || if ok() { "conn-icon on" } else { "conn-icon off" }) {
                svg(viewBox="0 0 16 16", width="13", height="13") {
                    path(d=d, fill="none", stroke="currentColor", stroke-width="1.3",
                         stroke-linecap="round", stroke-linejoin="round")
                }
            }
            span(class="tooltip") {
                (move || format!("{label} {}", if ok() { "正常" } else { "未连" }))
            }
        }
    }
}

fn show(ctx: AppCtx, page: Page) -> impl Fn() -> &'static str {
    move || {
        if ctx.page.get() == page {
            "page"
        } else {
            "page hidden"
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
