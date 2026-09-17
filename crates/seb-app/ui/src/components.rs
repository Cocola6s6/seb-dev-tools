use crate::api;
use crate::state::{AppCtx, LogEntry};
use gloo_timers::future::TimeoutFuture;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

pub fn select_value(ev: web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

#[component(inline_props)]
pub fn Check(label: &'static str, checked: Signal<bool>) -> View {
    view! {
        label(class="check") {
            input(r#type="checkbox", bind:checked=checked)
            span(class="box") {}
            span { (label) }
        }
    }
}

#[component(inline_props)]
pub fn Field(
    label: &'static str,
    value: Signal<String>,
    #[prop(default)] placeholder: &'static str,
) -> View {
    view! {
        div(class="field") {
            label { (label) }
            input(r#type="text", placeholder=placeholder, bind:value=value)
        }
    }
}

#[component]
pub fn DeviceBar() -> View {
    let ctx = use_context::<AppCtx>();
    view! {
        div(class="section") {
            div(class="section-title") { "设备" }
            div(class="grid grid-3") {
                div(class="field") {
                    label { "中控设备序列号 (DeviceNo)" }
                    input(
                        r#type="text",
                        placeholder="019552878",
                        bind:value=ctx.device_no,
                        on:change=move |_| ctx.refresh_instance(true)
                    )
                }
                Field(label="车辆编号 (仅用于日志)", value=ctx.bike_no, placeholder="选填")
                div(class="field") {
                    label { "实例号 (路由键后缀)" }
                    div(class="field-inline") {
                        input(r#type="text", placeholder="0", bind:value=ctx.instance)
                        button(on:click=move |_| ctx.refresh_instance(false)) { "查实例号" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn LogPane() -> View {
    let ctx = use_context::<AppCtx>();

    create_effect(move || {
        let _ = ctx.logs.get_clone().len();
        spawn_local(async move {
            TimeoutFuture::new(0).await;
            if let Some(el) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.query_selector(".log-body").ok().flatten())
            {
                el.set_scroll_top(el.scroll_height());
            }
        });
    });

    let copy = move |_| {
        let text = ctx
            .logs
            .get_clone()
            .iter()
            .map(|e| format!("{}  {}", e.ts, e.text))
            .collect::<Vec<_>>()
            .join("\n");
        spawn_local(async move {
            if let Err(e) = api::copy_to_clipboard(&text).await {
                ctx.log_error(format!("【错误】复制失败: {e}"));
            }
        });
    };

    view! {
        div(class="log-pane") {
            div(class="log-head") {
                span { "日志" }
                span(class="spacer") {}
                button(class="link", on:click=copy) { "复制" }
                button(class="link", on:click=move |_| ctx.logs.set(Vec::new())) { "清空" }
            }
            div(class="log-body") {
                Indexed(
                    list=ctx.logs,
                    view=move |entry: LogEntry| {
                        let cls = format!("log-line {}", entry.level.css());
                        view! {
                            div(class=cls) {
                                span(class="log-ts") { (entry.ts) }
                                span { (entry.text) }
                            }
                        }
                    }
                )
            }
        }
    }
}
