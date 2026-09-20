mod devices;

use crate::actions::run_battery_named;
use crate::api;
use crate::components::{Check, MapPickerModal};
use crate::state::{AppCtx, LogLevel};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const GW_INNER: &str = "10.12.55.31:32402";
const GW_TEST: &str = "140.143.180.28:28081";
const GW_PROD: &str = "140.143.214.51:28081";

fn gw_or(configured: String, fallback: &str) -> String {
    if configured.trim().is_empty() {
        fallback.to_string()
    } else {
        configured.trim().to_string()
    }
}

use devices::{connect_single_battery, BatteryQrPanel, BatteryDeviceList};

#[component]
pub fn BatteryPage() -> View {
    let ctx = use_context::<AppCtx>();
    let b = ctx.battery;
    let gw_inner = create_memo(move || gw_or(ctx.global_settings.get_clone().battery.default_inner_gw, GW_INNER));
    let gw_test = create_memo(move || gw_or(ctx.global_settings.get_clone().battery.default_test_gw, GW_TEST));
    let gw_prod = create_memo(move || gw_or(ctx.global_settings.get_clone().battery.default_prod_gw, GW_PROD));
    let map_picker_open = create_signal(false);

    // 表单任一项变动就把配置同步给后台落盘；owner 为空说明正在切设备，这一轮不能写。
    let seq = Rc::new(Cell::new(0u32));
    create_effect(move || {
        let config = b.config(b.owner.get_clone());
        if config.battery_no.is_empty() {
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
            match api::battery_update_device(config).await {
                Ok(list) => b.devices.set(list),
                Err(e) => ctx.log_battery(format!("【错误】{e}"), LogLevel::Error),
            }
        });
    });

    let conn_btn_text = create_memo(move || {
        let n = b.selected_list().len();
        if n > 1 {
            format!("批量连接 ({n})")
        } else {
            "连接并登录".to_string()
        }
    });

    let disconn_btn_text = create_memo(move || {
        let n = b.selected_list().len();
        if n > 1 {
            format!("批量断开 ({n})")
        } else {
            "断开".to_string()
        }
    });

    let connect = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if b.split_gateway().1 == 0 {
            ctx.log_battery("【警告】网关端口不合法", LogLevel::Warn);
            return;
        }
        for no in targets {
            connect_single_battery(ctx, no);
        }
    };

    let disconnect = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "断开连接", async move {
                api::battery_disconnect(&no).await.map(|_| ())
            });
        } else {
            run_battery_named(ctx, "批量断开", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_disconnect(&no).await {
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

    let send_location = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报定位", async move {
                api::battery_send_location(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报定位", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_location(no).await {
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
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报告警", async move {
                api::battery_send_alarm(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报告警", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_alarm(no).await {
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

    let send_runtime = move |_| {
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "上报遥测", async move {
                api::battery_send_runtime(no).await
            });
        } else {
            run_battery_named(ctx, "批量上报遥测", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_runtime(no).await {
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
        let targets = b.selected_list();
        if targets.is_empty() {
            ctx.log_battery("【警告】请先在左侧选择设备", LogLevel::Warn);
            return;
        }
        if targets.len() == 1 {
            let no = targets[0].clone();
            run_battery_named(ctx, "发送心跳", async move {
                api::battery_send_ping(no).await
            });
        } else {
            run_battery_named(ctx, "批量发送心跳", async move {
                let mut err_cnt = 0;
                let mut ok_cnt = 0;
                for no in targets {
                    match api::battery_send_ping(no).await {
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
                BatteryQrPanel {}
            }
            div(class="client-split-right") {
                div(class="page-head") {
                    div(class="page-title") { "电池客户端" }
                    div(class="page-desc") {
                        "以电池设备的身份连上 IoT 网关，可同时模拟多台。"
                    }
                }

                div(class="device-layout") {
                    BatteryDeviceList {}

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
                                                class=move || if b.gateway.get_clone().trim() == gw_inner.get_clone() { "env-chip inner active" } else { "env-chip inner" },
                                                title="快捷填入内网网关",
                                                on:click=move |_| b.gateway.set(gw_inner.get_clone())
                                            ) { "内网" }
                                            button(
                                                r#type="button",
                                                class=move || if b.gateway.get_clone().trim() == gw_test.get_clone() { "env-chip test active" } else { "env-chip test" },
                                                title="快捷填入外网网关",
                                                on:click=move |_| b.gateway.set(gw_test.get_clone())
                                            ) { "外网" }
                                            button(
                                                r#type="button",
                                                class=move || if b.gateway.get_clone().trim() == gw_prod.get_clone() { "env-chip prod active" } else { "env-chip prod" },
                                                title="快捷填入正式网关",
                                                on:click=move |_| b.gateway.set(gw_prod.get_clone())
                                            ) { "正式" }
                                        }
                                    }
                                    input(r#type="text", placeholder="域名:端口", bind:value=b.gateway)
                                }
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=connect) { (conn_btn_text.get_clone()) }
                                button(on:click=disconnect) { (disconn_btn_text.get_clone()) }
                                span(class="spacer") {}
                                Check(label="自动心跳", checked=b.heartbeat)
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "电池定位" }
                            div(class="grid grid-2") {
                                div(class="field") {
                                    label { "坐标(高德)" }
                                    div(class="field-inline") {
                                        input(
                                            r#type="text",
                                            placeholder="如 108.38,22.77",
                                            bind:value=b.coordinates,
                                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                                if ev.key() == "Tab" && !ev.shift_key() && b.coordinates.get_clone().trim().is_empty() {
                                                    b.coordinates.set("108.38,22.77".to_string());
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
                            }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_location) { "上报定位" }
                            }
                        }

                        div(class="section") {
                            div(class="section-title") { "快捷上报与操作" }
                            div(class="card-actions") {
                                button(class="primary", on:click=send_runtime) { "上报遥测" }
                                button(on:click=send_alarm) { "上报告警" }
                                button(on:click=send_ping) { "发送心跳" }
                            }
                        }
                    }
                }
            }
        }

        MapPickerModal(open=map_picker_open, target_coord=b.coordinates)
    }
}
