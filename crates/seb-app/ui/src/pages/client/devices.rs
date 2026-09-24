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
        noun: "设备",
        qr_noun: "车辆",
        rows: Rc::new(move || {
            let map = c.bike_map.get_clone();
            let tmpl = ctx.global_settings.get_clone().client.qr_url_template;
            c.devices
                .get_clone()
                .into_iter()
                .map(|d| {
                    let bike_no = map.get(&d.config.device_no).cloned().unwrap_or_default();
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
        add: Rc::new(move |no: String| {
            let no = normalize_ecu_no(&no);
            if no.is_empty() {
                return;
            }
            if c.devices.get_clone().iter().any(|d| d.config.device_no == no) {
                c.select(&no);
                return;
            }
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
        rename: Some(Rc::new(move |old_no: String, new_no: String| {
            let new_no = normalize_ecu_no(&new_no);
            if new_no.is_empty() || new_no == old_no {
                return;
            }
            let old_no_clone = old_no.clone();
            let new_no_clone = new_no.clone();
            spawn_local(async move {
                match api::client_rename_device(&old_no_clone, &new_no_clone).await {
                    Ok(list) => {
                        c.devices.set(list);
                        if c.selected.get_clone() == old_no_clone {
                            c.select(&new_no_clone);
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
