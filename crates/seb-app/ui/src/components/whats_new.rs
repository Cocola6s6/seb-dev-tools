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
                                span(class="whats-new-label") { "修改设备号" }
                                "：左侧设备可以点小图标直接改序列号，改完敲回车就能保存。"
                            }
                        }
                        div(class="whats-new-item") {
                            span(class="whats-new-bullet") {}
                            div(class="whats-new-text") {
                                span(class="whats-new-label") { "高德地图选点" }
                                "：自带高德地图，可以在地图上直接戳位置选点，也支持输入经纬度和切换快捷城市。"
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
