use crate::actions::run_battery_named;
use crate::api;
use crate::components::{device_list, qr_panel, DeviceRow, DeviceSource};
use crate::state::{qr_url, AppCtx, BatteryCtx, LogLevel, Page, DEFAULT_BATTERY_QR};
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub(super) fn connect_single_battery(ctx: AppCtx, no: String) {
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

pub(super) fn trigger_battery_connect(b: BatteryCtx, ctx: AppCtx, no: String) {
    b.load_device(&no);
    if b.split_gateway().1 == 0 {
        ctx.log_battery("【警告】网关端口不合法", LogLevel::Warn);
        return;
    }
    connect_single_battery(ctx, no);
}

pub(super) fn toggle_battery_connect(b: BatteryCtx, ctx: AppCtx, no: String) {
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

fn battery_source(ctx: AppCtx) -> DeviceSource {
    let b = ctx.battery;
    DeviceSource {
        page: Page::Battery,
        noun: "电池",
        qr_noun: "电池",
        rows: Rc::new(move || {
            let tmpl = ctx.global_settings.get_clone().battery.qr_url_template;
            b.devices
                .get_clone()
                .into_iter()
                .map(|d| {
                    let no = d.config.battery_no;
                    DeviceRow {
                        qr_url: Some(qr_url(&tmpl, DEFAULT_BATTERY_QR, "{battery_no}", &no)),
                        qr_label: Some(no.clone()),
                        no,
                        host: d.config.host,
                        connected: d.connected,
                        coordinates: d.config.coordinates,
                    }
                })
                .collect()
        }),
        selected: b.selected,
        is_selected: Rc::new(move |no| b.is_selected(no)),
        selected_list: Rc::new(move || b.selected_list()),
        select: Rc::new(move |no| b.select(no)),
        toggle_select: Rc::new(move |no| b.toggle_select(no)),
        range_select: Rc::new(move |no| b.range_select(no)),
        add: Rc::new(move |no: String| {
            if b.devices.get_clone().iter().any(|d| d.config.battery_no == no) {
                b.select(&no);
                return;
            }
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
        }),
        remove: Rc::new(move |no: String| {
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
        }),
        toggle_connect: Rc::new(move |no: String| toggle_battery_connect(b, ctx, no)),
    }
}

#[component]
pub(super) fn BatteryQrPanel() -> View {
    qr_panel(battery_source(use_context::<AppCtx>()))
}

#[component]
pub(super) fn BatteryDeviceList() -> View {
    device_list(battery_source(use_context::<AppCtx>()))
}
