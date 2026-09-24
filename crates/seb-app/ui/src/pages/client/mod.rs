mod devices;

use crate::actions::run_client_named;
use crate::api;
use crate::components::{
    select_value, Check, Field, InlineMapPicker,
};
use crate::state::{normalize_ecu_no, AlarmType, AppCtx, LogLevel};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const GW_INNER: &str = "bike-seb-inner-test.costrip.cn:32405";
const GW_TEST: &str = "bike-seb-test.costrip.cn:8514";
const GW_PROD: &str = "bike-seb.costrip.cn:8514";

fn gw_or(configured: String, fallback: &str) -> String {
    if configured.trim().is_empty() {
        fallback.to_string()
    } else {
        configured.trim().to_string()
    }
}

const VEHICLE_STATES: &[(&str, &str)] = &[
    ("0", "借车"),
    ("1", "还车"),
    ("2", "撤防"),
    ("3", "运输模式"),
];

#[derive(Clone, PartialEq)]
struct BatteryDropdownItem {
    battery_no: String,
    bound_bike_no: Option<String>,
    is_current_bound: bool,
}

use devices::{connect_single_device, selected_no, trigger_device_connect, ClientQrPanel, DeviceList};

#[component]
pub fn ClientPage() -> View {
    let ctx = use_context::<AppCtx>();
    let c = ctx.client;
    let gw_inner = create_memo(move || gw_or(ctx.global_settings.get_clone().client.default_inner_gw, GW_INNER));
    let gw_test = create_memo(move || gw_or(ctx.global_settings.get_clone().client.default_test_gw, GW_TEST));
    let gw_prod = create_memo(move || gw_or(ctx.global_settings.get_clone().client.default_prod_gw, GW_PROD));
    let battery_dropdown_open = create_signal(false);
    let qr_selection_seq = Rc::new(Cell::new(0u32));

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

    // 车架号是异步反查来的，电池清单也是异步拉的，凑齐了才能定这台设备该用哪颗电池。
    // 当前值读 untrack，否则自己写完又把自己触发一遍
    create_effect(move || {
        let no = c.selected.get_clone();
        let owner = c.owner.get_clone();
        let map = c.bike_map.get_clone();
        let options = c.battery_options.get_clone();
        // owner 对不上说明表单还在切设备，这一轮写进去会落到上一台头上
        if no.is_empty() || owner != no || options.is_empty() {
            return;
        }
        let bike_no = map.get(&no).cloned().unwrap_or_default().trim().to_string();
        let bound = if bike_no.is_empty() {
            None
        } else {
            options
                .iter()
                .find(|b| b.bound_bike_no.as_deref().map(str::trim) == Some(bike_no.as_str()))
        };
        if let Some(b) = bound {
            if untrack(move || c.battery_no.get_clone().trim().to_string()) != b.battery_no {
                c.battery_no.set(b.battery_no.clone());
            }
            return;
        }

        if let Some(free) = options.iter().find(|b| b.bound_bike_no.is_none()) {
            c.battery_no.set(free.battery_no.clone());
        } else {
            c.battery_no.set(String::new());
            ctx.log_client(format!("【警告】{no} 没有可用的空闲电池"), LogLevel::Warn);
        }
    });

    let ordered_batteries = create_memo(move || {
        let selected_no = c.selected.get_clone();
        let cur_bike = c
            .bike_map
            .get_clone()
            .get(&selected_no)
            .cloned()
            .unwrap_or_default()
            .trim()
            .to_string();
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
        let map = c.bike_map.get_clone();
        let bike_no = map.get(&last_no).cloned().unwrap_or_default();
        let battery_no = c.battery_no.get_clone().trim().to_string();
        let qr_bike_no = bike_no.clone();
        let qr_selection_seq = qr_selection_seq.clone();
        let seq = qr_selection_seq.get() + 1;
        qr_selection_seq.set(seq);
        spawn_local(async move {
            TimeoutFuture::new(50).await;
            if qr_selection_seq.get() == seq {
                if let Err(e) = api::set_dock_qr_selection(&qr_bike_no, &battery_no).await {
                    ctx.log_client(format!("二维码同步失败: {e}"), LogLevel::Error);
                }
            }
        });
        if last_no.is_empty() {
            return;
        }
        ctx.device_no.set(last_no.clone());
        ctx.bike_no.set(bike_no.clone());
        if !bike_no.trim().is_empty() {
            return;
        }
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
            format!("批量上报电池 ({})", list.len())
        } else {
            "上报电池".to_string()
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
            run_client_named(ctx, "上报电池", api::client_send_bms(no));
        } else {
            run_client_named(ctx, "批量上报电池", async move {
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
                            div(class="section-title") { "中控与网关" }
                            div(class="grid grid-2") {
                                div(class="field") {
                                    label { "中控序列号 (ECU)" }
                                    input(
                                        r#type="text",
                                        placeholder="9位中控序列号 (如 799497080)",
                                        bind:value=c.device_no,
                                        on:blur=move |_| {
                                            let cur = c.device_no.get_clone();
                                            let norm = normalize_ecu_no(&cur);
                                            if norm != cur {
                                                c.device_no.set(norm.clone());
                                            }
                                            let owner = c.owner.get_clone();
                                            if !norm.is_empty() && !owner.is_empty() && norm != owner {
                                                let owner_clone = owner.clone();
                                                let norm_clone = norm.clone();
                                                spawn_local(async move {
                                                    match api::client_rename_device(&owner_clone, &norm_clone).await {
                                                        Ok(list) => {
                                                            c.devices.set(list);
                                                            c.select(&norm_clone);
                                                        }
                                                        Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                                                    }
                                                });
                                            }
                                        },
                                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                                            if ev.key() == "Enter" {
                                                let cur = c.device_no.get_clone();
                                                let norm = normalize_ecu_no(&cur);
                                                if norm != cur {
                                                    c.device_no.set(norm.clone());
                                                }
                                                let owner = c.owner.get_clone();
                                                if !norm.is_empty() && !owner.is_empty() && norm != owner {
                                                    let owner_clone = owner.clone();
                                                    let norm_clone = norm.clone();
                                                    spawn_local(async move {
                                                        match api::client_rename_device(&owner_clone, &norm_clone).await {
                                                            Ok(list) => {
                                                                c.devices.set(list);
                                                                c.select(&norm_clone);
                                                            }
                                                            Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    )
                                }
                                div(class="field") {
                                    label {
                                        span { "网关地址" }
                                        div(class="gateway-env-shortcuts") {
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == gw_inner.get_clone() { "env-chip inner active" } else { "env-chip inner" },
                                                title="快捷填入内网网关",
                                                on:click=move |_| c.gateway.set(gw_inner.get_clone())
                                            ) { "内网" }
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == gw_test.get_clone() { "env-chip test active" } else { "env-chip test" },
                                                title="快捷填入外网网关",
                                                on:click=move |_| c.gateway.set(gw_test.get_clone())
                                            ) { "外网" }
                                            button(
                                                r#type="button",
                                                class=move || if c.gateway.get_clone().trim() == gw_prod.get_clone() { "env-chip prod active" } else { "env-chip prod" },
                                                title="快捷填入正式网关",
                                                on:click=move |_| c.gateway.set(gw_prod.get_clone())
                                            ) { "正式" }
                                        }
                                    }
                                    input(r#type="text", placeholder="域名:端口", bind:value=c.gateway)
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=connect) { (conn_btn_text.get_clone()) }
                                button(on:click=disconnect) { (disconn_btn_text.get_clone()) }
                                span(class="spacer") {}
                                Check(label="自动心跳", checked=c.heartbeat)
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "车辆姿态与定位" }
                            div(class="grid grid-3") {
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
                                div(class="field") {
                                    label { "运动状态" }
                                    select(on:change=move |ev| {
                                        let val = select_value(ev);
                                        c.motion.set(val == "true");
                                    }) {
                                        option(value="false", selected=!c.motion.get()) { "静止" }
                                        option(value="true", selected=c.motion.get()) { "运动中" }
                                    }
                                }
                            }
                            div(class="field hardware-status-group") {
                                label { "硬件状态" }
                                div(class="hardware-status-bar") {
                                    Check(label="头盔锁（已解锁）", checked=c.helmet_lock_unlocked)
                                    Check(label="头盔在位", checked=c.helmet_present)
                                    Check(label="尾箱在位（加锁）", checked=c.trunk_latch)
                                    Check(label="ACC供电", checked=c.acc_on)
                                }
                            }
                            InlineMapPicker(container_id="client-inline-map", target_coord=c.coordinates)
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
            }
        }
    }
}
