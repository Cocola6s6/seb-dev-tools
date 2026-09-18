use crate::api;
use crate::state::{AppCtx, LogEntry, INNER_HOST};
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
                            button(class="link", on:click=cancel, title="关闭") {
                                svg(viewBox="0 0 24 24", width="16", height="16", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                                    line(x1="18", y1="6", x2="6", y2="18") {}
                                    line(x1="6", y1="6", x2="18", y2="18") {}
                                }
                            }
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

#[component]
pub fn DeviceBar() -> View {
    let ctx = use_context::<AppCtx>();
    let open = create_signal(false);

    let inner_devices = create_memo(move || {
        ctx.client
            .devices
            .get_clone()
            .into_iter()
            // 中控指令和配置只打内网网关
            .filter(|d| d.config.host == INNER_HOST)
            .collect::<Vec<_>>()
    });

    let online_count = create_memo(move || {
        inner_devices
            .get_clone()
            .into_iter()
            .filter(|d| d.connected)
            .count()
    });

    view! {
        div(class="section") {
            div(class="section-title") {
                span { "设备" }
            }

            div(class="device-bar-row") {
                div(class="field device-no-field") {
                    input(
                        r#type="text",
                        placeholder="输入中控设备序列号 (如 799497080)",
                        bind:value=ctx.device_no,
                        on:change=move |_| ctx.refresh_instance(true),
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                ctx.refresh_instance(true);
                            } else if ev.key() == "Tab" && !ev.shift_key() && ctx.device_no.get_clone().trim().is_empty() {
                                ctx.device_no.set("799497080".to_string());
                                ctx.refresh_instance(true);
                            }
                        }
                    )
                }

                div(class="device-select-wrapper") {
                    button(
                        class=move || if open.get() { "device-select-trigger-btn active" } else { "device-select-trigger-btn" },
                        title="选择设备",
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            open.set(!open.get());
                        }
                    ) {
                        svg(viewBox="0 0 24 24", width="14", height="14", fill="none", stroke="currentColor", stroke-width="2", stroke-linecap="round", stroke-linejoin="round") {
                            rect(x="2", y="3", width="20", height="14", rx="2", ry="2") {}
                            line(x1="8", y1="21", x2="16", y2="21") {}
                            line(x1="12", y1="17", x2="12", y2="21") {}
                        }
                        span {
                            (move || {
                                let total = inner_devices.get_clone().len();
                                if total > 0 {
                                    let online = online_count.get();
                                    format!("设备 ({online}/{total})")
                                } else {
                                    "选择设备".to_string()
                                }
                            })
                        }
                        svg(viewBox="0 0 10 6", width="10", height="6", class=move || if open.get() { "caret open" } else { "caret" }) {
                            path(d="M1 1l4 4 4-4", fill="none", stroke="currentColor", stroke-width="1.6", stroke-linecap="round", stroke-linejoin="round") {}
                        }
                    }

                    (if open.get() {
                        view! {
                            div(class="combo-backdrop", on:click=move |_| open.set(false)) {}
                            div(class="device-dropdown-popover") {
                                (if inner_devices.get_clone().is_empty() {
                                    view! {
                                        div(class="device-dropdown-empty") {
                                            "暂无模拟设备，可在「中控客户端」添加或连接"
                                        }
                                    }
                                } else {
                                    view! {
                                        div(class="device-dropdown-header") {
                                            span { "选择设备" }
                                            span(class="device-dropdown-count") {
                                                (format!("共 {} 台", inner_devices.get_clone().len()))
                                            }
                                        }
                                        div(class="device-dropdown-items") {
                                            Indexed(
                                                list=inner_devices,
                                                view=move |st: crate::state::DeviceState| {
                                                    let no = st.config.device_no.clone();
                                                    let connected = st.connected;
                                                    let pick = no.clone();
                                                    let is_current = {
                                                        let no = no.clone();
                                                        move || ctx.device_no.get_clone().trim() == no
                                                    };
                                                    let item_cls = move || {
                                                        if is_current() {
                                                            "device-dropdown-item active"
                                                        } else {
                                                            "device-dropdown-item"
                                                        }
                                                    };
                                                    view! {
                                                        div(class=item_cls, on:click=move |_| {
                                                            ctx.device_no.set(pick.clone());
                                                            ctx.refresh_instance(true);
                                                            open.set(false);
                                                        }) {
                                                            span(class=if connected { "dev-dot on" } else { "dev-dot off" }) {}
                                                            span(class="dev-no") { (no) }
                                                        }
                                                    }
                                                }
                                            )
                                        }
                                    }
                                })
                            }
                        }
                    } else {
                        view! {}
                    })
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

    let is_battery = move || ctx.page.get() == crate::state::Page::Battery;

    let display_title = move || {
        if is_battery() {
            "电池二维码"
        } else {
            "车辆二维码"
        }
    };

    let display_no = move || {
        if is_battery() {
            let b = ctx.battery.selected.get_clone();
            if b.trim().is_empty() {
                "CMAH030799497009".to_string()
            } else {
                b
            }
        } else {
            let b = ctx.bike_no.get_clone().trim().to_string();
            if b.is_empty() {
                "A60004000180".to_string()
            } else {
                b
            }
        }
    };

    let qr_url = move || {
        if is_battery() {
            format!("https://cosbike.net.cn/qr?{}", display_no())
        } else {
            format!("https://gycx.cn?s={}", display_no())
        }
    };

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
                title=move || display_title(),
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
                            span { (display_title()) }
                        }
                        div(class="qr-svg-container") {
                            img(src=qr_data_url, alt="二维码", style="width:176px;height:176px;display:block;")
                        }
                        div(class="nav-qr-foot") {
                            span(class="qr-bike-no") { (display_no()) }
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

    // 日志收起逻辑：
    // 1. 用户手动收起或已收起时，常规功能页切换不主动展开；
    // 2. 只有从客户端页面（中控/电池客户端）切换回其它功能页时，才主动恢复展开日志；
    // 3. 切入客户端页面时，若处于分屏或多设备场景则自动收起。
    let prev = Rc::new(Cell::new((ctx.page.get(), false)));
    create_effect(move || {
        let page = ctx.page.get();
        let split_or_crowded = page.is_client() && (is_wide.get() || ctx.client.devices.get_clone().len() > 4 || ctx.battery.devices.get_clone().len() > 4);
        let (prev_page, prev_state) = prev.get();
        prev.set((page, split_or_crowded));

        if page != prev_page {
            if prev_page.is_client() && !page.is_client() {
                // 从客户端页面切换回其它功能页：主动展开日志
                minimized.set(false);
            } else if !prev_page.is_client() && page.is_client() && split_or_crowded {
                // 切入客户端页面且处于分屏/多设备环境：自动收起日志
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

