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
        let is_wide = is_wide;
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut()>::wrap(Box::new(move || {
            let w = web_sys::window()
                .and_then(|w| w.inner_width().ok())
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            is_wide.set(w >= 1300.0);
        }));
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
            // 日志面板随分屏反复挂载，监听不摘就会堆在已销毁的作用域上，回调一跑就炸
            on_cleanup(move || {
                let _ = w.remove_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
            });
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

    let match_indices = create_memo(move || {
        let terms = terms.get_clone();
        if terms.is_empty() {
            return Vec::new();
        }
        let all = ctx.logs.get_clone();
        all.iter()
            .enumerate()
            .filter_map(|(idx, e)| if e.hit(&terms).is_some() { Some(idx) } else { None })
            .collect::<Vec<usize>>()
    });

    create_effect(move || {
        let _ = ctx.logs.get_clone().len();
        // 搜索时新日志不该把正在看的位置顶走
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

    let copy = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let all = ctx.logs.get_clone();
        let indices = match_indices.get_clone();
        let list: Vec<&LogEntry> = if indices.is_empty() {
            all.iter().collect()
        } else {
            indices.iter().filter_map(|&i| all.get(i)).collect()
        };
        let text = list
            .iter()
            .map(|e| {
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

    let focus_search = || {
        spawn_local(async move {
            TimeoutFuture::new(20).await;
            if let Some(list) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.query_selector_all(".log-search.open input, .log-search input").ok())
            {
                for i in 0..list.length() {
                    if let Some(el) = list
                        .item(i)
                        .and_then(|n| n.dyn_into::<web_sys::HtmlInputElement>().ok())
                    {
                        if el.offset_parent().is_some() || el.client_width() > 0 {
                            let _ = el.focus();
                            el.select();
                            break;
                        }
                    }
                }
            }
        });
    };

    let cursor = create_signal(None::<usize>);

    let jump_to = move |dir_next: bool| {
        let indices = match_indices.get_clone();
        if indices.is_empty() {
            return;
        }
        let total = indices.len();
        let next_cursor = match cursor.get() {
            None => if dir_next { 0 } else { total.saturating_sub(1) },
            Some(curr) => if dir_next {
                (curr + 1) % total
            } else {
                (curr + total - 1) % total
            },
        };
        cursor.set(Some(next_cursor));
        let target_line_index = indices[next_cursor];
        api::scroll_log_to_hit(target_line_index);
    };

    create_effect(move || {
        let indices = match_indices.get_clone();
        if indices.is_empty() {
            cursor.set(None);
        } else {
            cursor.set(Some(0));
            api::scroll_log_to_hit(indices[0]);
        }
    });

    let on_search_key = move |ev: web_sys::KeyboardEvent| {
        if ev.is_composing() {
            return;
        }
        match ev.key().as_str() {
            "Escape" => {
                ev.prevent_default();
                ev.stop_propagation();
                ctx.log_query.set(String::new());
                ctx.log_search_open.set(false);
                cursor.set(None);
            }
            "Enter" => {
                ev.prevent_default();
                ev.stop_propagation();
                jump_to(!ev.shift_key());
            }
            _ => {}
        }
    };

    let on_search_blur = move |_| {
        spawn_local(async move {
            TimeoutFuture::new(120).await;
            let still_in_search = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.active_element())
                .and_then(|el| el.closest(".log-search").ok().flatten())
                .is_some();
            if !still_in_search && ctx.log_query.get_clone().trim().is_empty() {
                ctx.log_search_open.set(false);
            }
        });
    };

    let is_hovered = create_signal(false);

    // Cmd/Ctrl+F 只有在日志区域（鼠标悬停或焦点在日志内）才触发
    {
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(
            Box::new(move |ev: web_sys::KeyboardEvent| {
                if ev.key() == "f" && (ev.meta_key() || ev.ctrl_key()) {
                    let in_log = is_hovered.get() || {
                        web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.active_element())
                            .and_then(|el| el.closest(".log-pane").ok().flatten())
                            .is_some()
                    };
                    if in_log {
                        ev.prevent_default();
                        ctx.log_search_open.set(true);
                        minimized.set(false);
                        focus_search();
                    }
                }
            }),
        );
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            on_cleanup(move || {
                let _ = w.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            });
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
        div(
            class=pane_cls(),
            style=pane_style(),
            on:mouseenter=move |_| is_hovered.set(true),
            on:mouseleave=move |_| is_hovered.set(false),
        ) {
            (move || if !docked && !minimized.get() {
                view! {
                    div(class="log-resizer", on:mousedown=on_resizer_down) {}
                }
            } else {
                view! {}
            })
            div(class="log-head", on:click=on_head_click, on:dblclick=on_dblclick) {
                span(
                    style="cursor: pointer;",
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        if !docked && minimized.get() {
                            minimized.set(false);
                        }
                        ctx.log_search_open.set(true);
                        focus_search();
                    }
                ) { "日志" }
                div(
                    class=move || if ctx.log_search_open.get() || !ctx.log_query.get_clone().is_empty() { "log-search open" } else { "log-search" },
                    on:click=move |ev: web_sys::MouseEvent| {
                        ev.stop_propagation();
                        ctx.log_search_open.set(true);
                        focus_search();
                    },
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
                span(class="spacer") {}
                div(class="btn-with-tip") {
                    button(class="link", on:click=open_terminal) { "真车日志" }
                    span(class="tooltip") { "真实车辆日志需要通过IOT网关查看" }
                }
                button(class="link", on:click=copy) { "复制" }
                button(class="link", on:click=clear) { "清空" }
            }
            (move || if !collapsed() {
                view! {
                    div(class="log-body") {
                        Indexed(
                            list=ctx.logs,
                            view=move |entry: LogEntry| {
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
                                let open = create_signal(false);
                                let hover = create_signal(None::<(usize, usize)>);
                                let expandable = !entry.fields.is_empty() || !entry.hex.is_empty();
                                let entry_cls = entry.clone();
                                let cls = {
                                    let base = format!("log-line {}", entry.level.css());
                                    move || if expandable {
                                        let t = terms.get_clone();
                                        let has_match = !t.is_empty() && entry_cls.hit(&t).unwrap_or(false);
                                        if open.get() {
                                            format!("{base} expandable open")
                                        } else if has_match {
                                            format!("{base} expandable has-match")
                                        } else {
                                            format!("{base} expandable")
                                        }
                                    } else {
                                        base.clone()
                                    }
                                };
                                let toggle = move |_| {
                                    if expandable {
                                        open.set(!open.get());
                                    }
                                };
                                let entry_caret = entry.clone();
                                let caret_cls = move || {
                                    let t = terms.get_clone();
                                    let has_match = !t.is_empty() && entry_caret.hit(&t).unwrap_or(false);
                                    if !open.get() && has_match {
                                        "log-caret has-match"
                                    } else {
                                        "log-caret"
                                    }
                                };
                                let caret = if expandable {
                                    view! {
                                        svg(class=caret_cls(), viewBox="0 0 24 24", fill="currentColor", on:click=toggle) {
                                            path(d="M9 5l8 7-8 7z") {}
                                        }
                                    }
                                } else {
                                    view! { span(class="log-caret-gap") {} }
                                };
                                let hex = entry.hex.clone();
                                let fields = entry.fields.clone();
                                let detail = move || {
                                    if !open.get() {
                                        return view! {};
                                    }
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
                                                        (terms.with(|t| mark(&head, t)))
                                                        span(class="hex-hit") { (terms.with(|t| mark(&mid, t))) }
                                                        (terms.with(|t| mark(&tail, t)))
                                                    }
                                                }
                                            }
                                            None => {
                                                let hex = hex.clone();
                                                view! { div(class="log-hex") { (terms.with(|t| mark(&hex, t))) } }
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
                            },
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
