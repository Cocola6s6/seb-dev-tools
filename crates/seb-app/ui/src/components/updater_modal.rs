use crate::api;
use crate::state::AppCtx;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn UpdaterModal() -> View {
    let ctx = use_context::<AppCtx>();

    let close = move |_| {
        if !ctx.update_downloading.get() {
            ctx.show_updater.set(false);
        }
    };

    let start_update = move |_| {
        ctx.update_downloading.set(true);
        ctx.update_status_text.set("正在从腾讯云 COS 下载更新包...".to_string());

        let ctx_clone = ctx;
        api::listen_update_progress(move |prog| {
            if let Some(p) = prog.percent {
                ctx_clone.update_percent.set(Some(p));
            }
            match prog.stage.as_str() {
                "downloading" => {
                    let mb = prog.downloaded_bytes as f64 / 1024.0 / 1024.0;
                    if let Some(total) = prog.total_bytes {
                        let total_mb = total as f64 / 1024.0 / 1024.0;
                        ctx_clone.update_status_text.set(format!(
                            "正在下载更新包: {:.1}MB / {:.1}MB ({:.0}%)",
                            mb,
                            total_mb,
                            prog.percent.unwrap_or(0.0)
                        ));
                    } else {
                        ctx_clone.update_status_text.set(format!("正在下载更新包: {:.1}MB", mb));
                    }
                }
                "installing" => {
                    ctx_clone.update_status_text.set("正在安装并替换新版本文件...".to_string());
                }
                "ready" => {
                    ctx_clone.update_downloading.set(false);
                    ctx_clone.update_ready_to_restart.set(true);
                    ctx_clone.update_status_text.set("🎉 更新安装完成！请重启应用生效。".to_string());
                }
                "error" => {
                    ctx_clone.update_downloading.set(false);
                    ctx_clone.update_status_text.set("更新失败，请稍后重试".to_string());
                }
                _ => {}
            }
        });

        spawn_local(async move {
            match api::download_and_install_update().await {
                Ok(_) => {
                    ctx.update_downloading.set(false);
                    ctx.update_ready_to_restart.set(true);
                    ctx.update_status_text.set("🎉 更新安装完成！请重启应用生效。".to_string());
                }
                Err(e) => {
                    ctx.update_downloading.set(false);
                    ctx.update_status_text.set(format!("更新失败: {e}"));
                    ctx.toast(format!("更新失败: {e}"));
                }
            }
        });
    };

    let restart = move |_| {
        spawn_local(async move {
            let _ = api::restart_app().await;
        });
    };

    let show_progress = move || {
        ctx.update_downloading.get()
            || ctx.update_ready_to_restart.get()
            || !ctx.update_status_text.get_clone().is_empty()
    };

    view! {
        (move || {
            if ctx.show_updater.get() {
                if let Some(info) = ctx.update_info.get_clone() {
                    let latest = create_signal(info.latest_version);
                    let current = create_signal(info.current_version);
                    let notes = create_signal(info.body.unwrap_or_else(|| "包含重要的体验优化和功能改进。".to_string()));

                    view! {
                        div(class="confirm-modal-backdrop", on:click=close)
                        div(class="updater-modal-card") {
                            div(class="updater-modal-head") {
                                div(class="updater-modal-title-group") {
                                    span(class="updater-badge") { "UPGRADE" }
                                    span(class="updater-modal-title") { (format!("发现新版本 v{}", latest.get_clone())) }
                                }
                                (if !ctx.update_downloading.get() {
                                    view! {
                                        button(
                                            class="whats-new-close",
                                            title="稍后提醒",
                                            on:click=close
                                        ) {
                                            svg(viewBox="0 0 24 24", width="13", height="13", fill="none", stroke="currentColor", stroke-width="2", stroke-linecap="round", stroke-linejoin="round") {
                                                path(d="M18 6L6 18M6 6l12 12") {}
                                            }
                                        }
                                    }
                                } else {
                                    view! {}
                                })
                            }

                            div(class="updater-modal-body") {
                                div(class="updater-version-row") {
                                    div(class="updater-v-chip") {
                                        span(class="updater-v-label") { "当前版本" }
                                        span(class="updater-v-val") { (format!("v{}", current.get_clone())) }
                                    }
                                    div(class="updater-arrow") { "➜" }
                                    div(class="updater-v-chip highlight") {
                                        span(class="updater-v-label") { "最新版本" }
                                        span(class="updater-v-val") { (format!("v{}", latest.get_clone())) }
                                    }
                                }

                                div(class="updater-notes-box") {
                                    div(class="updater-notes-title") { "更新内容：" }
                                    div(class="updater-notes-content") {
                                        (notes.get_clone())
                                    }
                                }

                                (if show_progress() {
                                    view! {
                                        div(class="updater-progress-section") {
                                            (if ctx.update_downloading.get() {
                                                view! {
                                                    div(class="updater-progress-bar-bg") {
                                                        div(
                                                            class="updater-progress-bar-fill",
                                                            style=format!("width: {:.1}%", ctx.update_percent.get().unwrap_or(0.0).min(100.0).max(0.0))
                                                        ) {}
                                                    }
                                                }
                                            } else {
                                                view! {}
                                            })
                                            div(class="updater-status-label") {
                                                (ctx.update_status_text.get_clone())
                                            }
                                        }
                                    }
                                } else {
                                    view! {}
                                })
                            }

                            div(class="updater-modal-foot") {
                                (if ctx.update_ready_to_restart.get() {
                                    view! {
                                        button(class="updater-btn-primary", on:click=restart) {
                                            "立即重启生效"
                                        }
                                    }
                                } else if ctx.update_downloading.get() {
                                    view! {
                                        button(class="updater-btn-disabled", disabled=true) {
                                            "正在下载更新..."
                                        }
                                    }
                                } else {
                                    view! {
                                        button(class="updater-btn-secondary", on:click=close) {
                                            "稍后提醒"
                                        }
                                        button(class="updater-btn-primary", on:click=start_update) {
                                            "立即升级"
                                        }
                                    }
                                })
                            }
                        }
                    }
                } else {
                    view! {}
                }
            } else {
                view! {}
            }
        })
    }
}
