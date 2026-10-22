use crate::api;
use crate::state::AppCtx;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[component]
pub fn WhatsNewNotice() -> View {
    let ctx = use_context::<AppCtx>();

    // 不记录已读，下次打开仍会提醒
    let close_temp = move |_| {
        ctx.show_whats_new.set(false);
    };

    let acknowledge = move |_| {
        ctx.show_whats_new.set(false);
        let mut cfg = ctx.current_config();
        cfg.last_seen_version = CURRENT_VERSION.to_string();
        ctx.cfg.set(cfg.clone());
        spawn_local(async move {
            let _ = api::save_config(&cfg).await;
        });
    };

    view! {
        (move || if ctx.show_whats_new.get() {
            view! {
                div(class="whats-new-backdrop", on:click=close_temp)
                div(class="whats-new-card") {
                    div(class="whats-new-head") {
                        div(class="whats-new-title-group") {
                            span(class="whats-new-tag") { "NEW" }
                            span(class="whats-new-title") { (format!("版本更新 (v{})", CURRENT_VERSION)) }
                        }
                        button(
                            class="whats-new-close",
                            title="暂时关闭",
                            on:click=close_temp
                        ) {
                            svg(viewBox="0 0 24 24", width="13", height="13", fill="none", stroke="currentColor", stroke-width="2", stroke-linecap="round", stroke-linejoin="round") {
                                path(d="M18 6L6 18M6 6l12 12") {}
                            }
                        }
                    }
                    div(class="whats-new-body") {
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "外网/正式数据查询" }
                                "：支持通过 JSW 接口查询外网与正式环境真实车辆、ECU 及电池绑定，内网直连与外网正式自动分流。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "数据库配置全局化" }
                                "：MySQL 直连与 JSW 查询配置整合至中控指令全局设置中，支持自定义配置并可一键恢复默认。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "环境登录二次确认" }
                                "：中控与电池客户端在外网与正式环境连接时增加二次安全确认，弹窗颜色自动契合当前模块主题色。"
                            }
                        }
                    }
                    div(class="whats-new-foot") {
                        button(
                            class="whats-new-btn",
                            on:click=acknowledge
                        ) {
                            "知道了"
                        }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}
