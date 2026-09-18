use crate::actions::run_client_named;
use crate::api;
use crate::components::{render_qr_svg, select_value, Check, Field, MapPickerModal};
use crate::state::{host_env_tag, AlarmType, AppCtx, ClientCtx, DeviceState, LogLevel, Page};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const GW_INNER: &str = "bike-seb-inner-test.costrip.cn:32405";
const GW_TEST: &str = "bike-seb-test.costrip.cn:8514";
const GW_PROD: &str = "bike-seb.costrip.cn:8514";

const VEHICLE_STATES: &[(&str, &str)] = &[
    ("0", "借车"),
    ("1", "还车"),
    ("2", "撤防"),
    ("3", "运输模式"),
];

fn connect_single_device(ctx: AppCtx, no: String) {
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
            Err(e) => ctx.log_client(format!("【错误】{no}: {e}"), LogLevel::Error),
        }
    });
}

fn trigger_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
    c.load_device(&no);
    if c.split_gateway().1 == 0 {
        ctx.log_client("【警告】网关端口不合法", LogLevel::Warn);
        return;
    }
    connect_single_device(ctx, no);
}

fn toggle_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
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

fn selected_no(c: ClientCtx, ctx: AppCtx) -> Option<String> {
    let no = c.selected.get_clone();
    if no.is_empty() {
        ctx.log_client("【警告】请先在左侧选择一台设备", LogLevel::Warn);
        return None;
    }
    Some(no)
}

#[derive(Clone, PartialEq)]
struct BatteryDropdownItem {
    battery_no: String,
    bound_bike_no: Option<String>,
    is_current_bound: bool,
}

#[component]
pub fn ClientPage() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;
    let map_picker_open = create_signal(false);
    let battery_dropdown_open = create_signal(false);

    // 表单任一项变动就把配置同步给后台；owner 为空说明正在切设备，这一轮不能写。
    // 配置要落盘，所以连续输入时只认最后一次
    // 计数器不能用信号：切页会销毁本页作用域，延时里再读就是读已释放的信号
    let seq = Rc::new(Cell::new(0u32));
    create_effect(move || {
        let config = c.config(c.owner.get_clone());
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

    create_effect(move || {
        if c.battery_options.get_clone().is_empty() {
            spawn_local(async move {
                if let Ok(list) = api::client_load_batteries().await {
                    c.battery_options.set(list);
                }
            });
        }
    });

    let ordered_batteries = create_memo(move || {
        let cur_bike = ctx.bike_no.get_clone().trim().to_string();
        let raw = c.battery_options.get_clone();
        let mut list: Vec<BatteryDropdownItem> = raw
            .into_iter()
            .map(|b| {
                let is_current_bound = match &b.bound_bike_no {
                    Some(bike) => !cur_bike.is_empty() && bike.trim() == cur_bike,
                    None => false,
                };
                BatteryDropdownItem {
                    battery_no: b.battery_no,
                    bound_bike_no: b.bound_bike_no,
                    is_current_bound,
                }
            })
            .collect();

        list.sort_by(|a, b| {
            b.is_current_bound
                .cmp(&a.is_current_bound)
                .then_with(|| a.bound_bike_no.is_none().cmp(&b.bound_bike_no.is_none()).reverse())
                .then_with(|| a.battery_no.cmp(&b.battery_no))
        });
        list
    });

    let filtered_batteries = create_memo(move || {
        let all = ordered_batteries.get_clone();
        let query = c.battery_no.get_clone().trim().to_lowercase();
        if query.is_empty() {
            all
        } else {
            all.into_iter()
                .filter(|item| {
                    item.battery_no.to_lowercase().contains(&query)
                        || item
                            .bound_bike_no
                            .as_deref()
                            .map(|s| s.to_lowercase().contains(&query))
                            .unwrap_or(false)
                })
                .collect()
        }
    });

    create_effect(move || {
        let list = c.selected_list();
        let last_no = list.last().cloned().unwrap_or_else(|| c.selected.get_clone());
        if last_no.is_empty() {
            return;
        }
        ctx.device_no.set(last_no.clone());
        let map = c.bike_map.get_clone();
        if let Some(bike) = map.get(&last_no) {
            if !bike.trim().is_empty() {
                ctx.bike_no.set(bike.clone());
            }
        } else {
            let last_no = last_no.clone();
            spawn_local(async move {
                if let Ok(m) = api::client_get_bike_nos(vec![last_no.clone()]).await {
                    if let Some(bike) = m.get(&last_no) {
                        if !bike.trim().is_empty() {
                            ctx.bike_no.set(bike.clone());
                        }
                    }
                    if !m.is_empty() {
                        let mut cur = c.bike_map.get_clone();
                        cur.extend(m);
                        c.bike_map.set(cur);
                    }
                }
            });
        }
    });

    let conn_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量连接 ({})", list.len())
        } else {
            "连接并登录".to_string()
        }
    });

    let disconn_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量断开 ({})", list.len())
        } else {
            "断开".to_string()
        }
    });

    let loc_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量上报定位 ({})", list.len())
        } else {
            "上报定位".to_string()
        }
    });

    let bms_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量上报 BMS ({})", list.len())
        } else {
            "上报 BMS".to_string()
        }
    });


    let alarm_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量上报告警 ({})", list.len())
        } else {
            "上报告警".to_string()
        }
    });

    let ping_btn_text = create_memo(move || {
        let list = c.selected_list();
        if list.len() > 1 {
            format!("批量心跳 ({})", list.len())
        } else {
            "发心跳".to_string()
        }
    });

    let connect = move |_| {
        let targets = c.selected_list();
        if targets.is_empty() {
            ctx.log_client("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            trigger_device_connect(c, ctx, no);
        } else {
            ctx.log_client(format!("正在批量连接 {} 设备...", targets.len()), LogLevel::Info);
            for no in targets {
                connect_single_device(ctx, no);
            }
        }
    };

    let disconnect = move |_| {
        let targets = c.selected_list();
        if targets.is_empty() {
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_client_named(ctx, "断开连接", async move {
                api::client_disconnect(&no).await.map(|_| ())
            });
        } else {
            run_client_named(ctx, "批量断开", async move {
                for no in targets {
                    let _ = api::client_disconnect(&no).await;
                }
                Ok(())
            });
        }
    };

    let send_location = move |_| {
        let targets = c.selected_list();
        if targets.is_empty() {
            ctx.log_client("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_client_named(ctx, "上报定位", api::client_send_location(no));
        } else {
            run_client_named(ctx, "批量上报定位", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::client_send_location(no).await {
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

    let send_bms = move |_| {
        let targets = c.selected_list();
        if targets.is_empty() {
            ctx.log_client("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_client_named(ctx, "上报 BMS", api::client_send_bms(no));
        } else {
            run_client_named(ctx, "批量上报 BMS", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::client_send_bms(no).await {
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
        let targets = c.selected_list();
        if targets.is_empty() {
            ctx.log_client("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        let code = c.alarm_type.get_clone();
        let found_alarm = c
            .alarm_types
            .get_clone()
            .into_iter()
            .find(|a| a.code.to_string() == code);
        let Some(a) = found_alarm else {
            ctx.log_client("【警告】请先选择告警类型", LogLevel::Warn);
            return;
        };
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_client_named(ctx, "上报告警", async move {
                api::client_send_alarm(no, a.code, a.name).await
            });
        } else {
            run_client_named(ctx, "批量上报告警", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::client_send_alarm(no, a.code, a.name.clone()).await {
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
        let targets = c.selected_list();
        if targets.is_empty() {
            ctx.log_client("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_client_named(ctx, "发送心跳", api::client_send_ping(no));
        } else {
            run_client_named(ctx, "批量发送心跳", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::client_send_ping(no).await {
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
                                        div(class="gateway-env-shortcuts") {
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == GW_INNER { "env-chip inner active" } else { "env-chip inner" },
                                                title="快捷填入内网网关",
                                                on:click=move |_| c.gateway.set(GW_INNER.to_string())
                                            ) { "内网" }
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == GW_TEST { "env-chip test active" } else { "env-chip test" },
                                                title="快捷填入外网网关",
                                                on:click=move |_| c.gateway.set(GW_TEST.to_string())
                                            ) { "外网" }
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == GW_PROD { "env-chip prod active" } else { "env-chip prod" },
                                                title="快捷填入正式网关",
                                                on:click=move |_| c.gateway.set(GW_PROD.to_string())
                                            ) { "正式" }
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
                                button(class="primary", on:click=connect) { (conn_btn_text.get_clone()) }
                                button(on:click=disconnect) { (disconn_btn_text.get_clone()) }
                                span(class="spacer") {}
                                Check(label="60 秒心跳", checked=c.heartbeat)
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "车辆姿态" }
                            div(class="grid grid-3") {
                                div(class="field") {
                                    label { "坐标(高德)" }
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
                                                path(d="M12 2C8.13 2 5 5.13 5 9c0 5.25 7 13 7 13s7-7.75 7-13c0-3.87-3.13-7-7-7zm0 9.5c-1.38 0-2.5-1.12-2.5-2.5s1.12-2.5 2.5-2.5 2.5 1.12 2.5 2.5-1.12 2.5 2.5-1.12 2.5 2.5z") {}
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
                                Field(label="电池电量", value=c.soc)
                                div(class="field") {
                                    label { "电池编号" }
                                    div(class="battery-combo-wrapper") {
                                        input(
                                            r#type="text",
                                            placeholder="输入电池编号或下拉选择",
                                            bind:value=c.battery_no,
                                            on:focus=move |_| battery_dropdown_open.set(true),
                                            on:click=move |_| battery_dropdown_open.set(true)
                                        )
                                        (if battery_dropdown_open.get() {
                                            let bats_empty = filtered_batteries.get_clone().is_empty();
                                            let bats_cnt = filtered_batteries.get_clone().len();
                                            view! {
                                                div(class="combo-backdrop", on:click=move |_| battery_dropdown_open.set(false)) {}
                                                div(class="battery-dropdown-popover") {
                                                    (if bats_empty {
                                                        view! {
                                                            div(class="device-dropdown-empty") {
                                                                "无匹配电池"
                                                            }
                                                        }
                                                    } else {
                                                        view! {
                                                            div(class="device-dropdown-header") {
                                                                span { "选择电池" }
                                                                span(class="device-dropdown-count") {
                                                                    (format!("共 {bats_cnt} 块"))
                                                                }
                                                            }
                                                            div(class="battery-dropdown-items") {
                                                                Indexed(
                                                                    list=filtered_batteries,
                                                                    view=move |item: BatteryDropdownItem| {
                                                                        let b_no = item.battery_no.clone();
                                                                        let is_cur = c.battery_no.get_clone() == b_no;
                                                                        let item_cls = if is_cur {
                                                                            "battery-dropdown-item active"
                                                                        } else {
                                                                            "battery-dropdown-item"
                                                                        };
                                                                        let bound_badge = match &item.bound_bike_no {
                                                                            Some(bike) if item.is_current_bound => {
                                                                                let bike = bike.clone();
                                                                                view! { span(class="bat-binding bound-cur") { (format!("已被 {bike} 绑定")) } }
                                                                            }
                                                                            Some(bike) if !bike.trim().is_empty() => {
                                                                                let bike = bike.clone();
                                                                                view! { span(class="bat-binding bound-other") { (format!("已被 {bike} 绑定")) } }
                                                                            }
                                                                            _ => {
                                                                                view! { span(class="bat-binding") { "未绑定" } }
                                                                            }
                                                                        };
                                                                        let pick_no = b_no.clone();
                                                                        view! {
                                                                            div(
                                                                                class=item_cls,
                                                                                on:click=move |_| {
                                                                                    c.battery_no.set(pick_no.clone());
                                                                                    battery_dropdown_open.set(false);
                                                                                }
                                                                            ) {
                                                                                span(class="bat-no") { (b_no) }
                                                                                (bound_badge)
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
                                Field(label="偏向角", value=c.deflection_angle)
                                Field(label="速度", value=c.speed)
                            }
                            div(class="row checks", style="margin-top:14px") {
                                Check(label="车辆运动状态（运动中）", checked=c.motion)
                                Check(label="头盔锁状态（已解锁）", checked=c.helmet_lock_unlocked)
                                Check(label="头盔在位状态（在位）", checked=c.helmet_present)
                                Check(label="尾箱在位状态（加锁/在位）", checked=c.trunk_latch)
                                Check(label="供电状态（供电）", checked=c.acc_on)
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_location) { (loc_btn_text.get_clone()) }
                                button(on:click=send_bms) { (bms_btn_text.get_clone()) }
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
                                button(class="primary", on:click=send_alarm) { (alarm_btn_text.get_clone()) }
                                button(on:click=send_ping) { (ping_btn_text.get_clone()) }
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "指令应答" }
                            div(class="row checks") {
                                Check(label="是否自动回复命令", checked=c.auto_reply)
                                Check(label="自动回复成功", checked=c.reply_success)
                                Check(label="回复后附带定位", checked=c.reply_with_location)
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
    host: String,
    connected: bool,
}

#[component]
fn ClientQrPanel() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;

    let known_ecus = Rc::new(std::cell::RefCell::new(std::collections::HashSet::<String>::new()));
    create_effect(move || {
        let devs = c.devices.get_clone();
        let ecu_nos: Vec<String> = devs
            .into_iter()
            .map(|d| d.config.device_no.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if ecu_nos.is_empty() {
            c.bike_map.set(std::collections::HashMap::new());
            known_ecus.borrow_mut().clear();
            return;
        }
        let missing: Vec<String> = {
            let mut known = known_ecus.borrow_mut();
            let missing: Vec<String> = ecu_nos.iter().filter(|no| !known.contains(*no)).cloned().collect();
            for no in &missing {
                known.insert(no.clone());
            }
            missing
        };
        if missing.is_empty() {
            return;
        }
        spawn_local(async move {
            if let Ok(new_map) = api::client_get_bike_nos(missing).await {
                if !new_map.is_empty() {
                    let mut cur = c.bike_map.get_clone();
                    cur.extend(new_map);
                    c.bike_map.set(cur);
                }
            }
        });
    });

    let valid_items = create_memo(move || {
        let devs = c.devices.get_clone();
        let map = c.bike_map.get_clone();
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
                    host: d.config.host,
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
                        let host = item.host.clone();
                        let connected = item.connected;
                        let (env_label, env_class) = host_env_tag(&host);
                        let is_selected = {
                            let ecu_no = ecu_no.clone();
                            move || c.is_selected(&ecu_no)
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
                            let bike_no = bike_no.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let url = url.clone();
                                let bike_no = bike_no.clone();
                                spawn_local(async move {
                                    let _ = api::copy_to_clipboard(&url).await;
                                    copied.set(true);
                                    ctx.toast(format!("已复制车辆 {bike_no} 二维码链接"));
                                    TimeoutFuture::new(1500).await;
                                    copied.set(false);
                                });
                            }
                        };
                        let select_dev = {
                            let ecu_no = ecu_no.clone();
                            move |ev: web_sys::MouseEvent| {
                                if ev.meta_key() || ev.ctrl_key() {
                                    c.toggle_select(&ecu_no);
                                } else if ev.shift_key() {
                                    c.range_select(&ecu_no);
                                } else {
                                    c.select(&ecu_no);
                                }
                            }
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
                                title=if connected { "单击选中，双击断开连接 (按住 Cmd/Shift 可多选)" } else { "单击选中，双击连接并登录 (按住 Cmd/Shift 可多选)" },
                                on:click=select_dev,
                                on:dblclick=dblclick_qr
                            ) {
                                span(class=format!("qr-env-badge {env_class}")) { (env_label) }
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
        let config = c.config(no.clone());
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

    let head_text = create_memo(move || {
        let total = c.devices.get_clone().len();
        let selected_cnt = c.selected_list().len();
        if selected_cnt > 1 {
            format!("设备 (已选 {}/{})", selected_cnt, total)
        } else {
            format!("设备 ({})", total)
        }
    });

    let copied = create_signal(false);
    let copy_selected = {
        let c = c.clone();
        let ctx = ctx.clone();
        let copied = copied.clone();
        move || {
            let list = c.selected_list();
            let text = if list.is_empty() {
                let sel = c.selected.get_clone();
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
                    ctx.toast(format!("已复制设备序列号：{text}"));
                } else {
                    ctx.toast(format!("已复制 {count} 设备序列号到剪贴板"));
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
                if ctx.page.get() == Page::Client {
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
                        let host = st.config.host.clone();
                        let connected = st.connected;
                        let coords = st.config.profile.coordinates.clone();
                        let (env_label, env_class) = host_env_tag(&host);
                        let pick = {
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| {
                                if ev.meta_key() || ev.ctrl_key() {
                                    c.toggle_select(&no);
                                } else if ev.shift_key() {
                                    c.range_select(&no);
                                } else {
                                    c.select(&no);
                                }
                            }
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
                        let is_selected = {
                            let no = no.clone();
                            move || c.is_selected(&no)
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
