use crate::api;
use crate::components::select_value;
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

    let inputs = move || -> Option<(String, String)> {
        let bike_no = ctx.bike_no.get_clone().trim().to_string();
        let ecu_no = ctx.device_no.get_clone().trim().to_string();
        if bike_no.is_empty() {
            ctx.log_warn("【警告】请先填写车辆编号 (bikeNo)");
            return None;
        }
        if ecu_no.is_empty() {
            ctx.log_warn("【警告】请先填写中控设备序列号 (ecuNo)");
            return None;
        }
        Some((bike_no, ecu_no))
    };

    let deploy = move |_| {
        let Some((bike_no, ecu_no)) = inputs() else {
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
        ctx.cfg.set(cfg.clone());

        let (battery_no, bat_type_id, bat_pid) = (
            ctx.battery_no.get_clone().trim().to_string(),
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
        spawn_local(async move {
            match api::bike_lookup(&bike_no).await {
                Ok(s) if s.is_empty() => ctx.log_warn(format!("bike_tb 中没有 {bike_no}")),
                Ok(s) => ctx.log_info(s),
                Err(e) => ctx.log_error(format!("【错误】{e}")),
            }
        });
    };

    view! {
        div {
            div(class="page-head") {
                div(class="page-title") { "一键接入" }
                div(class="page-desc") {
                    "快速接入车辆与电池，完成接入后即可直接在「中控指令」和「中控配置」中进行调试。"
                }
            }

            div(class="section") {
                div(class="section-title") {
                    "基础信息"
                    span(class="tag") { "必填" }
                }
                div(class="grid grid-3") {
                    div(class="field") {
                        label { "车辆编号 (bikeNo)" }
                        input(r#type="text", bind:value=ctx.bike_no, placeholder="如 100000001")
                    }
                    div(class="field") {
                        label { "中控设备序列号 (ecuNo)" }
                        input(
                            r#type="text",
                            bind:value=ctx.device_no,
                            placeholder="如 019552878",
                            on:change=move |_| ctx.refresh_instance(true)
                        )
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
            }

            div(class="section") {
                div(class="section-title") {
                    "车辆与硬件配置"
                    span(class="tag") { "选填 / 默认参数" }
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
                        label { "电池编号 (batteryNo)" }
                        input(r#type="text", bind:value=ctx.battery_no, placeholder="选填，如 B00001")
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
                    button(on:click=lookup) { "查询车辆状态" }
                }
            }
        }
    }
}
