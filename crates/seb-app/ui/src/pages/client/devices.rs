use crate::actions::run_client_named;
use crate::api;
use gloo_timers::future::TimeoutFuture;
use crate::components::{device_list, qr_panel, DeviceRow, DeviceSource};
use crate::state::{is_external_or_prod, normalize_ecu_no, qr_url, AppCtx, ClientCtx, LogLevel, Page, DEFAULT_BIKE_QR};
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub(super) fn connect_single_device(ctx: AppCtx, no: String) {
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

pub(super) fn trigger_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
    c.load_device(&no);
    if c.split_gateway().1 == 0 {
        ctx.log_client("【警告】网关端口不合法", LogLevel::Warn);
        return;
    }
    connect_single_device(ctx, no);
}

pub(super) fn toggle_device_connect(c: ClientCtx, ctx: AppCtx, no: String) {
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
        let sets = ctx.global_settings.get_clone();
        let dev_cfg = c.config(no.clone());
        let cur_gw = c.gateway.get_clone();
        let dev_gw = if dev_cfg.host.trim().is_empty() {
            cur_gw
        } else {
            format!("{}:{}", dev_cfg.host, dev_cfg.port)
        };
        let is_ext = is_external_or_prod(&sets, &dev_gw);

        if is_ext {
            let no = no.clone();
            ctx.show_confirm(
                "环境登录确认",
                "设备将在外网/正式登录，请再次确认设备是测试设备",
                move || {
                    trigger_device_connect(c, ctx, no.clone());
                },
            );
        } else {
            trigger_device_connect(c, ctx, no);
        }
    }
}

pub(super) fn selected_no(c: ClientCtx, ctx: AppCtx) -> Option<String> {
    let no = c.selected.get_clone();
    if no.is_empty() {
        ctx.log_client("【警告】请先在左侧选择一台设备", LogLevel::Warn);
        return None;
    }
    Some(no)
}

fn client_source(ctx: AppCtx) -> DeviceSource {
    let c = ctx.client;
    DeviceSource {
        page: Page::Client,
        noun: "车辆",
        qr_noun: "车辆",
        rows: Rc::new(move || {
            let map = c.bike_map.get_clone();
            let tmpl = ctx.global_settings.get_clone().client.qr_url_template;
            c.devices
                .get_clone()
                .into_iter()
                .map(|d| {
                    let ecu_no = d.config.ecu_no.trim().to_string();
                    let bike_no = if !d.config.bike_no.trim().is_empty() {
                        d.config.bike_no.trim().to_string()
                    } else {
                        map.get(if ecu_no.is_empty() { &d.config.device_no } else { &ecu_no })
                            .cloned()
                            .unwrap_or_default()
                    };
                    let bike_no = (!bike_no.trim().is_empty()).then_some(bike_no);
                    DeviceRow {
                        qr_url: bike_no
                            .as_ref()
                            .map(|no| qr_url(&tmpl, DEFAULT_BIKE_QR, "{bike_no}", no)),
                        qr_label: bike_no,
                        no: d.config.device_no,
                        ecu_no: (!ecu_no.is_empty()).then_some(ecu_no),
                        host: d.config.host,
                        connected: d.connected,
                        coordinates: d.config.profile.coordinates,
                    }
                })
                .collect()
        }),
        selected: c.selected,
        is_selected: Rc::new(move |no| c.is_selected(no)),
        selected_list: Rc::new(move || c.selected_list()),
        select: Rc::new(move |no| c.select(no)),
        toggle_select: Rc::new(move |no| c.toggle_select(no)),
        range_select: Rc::new(move |no| c.range_select(no)),
        add: Rc::new(move |input_no: String| {
            let input_no = input_no.trim().to_string();
            if input_no.is_empty() {
                return;
            }
            if c.querying.get() {
                return;
            }
            let add_gateway = c.add_gateway.get_clone();
            let current_gw = if add_gateway.trim().is_empty() {
                c.gateway.get_clone()
            } else {
                add_gateway
            };
            c.device_input.set(input_no.clone());
            c.querying.set(true);
            ctx.show_loading("正在查询车辆信息...");
            spawn_local(async move {
                let query_gateway = (!current_gw.trim().is_empty()).then_some(current_gw.as_str());
                match api::client_resolve_bike(&input_no, query_gateway).await {
                    Ok(Some(info)) => {
                        let ecu = info.ecu_no;
                        let bike = info.bike_no;
                        let mut map = c.bike_map.get_clone();
                        map.insert(ecu.clone(), bike.clone());
                        c.bike_map.set(map);

                        if let Some(existing) = existing_device_no(c, &ecu, &current_gw) {
                            c.select(&existing);
                            if let Some(bat) = info.battery_no {
                                if !bat.trim().is_empty() {
                                    c.battery_no.set(bat);
                                }
                            }
                            ctx.toast(format!("设备已存在：{bike}"));
                        } else {
                            let device_no = next_device_no(c, &ecu);
                            let mut config = c.config_for_gateway(device_no.clone(), &current_gw);
                            config.bike_no = bike.clone();
                            config.ecu_no = ecu.clone();
                            match api::client_add_device(config).await {
                                Ok(list) => {
                                    c.devices.set(list);
                                    c.select(&device_no);
                                    if let Some(bat) = info.battery_no {
                                        if !bat.trim().is_empty() {
                                            c.battery_no.set(bat);
                                        }
                                    }
                                    ctx.toast(format!("已添加车辆 {bike}"));
                                }
                                Err(e) => {
                                    ctx.log_client(format!("【错误】{e}"), LogLevel::Error);
                                    ctx.toast(format!("添加车辆失败: {e}"));
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        let norm = normalize_ecu_no(&input_no);
                        let ecu = if !norm.is_empty() && norm.chars().all(|ch| ch.is_ascii_digit()) {
                            norm
                        } else {
                            input_no.clone()
                        };
                        if let Some(existing) = existing_device_no(c, &ecu, &current_gw) {
                            c.select(&existing);
                            ctx.toast(format!("设备已存在：{ecu}"));
                        } else {
                            let device_no = next_device_no(c, &ecu);
                            let mut config = c.config_for_gateway(device_no.clone(), &current_gw);
                            config.bike_no = String::new();
                            config.ecu_no = ecu.clone();
                            match api::client_add_device(config).await {
                                Ok(list) => {
                                    c.devices.set(list);
                                    c.select(&device_no);
                                    ctx.log_client(format!("【警告】未查到车辆编号 {input_no}，已按设备编号新增"), LogLevel::Warn);
                                    ctx.toast(format!("未查到车辆，已按设备 {ecu} 添加"));
                                }
                                Err(e) => {
                                    ctx.log_client(format!("【错误】{e}"), LogLevel::Error);
                                    ctx.toast(format!("添加设备失败: {e}"));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        ctx.log_client(format!("【错误】查询车辆失败: {e}"), LogLevel::Error);
                        ctx.toast(format!("查询车辆失败: {e}"));
                    }
                }
                c.querying.set(false);
                c.adding_device.set(false);
            });
        }),
        remove: Rc::new(move |no: String| {
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
        }),
        rename: Some(Rc::new(move |old_no: String, new_val: String| {
            let new_val = new_val.trim().to_string();
            if new_val.is_empty() || c.querying.get() {
                return;
            }
            let old_no_clone = old_no.clone();
            let gw = {
                let dev_cfg = c.config(old_no.clone());
                if dev_cfg.host.trim().is_empty() {
                    c.gateway.get_clone()
                } else {
                    format!("{}:{}", dev_cfg.host, dev_cfg.port)
                }
            };
            c.querying.set(true);
            ctx.show_loading("正在查询车辆信息...");
            spawn_local(async move {
                match api::client_resolve_bike(&new_val, Some(&gw)).await {
                    Ok(Some(info)) => {
                        let new_ecu = info.ecu_no;
                        let new_bike = info.bike_no;
                        let mut map = c.bike_map.get_clone();
                        map.insert(new_ecu.clone(), new_bike.clone());
                        c.bike_map.set(map);

                        if new_ecu == old_no_clone {
                            c.bike_no.set(new_bike.clone());
                            c.ecu_no.set(String::new());
                            let mut config = c.config(new_ecu.clone());
                            config.bike_no = new_bike.clone();
                            config.ecu_no = String::new();
                            if let Ok(list) = api::client_update_device(config).await {
                                c.devices.set(list);
                            }
                            ctx.toast(format!("已更新车辆编号为 {new_bike}"));
                            c.querying.set(false);
                            return;
                        }
                        match api::client_rename_device(&old_no_clone, &new_ecu).await {
                            Ok(mut list) => {
                                let mut config = c.config(new_ecu.clone());
                                config.bike_no = new_bike.clone();
                                config.ecu_no = String::new();
                                if let Ok(updated) = api::client_update_device(config).await {
                                    list = updated;
                                }
                                c.devices.set(list);
                                if c.selected.get_clone() == old_no_clone {
                                    c.select(&new_ecu);
                                }
                                ctx.toast(format!("已更新车辆 {new_bike}"));
                            }
                            Err(e) => {
                                ctx.log_client(format!("【错误】{e}"), LogLevel::Error);
                                ctx.toast(format!("更新车辆失败: {e}"));
                            }
                        }
                    }
                    Ok(None) => {
                        let norm = normalize_ecu_no(&new_val);
                        if !norm.is_empty() && norm != old_no_clone {
                            match api::client_rename_device(&old_no_clone, &norm).await {
                                Ok(mut list) => {
                                    let mut config = c.config(norm.clone());
                                    config.bike_no = String::new();
                                    config.ecu_no = norm.clone();
                                    if let Ok(updated) = api::client_update_device(config).await {
                                        list = updated;
                                    }
                                    c.devices.set(list);
                                    if c.selected.get_clone() == old_no_clone {
                                        c.select(&norm);
                                    }
                                    ctx.toast(format!("已按中控 ECU {norm} 更新"));
                                }
                                Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                            }
                        } else {
                            ctx.log_client(format!("【警告】未在数据库查到车辆编号 {new_val} 绑定的中控 ECU"), LogLevel::Warn);
                            ctx.toast(format!("未查到车辆编号 {new_val}"));
                        }
                    }
                    Err(e) => {
                        ctx.log_client(format!("【错误】查询失败: {e}"), LogLevel::Error);
                        ctx.toast(format!("查询失败: {e}"));
                    }
                }
                c.querying.set(false);
            });
        })),
        toggle_connect: Rc::new(move |no: String| toggle_device_connect(c, ctx, no)),
    }
}

fn next_device_no(c: ClientCtx, ecu_no: &str) -> String {
    let devices = c.devices.get_clone();
    if !devices.iter().any(|d| d.config.device_no == ecu_no) {
        return ecu_no.to_string();
    }
    let mut index = 2;
    loop {
        let candidate = format!("{ecu_no}#{index}");
        if !devices.iter().any(|d| d.config.device_no == candidate) {
            return candidate;
        }
        index += 1;
    }
}

fn existing_device_no(c: ClientCtx, ecu_no: &str, gateway: &str) -> Option<String> {
    let (host, port) = gateway.trim().rsplit_once(':').unwrap_or((gateway.trim(), ""));
    let host = host.trim();
    let port = port.trim().parse::<u16>().unwrap_or(0);
    c.devices.get_clone().into_iter().find_map(|d| {
        let existing_ecu = if d.config.ecu_no.trim().is_empty() {
            d.config.device_no.trim()
        } else {
            d.config.ecu_no.trim()
        };
        (existing_ecu == ecu_no.trim()
            && d.config.host.trim() == host
            && d.config.port == port)
            .then_some(d.config.device_no)
    })
}

#[component]
pub(super) fn ClientQrPanel() -> View {
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

    qr_panel(client_source(ctx))
}

#[component]
pub(super) fn DeviceList() -> View {
    device_list(client_source(use_context::<AppCtx>()))
}
