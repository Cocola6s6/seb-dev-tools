use crate::actions::run_client_named;
use crate::api;
use gloo_timers::future::TimeoutFuture;
use crate::components::{device_list, qr_panel, DeviceRow, DeviceSource};
use crate::state::{normalize_ecu_no, qr_url, AppCtx, ClientCtx, LogLevel, Page, DEFAULT_BIKE_QR};
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
        trigger_device_connect(c, ctx, no);
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
                    let bike_no = if !d.config.bike_no.trim().is_empty() {
                        d.config.bike_no.trim().to_string()
                    } else {
                        map.get(&d.config.device_no).cloned().unwrap_or_default()
                    };
                    let bike_no = (!bike_no.trim().is_empty()).then_some(bike_no);
                    DeviceRow {
                        qr_url: bike_no
                            .as_ref()
                            .map(|no| qr_url(&tmpl, DEFAULT_BIKE_QR, "{bike_no}", no)),
                        qr_label: bike_no,
                        no: d.config.device_no,
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
            spawn_local(async move {
                match api::client_resolve_bike(&input_no).await {
                    Ok(Some(info)) => {
                        let ecu = info.ecu_no;
                        let bike = info.bike_no;
                        let mut map = c.bike_map.get_clone();
                        map.insert(ecu.clone(), bike.clone());
                        c.bike_map.set(map);

                        if c.devices.get_clone().iter().any(|d| d.config.device_no == ecu) {
                            c.select(&ecu);
                            ctx.toast(format!("已选中车辆 {bike}"));
                            return;
                        }
                        let mut config = c.config(ecu.clone());
                        config.bike_no = bike.clone();
                        config.ecu_no = String::new();
                        match api::client_update_device(config).await {
                            Ok(list) => {
                                c.devices.set(list);
                                c.select(&ecu);
                                if let Some(bat) = info.battery_no {
                                    if !bat.trim().is_empty() {
                                        c.battery_no.set(bat);
                                    }
                                }
                                ctx.toast(format!("已添加车辆 {bike}"));
                            }
                            Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                        }
                    }
                    Ok(None) => {
                        let norm = normalize_ecu_no(&input_no);
                        if !norm.is_empty() && norm.chars().all(|ch| ch.is_ascii_digit()) {
                            let mut config = c.config(norm.clone());
                            config.bike_no = String::new();
                            config.ecu_no = norm.clone();
                            match api::client_update_device(config).await {
                                Ok(list) => {
                                    c.devices.set(list);
                                    c.select(&norm);
                                    ctx.toast(format!("已按中控 ECU {norm} 添加"));
                                }
                                Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                            }
                        } else {
                            ctx.log_client(format!("【警告】未在数据库查到车辆编号 {input_no} 绑定的中控 ECU"), LogLevel::Warn);
                            ctx.toast(format!("未查到车辆编号 {input_no}"));
                        }
                    }
                    Err(e) => {
                        ctx.log_client(format!("【错误】查询车辆失败: {e}"), LogLevel::Error);
                    }
                }
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
            if new_val.is_empty() {
                return;
            }
            let old_no_clone = old_no.clone();
            spawn_local(async move {
                match api::client_resolve_bike(&new_val).await {
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
                            Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
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
                    Err(e) => ctx.log_client(format!("【错误】{e}"), LogLevel::Error),
                }
            });
        })),
        toggle_connect: Rc::new(move |no: String| toggle_device_connect(c, ctx, no)),
    }
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
