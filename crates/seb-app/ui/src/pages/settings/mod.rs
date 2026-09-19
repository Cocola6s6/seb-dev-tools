mod battery;
mod client;
mod control;
mod deploy;

use crate::api;
use crate::components::select_value;
use crate::state::{
    AppCtx, DeployDefaults, GlobalSettings, Page, ToolboxMode, TripleClickAction,
};
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const EGG_BOX_PLACEHOLDER: &str = "一行一个地址\n图片直接展示，网页用浏览器打开";

#[component]
pub fn SettingsPage() -> View {
    let ctx = use_context::<AppCtx>();

    let deploy = deploy::DeploySection::new();
    let client = client::ClientSection::new();
    let battery = battery::BatterySection::new();
    let control = control::ControlSection::new();
    let egg_url = create_signal(String::new());
    let triple_click = create_signal(TripleClickAction::default());
    let hosts_inner = create_signal(String::new());
    let hosts_uat = create_signal(String::new());
    let hosts_prod = create_signal(String::new());
    let hosts_free = create_signal(false);

    let check_hosts_free = move || {
        spawn_local(async move {
            if let Ok(v) = api::hosts_writable().await {
                hosts_free.set(v);
            }
        });
    };
    check_hosts_free();

    let toggle_hosts_free = move |_| {
        let enable = !hosts_free.get();
        spawn_local(async move {
            match api::set_hosts_free(enable).await {
                Ok(_) => {
                    ctx.toast(if enable { "已免密，切换不再要密码" } else { "已取消免密" });
                    check_hosts_free();
                }
                Err(e) => ctx.toast(format!("操作失败: {e}")),
            }
        });
    };

    let load_from_ctx = move || {
        let cfg = ctx.cfg.get_clone();
        let sets = ctx.global_settings.get_clone();
        deploy.load(cfg.deploy);
        client.load(sets.client);
        battery.load(sets.battery);
        control.load(sets.control);
        egg_url.set(sets.egg_url);
        triple_click.set(sets.triple_click);
        hosts_inner.set(sets.hosts_inner);
        hosts_uat.set(sets.hosts_uat);
        hosts_prod.set(sets.hosts_prod);
    };

    create_effect(move || {
        let _ = ctx.global_settings.get_clone();
        load_from_ctx();
    });

    let save = move |_| {
        let mut cfg = ctx.current_config();
        cfg.deploy = deploy.collect();

        let settings = GlobalSettings {
            client: client.collect(),
            battery: battery.collect(),
            control: control.collect(),
            egg_url: egg_url.get_clone().trim().to_string(),
            triple_click: triple_click.get(),
            hosts_inner: hosts_inner.get_clone(),
            hosts_uat: hosts_uat.get_clone(),
            hosts_prod: hosts_prod.get_clone(),
        };

        cfg.settings = settings.clone();
        ctx.global_settings.set(settings);
        ctx.cfg.set(cfg.clone());

        spawn_local(async move {
            match api::save_config(&cfg).await {
                Ok(_) => ctx.toast("配置已保存"),
                Err(e) => ctx.toast(format!("保存失败: {e}")),
            }
        });
    };

    let reset_defaults = move |_| {
        let def_settings = GlobalSettings::default();
        let mut cfg = ctx.current_config();
        cfg.deploy = DeployDefaults::default();
        cfg.settings = def_settings.clone();

        ctx.global_settings.set(def_settings);
        ctx.cfg.set(cfg.clone());
        load_from_ctx();

        spawn_local(async move {
            let _ = api::save_config(&cfg).await;
            ctx.toast("已恢复默认配置");
        });
    };

    // 系统配置不属于任何功能页，只由百宝箱切换进来
    let sys_view = move || ctx.toolbox_mode.get() == ToolboxMode::System;

    let actions = move || {
        view! {
            div(class="section") {
                div(class="row") {
                    button(class="primary", on:click=save) { "保存配置" }
                    button(on:click=reset_defaults) { "恢复默认" }
                }
            }
        }
    };

    view! {
        div {
            div(class="page-head") {
                div(class="page-title") { (move || ctx.toolbox_mode.get().label()) }
            }

            // 系统配置里的 host 很长，按钮放顶上，省得每次滚到底
            (move || if sys_view() { actions() } else { view! {} })

            (move || if sys_view() {
                view! {
                    div(class="section stack") {
                        div(class="field") {
                            label { "三击百宝箱" }
                            select(
                                class="w-lg",
                                on:change=move |ev| triple_click.set(match select_value(ev).as_str() {
                                    "hosts" => TripleClickAction::Hosts,
                                    _ => TripleClickAction::Egg,
                                })
                            ) {
                                option(
                                    value="egg",
                                    selected=move || triple_click.get() == TripleClickAction::Egg
                                ) { "敲鸡蛋" }
                                option(
                                    value="hosts",
                                    selected=move || triple_click.get() == TripleClickAction::Hosts
                                ) { "切换本地 host" }
                            }
                        }

                        (move || match triple_click.get() {
                            TripleClickAction::Egg => view! {
                                div(class="field") {
                                    label { "鸡蛋框" }
                                    textarea(class="hosts", placeholder=EGG_BOX_PLACEHOLDER, bind:value=egg_url)
                                }
                            },
                            TripleClickAction::Hosts => view! {
                                div(class="row") {
                                    button(on:click=toggle_hosts_free) {
                                        (move || if hosts_free.get() { "取消免密" } else { "免密授权" })
                                    }
                                }
                                div(class="field") {
                                    label { "内网 host" }
                                    textarea(class="hosts", bind:value=hosts_inner)
                                }
                                div(class="field") {
                                    label { "外网 host" }
                                    textarea(class="hosts", bind:value=hosts_uat)
                                }
                                div(class="field") {
                                    label { "正式 host" }
                                    textarea(class="hosts", bind:value=hosts_prod)
                                }
                            },
                        })
                    }
                }
            } else {
                match ctx.page.get() {
                    Page::Deploy => deploy.view(),
                    Page::Client => client.view(),
                    Page::Battery => battery.view(),
                    Page::Control => control.view(),
                    Page::Ecu => view! {},
                }
            })

            (move || if sys_view() { view! {} } else { actions() })
        }
    }
}
