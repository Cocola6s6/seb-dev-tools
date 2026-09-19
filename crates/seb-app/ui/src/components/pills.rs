use crate::api;
use crate::state::{
    host_label, AppCtx, ConnState, ControlGlobalSettings,
};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const ICON_DB: &str = "M8 1.6c2.8 0 5 .8 5 1.8s-2.2 1.8-5 1.8-5-.8-5-1.8 2.2-1.8 5-1.8zM3 3.4v9.2c0 1 2.2 1.8 5 1.8s5-.8 5-1.8V3.4M3 8c0 1 2.2 1.8 5 1.8s5-.8 5-1.8";
const ICON_CACHE: &str = "M8.8 1.5L3.8 8.6H7.6L7.2 14.5L12.2 7.4H8.4L8.8 1.5z";
const ICON_MQ: &str = "M2 3.5h12v9H2zM2 4l6 4.2L14 4";
const ICON_PULSE: &str = "M1.5 8h2.5l2-4.5 3 9 2-4.5h3.5";
const ICON_PIN: &str = "M8 1.5a4 4 0 0 0-4 4c0 3.2 4 8.5 4 8.5s4-5.3 4-8.5a4 4 0 0 0-4-4zm0 5.5a1.5 1.5 0 1 1 0-3 1.5 1.5 0 0 1 0 3z";

fn conn_svg(ok: impl Fn() -> bool + 'static, d: &'static str) -> View {
    view! {
        span(class=move || if ok() { "conn-icon on" } else { "conn-icon off" }) {
            svg(viewBox="0 0 16 16", width="13", height="13") {
                path(d=d, fill="none", stroke="currentColor", stroke-width="1.3",
                     stroke-linecap="round", stroke-linejoin="round")
            }
        }
    }
}

fn conn_icon(ctx: AppCtx, label: &'static str, pick: fn(&ConnState) -> bool, d: &'static str) -> View {
    let ok = move || pick(&ctx.conn.get_clone());
    view! {
        div(class="btn-with-tip") {
            (conn_svg(ok, d))
            span(class="tooltip") {
                (move || format!("{label} {}", if ok() { "正常" } else { "未连" }))
            }
        }
    }
}

fn flink_icon(
    ctx: AppCtx,
    label: &'static str,
    pick: fn(&ConnState) -> bool,
    d: &'static str,
    url: fn(&ControlGlobalSettings) -> String,
) -> View {
    let ok = move || pick(&ctx.conn.get_clone());
    let open = move |_| {
        let target = url(&ctx.global_settings.get_clone().control);
        spawn_local(async move {
            let _ = api::open_external_url(&target).await;
        });
    };
    view! {
        div(class="btn-with-tip", on:dblclick=open, style="cursor:pointer;") {
            (conn_svg(ok, d))
            span(class="tooltip") {
                (move || format!("{label} {} (双击打开)", if ok() { "正常" } else { "未连" }))
            }
        }
    }
}

/// 中间件状态：MySQL / Redis / MQ / Flink 的连通性，双击 Flink 打开看板
#[component]
pub fn InfraPill() -> View {
    let ctx = use_context::<AppCtx>();
    view! {
        span(class="pill") {
            (conn_icon(ctx, "MySQL", |c| c.mysql, ICON_DB))
            (conn_icon(ctx, "Redis", |c| c.redis, ICON_CACHE))
            (conn_icon(ctx, "MQ", |c| c.mq, ICON_MQ))
            (flink_icon(ctx, "Flink 在线/心跳 (high)", |c| c.flink.high.running, ICON_PULSE, |s| s.flink_high_url.clone()))
            (flink_icon(ctx, "Flink 定位/遥测 (iot)", |c| c.flink.iot.running, ICON_PIN, |s| s.flink_iot_url.clone()))
            span { "内网" }
        }
    }
}

/// 在线数量：通用小组件，数据源跟着当前功能页走。
/// 同时可能连不同环境，再按网关地址分组，别把两个网关的在线数混在一起
#[component]
pub fn OnlinePills() -> View {
    let ctx = use_context::<AppCtx>();
    let groups = create_memo(move || {
        let mut groups: Vec<(String, usize, usize)> = Vec::new();
        for (host, connected) in ctx.page_devices().unwrap_or_default() {
            let label = host_label(&ctx.global_settings.get_clone(), &host);
            match groups.iter_mut().find(|g| g.0 == label) {
                Some(g) => {
                    g.1 += usize::from(connected);
                    g.2 += 1;
                }
                None => groups.push((label, usize::from(connected), 1)),
            }
        }
        groups
    });

    view! {
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

#[component]
pub fn InstancePill() -> View {
    let ctx = use_context::<AppCtx>();
    let editing_instance = create_signal(false);

    let refresh_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        ctx.refresh_instance(false);
    };

    let inst = move || {
        let i = ctx.instance.get_clone();
        if i.trim().is_empty() { "0".to_string() } else { i }
    };

    view! {
        (move || if editing_instance.get() {
            view! {
                div(class="combo-backdrop", on:mousedown=move |_| editing_instance.set(false)) {}
                div(class="pill instance-inline-editor") {
                    span(class="editor-label") { "实例:" }
                    input(
                        r#type="text",
                        class="instance-mini-input",
                        placeholder="0",
                        bind:value=ctx.instance,
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" || ev.key() == "Escape" {
                                editing_instance.set(false);
                            }
                        }
                    )
                    button(
                        class="instance-action-btn ok",
                        title="完成修改",
                        on:click=move |_| editing_instance.set(false)
                    ) {
                        svg(viewBox="0 0 24 24", width="11", height="11", fill="none", stroke="currentColor", stroke-width="2.6", stroke-linecap="round", stroke-linejoin="round") {
                            path(d="M20 6L9 17l-5-5") {}
                        }
                    }
                    button(
                        class="instance-action-btn",
                        title="从 Redis 重新查询实例号",
                        on:click=refresh_click
                    ) {
                        svg(viewBox="0 0 24 24", width="11", height="11", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                            path(d="M23 4v6h-6M1 20v-6h6") {}
                            path(d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15") {}
                        }
                    }
                }
            }
        } else {
            view! {
                span(
                    class="pill instance-pill-badge",
                    title="自动根据中控序列号从 Redis 查询，双击可手动修改",
                    on:dblclick=move |_| editing_instance.set(true)
                ) {
                    span(class="instance-dot") {}
                    span(class="instance-pill-text") { (format!("实例 {}", inst())) }
                    button(
                        class="instance-pill-refresh",
                        title="从 Redis 重新查询实例号",
                        on:click=refresh_click
                    ) {
                        svg(viewBox="0 0 24 24", width="11", height="11", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                            path(d="M23 4v6h-6M1 20v-6h6") {}
                            path(d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15") {}
                        }
                    }
                }
            }
        })
    }
}
