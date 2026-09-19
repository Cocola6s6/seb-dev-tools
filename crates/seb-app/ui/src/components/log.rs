use crate::api;
use crate::state::{
    AppCtx, LogEntry, Page,
};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

/// 标出命中的过滤词。大小写折叠后字节长度变了就放弃高亮，免得切在字符中间
fn mark(text: &str, terms: &[String]) -> View {
    let lower = text.to_lowercase();
    if terms.is_empty() || lower.len() != text.len() {
        let plain = text.to_string();
        return view! { (plain) };
    }
    let mut parts: Vec<View> = Vec::new();
    let mut i = 0;
    while i < text.len() {
        let next = terms
            .iter()
            .filter_map(|t| lower[i..].find(t.as_str()).map(|p| (i + p, t.len())))
            .min_by_key(|(at, len)| (*at, std::cmp::Reverse(*len)));
        match next {
            Some((at, len)) => {
                if at > i {
                    let head = text[i..at].to_string();
                    parts.push(view! { (head) });
                }
                let hit = text[at..at + len].to_string();
                parts.push(view! { span(class="term-hit") { (hit) } });
                i = at + len;
            }
            None => {
                let tail = text[i..].to_string();
                parts.push(view! { (tail) });
                break;
            }
        }
    }
    view! { (parts) }
}

/// 分屏时左栏放日志，窄屏折回底部那条：和中控客户端同一套容器，比例动画都一致
#[component(inline_props)]
pub fn LogSplit(children: Children) -> View {
    let body = children.call();
    view! {
        div(class="client-split-container") {
            div(class="client-split-left log-col") {
                LogPane(docked=true)
            }
            div(class="client-split-right") { (body) }
        }
    }
}

#[component(inline_props)]
pub fn LogPane(#[prop(default)] docked: bool) -> View {
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
        if docked || ctx.is_settings.get() {
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

    let terms = create_memo(move || {
        ctx.log_query
            .get_clone()
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    // 过滤后的日志，bool 是「只在展开区命中」，用来自动把那条展开
    let visible = create_memo(move || {
        let terms = terms.get_clone();
        let all = ctx.logs.get_clone();
        if terms.is_empty() {
            return all.into_iter().map(|e| (e, false)).collect::<Vec<_>>();
        }
        all.into_iter()
            .filter_map(|e| e.hit(&terms).map(|deep| (e, deep)))
            .collect()
    });

    create_effect(move || {
        let _ = ctx.logs.get_clone().len();
        // 过滤时新日志不该把正在看的位置顶走
        if !minimized.get() && terms.with(Vec::is_empty) {
            spawn_local(async move {
                TimeoutFuture::new(0).await;
                // 底部和左栏两处日志同时在 DOM 里，都得滚到底
                if let Some(list) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.query_selector_all(".log-body").ok())
                {
                    for i in 0..list.length() {
                        if let Some(el) = list.item(i).and_then(|n| n.dyn_into::<web_sys::Element>().ok()) {
                            el.set_scroll_top(el.scroll_height());
                        }
                    }
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

    // 过滤状态下只复制筛出来的那些行
    let copy = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let text = visible
            .get_clone()
            .iter()
            .map(|(e, _)| {
                if e.hex.is_empty() {
                    format!("{}  {}", e.ts, e.text)
                } else {
                    format!("{}  {} | {}", e.ts, e.text, e.hex)
                }
            })
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

    // 输入框在两处日志里各有一份，焦点给当前看得见的那份
    let focus_search = || {
        spawn_local(async move {
            TimeoutFuture::new(0).await;
            if let Some(list) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.query_selector_all(".log-search.open input").ok())
            {
                for i in 0..list.length() {
                    if let Some(el) = list
                        .item(i)
                        .and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok())
                    {
                        if el.client_width() > 0 {
                            let _ = el.focus();
                            el.select();
                        }
                    }
                }
            }
        });
    };

    let on_search_key = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Escape" {
            ctx.log_query.set(String::new());
            ctx.log_search_open.set(false);
        }
    };

    // 没有图标可点，空着离开就自己收起来
    let on_search_blur = move |_| {
        if ctx.log_query.get_clone().trim().is_empty() {
            ctx.log_search_open.set(false);
        }
    };

    // Cmd/Ctrl+F 直接开过滤框，只在底部那份注册，避免重复监听
    if !docked {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(
            Box::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "f" && (ev.meta_key() || ev.ctrl_key()) {
                    ev.prevent_default();
                    ctx.log_search_open.set(true);
                    minimized.set(false);
                    focus_search();
                }
            }),
        );
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }

    let on_head_click = move |_| {
        if !docked && minimized.get() {
            minimized.set(false);
        }
    };

    let on_dblclick = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if !docked && !minimized.get() {
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

    // 分屏后日志已经在左栏，底部那条让位
    let moved_to_column = move || {
        is_wide.get() && matches!(ctx.page.get(), Page::Deploy | Page::Control | Page::Ecu)
    };
    let collapsed = move || !docked && minimized.get();

    let pane_cls = move || {
        let mut classes = vec!["log-pane"];
        if docked {
            classes.push("docked");
        }
        if collapsed() {
            classes.push("minimized");
        }
        if is_resizing.get() {
            classes.push("is-resizing");
        }
        classes.join(" ")
    };

    let pane_style = move || {
        if docked {
            String::new()
        } else if ctx.is_settings.get() || moved_to_column() {
            "display: none;".to_string()
        } else if minimized.get() {
            String::new()
        } else {
            format!("height: {}px;", height.get())
        }
    };

    view! {
        div(class=pane_cls(), style=pane_style()) {
            (if !docked && !minimized.get() {
                view! {
                    div(class="log-resizer", on:mousedown=on_resizer_down) {}
                }
            } else {
                view! {}
            })
            div(class="log-head", on:click=on_head_click, on:dblclick=on_dblclick) {
                span { "日志" }
                div(
                    class=move || if ctx.log_search_open.get() { "log-search open" } else { "log-search" },
                    on:click=|ev: web_sys::MouseEvent| ev.stop_propagation(),
                    on:dblclick=|ev: web_sys::MouseEvent| ev.stop_propagation(),
                ) {
                    input(
                        r#type="text",
                        placeholder="车辆号 / 设备号 / 关键字，空格分隔",
                        bind:value=ctx.log_query,
                        on:keydown=on_search_key,
                        on:blur=on_search_blur,
                    )
                }
                (if terms.with(Vec::is_empty) {
                    view! {}
                } else {
                    view! {
                        span(class="log-count") {
                            (format!("{} / {}", visible.with(Vec::len), ctx.logs.with(Vec::len)))
                        }
                    }
                })
                span(class="spacer") {}
                div(class="btn-with-tip") {
                    button(class="link", on:click=open_terminal) { "真车日志" }
                    span(class="tooltip") { "真实车辆日志需要通过IOT网关查看" }
                }
                button(class="link", on:click=copy) { "复制" }
                button(class="link", on:click=clear) { "清空" }
            }
            (if !collapsed() {
                view! {
                    div(class="log-body") {
                        Indexed(
                            list=visible,
                            view=move |(entry, deep): (LogEntry, bool)| {
                                let ts_cls = format!("log-ts ts-{}", entry.tint);
                                let dir = match entry.dir {
                                    Some("up") => view! { span(class="dir up") { "↑" } },
                                    Some(_) => view! { span(class="dir down") { "↓" } },
                                    None => view! {},
                                };
                                let dev = if entry.device.is_empty() {
                                    view! {}
                                } else {
                                    let dev_txt = format!("[{}]", entry.device);
                                    view! { span(class="log-dev") { (terms.with(|t| mark(&dev_txt, t))) } }
                                };
                                // 命中落在展开区里，直接展开给看
                                let open = create_signal(deep);
                                let hover = create_signal(None::<(usize, usize)>);
                                let expandable = !entry.fields.is_empty() || !entry.hex.is_empty();
                                let cls = {
                                    let base = format!("log-line {}", entry.level.css());
                                    move || if expandable {
                                        format!("{base} expandable{}", if open.get() { " open" } else { "" })
                                    } else {
                                        base.clone()
                                    }
                                };
                                let toggle = move |_| {
                                    if expandable {
                                        open.set(!open.get());
                                    }
                                };
                                let caret = if expandable {
                                    view! {
                                        svg(class="log-caret", viewBox="0 0 24 24", fill="currentColor", on:click=toggle) {
                                            path(d="M9 5l8 7-8 7z") {}
                                        }
                                    }
                                } else {
                                    view! { span(class="log-caret-gap") {} }
                                };
                                let hex = entry.hex.clone();
                                let fields = entry.fields.clone();
                                // 报文和字段表都收在展开区里，日志行本身一条只占一行
                                let detail = move || {
                                    if !open.get() {
                                        return view! {};
                                    }
                                    // 悬停字段时把它占的那几个字节从报文里挑出来
                                    let hex_node = if hex.is_empty() {
                                        view! {}
                                    } else {
                                        match hover.get().filter(|(s, e)| e * 2 <= hex.len() && s < e) {
                                            Some((s, e)) => {
                                                let (head, rest) = hex.split_at(s * 2);
                                                let (mid, tail) = rest.split_at((e - s) * 2);
                                                let (head, mid, tail) =
                                                    (head.to_string(), mid.to_string(), tail.to_string());
                                                view! {
                                                    div(class="log-hex") {
                                                        (head) span(class="hex-hit") { (mid) } (tail)
                                                    }
                                                }
                                            }
                                            None => {
                                                let hex = hex.clone();
                                                view! { div(class="log-hex") { (hex) } }
                                            }
                                        }
                                    };
                                    let rows: Vec<View> = fields
                                        .iter()
                                        .map(|f| {
                                            let (start, end) = (f.start, f.end);
                                            let cells = terms.with(|t| {
                                                (mark(&f.label, t), mark(&f.key, t), mark(&f.value, t))
                                            });
                                            view! {
                                                div(
                                                    class="log-field",
                                                    on:mouseenter=move |_| hover.set(Some((start, end))),
                                                    on:mouseleave=move |_| hover.set(None),
                                                ) {
                                                    span(class="f-label") { (cells.0) }
                                                    span(class="f-key") { (cells.1) }
                                                    span(class="f-value") { (cells.2) }
                                                }
                                            }
                                        })
                                        .collect();
                                    view! {
                                        div(class="log-detail") {
                                            (hex_node)
                                            (rows)
                                        }
                                    }
                                };
                                view! {
                                    div(class=cls) {
                                        div(class="log-row") {
                                            span(class=ts_cls) { (entry.ts) }
                                            (caret)
                                            (dir)
                                            (dev)
                                            span(class="log-msg", on:click=toggle) {
                                                (terms.with(|t| mark(&entry.text, t)))
                                            }
                                        }
                                        (detail)
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
