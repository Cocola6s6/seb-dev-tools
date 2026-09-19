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
    let open = create_signal(false);

    // 输入框既是参数项也是搜索框：没选中时按输入过滤，选中后只列同名项
    let filtered = create_memo(move || {
        let kw = keyword.get_clone();
        ctx.ecu_params
            .get_clone()
            .into_iter()
            .filter(|p| p.matches(&kw))
            .take(80)
            .collect::<Vec<EcuParam>>()
    });

    create_effect(move || {
        let kw = keyword.get_clone().trim().to_uppercase();
        if kw.is_empty() {
            param_key.set(String::new());
            return;
        }
        let hit = ctx
            .ecu_params
            .get_clone()
            .into_iter()
            .find(|p| p.key.to_uppercase() == kw || p.name.trim().to_uppercase() == kw)
            .map(|p| p.key);
        if let Some(k) = hit {
            param_key.set(k);
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

    // 仅在切换参数项（param_key 变更）时带出该项的默认值，不监听 param_value 自身避免用户无法清空输入
    let prev_key = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    create_effect(move || {
        let key = param_key.get_clone();
        let old = prev_key.borrow().clone();
        if !key.is_empty() && key != old {
            *prev_key.borrow_mut() = key.clone();
            if op_mode.get_clone() == "设置" {
                if let Some(def) = ctx
                    .ecu_params
                    .get_clone()
                    .into_iter()
                    .find(|p| p.key == key)
                    .and_then(|p| p.default_value)
                {
                    param_value.set(def);
                }
            }
        }
    });

    let submit = move |_| {
        let key = param_key.get_clone().trim().to_string();
        if key.is_empty() {
            ctx.log_warn("【警告】请先从列表里选中一个参数项");
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
                    div(class="field combo") {
                        label { (format!("参数项（{} 项可选）", filtered.get_clone().len())) }
                        input(
                            r#type="text",
                            placeholder="输入参数名或中文名，如 HELMET / 头盔",
                            bind:value=keyword,
                            on:focus=move |_| open.set(true),
                            on:blur=move |_| open.set(false),
                        )
                        (if open.get() {
                            view! {
                                div(class="combo-list", on:mousedown=move |ev: web_sys::MouseEvent| ev.prevent_default()) {
                                    Indexed(
                                        list=filtered,
                                        view=move |p: EcuParam| {
                                            let key = p.key.clone();
                                            let pick = move |_| {
                                                keyword.set(key.clone());
                                                param_key.set(key.clone());
                                                open.set(false);
                                            };
                                            view! {
                                                div(class="combo-item", on:click=pick) {
                                                    span(class="k") { (p.key) }
                                                    span(class="n") { (p.name) }
                                                }
                                            }
                                        }
                                    )
                                }
                            }
                        } else {
                            view! {}
                        })
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

                div(class="hint", style="margin-top:10px") { (description.get_clone()) }

                div(class="row", style="margin-top:18px") {
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
