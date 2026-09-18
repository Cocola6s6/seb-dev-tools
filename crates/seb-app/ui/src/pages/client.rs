use crate::actions::run_client_named;
use crate::api;
use crate::components::{render_qr_svg, select_value, Check, Field, MapPickerModal};
use crate::state::{AlarmType, AppCtx, ClientCtx, DeviceState, LogLevel};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const VEHICLE_STATES: &[(&str, &str)] = &[
    ("0", "借车"),
    ("1", "还车"),
    ("2", "撤防"),
    ("3", "运输模式"),
];

fn trigger_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
    c.select(&no);
    if c.split_gateway().1 == 0 {
        ctx.log_client("【警告】网关端口不合法", LogLevel::Warn);
        return;
    }
    ctx.log_client(format!("正在连接设备 {no}..."), LogLevel::Info);
    spawn_local(async move {
        match api::client_connect(&no).await {
            Ok(st) => {
                ctx.log_client(format!("{no} 已连接 {}", st.endpoint), LogLevel::Info);
                // 登录后网关才会写 ecu:instance:id，实例号变了不刷新的话指令会投到旧实例
                TimeoutFuture::new(1000).await;
                if ctx.device_no.get_clone().trim() == no {
                    ctx.refresh_instance(false);
                }
            }
            Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
        }
    });
}

fn toggle_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
    c.select(&no);
    let is_connected = c
        .devices
        .get_clone()
        .into_iter()
        .find(|d| d.config.device_no == no)
        .map(|d| d.connected)
        .unwrap_or(false);

    if is_connected {
        run_client_named(ctx, "断开连接", async move {
            api::client_disconnect(&no).await.map(|_| ())
        });
    } else {
        trigger_device_connect(c, ctx, no);
    }
}

#[component]
pub fn ClientPage() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;
    let map_picker_open = create_signal(false);

    // 表单任一项变动就把配置同步给后台；owner 为空说明正在切设备，这一轮不能写。
    // 配置要落盘，所以连续输入时只认最后一次
    // 计数器不能用信号：切页会销毁本页作用域，延时里再读就是读已释放的信号
    let seq = Rc::new(Cell::new(0u32));
    create_effect(move || {
        let config = c.config(
            c.owner.get_clone(),
            ctx.battery_no.get_clone().trim().to_string(),
        );
        if config.device_no.is_empty() {
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
            match api::client_update_device(config).await {
                Ok(list) => c.devices.set(list),
                Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
            }
        });
    });

    let connect = move |_| {
        if let Some(no) = selected_no(c, ctx) {
            trigger_device_connect(c, ctx, no);
        }
    };

    let disconnect = move |_| {
        let Some(no) = selected_no(c, ctx) else { return };
        run_client_named(ctx, "断开连接", async move { api::client_disconnect(&no).await.map(|_| ()) });
    };

    let send_alarm = move |_| {
        let Some(no) = selected_no(c, ctx) else { return };
        let code = c.alarm_type.get_clone();
        match c
            .alarm_types
            .get_clone()
            .into_iter()
            .find(|a| a.code.to_string() == code)
        {
            Some(a) => run_client_named(ctx, "上报告警", async move {
                api::client_send_alarm(no, a.code, a.name).await
            }),
            None => ctx.log_client("【警告】请先选择告警类型", LogLevel::Warn),
        }
    };

    view! {
        div(class="client-split-container") {
            div(class="client-split-left") {
                ClientQrPanel {}
            }
            div(class="client-split-right") {
                div(class="page-head") {
                    div(class="page-title") { "中控客户端" }
                    div(class="page-desc") {
                        "以中控设备的身份连上 IoT 网关，可同时模拟多台。"
                    }
                }

                div(class="device-layout") {
                    DeviceList {}

                    div(class="device-main") {
                        div(class="section") {
                            div(class="section-title") { "网关连接" }
                            div(class="grid grid-2") {
                                div(class="field") {
                                    label {
                                        span { "网关地址" }
                                        span(class="help") {
                                            "?"
                                            span(class="tip") {
                                                div { "内网 bike-seb-inner-test.costrip.cn:32405" }
                                                div { "外网 bike-seb-test.costrip.cn:8514" }
                                                div { "正式 bike-seb.costrip.cn:8514" }
                                            }
                                        }
                                    }
                                    input(r#type="text", placeholder="域名:端口", bind:value=c.gateway)
                                }
                                div(class="field") {
                                    label { "软件版本号" }
                                    input(r#type="text", bind:value=c.soft_version)
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=connect) { "连接并登录" }
                                button(on:click=disconnect) { "断开" }
                                span(class="spacer") {}
                                Check(label="60 秒心跳", checked=c.heartbeat)
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "车辆姿态" }
                            div(class="grid grid-3") {
                                div(class="field") {
                                    label { "坐标" }
                                    div(class="field-inline") {
                                        input(
                                            r#type="text",
                                            placeholder="如 108.367035,22.756302",
                                            bind:value=c.coordinates,
                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                if ev.key() == "Tab" && !ev.shift_key() && c.coordinates.get_clone().trim().is_empty() {
                                                    c.coordinates.set("108.367035,22.756302".to_string());
                                                }
                                            }
                                        )
                                        button(
                                            class="icon-btn",
                                            title="选择地图坐标",
                                            on:click=move |_| map_picker_open.set(true)
                                        ) {
                                            svg(viewBox="0 0 24 24", width="16", height="16", fill="currentColor") {
                                                path(d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5-2.5 2.5z") {}
                                            }
                                        }
                                    }
                                }
                                div(class="field") {
                                    label { "车辆状态" }
                                    select(on:change=move |ev| c.vehicle_state.set(select_value(ev))) {
                                        Indexed(
                                            list=VEHICLE_STATES.to_vec(),
                                            view=move |(value, label): (&'static str, &'static str)| {
                                                let selected = c.vehicle_state.get_clone() == value;
                                                view! { option(value=value, selected=selected) { (label) } }
                                            }
                                        )
                                    }
                                }
                                Field(label="电量 SOC", value=c.soc)
                                Field(label="速度", value=c.speed)
                                Field(label="预还车偏向角", value=c.deflection_angle)
                            }
                            div(class="row checks", style="margin-top:14px") {
                                Check(label="运动中", checked=c.motion)
                                Check(label="头盔锁已解锁", checked=c.helmet_lock_unlocked)
                                Check(label="头盔在位", checked=c.helmet_present)
                                Check(label="尾箱在位", checked=c.trunk_latch)
                                Check(label="ACC 供电", checked=c.acc_on)
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=move |_| {
                                    if let Some(no) = selected_no(c, ctx) {
                                        run_client_named(ctx, "上报定位", api::client_send_location(no));
                                    }
                                }) { "上报定位" }
                                button(on:click=move |_| {
                                    if let Some(no) = selected_no(c, ctx) {
                                        run_client_named(ctx, "上报 BMS", api::client_send_bms(no));
                                    }
                                }) { "上报 BMS" }
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "告警与心跳" }
                            div(class="field") {
                                label { "告警类型" }
                                select(class="w-lg", on:change=move |ev| c.alarm_type.set(select_value(ev))) {
                                    Indexed(
                                        list=c.alarm_types,
                                        view=move |a: AlarmType| {
                                            let value = a.code.to_string();
                                            let selected = c.alarm_type.get_clone() == value;
                                            let text = format!("{}  ({})", a.name, a.hex);
                                            view! { option(value=value, selected=selected) { (text) } }
                                        }
                                    )
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_alarm) { "上报告警" }
                                button(on:click=move |_| {
                                    if let Some(no) = selected_no(c, ctx) {
                                        run_client_named(ctx, "发送心跳", api::client_send_ping(no));
                                    }
                                }) { "发心跳" }
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "指令应答" }
                            div(class="row checks") {
                                Check(label="自动应答", checked=c.auto_reply)
                                Check(label="应答结果为成功", checked=c.reply_success)
                                Check(label="应答后补发定位", checked=c.reply_with_location)
                            }
                            div(class="hint", style="margin-top:12px") {
                                (move || match c.current().and_then(|s| s.last_msg_id) {
                                    Some(id) => format!("最近收到的 msgId: {id}"),
                                    None => "尚未收到下发指令".to_string(),
                                })
                            }
                            div(class="card-actions") {
                                button(on:click=move |_| {
                                    if let Some(no) = selected_no(c, ctx) {
                                        run_client_named(ctx, "手动回成功", api::client_send_reply(no, true));
                                    }
                                }) { "手动回成功" }
                                button(on:click=move |_| {
                                    if let Some(no) = selected_no(c, ctx) {
                                        run_client_named(ctx, "手动回失败", api::client_send_reply(no, false));
                                    }
                                }) { "手动回失败" }
                            }
                        }
                    }
                }

                MapPickerModal(open=map_picker_open, target_coord=c.coordinates)
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct QrBikeItem {
    device_no: String,
    bike_no: String,
    connected: bool,
}

#[component]
fn ClientQrPanel() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;

    let bike_map = create_signal(std::collections::HashMap::<String, String>::new());

    create_effect(move || {
        let devs = c.devices.get_clone();
        let ecu_nos: Vec<String> = devs.into_iter().map(|d| d.config.device_no).collect();
        if ecu_nos.is_empty() {
            bike_map.set(std::collections::HashMap::new());
            return;
        }
        spawn_local(async move {
            if let Ok(map) = api::client_get_bike_nos(ecu_nos).await {
                bike_map.set(map);
            }
        });
    });

    let selected_dev = create_memo(move || c.selected.get_clone());

    let valid_items = create_memo(move || {
        let devs = c.devices.get_clone();
        let map = bike_map.get_clone();
        devs.into_iter()
            .filter_map(|d| {
                let ecu = d.config.device_no;
                let bike_no = map.get(&ecu)?.clone();
                if bike_no.trim().is_empty() {
                    return None;
                }
                Some(QrBikeItem {
                    device_no: ecu,
                    bike_no,
                    connected: d.connected,
                })
            })
            .collect::<Vec<_>>()
    });

    view! {
        div(class="client-qr-panel") {
            div(class="client-qr-grid") {
                Indexed(
                    list=valid_items,
                    view=move |item: QrBikeItem| {
                        let ecu_no = item.device_no.clone();
                        let bike_no = item.bike_no.clone();
                        let connected = item.connected;
                        let is_selected = {
                            let ecu_no = ecu_no.clone();
                            move || selected_dev.get_clone() == ecu_no
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
                        let qr_url = format!("https://gycx.cn?s={bike_no}");
                        let qr_data_url = {
                            let svg = render_qr_svg(&qr_url).unwrap_or_default();
                            format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
                        };
                        let copied = create_signal(false);
                        let copy_link = {
                            let url = qr_url.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let url = url.clone();
                                spawn_local(async move {
                                    let _ = api::copy_to_clipboard(&url).await;
                                    copied.set(true);
                                    TimeoutFuture::new(1500).await;
                                    copied.set(false);
                                });
                            }
                        };
                        let select_dev = {
                            let ecu_no = ecu_no.clone();
                            move |_| c.select(&ecu_no)
                        };
                        let dblclick_qr = {
                            let ecu_no = ecu_no.clone();
                            move |_| {
                                toggle_device_connect(c, ctx, ecu_no.clone());
                            }
                        };
                        view! {
                            div(
                                class=cls,
                                title=if connected { "单击选中，双击断开连接" } else { "单击选中，双击连接并登录" },
                                on:click=select_dev,
                                on:dblclick=dblclick_qr
                            ) {
                                div(class="qr-svg-container") {
                                    img(src=qr_data_url, alt="二维码", style="width:100%;height:100%;display:block;")
                                }
                                div(class="nav-qr-foot") {
                                    span(class="qr-bike-no") { (bike_no) }
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

fn selected_no(c: ClientCtx, ctx: AppCtx) -> Option<String> {
    let no = c.selected.get_clone();
    if no.is_empty() {
        ctx.log_client("【警告】请先在左侧选择一台设备", LogLevel::Warn);
        return None;
    }
    Some(no)
}

#[component]
fn DeviceList() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;
    let adding = create_signal(false);
    let new_no = create_signal(String::new());

    let confirm_add = move || {
        let no = new_no.get_clone().trim().to_string();
        adding.set(false);
        new_no.set(String::new());
        if no.is_empty() {
            return;
        }
        if c.devices.get_clone().iter().any(|d| d.config.device_no == no) {
            c.select(&no);
            return;
        }
        // 新设备沿用当前表单的网关与姿态，省得每台重填
        let config = c.config(no.clone(), ctx.battery_no.get_clone().trim().to_string());
        spawn_local(async move {
            match api::client_update_device(config).await {
                Ok(list) => {
                    c.devices.set(list);
                    c.select(&no);
                }
                Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
            }
        });
    };

    view! {
        div(class="device-list") {
            div(class="device-list-head") {
                span { (move || format!("设备 ({})", c.devices.get_clone().len())) }
                button(class="icon-btn", title="新增设备", on:click=move |_| {
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
                            placeholder="设备序列号，回车确认",
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
                    list=c.devices,
                    view=move |st: DeviceState| {
                        let no = st.config.device_no.clone();
                        let connected = st.connected;
                        let coords = st.config.profile.coordinates.clone();
                        let pick = {
                            let no = no.clone();
                            move |_| c.select(&no)
                        };
                        let dblpick = {
                            let no = no.clone();
                            move |_| {
                                toggle_device_connect(c, ctx, no.clone());
                            }
                        };
                        let remove = {
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let no = no.clone();
                                spawn_local(async move {
                                    match api::client_remove_device(&no).await {
                                        Ok(list) => {
                                            let next = list.first().map(|d| d.config.device_no.clone());
                                            c.devices.set(list);
                                            if c.selected.get_clone() == no {
                                                c.select(next.as_deref().unwrap_or(""));
                                            }
                                        }
                                        Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                                    }
                                });
                            }
                        };
                        let cls = {
                            let no = no.clone();
                            move || {
                                if c.selected.get_clone() == no { "device-item active" } else { "device-item" }
                            }
                        };
                        let sub = if connected {
                            coords
                        } else {
                            "未连接".to_string()
                        };
                        view! {
                            div(
                                class=cls,
                                title=if connected { "单击切换当前配置，双击断开连接" } else { "单击切换当前配置，双击连接并登录" },
                                on:click=pick,
                                on:dblclick=dblpick
                            ) {
                                div(class="device-item-top") {
                                    span(class=if connected { "dot on" } else { "dot off" }) {}
                                    span(class="device-no") { (no) }
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
