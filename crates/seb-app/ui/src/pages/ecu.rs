use crate::actions::run_send;
use crate::api;
use crate::components::{select_value, DeviceBar};
use crate::state::{AppCtx, EcuParam};
use sycamore::prelude::*;

#[component]
pub fn EcuPage() -> View {
    let ctx = use_context::<AppCtx>();

    let op_mode = create_signal("查询".to_string());
    let keyword = create_signal(String::new());
    let param_key = create_signal(String::new());
    let param_value = create_signal(String::new());

    let filtered = create_memo(move || {
        let kw = keyword.get_clone();
        ctx.ecu_params
            .get_clone()
            .into_iter()
            .filter(|p| p.matches(&kw))
            .collect::<Vec<EcuParam>>()
    });

    create_effect(move || {
        let list = filtered.get_clone();
        let current = param_key.get_clone();
        if list.iter().all(|p| p.key != current) {
            param_key.set(list.first().map(|p| p.key.clone()).unwrap_or_default());
        }
    });

    let description = create_memo(move || {
        let key = param_key.get_clone();
        ctx.ecu_params
            .get_clone()
            .into_iter()
            .find(|p| p.key == key)
            .map(|p| p.description())
            .unwrap_or_default()
    });

    create_effect(move || {
        let key = param_key.get_clone();
        if op_mode.get_clone() != "设置" || !param_value.get_clone().trim().is_empty() {
            return;
        }
        if let Some(def) = ctx
            .ecu_params
            .get_clone()
            .into_iter()
            .find(|p| p.key == key)
            .and_then(|p| p.default_value)
        {
            param_value.set(def);
        }
    });

    let submit = move |_| {
        let key = param_key.get_clone().trim().to_string();
        if key.is_empty() {
            ctx.log_warn("【警告】请选择要操作的 ECU 参数项");
            return;
        }
        if op_mode.get_clone() == "查询" {
            run_send(ctx, api::send_ecu_query(vec![key]));
        } else {
            let value = param_value.get_clone().trim().to_string();
            run_send(ctx, api::send_ecu_set(vec![(key, value)]));
        }
    };

    view! {
        div {
            div(class="page-head") {
                div(class="page-title") { "中控配置" }
                div(class="page-desc") {
                    "查询与设置中控设备各项底层运行参数。"
                }
            }

            DeviceBar {}

            div(class="section") {
                div(class="section-title") { "单项配置" }
                div(class="grid grid-3") {
                    div(class="field") {
                        label { "操作类型" }
                        select(on:change=move |ev| op_mode.set(select_value(ev))) {
                            option(value="查询", selected=op_mode.get_clone() == "查询") { "查询" }
                            option(value="设置", selected=op_mode.get_clone() == "设置") { "设置" }
                        }
                    }
                    div(class="field") {
                        label { "筛选（参数名 / 中文名）" }
                        input(r#type="text", placeholder="如 HELMET 或 头盔", bind:value=keyword)
                    }
                    div(class="field") {
                        label { "参数值" }
                        input(
                            r#type="text",
                            bind:value=param_value,
                            disabled=op_mode.get_clone() == "查询",
                            placeholder="设置模式下填写"
                        )
                    }
                }

                div(class="field", style="margin-top:10px") {
                    label { (format!("参数项（共 {} 项）", filtered.get_clone().len())) }
                    select(on:change=move |ev| param_key.set(select_value(ev))) {
                        Indexed(
                            list=filtered,
                            view=move |p: EcuParam| {
                                let is_selected = param_key.get_clone() == p.key;
                                let value = p.key.clone();
                                let text = format!("{}  —  {}", p.key, p.name);
                                view! {
                                    option(value=value, selected=is_selected) { (text) }
                                }
                            }
                        )
                    }
                }

                div(class="hint", style="margin-top:6px") { (description.get_clone()) }

                div(class="row", style="margin-top:10px") {
                    button(class="primary", on:click=submit) {
                        (if op_mode.get_clone() == "查询" { "发送查询" } else { "发送设置" })
                    }
                }
            }

            div(class="section") {
                div(class="section-title") { "一键场景（头盔）" }
                div(class="row") {
                    button(on:click=move |_| run_send(ctx, api::send_preset("helmet_enable"))) {
                        "开启佩戴全功能 (15)"
                    }
                    button(on:click=move |_| run_send(ctx, api::send_preset("helmet_disable"))) {
                        "恢复传统模式 (3)"
                    }
                    button(on:click=move |_| run_send(ctx, api::send_preset("helmet_query"))) {
                        "查询头盔全套配置"
                    }
                }
                div(class="hint", style="margin-top:6px") {
                    "一键下发会同时设置 DFTHELMETTYPE / DFTHELMETSUPPORTFUN / DFTHELMETWEARCHECK / DFTHELMETCTRLVEHPOWER。"
                }
            }
        }
    }
}
