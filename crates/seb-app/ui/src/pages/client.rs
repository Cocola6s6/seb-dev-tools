use crate::actions::run_client;
use crate::api;
use crate::components::{select_value, Check, Field, MapPickerModal};
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
        let Some(no) = selected_no(c, ctx) else { return };
        if c.split_gateway().1 == 0 {
            ctx.log_client("【警告】网关端口不合法", LogLevel::Warn);
            return;
        }
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
    };

    let disconnect = move |_| {
        let Some(no) = selected_no(c, ctx) else { return };
        run_client(ctx, async move { api::client_disconnect(&no).await.map(|_| ()) });
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
            Some(a) => run_client(ctx, async move {
                api::client_send_alarm(no, a.code, a.name).await
            }),
            None => ctx.log_client("【警告】请先选择告警类型", LogLevel::Warn),
        }
    };

    view! {
        div {
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
                                    run_client(ctx, api::client_send_location(no));
                                }
                            }) { "上报定位" }
                            button(on:click=move |_| {
                                if let Some(no) = selected_no(c, ctx) {
                                    run_client(ctx, api::client_send_bms(no));
                                }
                            }) { "上报 BMS" }
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
                            (move || match c.current().and_then(|st| st.last_msg_id) {
                                Some(id) => format!("最近收到的 msgId: {id}"),
                                None => "尚未收到下发指令".to_string(),
                            })
                        }
                        div(class="card-actions") {
                            button(on:click=move |_| {
                                if let Some(no) = selected_no(c, ctx) {
                                    run_client(ctx, api::client_send_reply(no, true));
                                }
                            }) { "手动回成功" }
                            button(on:click=move |_| {
                                if let Some(no) = selected_no(c, ctx) {
                                    run_client(ctx, api::client_send_reply(no, false));
                                }
                            }) { "手动回失败" }
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
                                    run_client(ctx, api::client_send_ping(no));
                                }
                            }) { "发心跳" }
                        }
                    }
                }
            }

            MapPickerModal(open=map_picker_open, target_coord=c.coordinates)
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
                }) { "＋" }
            }

            (if adding.get() {
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
                        let pick = {
                            let no = no.clone();
                            move |_| c.select(&no)
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
                        let sub = if st.connected {
                            st.config.profile.coordinates.clone()
                        } else {
                            "未连接".to_string()
                        };
                        view! {
                            div(class=cls, on:click=pick) {
                                div(class="device-item-top") {
                                    span(class=if st.connected { "dot on" } else { "dot off" }) {}
                                    span(class="device-no") { (no) }
                                    button(class="device-del", title="移除", on:click=remove) { "×" }
                                }
                                div(class="device-sub") { (sub) }
                            }
                        }
                    }
                )
            }

            div(class="device-batch") {
                div(class="device-batch-title") { "批量" }
                div(class="row") {
                    div(class="btn-with-tip") {
                        button(class="icon-btn ok", on:click=move |_| run_client(ctx, api::client_connect_all())) { "⏻" }
                        span(class="tooltip") { "全部连接" }
                    }
                    div(class="btn-with-tip") {
                        button(class="icon-btn", on:click=move |_| run_client(ctx, api::client_disconnect_all())) { "⭘" }
                        span(class="tooltip") { "全部断开" }
                    }
                    div(class="btn-with-tip") {
                        button(class="icon-btn", on:click=move |_| run_client(ctx, api::client_send_location_all())) { "⌖" }
                        span(class="tooltip") { "全部上报定位" }
                    }
                }
            }
        }
    }
}
