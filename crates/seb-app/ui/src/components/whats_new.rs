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
                                span(class="whats-new-label") { "百宝箱快捷方式" }
                                "：单击展开二维码，双击切换全局/系统配置，三击敲鸡蛋或切换本地 host。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "电池客户端与全局配置" }
                                "：新增电池客户端调试能力，并增加全局配置页统一管理报文与路由参数。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "设备环境区分" }
                                "：设备列表清晰区分内网/测试等网关环境，顶栏在线状态按环境精准分流。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "客户端批量操作" }
                                "：中控与电池客户端均支持多设备管理，提供一键批量上线、下线与状态同步。"
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
