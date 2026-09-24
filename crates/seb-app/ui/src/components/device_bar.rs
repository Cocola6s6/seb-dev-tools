use crate::state::{
    inner_host, normalize_ecu_no, AppCtx,
};
use sycamore::prelude::*;

#[component]
pub fn DeviceBar() -> View {
    let ctx = use_context::<AppCtx>();
    let open = create_signal(false);

    let normalize_and_refresh = move || {
        let cur = ctx.device_no.get_clone();
        let norm = normalize_ecu_no(&cur);
        if norm != cur {
            ctx.device_no.set(norm);
        }
        ctx.refresh_instance(true);
    };

    let inner_devices = create_memo(move || {
        let host = inner_host(&ctx.global_settings.get_clone());
        ctx.client
            .devices
            .get_clone()
            .into_iter()
            .filter(|d| d.config.host == host)
            .collect::<Vec<_>>()
    });

    let online_count = create_memo(move || {
        inner_devices
            .get_clone()
            .into_iter()
            .filter(|d| d.connected)
            .count()
    });

    view! {
        div(class="section") {
            div(class="section-title") {
                span { "设备" }
            }

            div(class="device-bar-row") {
                div(class="field device-no-field") {
                    input(
                        r#type="text",
                        placeholder="输入中控设备序列号 (如 799497080)",
                        bind:value=ctx.device_no,
                        on:blur=move |_| normalize_and_refresh(),
                        on:change=move |_| normalize_and_refresh(),
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                normalize_and_refresh();
                            } else if ev.key() == "Tab" && !ev.shift_key() && ctx.device_no.get_clone().trim().is_empty() {
                                ctx.device_no.set("799497080".to_string());
                                ctx.refresh_instance(true);
                            }
                        }
                    )
                }

                div(class="device-select-wrapper") {
                    button(
                        class=move || if open.get() { "device-select-trigger-btn active" } else { "device-select-trigger-btn" },
                        title="选择设备",
                        on:click=move |ev: web_sys::MouseEvent| {
                            ev.stop_propagation();
                            open.set(!open.get());
                        }
                    ) {
                        svg(viewBox="0 0 24 24", width="14", height="14", fill="none", stroke="currentColor", stroke-width="2", stroke-linecap="round", stroke-linejoin="round") {
                            rect(x="2", y="3", width="20", height="14", rx="2", ry="2") {}
                            line(x1="8", y1="21", x2="16", y2="21") {}
                            line(x1="12", y1="17", x2="12", y2="21") {}
                        }
                        span {
                            (move || {
                                let total = inner_devices.get_clone().len();
                                if total > 0 {
                                    let online = online_count.get();
                                    format!("设备 ({online}/{total})")
                                } else {
                                    "选择设备".to_string()
                                }
                            })
                        }
                        svg(viewBox="0 0 10 6", width="10", height="6", class=move || if open.get() { "caret open" } else { "caret" }) {
                            path(d="M1 1l4 4 4-4", fill="none", stroke="currentColor", stroke-width="1.6", stroke-linecap="round", stroke-linejoin="round") {}
                        }
                    }

                    (if open.get() {
                        view! {
                            div(class="combo-backdrop", on:click=move |_| open.set(false)) {}
                            div(class="device-dropdown-popover") {
                                (if inner_devices.get_clone().is_empty() {
                                    view! {
                                        div(class="device-dropdown-empty") {
                                            "暂无模拟设备，可在「中控客户端」添加或连接"
                                        }
                                    }
                                } else {
                                    view! {
                                        div(class="device-dropdown-header") {
                                            span { "选择设备" }
                                            span(class="device-dropdown-count") {
                                                (format!("共 {} 台", inner_devices.get_clone().len()))
                                            }
                                        }
                                        div(class="device-dropdown-items") {
                                            Indexed(
                                                list=inner_devices,
                                                view=move |st: crate::state::DeviceState| {
                                                    let no = st.config.device_no.clone();
                                                    let connected = st.connected;
                                                    let pick = no.clone();
                                                    let full_no = no.clone();
                                                    let is_current = {
                                                        let no = no.clone();
                                                        move || ctx.device_no.get_clone().trim() == no
                                                    };
                                                    let item_cls = move || {
                                                        if is_current() {
                                                            "device-dropdown-item active"
                                                        } else {
                                                            "device-dropdown-item"
                                                        }
                                                    };
                                                    view! {
                                                        div(class=item_cls, on:click=move |_| {
                                                            ctx.device_no.set(pick.clone());
                                                            ctx.refresh_instance(true);
                                                            open.set(false);
                                                        }) {
                                                            span(class=if connected { "dev-dot on" } else { "dev-dot off" }) {}
                                                            span(class="dev-no", title=full_no) { (no) }
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
        }
    }
}
