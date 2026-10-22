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
                                span(class="whats-new-label") { "安装包产物精简" }
                                "：构建产物精简收敛，macOS 仅提供 DMG 安装包，Windows 仅提供 MSI 安装包。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "腾讯云 COS 自动更新" }
                                "：接入腾讯云 COS 增量更新与安全验签，支持后台静默检测、下载进度展示及重启生效。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "设置页手动检查" }
                                "：在系统设置中提供「版本与更新」卡片，随时可一键手动检查最新版本与说明。"
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
