use crate::actions::run_battery_named;
use crate::api;
use crate::components::{render_qr_svg, Check, MapPickerModal};
use crate::state::{host_env_tag, AppCtx, BatteryCtx, BatteryState, LogLevel, Page};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const GW_INNER: &str = "10.12.55.31:32402";
const GW_TEST: &str = "140.143.180.28:28081";
const GW_PROD: &str = "140.143.214.51:28081";

fn connect_single_battery(ctx: AppCtx, no: String) {
    ctx.log_battery(format!("正在连接电池 {no}..."), LogLevel::Info);
    spawn_local(async move {
        match api::battery_connect(&no).await {
            Ok(st) => {
                ctx.log_battery(format!("{no} 已连接 {}", st.endpoint), LogLevel::Info);
            }
            Err(e) => ctx.log_battery(format!("【错误】{no}: {e}"), LogLevel::Error),
        }
    });
}

fn trigger_battery_connect(b: BatteryCtx, ctx: AppCtx, no: String) {
    b.load_device(&no);
    if b.split_gateway().1 == 0 {
        ctx.log_battery("【警告】网关端口不合法", LogLevel::Warn);
        return;
    }
    connect_single_battery(ctx, no);
}

fn toggle_battery_connect(b: BatteryCtx, ctx: AppCtx, no: String) {
    let is_connected = b
        .devices
        .get_clone()
        .into_iter()
        .find(|d| d.config.battery_no == no)
        .map(|d| d.connected)
        .unwrap_or(false);

    if is_connected {
        run_battery_named(ctx, "断开连接", async move {
            api::battery_disconnect(&no).await.map(|_| ())
        });
    } else {
        trigger_battery_connect(b, ctx, no);
    }
}

#[derive(Clone, PartialEq)]
struct QrBatteryItem {
    battery_no: String,
    host: String,
    connected: bool,
}

#[component]
fn BatteryQrPanel() -> View {
    let ctx = use_context::<AppCtx>();
    let b = ctx.battery;

    let valid_items = create_memo(move || {
        let devs = b.devices.get_clone();
        devs.into_iter()
            .map(|d| QrBatteryItem {
                battery_no: d.config.battery_no,
                host: d.config.host,
                connected: d.connected,
            })
            .collect::<Vec<_>>()
    });

    view! {
        div(class="client-qr-panel") {
            div(class="client-qr-grid") {
                Indexed(
                    list=valid_items,
                    view=move |item: QrBatteryItem| {
                        let battery_no = item.battery_no.clone();
                        let host = item.host.clone();
                        let connected = item.connected;
                        let (env_label, env_class) = host_env_tag(&host);
                        let is_selected = {
                            let battery_no = battery_no.clone();
                            move || b.is_selected(&battery_no)
                        };
                        let cls = {
                            let is_selected = is_selected.clone();
                            move || {
                                let mut res = String::from("client-qr-card");
                                if connected {
                                    res.push_str(" online");
                                } else {
                                    res.push_str(" offline");
                                }
                                if is_selected() {
                                    res.push_str(" active");
                                }
                                res
                            }
                        };
                        let qr_url = format!("https://cosbike.net.cn/qr?{battery_no}");
                        let qr_data_url = {
                            let svg = render_qr_svg(&qr_url).unwrap_or_default();
                            format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
                        };
                        let copied = create_signal(false);
                        let copy_link = {
                            let url = qr_url.clone();
                            let battery_no = battery_no.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let url = url.clone();
                                let battery_no = battery_no.clone();
                                spawn_local(async move {
                                    let _ = api::copy_to_clipboard(&url).await;
                                    copied.set(true);
                                    ctx.toast(format!("已复制电池 {battery_no} 二维码链接"));
                                    TimeoutFuture::new(1500).await;
                                    copied.set(false);
                                });
                            }
                        };
                        let select_dev = {
                            let battery_no = battery_no.clone();
                            move |ev: web_sys::MouseEvent| {
                                if ev.meta_key() || ev.ctrl_key() {
                                    b.toggle_select(&battery_no);
                                } else if ev.shift_key() {
                                    b.range_select(&battery_no);
                                } else {
                                    b.select(&battery_no);
                                }
                            }
                        };
                        let dblclick_qr = {
                            let battery_no = battery_no.clone();
                            move |_| {
                                toggle_battery_connect(b, ctx, battery_no.clone());
                            }
                        };
                        view! {
                            div(
                                class=cls,
                                title=if connected { "单击选中，双击断开连接 (按住 Cmd/Shift 可多选)" } else { "单击选中，双击连接并登录 (按住 Cmd/Shift 可多选)" },
                                on:click=select_dev,
                                on:dblclick=dblclick_qr
                            ) {
                                span(class=format!("qr-env-badge {env_class}")) { (env_label) }
                                div(class="qr-svg-container") {
                                    img(src=qr_data_url, alt="二维码", style="width:100%;height:100%;display:block;")
                                }
                                div(class="nav-qr-foot") {
                                    span(class="qr-bike-no") { (battery_no) }
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
                    }
                )
            }
        }
    }
}

#[component]
pub fn BatteryPage() -> View {
    let ctx = use_context::<AppCtx>();
    let b = ctx.battery;
    let map_picker_open = create_signal(false);

    // 表单任一项变动就把配置同步给后台落盘；owner 为空说明正在切设备，这一轮不能写。
    let seq = Rc::new(Cell::new(0u32));
    create_effect(move || {
        let config = b.config(b.owner.get_clone());
        if config.battery_no.is_empty() {
            return;
        }
        let seq = seq.clone();
        let n = seq.get() + 1;
        seq.set(n);
        spawn_local(async move {
            TimeoutFuture::new(300).await;
            if seq.get() != n {
                return;
            }
            match api::battery_update_device(config).await {
                Ok(list) => b.devices.set(list),
                Err(e) => ctx.log_battery(format!("【错误】{e}"), LogLevel::Error),
            }
        });
    });

    let conn_btn_text = create_memo(move || {
        let n = b.selected_list().len();
        if n > 1 {
            format!("批量连接 ({n})")
        } else {
            "连接并登录".to_string()
        }
    });

    let disconn_btn_text = create_memo(move || {
        let n = b.selected_list().len();
        if n > 1 {
            format!("批量断开 ({n})")
        } else {
            "断开".to_string()
        }
    });

    let connect = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if b.split_gateway().1 == 0 {
            ctx.log_battery("【警告】网关端口不合法", LogLevel::Warn);
            return;
        }
        for no in targets {
            connect_single_battery(ctx, no);
        }
    };

    let disconnect = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "断开连接", async move {
                api::battery_disconnect(&no).await.map(|_| ())
            });
        } else {
            run_battery_named(ctx, "批量断开", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_disconnect(&no).await {
                        Ok(_) => ok_cnt += 1,
                        Err(_) => err_cnt += 1,
                    }
                }
                if err_cnt == 0 {
                    Ok(())
                } else {
                    Err(format!("成功 {ok_cnt}，失败 {err_cnt}"))
                }
            });
        }
    };

    let send_location = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报定位", async move {
                api::battery_send_location(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报定位", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_location(no).await {
                        Ok(_) => ok_cnt += 1,
                        Err(_) => err_cnt += 1,
                    }
                }
                if err_cnt == 0 {
                    Ok(())
                } else {
                    Err(format!("成功 {ok_cnt}，失败 {err_cnt}"))
                }
            });
        }
    };

    let send_alarm = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报告警", async move {
                api::battery_send_alarm(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报告警", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_alarm(no).await {
                        Ok(_) => ok_cnt += 1,
                        Err(_) => err_cnt += 1,
                    }
                }
                if err_cnt == 0 {
                    Ok(())
                } else {
                    Err(format!("成功 {ok_cnt}，失败 {err_cnt}"))
                }
            });
        }
    };

    let send_runtime = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报遥测", async move {
                api::battery_send_runtime(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报遥测", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_runtime(no).await {
                        Ok(_) => ok_cnt += 1,
                        Err(_) => err_cnt += 1,
                    }
                }
                if err_cnt == 0 {
                    Ok(())
                } else {
                    Err(format!("成功 {ok_cnt}，失败 {err_cnt}"))
                }
            });
        }
    };

    let send_ping = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "发送心跳", async move {
                api::battery_send_ping(no).await
            });
        } else {
            run_battery_named(ctx, "批量发送心跳", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_ping(no).await {
                        Ok(_) => ok_cnt += 1,
                        Err(_) => err_cnt += 1,
                    }
                }
                if err_cnt == 0 {
                    Ok(())
                } else {
                    Err(format!("成功 {ok_cnt}，失败 {err_cnt}"))
                }
            });
        }
    };

    view! {
        div(class="client-split-container") {
            div(class="client-split-left") {
                BatteryQrPanel {}
            }
            div(class="client-split-right") {
                div(class="page-head") {
                    div(class="page-title") { "电池客户端" }
                    div(class="page-desc") {
                        "以电池设备的身份连上 IoT 网关，可同时模拟多台。"
                    }
                }

                div(class="device-layout") {
                    BatteryDeviceList {}

                    div(class="device-main") {
                        div(class="section") {
                            div(class="section-title") { "网关连接" }
                            div(class="grid grid-2") {
                                div(class="field") {
                                    label {
                                        span { "网关地址" }
                                        div(class="gateway-env-shortcuts") {
                                            button(
                                                r#type="button",
                                                class=move || if b.gateway.get_clone().trim() == GW_INNER { "env-chip inner active" } else { "env-chip inner" },
                                                title="快捷填入内网网关",
                                                on:click=move |_| b.gateway.set(GW_INNER.to_string())
                                            ) { "内网" }
                                            button(
                                                r#type="button",
                                                class=move || if b.gateway.get_clone().trim() == GW_TEST { "env-chip test active" } else { "env-chip test" },
                                                title="快捷填入外网网关",
                                                on:click=move |_| b.gateway.set(GW_TEST.to_string())
                                            ) { "外网" }
                                            button(
                                                r#type="button",
                                                class=move || if b.gateway.get_clone().trim() == GW_PROD { "env-chip prod active" } else { "env-chip prod" },
                                                title="快捷填入正式网关",
                                                on:click=move |_| b.gateway.set(GW_PROD.to_string())
                                            ) { "正式" }
                                        }
                                    }
                                    input(r#type="text", placeholder="域名:端口", bind:value=b.gateway)
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=connect) { (conn_btn_text.get_clone()) }
                                button(on:click=disconnect) { (disconn_btn_text.get_clone()) }
                                span(class="spacer") {}
                                Check(label="60 秒心跳", checked=b.heartbeat)
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "电池定位" }
                            div(class="grid grid-2") {
                                div(class="field") {
                                    label { "坐标(高德)" }
                                    div(class="field-inline") {
                                        input(
                                            r#type="text",
                                            placeholder="如 116.302928,40.054926",
                                            bind:value=b.coordinates,
                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                if ev.key() == "Tab" && !ev.shift_key() && b.coordinates.get_clone().trim().is_empty() {
                                                    b.coordinates.set("116.302928,40.054926".to_string());
                                                }
                                            }
                                        )
                                        button(
                                            class="icon-btn",
                                            title="选择地图坐标",
                                            on:click=move |_| map_picker_open.set(true)
                                        ) {
                                            svg(viewBox="0 0 24 24", width="16", height="16", fill="currentColor") {
                                                path(d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5 2.5z") {}
                                            }
                                        }
                                    }
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_location) { "上报定位" }
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "快捷上报与操作" }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_runtime) { "上报遥测" }
                                button(on:click=send_alarm) { "上报告警" }
                                button(on:click=send_ping) { "发送心跳" }
                            }
                        }
                    }
                }
            }
        }

        MapPickerModal(open=map_picker_open, target_coord=b.coordinates)
    }
}

#[component]
fn BatteryDeviceList() -> View {
    let ctx = use_context::<AppCtx>();
    let b = ctx.battery;
    let adding = create_signal(false);
    let new_no = create_signal(String::new());

    let confirm_add = move || {
        let no = new_no.get_clone().trim().to_string();
        adding.set(false);
        new_no.set(String::new());
        if no.is_empty() {
            return;
        }
        if b.devices.get_clone().iter().any(|d| d.config.battery_no == no) {
            b.select(&no);
            return;
        }
        // 新设备沿用当前表单的网关与坐标
        let config = b.config(no.clone());
        spawn_local(async move {
            match api::battery_update_device(config).await {
                Ok(list) => {
                    b.devices.set(list);
                    b.select(&no);
                }
                Err(e) => ctx.log_battery(format!("【错误】{e}"), LogLevel::Error),
            }
        });
    };

    let head_text = create_memo(move || {
        let total = b.devices.get_clone().len();
        let selected_cnt = b.selected_list().len();
        if selected_cnt > 1 {
            format!("设备 (已选 {}/{})", selected_cnt, total)
        } else {
            format!("设备 ({})", total)
        }
    });

    let copied = create_signal(false);
    let copy_selected = {
        let b = b.clone();
        let ctx = ctx.clone();
        let copied = copied.clone();
        move || {
            let list = b.selected_list();
            let text = if list.is_empty() {
                let sel = b.selected.get_clone();
                if sel.trim().is_empty() {
                    return;
                }
                sel
            } else {
                list.join("\n")
            };
            let count = if list.is_empty() { 1 } else { list.len() };
            let copied = copied.clone();
            spawn_local(async move {
                let _ = api::copy_to_clipboard(&text).await;
                copied.set(true);
                if count == 1 {
                    ctx.toast(format!("已复制电池序列号：{text}"));
                } else {
                    ctx.toast(format!("已复制 {count} 电池序列号到剪贴板"));
                }
                TimeoutFuture::new(1000).await;
                copied.set(false);
            });
        }
    };

    let on_list_keydown = {
        let copy_selected = copy_selected.clone();
        move |ev: web_sys::KeyboardEvent| {
            if (ev.meta_key() || ev.ctrl_key()) && (ev.key() == "c" || ev.key() == "C") {
                ev.prevent_default();
                copy_selected();
            }
        }
    };

    // 全局快捷键监听：选中设备后按 Cmd+C/Ctrl+C 快速复制
    {
        let copy_selected = copy_selected.clone();
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if (ev.meta_key() || ev.ctrl_key()) && (ev.key() == "c" || ev.key() == "C") {
                if let Some(target) = ev.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let tag = el.tag_name().to_lowercase();
                        if tag == "input" || tag == "textarea" {
                            return;
                        }
                    }
                }
                if ctx.page.get() == Page::Battery {
                    ev.prevent_default();
                    copy_selected();
                }
            }
        }));
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            cb.forget();
        }
    }

    view! {
        div(class="device-list", tabindex="0", on:keydown=on_list_keydown) {
            div(class="device-list-head") {
                span { (head_text.get_clone()) }
                button(class="icon-btn ok", title="新增设备", on:click=move |_| {
                    adding.set(true);
                    new_no.set(String::new());
                }) {
                    svg(viewBox="0 0 24 24", width="15", height="15", fill="none", stroke="currentColor", stroke-width="2.4", stroke-linecap="round", stroke-linejoin="round") {
                        line(x1="12", y1="5", x2="12", y2="19") {}
                        line(x1="5", y1="12", x2="19", y2="12") {}
                    }
                }
            }

            (move || if adding.get() {
                view! {
                    div(class="device-add") {
                        input(
                            r#type="text",
                            placeholder="电池序列号，回车确认",
                            bind:value=new_no,
                            on:blur=move |_| confirm_add(),
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                match ev.key().as_str() {
                                    "Enter" => confirm_add(),
                                    "Escape" => { adding.set(false); new_no.set(String::new()); }
                                    _ => {}
                                }
                            }
                        )
                    }
                }
            } else {
                view! {}
            })

            div(class="device-items") {
                Indexed(
                    list=b.devices,
                    view=move |st: BatteryState| {
                        let no = st.config.battery_no.clone();
                        let host = st.config.host.clone();
                        let connected = st.connected;
                        let coords = st.config.coordinates.clone();
                        let (env_label, env_class) = host_env_tag(&host);
                        let pick = {
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| {
                                if ev.meta_key() || ev.ctrl_key() {
                                    b.toggle_select(&no);
                                } else if ev.shift_key() {
                                    b.range_select(&no);
                                } else {
                                    b.select(&no);
                                }
                            }
                        };
                        let dblpick = {
                            let no = no.clone();
                            move |_| {
                                toggle_battery_connect(b, ctx, no.clone());
                            }
                        };
                        let remove = {
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let no = no.clone();
                                spawn_local(async move {
                                    match api::battery_remove_device(&no).await {
                                        Ok(list) => {
                                            let next = list.first().map(|d| d.config.battery_no.clone());
                                            b.devices.set(list);
                                            if b.selected.get_clone() == no {
                                                b.select(next.as_deref().unwrap_or(""));
                                            }
                                        }
                                        Err(e) => ctx.log_battery(format!("【错误】{e}"), LogLevel::Error),
                                    }
                                });
                            }
                        };
                        let is_selected = {
                            let no = no.clone();
                            move || b.is_selected(&no)
                        };
                        let cls = {
                            let is_selected = is_selected.clone();
                            let copied = copied.clone();
                            move || {
                                if is_selected() {
                                    if copied.get() {
                                        "device-item active copied-flash"
                                    } else {
                                        "device-item active"
                                    }
                                } else {
                                    "device-item"
                                }
                            }
                        };
                        let sub = if connected {
                            if coords.is_empty() {
                                format!("{env_label} · 无坐标")
                            } else {
                                format!("{env_label} · {coords}")
                            }
                        } else {
                            format!("{env_label} · 未连接")
                        };
                        view! {
                            div(
                                class=cls,
                                title=if connected { "单击切换当前配置，双击断开连接 (按住 Cmd/Shift 可多选)" } else { "单击切换当前配置，双击连接并登录 (按住 Cmd/Shift 可多选)" },
                                on:click=pick,
                                on:dblclick=dblpick
                            ) {
                                div(class="device-item-top") {
                                    span(class=if connected { "dot on" } else { "dot off" }) {}
                                    span(class="device-no") { (no) }
                                    span(class=format!("device-env-tag {env_class}")) { (env_label) }
                                    button(class="device-del", title="移除", on:click=remove) {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="none", stroke="currentColor", stroke-width="2.4", stroke-linecap="round", stroke-linejoin="round") {
                                            line(x1="18", y1="6", x2="6", y2="18") {}
                                            line(x1="6", y1="6", x2="18", y2="18") {}
                                        }
                                    }
                                }
                                div(class="device-sub") { (sub) }
                            }
                        }
                    }
                )
            }
        }
    }
}

