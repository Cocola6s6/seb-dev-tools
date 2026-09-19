use crate::api;
use crate::state::{
    AppCtx, LogEntry,
};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn LogPane() -> View {
    let ctx = use_context::<AppCtx>();
    let minimized = create_signal(false);
    let height = create_signal(210);
    let is_resizing = create_signal(false);

    let is_wide = create_signal({
        web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) >= 1300.0
    });

    {
        let is_wide = is_wide.clone();
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let w = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            is_wide.set(w >= 1300.0);
        }));
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }

    // 客户端页在分屏或设备多时腾地方给设备卡，离开客户端页再把日志放回来
    let prev = Rc::new(Cell::new((ctx.page.get(), false)));
    create_effect(move || {
        if ctx.is_settings.get() {
            return;
        }
        let page = ctx.page.get();
        let split_or_crowded = page.is_client() && (is_wide.get() || ctx.client.devices.get_clone().len() > 4 || ctx.battery.devices.get_clone().len() > 4);
        let (prev_page, prev_state) = prev.get();
        prev.set((page, split_or_crowded));

        if page != prev_page {
            if prev_page.is_client() && !page.is_client() {
                minimized.set(false);
            } else if !prev_page.is_client() && page.is_client() && split_or_crowded {
                minimized.set(true);
            }
        } else if page.is_client() && split_or_crowded && !prev_state {
            minimized.set(true);
        }
    });

    create_effect(move || {
        let _ = ctx.logs.get_clone().len();
        if !minimized.get() {
            spawn_local(async move {
                TimeoutFuture::new(0).await;
                if let Some(el) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector(".log-body").ok().flatten())
                {
                    el.set_scroll_top(el.scroll_height());
                }
            });
        }
    });

    let open_terminal = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let dev = ctx.device_no.get_clone();
        let bike = ctx.bike_no.get_clone();
        spawn_local(async move {
            let dev_opt = if dev.trim().is_empty() { None } else { Some(dev) };
            let bike_opt = if bike.trim().is_empty() { None } else { Some(bike) };
            match api::open_terminal_log(dev_opt, bike_opt).await {
                Ok(cmd) => {
                    ctx.log_info(format!("已拉起终端监听（真实车辆日志需要通过IOT网关查看）: {cmd}"));
                }
                Err(e) => {
                    ctx.log_error(format!("拉起终端失败: {e}"));
                }
            }
        });
    };

    let copy = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
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

    let clear = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        ctx.logs.set(Vec::new());
    };

    let on_head_click = move |_| {
        if minimized.get() {
            minimized.set(false);
        }
    };

    let on_dblclick = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if !minimized.get() {
            minimized.set(true);
        }
    };

    let on_resizer_down = move |ev: web_sys::MouseEvent| {
        ev.prevent_default();
        ev.stop_propagation();
        is_resizing.set(true);
        api::start_log_resize(
            move |h| {
                height.set(h as i32);
            },
            move || {
                is_resizing.set(false);
            },
        );
    };

    let pane_cls = move || {
        let mut classes = vec!["log-pane"];
        if minimized.get() {
            classes.push("minimized");
        }
        if is_resizing.get() {
            classes.push("is-resizing");
        }
        classes.join(" ")
    };

    let pane_style = move || {
        if ctx.is_settings.get() {
            "display: none;".to_string()
        } else if minimized.get() {
            String::new()
        } else {
            format!("height: {}px;", height.get())
        }
    };

    view! {
        div(class=pane_cls(), style=pane_style()) {
            (if !minimized.get() {
                view! {
                    div(class="log-resizer", on:mousedown=on_resizer_down) {}
                }
            } else {
                view! {}
            })
            div(class="log-head", on:click=on_head_click, on:dblclick=on_dblclick) {
                span { "日志" }
                span(class="spacer") {}
                div(class="btn-with-tip") {
                    button(class="link", on:click=open_terminal) { "真车日志" }
                    span(class="tooltip") { "真实车辆日志需要通过IOT网关查看" }
                }
                button(class="link", on:click=copy) { "复制" }
                button(class="link", on:click=clear) { "清空" }
            }
            (if !minimized.get() {
                view! {
                    div(class="log-body") {
                        Indexed(
                            list=ctx.logs,
                            view=move |entry: LogEntry| {
                                let cls = format!("log-line {}", entry.level.css());
                                let ts_cls = format!("log-ts ts-{}", entry.tint);
                                let dir = match entry.dir {
                                    Some("up") => view! { span(class="dir up") { "↑" } },
                                    Some(_) => view! { span(class="dir down") { "↓" } },
                                    None => view! {},
                                };
                                let dev = if entry.device.is_empty() {
                                    view! {}
                                } else {
                                    view! { span(class="log-dev") { (format!("[{}]", entry.device)) } }
                                };
                                view! {
                                    div(class=cls) {
                                        span(class=ts_cls) { (entry.ts) }
                                        (dir)
                                        (dev)
                                        span { (entry.text) }
                                    }
                                }
                            }
                        )
                    }
                }
            } else {
                view! {}
            })
        }
    }
}

#[component]
pub fn GlobalToast() -> View {
    let ctx = use_context::<AppCtx>();
    view! {
        (if let Some(msg) = ctx.toast_msg.get_clone() {
            view! {
                div(class="global-toast-container") {
                    div(class="global-toast") {
                        div(class="global-toast-icon-wrap") {
                            svg(class="global-toast-icon", viewBox="0 0 24 24", fill="currentColor") {
                                path(d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z") {}
                            }
                        }
                        span(class="global-toast-text") { (msg) }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}
