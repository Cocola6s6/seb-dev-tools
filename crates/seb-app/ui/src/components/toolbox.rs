use super::widgets::render_qr_svg;
use crate::api;
use crate::state::{
    qr_url, AppCtx, ToolboxMode, TripleClickAction, DEFAULT_BATTERY_QR, DEFAULT_BIKE_QR,
};
use gloo_timers::future::TimeoutFuture;
use std::cell::Cell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

/// 敲蛋文案至少停留这么久，图片下载再快也不跳过
const EGG_KNOCK_MS: u32 = 2600;

/// 百宝箱三击敲鸡蛋。挂在根上渲染：顶栏 .nav 的 backdrop-filter 会困住固定定位的遮罩
#[component]
pub fn KnockEgg() -> View {
    let ctx = use_context::<AppCtx>();
    let ready = create_signal(false);
    let knocked = create_signal(false);
    let failed = create_signal(false);
    let picked = create_signal(String::new());

    create_effect(move || {
        if ctx.egg.get() {
            ready.set(false);
            knocked.set(false);
            failed.set(false);
            picked.set(pick_egg(&ctx.global_settings.get_clone().egg_url));
            spawn_local(async move {
                TimeoutFuture::new(EGG_KNOCK_MS).await;
                knocked.set(true);
            });
        }
    });

    // 当图片加载不出来，就把它当网页：蛋敲完直接丢给浏览器
    create_effect(move || {
        if knocked.get() && failed.get() {
            let url = picked.get_clone();
            ctx.egg.set(false);
            spawn_local(async move {
                if let Err(e) = api::open_external_url(&url).await {
                    ctx.log_error(format!("【错误】打开 {url} 失败: {e}"));
                }
            });
        }
    });

    let opened = create_memo(move || ready.get() && knocked.get());

    view! {
        (if ctx.egg.get() {
            view! {
                div(class="egg-overlay", on:click=move |_| ctx.egg.set(false)) {
                    img(
                        src=picked,
                        alt="",
                        class=move || if opened.get() { "egg-img" } else { "egg-img waiting" },
                        on:load=move |_| ready.set(true),
                        on:error=move |_| failed.set(true)
                    )
                    (if opened.get() {
                        view! {}
                    } else {
                        view! {
                            div(class="egg-knock") {
                                // 蛋和锤画在同一个 viewBox 里，省得用绝对定位对齐落点
                                svg(viewBox="0 0 40 32", width="150", height="120") {
                                    g(class="egg-shell knocking", fill="currentColor") {
                                        path(d="M12 9c3.9 0 6.8 5.1 6.8 10.2 0 4.8-3 8.8-6.8 8.8s-6.8-4-6.8-8.8C5.2 14.1 8.1 9 12 9z") {}
                                        path(class="egg-crack", d="M15.2 13.2l-1.5 1.6 1.7.8-1.4 1.5") {}
                                    }
                                    g(class="egg-hammer", stroke-linecap="round") {
                                        path(d="M34 27L18.6 10.1", stroke="#b98a5a", stroke-width="2.4") {}
                                        path(d="M16.1 11.15L19.9 7.65", stroke="#94a3b8", stroke-width="5.4") {}
                                    }
                                }
                                span { "正在给你敲鸡蛋" }
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

/// 鸡蛋框里一行一个地址，每次随机摸一个
fn pick_egg(box_text: &str) -> String {
    let list: Vec<&str> = box_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    match list.len() {
        0 => String::new(),
        n => list[(js_sys::Math::random() * n as f64) as usize % n].to_string(),
    }
}

#[component]
pub fn ToolboxNavButton() -> View {
    let ctx = use_context::<AppCtx>();
    let open = create_signal(false);
    let copied = create_signal(false);
    let click_count = Rc::new(Cell::new(0u32));

    let is_battery = move || ctx.page.get() == crate::state::Page::Battery;

    let display_title = move || {
        let mode = ctx.toolbox_mode.get();
        let now = match mode {
            ToolboxMode::QrCode if is_battery() => "电池二维码",
            ToolboxMode::QrCode => "车辆二维码",
            other => other.label(),
        };
        let triple_action = match ctx.global_settings.get_clone().triple_click {
            TripleClickAction::Egg => "三击敲鸡蛋",
            TripleClickAction::Hosts => "三击切换本地 host",
        };
        format!("{now} (双击切换为{}，{triple_action})", mode.next().label())
    };

    let display_no = move || {
        if is_battery() {
            let b = ctx.battery.selected.get_clone();
            if b.trim().is_empty() {
                "CMAH030799497009".to_string()
            } else {
                b
            }
        } else {
            let b = ctx.bike_no.get_clone().trim().to_string();
            if b.is_empty() {
                "A60004000180".to_string()
            } else {
                b
            }
        }
    };

    let qr_url = move || {
        let sets = ctx.global_settings.get_clone();
        if is_battery() {
            qr_url(&sets.battery.qr_url_template, DEFAULT_BATTERY_QR, "{battery_no}", &display_no())
        } else {
            qr_url(&sets.client.qr_url_template, DEFAULT_BIKE_QR, "{bike_no}", &display_no())
        }
    };

    let qr_data_url = create_memo(move || {
        let svg = render_qr_svg(&qr_url()).unwrap_or_default();
        format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
    });

    let copy_link = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        let url = qr_url();
        spawn_local(async move {
            let _ = api::copy_to_clipboard(&url).await;
            copied.set(true);
            ctx.toast("已复制二维码链接");
            TimeoutFuture::new(1500).await;
            copied.set(false);
        });
    };

    // 单击 / 双击 / 三击共用一个计数器：等连击窗口过去再决定做哪一件事
    let on_click = {
        let click_count = click_count.clone();
        move |_| {
            let count = click_count.get() + 1;
            click_count.set(count);
            let count_cell = click_count.clone();
            spawn_local(async move {
                TimeoutFuture::new(260).await;
                if count_cell.get() != count {
                    return;
                }
                count_cell.set(0);
                match count {
                    1 => match ctx.toolbox_mode.get() {
                        ToolboxMode::QrCode => {
                            if ctx.is_settings.get() {
                                ctx.is_settings.set(false);
                            } else {
                                open.set(!open.get());
                            }
                        }
                        _ => {
                            open.set(false);
                            ctx.is_settings.set(!ctx.is_settings.get());
                        }
                    },
                    2 => {
                        open.set(false);
                        let next = ctx.toolbox_mode.get().next();
                        ctx.toolbox_mode.set(next);
                        ctx.is_settings.set(next != ToolboxMode::QrCode);
                        ctx.toast(format!("百宝箱-切换{}", next.label()));
                    }
                    _ => {
                        open.set(false);
                        match ctx.global_settings.get_clone().triple_click {
                            TripleClickAction::Egg => {
                                if ctx.global_settings.get_clone().egg_url.trim().is_empty() {
                                    ctx.toast("鸡蛋框是空的");
                                } else {
                                    ctx.egg.set(true);
                                }
                            }
                            TripleClickAction::Hosts => {
                                ctx.toast("正在切换本地 hosts");
                                match api::switch_hosts().await {
                                    Ok(env) => {
                                        ctx.toast(format!("hosts 已切到{env}"));
                                        ctx.log_info(format!("本地 hosts 已切到{env}环境"));
                                    }
                                    Err(e) => {
                                        ctx.toast("hosts 切换失败");
                                        ctx.log_error(format!("【错误】切换 hosts 失败: {e}"));
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }
    };

    let btn_cls = move || {
        let is_active =
            (ctx.toolbox_mode.get() == ToolboxMode::QrCode && open.get()) || ctx.is_settings.get();
        if is_active {
            "nav-qr-icon-btn active"
        } else {
            "nav-qr-icon-btn"
        }
    };

    view! {
        div(class="nav-qr-container") {
            button(
                class=btn_cls,
                on:click=on_click
            ) {
                (move || match ctx.toolbox_mode.get() {
                    ToolboxMode::QrCode => view! {
                        svg(
                            viewBox="0 0 24 24",
                            width="21",
                            height="21",
                            fill="currentColor"
                        ) {
                            path(d="M3 3h8v8H3V3zm2 2v4h4V5H5zm8-2h8v8h-8V3zm2 2v4h4V5h-4zM3 13h8v8H3v-8zm2 2v4h4v-4H5zm13-2h3v2h-3v-2zm-5 0h3v3h-3v-3zm2 3h3v2h-3v-2zm3 0h3v5h-2v-3h-1v-2zm-5 2h2v3h-2v-3zm2 2h3v1h-3v-1zm-2-7h1v1h-1v-1zm6 4h1v1h-1v-1z") {}
                        }
                    },
                    ToolboxMode::Settings => view! {
                        svg(
                            viewBox="0 0 24 24",
                            width="20",
                            height="20",
                            fill="none",
                            stroke="currentColor",
                            stroke-width="2.1",
                            stroke-linecap="round",
                            stroke-linejoin="round"
                        ) {
                            path(d="M4 6h16M4 12h16M4 18h16") {}
                            circle(cx="9", cy="6", r="2", fill="var(--surface)") {}
                            circle(cx="15", cy="12", r="2", fill="var(--surface)") {}
                            circle(cx="8", cy="18", r="2", fill="var(--surface)") {}
                        }
                    },
                    ToolboxMode::System => view! {
                        svg(
                            viewBox="0 0 24 24",
                            width="20",
                            height="20",
                            fill="none",
                            stroke="currentColor",
                            stroke-width="2.1",
                            stroke-linecap="round",
                            stroke-linejoin="round"
                        ) {
                            circle(cx="12", cy="12", r="3") {}
                            path(d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z") {}
                        }
                    },
                })
            }
            // 弹层展开时二维码就在正下方，提示条得让位
            (if open.get() && ctx.toolbox_mode.get() == ToolboxMode::QrCode {
                view! {
                    div(class="popover-backdrop", on:click=move |_| open.set(false)) {}

                    div(class="nav-qr-popover") {
                        div(class="nav-qr-head") {
                            span { (if is_battery() { "电池二维码" } else { "车辆二维码" }) }
                        }
                        div(class="qr-svg-container") {
                            img(src=qr_data_url, alt="二维码", style="width:176px;height:176px;display:block;")
                        }
                        div(class="nav-qr-foot") {
                            span(class="qr-bike-no") { (display_no()) }
                            button(
                                class=move || if copied.get() { "qr-copy-btn copied" } else { "qr-copy-btn" },
                                title=move || if copied.get() { "已复制" } else { "复制链接" },
                                on:click=copy_link
                            ) {
                                (if copied.get() {
                                    view! {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="currentColor") {
                                            path(d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z") {}
                                        }
                                    }
                                } else {
                                    view! {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="currentColor") {
                                            path(d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z") {}
                                        }
                                    }
                                })
                            }
                        }
                    }
                }
            } else {
                view! { span(class="nav-qr-tip") { (display_title()) } }
            })
        }
    }
}
