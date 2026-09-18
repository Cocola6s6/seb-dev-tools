use crate::api;
use crate::state::{AppCtx, LogEntry, Page, INNER_HOST};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
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

const CITY_PRESETS: &[(&str, f64, f64)] = &[
    ("南宁良庆区", 108.367035, 22.756302),
    ("汕头澄海区", 116.765101, 23.461051),
    ("北京海淀区", 116.298056, 39.959912),
];

#[component(inline_props)]
pub fn MapPickerModal(
    open: Signal<bool>,
    target_coord: Signal<String>,
) -> View {
    let current_pick = create_signal(String::new());
    let custom_input = create_signal(String::new());

    create_effect(move || {
        if open.get() {
            let init_val = target_coord.get_clone();
            let init_val = if init_val.trim().is_empty() {
                "108.367035,22.756302".to_string()
            } else {
                init_val
            };
            current_pick.set(init_val.clone());
            custom_input.set(init_val.clone());

            spawn_local(async move {
                TimeoutFuture::new(60).await;
                api::init_map_picker("map-picker-container", &init_val, move |picked| {
                    current_pick.set(picked.clone());
                    custom_input.set(picked);
                });
            });
        }
    });

    let confirm = move |_| {
        let val = current_pick.get_clone();
        if !val.trim().is_empty() {
            target_coord.set(val);
        }
        open.set(false);
    };

    let cancel = move |_| {
        open.set(false);
    };

    let jump_custom = move |_| {
        let input_val = custom_input.get_clone();
        if input_val.contains(',') {
            let parts: Vec<&str> = input_val.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(lng), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    api::jump_map_coord(lng, lat);
                    current_pick.set(format!("{:.6},{:.6}", lng, lat));
                }
            }
        }
    };

    let locating = create_signal(false);

    let locate_my_pos = move |_| {
        locating.set(true);
        let current_pick_clone = current_pick.clone();
        let custom_input_clone = custom_input.clone();
        let locating_clone = locating.clone();

        api::locate_current_position(move |picked| {
            locating_clone.set(false);
            current_pick_clone.set(picked.clone());
            custom_input_clone.set(picked);
        });
    };

    view! {
        (if open.get() {
            view! {
                div(class="modal-backdrop", on:click=move |ev: web_sys::MouseEvent| {
                    if let Some(target) = ev.target() {
                        if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                            if el.class_name().contains("modal-backdrop") {
                                open.set(false);
                            }
                        }
                    }
                }) {
                    div(class="modal-dialog") {
                        div(class="modal-head") {
                            div(class="modal-title") { "高德地图坐标拾取 (GCJ-02)" }
                            button(class="link", on:click=cancel) { "✕" }
                        }
                        div(class="modal-body") {
                            div(class="row", style="gap:10px;justify-content:space-between;align-items:center;") {
                                div(class="field-inline", style="gap:6px;flex:none;") {
                                    input(
                                        r#type="text",
                                        placeholder="经度,纬度 (如 108.367035,22.756302)",
                                        bind:value=custom_input,
                                        style="width:230px;min-width:210px;height:32px;font-family:monospace;font-size:12px;padding:4px 10px;"
                                    )
                                    button(on:click=jump_custom, style="height:32px;padding:4px 12px;font-size:12px;") { "定位" }
                                    button(
                                        on:click=locate_my_pos,
                                        disabled=locating.get(),
                                        style="height:32px;padding:4px 12px;font-size:12px;background:#f0fdf4;color:#15803d;border:1px solid #bbf7d0;font-weight:500;"
                                    ) {
                                        (if locating.get() { "定位中..." } else { "当前位置" })
                                    }
                                }
                                div(class="map-preset-bar") {
                                    span(style="font-size:11px;color:var(--muted);") { "快捷城市:" }
                                    Indexed(
                                        list=CITY_PRESETS.to_vec(),
                                        view=move |(name, lng, lat): (&'static str, f64, f64)| view! {
                                            button(class="map-preset-btn", on:click=move |_| {
                                                api::jump_map_coord(lng, lat);
                                                let str_val = format!("{:.6},{:.6}", lng, lat);
                                                current_pick.set(str_val.clone());
                                                custom_input.set(str_val);
                                            }) { (name) }
                                        }
                                    )
                                }
                            }
                            div(id="map-picker-container", class="modal-map-container") {}
                            div(class="row", style="font-size:12px;color:var(--muted);justify-content:space-between;") {
                                span { "点击地图任意位置或拖拽红点即可选取精确坐标。" }
                                span(style="font-family:monospace;color:var(--text);font-weight:600;") {
                                    (format!("当前选中: {}", current_pick.get_clone()))
                                }
                            }
                        }
                        div(class="modal-footer") {
                            button(on:click=cancel) { "取消" }
                            button(class="primary", on:click=confirm) { "确定使用该坐标" }
                        }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}

#[component(inline_props)]
pub fn Field(
    label: &'static str,
    value: Signal<String>,
    #[prop(default)] placeholder: &'static str,
) -> View {
    let ph = placeholder;
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Tab" && !ev.shift_key() && value.get_clone().trim().is_empty() && !ph.is_empty() {
            let fill_val = if let Some(stripped) = ph.strip_prefix("如 ") {
                stripped
            } else {
                ph
            };
            value.set(fill_val.to_string());
        }
    };
    view! {
        div(class="field") {
            label { (label) }
            input(r#type="text", placeholder=placeholder, bind:value=value, on:keydown=on_keydown)
        }
    }
}

#[component]
pub fn DeviceBar() -> View {
    let ctx = use_context::<AppCtx>();
    let open = create_signal(false);
    // 点箭头展开的是整份清单，输入时才按已敲的位数过滤
    let show_all = create_signal(false);
    let matches = create_memo(move || {
        let kw = if show_all.get() {
            String::new()
        } else {
            ctx.device_no.get_clone().trim().to_string()
        };
        ctx.client
            .devices
            .get_clone()
            .into_iter()
            // 这两个页面只打内网后台，连别的网关的设备列出来也没法用
            .filter(|d| d.config.host == INNER_HOST && (kw.is_empty() || d.config.device_no.contains(&kw)))
            .collect::<Vec<_>>()
    });
    view! {
        div(class="section") {
            div(class="section-title") { "设备" }
            div(class="grid grid-2") {
                div(class="field combo") {
                    label { "中控设备序列号 (DeviceNo)" }
                    div(class="combo-input") {
                        input(
                            r#type="text",
                            placeholder="799497080",
                            bind:value=ctx.device_no,
                            on:focus=move |_| { show_all.set(false); open.set(true); },
                            on:blur=move |_| open.set(false),
                            on:change=move |_| ctx.refresh_instance(true),
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Tab" && !ev.shift_key() && ctx.device_no.get_clone().trim().is_empty() {
                                    ctx.device_no.set("799497080".to_string());
                                    ctx.refresh_instance(true);
                                }
                            }
                        )
                        // 拦下默认行为，免得抢走输入框的焦点、打断正在敲的内容
                        button(
                            class=move || if open.get() { "combo-caret open" } else { "combo-caret" },
                            on:mousedown=move |ev: web_sys::MouseEvent| {
                                ev.prevent_default();
                                let was = open.get();
                                show_all.set(!was);
                                open.set(!was);
                            }
                        ) {
                            svg(viewBox="0 0 10 6", width="10", height="6") {
                                path(d="M1 1l4 4 4-4", fill="none", stroke="currentColor", stroke-width="1.6", stroke-linecap="round", stroke-linejoin="round")
                            }
                        }
                    }
                    // 箭头展开时输入框没有焦点，得靠这层透明幕布收起列表
                    (if open.get() && show_all.get() {
                        view! { div(class="combo-backdrop", on:mousedown=move |_| open.set(false)) {} }
                    } else {
                        view! {}
                    })
                    (if open.get() && !matches.get_clone().is_empty() {
                        view! {
                            div(class="combo-list", on:mousedown=move |ev: web_sys::MouseEvent| ev.prevent_default()) {
                                Indexed(
                                    list=matches,
                                    view=move |st: crate::state::DeviceState| {
                                        let no = st.config.device_no.clone();
                                        let pick = no.clone();
                                        view! {
                                            div(class="combo-item", on:click=move |_| {
                                                ctx.device_no.set(pick.clone());
                                                ctx.refresh_instance(true);
                                                open.set(false);
                                            }) {
                                                span(class="k") { (no) }
                                                span(class=if st.connected { "dev-dot on" } else { "dev-dot off" }) {}
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

pub fn render_qr_svg(content: &str) -> Option<String> {
    if content.trim().is_empty() {
        return None;
    }
    let code = qrcode::QrCode::new(content.as_bytes()).ok()?;
    let svg_str = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(300, 300)
        .quiet_zone(false)
        .dark_color(qrcode::render::svg::Color("#16181d"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build();
    Some(svg_str)
}

#[component]
pub fn QrNavButton() -> View {
    let ctx = use_context::<AppCtx>();
    let open = create_signal(false);
    let copied = create_signal(false);

    let bike_no = move || {
        let b = ctx.bike_no.get_clone().trim().to_string();
        if b.is_empty() {
            "A60004000180".to_string()
        } else {
            b
        }
    };

    let qr_url = move || format!("https://gycx.cn?s={}", bike_no());

    let qr_data_url = create_memo(move || {
        let svg = render_qr_svg(&qr_url()).unwrap_or_default();
        format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
    });

    let copy_link = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let url = qr_url();
        spawn_local(async move {
            let _ = api::copy_to_clipboard(&url).await;
            copied.set(true);
            TimeoutFuture::new(1500).await;
            copied.set(false);
        });
    };

    view! {
        div(class="nav-qr-container") {
            button(
                class=move || if open.get() { "nav-qr-icon-btn active" } else { "nav-qr-icon-btn" },
                title="车辆二维码",
                on:click=move |_| open.set(!open.get())
            ) {
                svg(
                    viewBox="0 0 24 24",
                    width="20",
                    height="20",
                    fill="currentColor"
                ) {
                    path(d="M3 3h8v8H3V3zm2 2v4h4V5H5zm8-2h8v8h-8V3zm2 2v4h4V5h-4zM3 13h8v8H3v-8zm2 2v4h4v-4H5zm13-2h3v2h-3v-2zm-5 0h3v3h-3v-3zm2 3h3v2h-3v-2zm3 0h3v5h-2v-3h-1v-2zm-5 2h2v3h-2v-3zm2 2h3v1h-3v-1zm-2-7h1v1h-1v-1zm6 4h1v1h-1v-1z") {}
                }
            }
            (if open.get() {
                view! {
                    div(class="popover-backdrop", on:click=move |_| open.set(false)) {}
                    div(class="nav-qr-popover") {
                        div(class="nav-qr-head") {
                            span { "车辆二维码" }
                        }
                        div(class="qr-svg-container") {
                            img(src=qr_data_url, alt="二维码", style="width:176px;height:176px;display:block;")
                        }
                        div(class="nav-qr-foot") {
                            span(class="qr-bike-no") { (bike_no()) }
                            button(
                                class=move || if copied.get() { "qr-copy-btn copied" } else { "qr-copy-btn" },
                                title=move || if copied.get() { "已复制" } else { "复制链接" },
                                on:click=copy_link
                            ) {
                                (if copied.get() {
                                    view! {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="currentColor") {
                                            path(d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z") {}
                                        }
                                    }
                                } else {
                                    view! {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="currentColor") {
                                            path(d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z") {}
                                        }
                                    }
                                })
                            }
                        }
                    }
                }
            } else {
                view! {}
            })
        }
    }
}

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

    // 监听窗口尺寸变动以感知分屏状态
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

    // 中控客户端分屏或设备较多时，默认自动收起日志；切到其他页自动恢复展开
    let prev = Rc::new(Cell::new((Page::Deploy, false)));
    create_effect(move || {
        let page = ctx.page.get();
        let split_or_crowded = page == Page::Client && (is_wide.get() || ctx.client.devices.get_clone().len() > 4);
        let (prev_page, prev_state) = prev.get();
        prev.set((page, split_or_crowded));

        if page != Page::Client {
            if page != prev_page {
                minimized.set(false);
            }
        } else if (page != prev_page && split_or_crowded) || (split_or_crowded && !prev_state) {
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
        if minimized.get() {
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
