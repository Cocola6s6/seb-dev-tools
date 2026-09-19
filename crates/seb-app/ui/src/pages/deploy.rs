use crate::api;
use crate::components::{select_value, LogSplit};
use crate::state::{AppCtx, DeployOptionItem};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

fn default_zero() -> Vec<DeployOptionItem> {
    vec![DeployOptionItem {
        id: 0,
        label: "0".to_string(),
    }]
}

#[component]
pub fn DeployPage() -> View {
    let ctx = use_context::<AppCtx>();
    let bike_type_id = create_signal(String::new());
    let battery_type_id = create_signal(String::new());
    let battery_pid = create_signal(String::new());
    let supplier_id = create_signal(String::new());
    let dealer_id = create_signal(String::new());
    let device_company_id = create_signal(String::new());
    let batch_no = create_signal(String::new());
    let motor_no = create_signal(String::new());
    let frame_no = create_signal(String::new());
    let has_helmet = create_signal(true);
    let has_trunk = create_signal(true);
    let loaded = create_signal(false);
    let busy = create_signal(false);
    let busy_lookup = create_signal(false);

    let cities = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.cities.is_empty() {
            default_zero()
        } else {
            opts.cities
        }
    });

    let bike_types = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.bike_types.is_empty() {
            default_zero()
        } else {
            opts.bike_types
        }
    });

    let battery_types = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.battery_types.is_empty() {
            default_zero()
        } else {
            opts.battery_types
        }
    });

    let suppliers = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.suppliers.is_empty() {
            default_zero()
        } else {
            opts.suppliers
        }
    });

    let dealers = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.dealers.is_empty() {
            default_zero()
        } else {
            opts.dealers
        }
    });

    let device_companies = create_memo(move || {
        let opts = ctx.deploy_options.get_clone();
        if opts.device_companies.is_empty() {
            default_zero()
        } else {
            opts.device_companies
        }
    });

    let battery_dropdown_open = create_signal(false);

    create_effect(move || {
        if ctx.client.battery_options.get_clone().is_empty() {
            spawn_local(async move {
                if let Ok(list) = api::client_load_batteries().await {
                    ctx.client.battery_options.set(list);
                }
            });
        }
    });

    let ordered_batteries = create_memo(move || {
        let cur_bike = ctx.bike_no.get_clone().trim().to_string();
        let raw = ctx.client.battery_options.get_clone();
        let mut list = raw;

        list.sort_by(|a, b| {
            let a_cur = !cur_bike.is_empty() && a.bound_bike_no.as_deref().map(|s| s.trim()) == Some(&cur_bike);
            let b_cur = !cur_bike.is_empty() && b.bound_bike_no.as_deref().map(|s| s.trim()) == Some(&cur_bike);
            b_cur
                .cmp(&a_cur)
                .then_with(|| a.bound_bike_no.is_none().cmp(&b.bound_bike_no.is_none()).reverse())
                .then_with(|| a.battery_no.cmp(&b.battery_no))
        });
        list
    });

    let filtered_batteries = create_memo(move || {
        let all = ordered_batteries.get_clone();
        let query = ctx.battery_no.get_clone().trim().to_lowercase();
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
        let cfg = ctx.cfg.get_clone();
        if loaded.get() {
            return;
        }
        bike_type_id.set(cfg.deploy.bike_type_id.to_string());
        battery_type_id.set(cfg.deploy.battery_type_id.to_string());
        supplier_id.set(cfg.deploy.supplier_id.to_string());
        dealer_id.set(cfg.deploy.dealer_id.to_string());
        device_company_id.set(cfg.deploy.device_company_id.to_string());
        has_helmet.set(cfg.deploy.has_helmet);
        has_trunk.set(cfg.deploy.has_trunk);
        loaded.set(true);
    });

    let inputs = move || -> Option<(String, String, String)> {
        let bike_no = ctx.bike_no.get_clone().trim().to_string();
        let ecu_no = ctx.device_no.get_clone().trim().to_string();
        let battery_no = ctx.battery_no.get_clone().trim().to_string();
        if bike_no.is_empty() {
            ctx.log_warn("【警告】请先填写车辆编号 (bikeNo)");
            return None;
        }
        if ecu_no.is_empty() {
            ctx.log_warn("【警告】请先填写中控设备序列号 (ecuNo)");
            return None;
        }
        if battery_no.is_empty() {
            ctx.log_warn("【警告】请先填写电池编号 (batteryNo)");
            return None;
        }
        Some((bike_no, ecu_no, battery_no))
    };

    let deploy = move |_| {
        let Some((bike_no, ecu_no, battery_no)) = inputs() else {
            return;
        };
        let city_id = ctx.city_id_value();
        if city_id < 0 {
            ctx.log_warn("【警告】城市 ID (cityId) 不能小于 0");
            return;
        }
        let number = |s: String, fallback: i64| s.trim().parse::<i64>().unwrap_or(fallback);
        let mut cfg = ctx.current_config();
        cfg.deploy.bike_type_id = number(bike_type_id.get_clone(), cfg.deploy.bike_type_id);
        cfg.deploy.battery_type_id = number(battery_type_id.get_clone(), cfg.deploy.battery_type_id);
        cfg.deploy.supplier_id = number(supplier_id.get_clone(), cfg.deploy.supplier_id);
        cfg.deploy.dealer_id = number(dealer_id.get_clone(), cfg.deploy.dealer_id);
        cfg.deploy.device_company_id = number(device_company_id.get_clone(), cfg.deploy.device_company_id);
        cfg.deploy.has_helmet = has_helmet.get();
        cfg.deploy.has_trunk = has_trunk.get();
        cfg.battery_no = battery_no.clone();
        ctx.cfg.set(cfg.clone());

        let (bat_type_id, bat_pid) = (
            cfg.deploy.battery_type_id,
            battery_pid.get_clone().trim().to_string(),
        );
        let (batch, motor, frame) = (
            batch_no.get_clone().trim().to_string(),
            motor_no.get_clone().trim().to_string(),
            frame_no.get_clone().trim().to_string(),
        );
        let (bt_id, sp_id, dl_id, dc_id, hl, tr) = (
            cfg.deploy.bike_type_id,
            cfg.deploy.supplier_id,
            cfg.deploy.dealer_id,
            cfg.deploy.device_company_id,
            cfg.deploy.has_helmet,
            cfg.deploy.has_trunk,
        );

        busy.set(true);
        spawn_local(async move {
            if let Err(e) = api::save_config(&cfg).await {
                ctx.log_error(format!("【错误】保存配置失败: {e}"));
                busy.set(false);
                return;
            }
            let result = api::bike_deploy(
                &bike_no, &ecu_no, &battery_no, bat_type_id, &bat_pid, city_id, bt_id, sp_id, dl_id, dc_id,
                &batch, &motor, &frame, hl, tr
            ).await;
            ctx.apply_deploy(result);
            busy.set(false);
        });
    };

    let lookup = move |_| {
        let bike_no = ctx.bike_no.get_clone().trim().to_string();
        if bike_no.is_empty() {
            ctx.log_warn("【警告】请先填写车辆编号 (bikeNo)");
            return;
        }
        busy_lookup.set(true);
        spawn_local(async move {
            match api::bike_lookup(&bike_no).await {
                Ok(None) => ctx.log_warn(format!("seb_goods_db.bike_tb 中未找到车辆 {bike_no}")),
                Ok(Some(d)) => {
                    if !d.ecu_no.is_empty() {
                        ctx.device_no.set(d.ecu_no.clone());
                    }
                    if !d.battery_no.is_empty() {
                        ctx.battery_no.set(d.battery_no.clone());
                    }
                    if !d.battery_pid.is_empty() {
                        battery_pid.set(d.battery_pid.clone());
                    }
                    if d.city_id > 0 {
                        ctx.city_id.set(d.city_id.to_string());
                    }
                    if d.bike_type_id > 0 {
                        bike_type_id.set(d.bike_type_id.to_string());
                    }
                    if d.battery_type_id > 0 {
                        battery_type_id.set(d.battery_type_id.to_string());
                    }
                    if d.supplier_id > 0 {
                        supplier_id.set(d.supplier_id.to_string());
                    }
                    if d.dealer_id > 0 {
                        dealer_id.set(d.dealer_id.to_string());
                    }
                    if d.device_company_id > 0 {
                        device_company_id.set(d.device_company_id.to_string());
                    }
                    if !d.batch_no.is_empty() {
                        batch_no.set(d.batch_no.clone());
                    }
                    if !d.motor_no.is_empty() {
                        motor_no.set(d.motor_no.clone());
                    }
                    if !d.frame_no.is_empty() {
                        frame_no.set(d.frame_no.clone());
                    }
                    has_helmet.set(d.has_helmet);
                    has_trunk.set(d.has_trunk);

                    let mut cfg = ctx.current_config();
                    cfg.device_no = d.ecu_no.clone();
                    cfg.battery_no = d.battery_no.clone();
                    cfg.city_id = d.city_id;
                    cfg.deploy.bike_type_id = d.bike_type_id;
                    cfg.deploy.battery_type_id = d.battery_type_id;
                    cfg.deploy.supplier_id = d.supplier_id;
                    cfg.deploy.dealer_id = d.dealer_id;
                    cfg.deploy.device_company_id = d.device_company_id;
                    cfg.deploy.has_helmet = d.has_helmet;
                    cfg.deploy.has_trunk = d.has_trunk;
                    let _ = api::save_config(&cfg).await;
                    ctx.cfg.set(cfg);

                    let bat_str = if d.battery_no.is_empty() {
                        "未绑定".to_string()
                    } else {
                        d.battery_no
                    };
                    ctx.log_info(format!(
                        "ecuNo={}, batteryNo={}, cityId={}, 投放状态={}, 业务状态={}, 已删除=0, 设备状态={}",
                        d.ecu_no, bat_str, d.city_id, d.road_status, d.business_status, d.online_status
                    ));
                }
                Err(e) => ctx.log_error(format!("【错误】{e}")),
            }
            busy_lookup.set(false);
        });
    };

    view! {
        LogSplit {
            div(class="page-head") {
                div(class="page-title") { "一键接入" }
                div(class="page-desc") {
                    "快速接入车辆与电池，完成接入后即可直接在「中控指令」和「中控配置」中进行调试。"
                }
            }

            div(class="section") {
                div(class="section-title") {
                    "基础信息"
                    span(class="tag tag-required") { "必选" }
                }
                div(class="grid grid-4") {
                    div(class="field") {
                        label { "车辆编号 (bikeNo)" }
                        input(
                            r#type="text",
                            bind:value=ctx.bike_no,
                            placeholder="如 A60004000180",
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Tab" && !ev.shift_key() && ctx.bike_no.get_clone().trim().is_empty() {
                                    ctx.bike_no.set("A60004000180".to_string());
                                }
                            }
                        )
                    }
                    div(class="field") {
                        label { "中控设备序列号 (ecuNo)" }
                        input(
                            r#type="text",
                            bind:value=ctx.device_no,
                            placeholder="如 799497080",
                            on:change=move |_| ctx.refresh_instance(true),
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Tab" && !ev.shift_key() && ctx.device_no.get_clone().trim().is_empty() {
                                    ctx.device_no.set("799497080".to_string());
                                    ctx.refresh_instance(true);
                                }
                            }
                        )
                    }
                    div(class="field") {
                        label { "电池编号 (batteryNo)" }
                        div(class="battery-combo-wrapper") {
                            input(
                                r#type="text",
                                bind:value=ctx.battery_no,
                                placeholder="输入电池编号或下拉选择",
                                on:focus=move |_| battery_dropdown_open.set(true),
                                on:click=move |_| battery_dropdown_open.set(true),
                                on:keydown=move |ev: web_sys::KeyboardEvent| {
                                    if ev.key() == "Tab" && !ev.shift_key() && ctx.battery_no.get_clone().trim().is_empty() {
                                        ctx.battery_no.set("CTFG024B2E6S4012".to_string());
                                    }
                                }
                            )
                            (if battery_dropdown_open.get() {
                                let bats_empty = filtered_batteries.get_clone().is_empty();
                                let bats_cnt = filtered_batteries.get_clone().len();
                                let cur_bike = ctx.bike_no.get_clone().trim().to_string();
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
                                                        view=move |item: crate::api::BatteryOptionItem| {
                                                            let b_no = item.battery_no.clone();
                                                            let is_cur = ctx.battery_no.get_clone().trim() == b_no;
                                                            let item_cls = if is_cur {
                                                                "battery-dropdown-item active"
                                                            } else {
                                                                "battery-dropdown-item"
                                                            };
                                                            let is_cur_bound = !cur_bike.is_empty() && item.bound_bike_no.as_deref().map(|s| s.trim()) == Some(&cur_bike);
                                                            let bound_badge = match &item.bound_bike_no {
                                                                Some(bike) if is_cur_bound => {
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
                                                                        ctx.battery_no.set(pick_no.clone());
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
                    div(class="field") {
                        label { "城市 (cityId)" }
                        select(on:change=move |ev| ctx.city_id.set(select_value(ev))) {
                            Indexed(
                                list=cities,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = ctx.city_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                }
                div(class="card-actions") {
                    button(disabled=busy_lookup.get(), on:click=lookup) {
                        (if busy_lookup.get() { "查询中…" } else { "查询车辆状态" })
                    }
                }
            }

            div(class="section") {
                div(class="section-title") {
                    "车辆与硬件配置"
                    span(class="tag") { "选填" }
                }
                div(class="grid grid-3") {
                    div(class="field") {
                        label { "车型 (bikeTypeId)" }
                        select(on:change=move |ev| bike_type_id.set(select_value(ev))) {
                            Indexed(
                                list=bike_types,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = bike_type_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                    div(class="field") {
                        label { "电池型号 (batteryTypeId)" }
                        select(on:change=move |ev| battery_type_id.set(select_value(ev))) {
                            Indexed(
                                list=battery_types,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = battery_type_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                    div(class="field") {
                        label { "供应商 (supplierId)" }
                        select(on:change=move |ev| supplier_id.set(select_value(ev))) {
                            Indexed(
                                list=suppliers,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = supplier_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                    div(class="field") {
                        label { "设备厂商 (deviceCompanyId)" }
                        select(on:change=move |ev| device_company_id.set(select_value(ev))) {
                            Indexed(
                                list=device_companies,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = device_company_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                    div(class="field") {
                        label { "经销商 / 运营商 (dealerId)" }
                        select(on:change=move |ev| dealer_id.set(select_value(ev))) {
                            Indexed(
                                list=dealers,
                                view=move |item: DeployOptionItem| {
                                    let val = item.id.to_string();
                                    let is_selected = dealer_id.get_clone() == val;
                                    let text = item.label.clone();
                                    view! {
                                        option(value=val, selected=is_selected) { (text) }
                                    }
                                }
                            )
                        }
                    }
                    div(class="field") {
                        label { "批次号 (batchNo)" }
                        input(r#type="text", bind:value=batch_no, placeholder="选填")
                    }
                    div(class="field") {
                        label { "电机编号 (motorNo)" }
                        input(r#type="text", bind:value=motor_no, placeholder="选填")
                    }
                    div(class="field") {
                        label { "车架号 (frameNo)" }
                        input(r#type="text", bind:value=frame_no, placeholder="选填")
                    }
                }
                div(class="row checks", style="margin-top:14px") {
                    label(class="check") {
                        input(r#type="checkbox", bind:checked=has_helmet)
                        span(class="box") {}
                        span { "带头盔锁" }
                    }
                    label(class="check") {
                        input(r#type="checkbox", bind:checked=has_trunk)
                        span(class="box") {}
                        span { "带尾箱" }
                    }
                }
            }

            div(class="section") {
                div(class="row") {
                    button(class="primary", disabled=busy.get(), on:click=deploy) {
                        (if busy.get() { "接入中…" } else { "一键接入" })
                    }
                }
            }
        }
    }
}
