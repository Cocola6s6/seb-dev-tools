use super::toolbox::pick_egg;
use super::widgets::render_qr_svg;
use crate::api;
use crate::state::{qr_url, AppCtx, DEFAULT_BATTERY_QR, DEFAULT_BIKE_QR};
use gloo_timers::future::TimeoutFuture;
use sycamore::prelude::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const ITEM_H: f64 = 66.0;
const STRIP_PAD: f64 = 10.0;
const DOCK_H: f64 = 306.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Card {
    None,
    Qr,
    Hosts,
    Egg,
    Main,
}

impl Card {
    fn height(self) -> f64 {
        match self {
            Card::Qr => 272.0,
            _ => 0.0,
        }
    }

    fn is_action(self) -> bool {
        matches!(self, Card::Hosts | Card::Egg | Card::Main)
    }
}

const ITEMS: [Card; 4] = [Card::Qr, Card::Hosts, Card::Egg, Card::Main];

#[component]
pub fn Dock() -> View {
    let ctx = AppCtx::new();
    provide_context(ctx);

    let active = create_signal(Card::None);
    let side = create_signal("right".to_string());
    let is_battery = create_signal(false);
    let copied = create_signal(false);
    let hosts_env = create_signal(String::new());
    let flashing = create_signal((Card::None, String::new()));

    spawn_local(async move {
        refresh(ctx, hosts_env).await;
    });

    let on_focus = Closure::<dyn Fn()>::new(move || {
        spawn_local(async move {
            refresh(ctx, hosts_env).await;
        });
    });
    if let Some(win) = web_sys::window() {
        let _ = win.add_event_listener_with_callback("focus", on_focus.as_ref().unchecked_ref());
    }
    on_focus.forget();

    let refresh_dock = Closure::<dyn Fn()>::new(move || {
        spawn_local(async move {
            refresh(ctx, hosts_env).await;
        });
    });
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &win,
            &wasm_bindgen::JsValue::from_str("__refreshDockQr"),
            refresh_dock.as_ref().unchecked_ref(),
        );
    }
    refresh_dock.forget();

    let open_card = move |card: Card| {
        if active.get_clone() == card {
            return;
        }
        spawn_local(async move {
            if let Ok(s) = api::dock_expand(true).await {
                side.set(s);
            }
            active.set(card);
        });
    };
    let close_card = move || {
        active.set(Card::None);
        spawn_local(async move {
            let _ = api::dock_expand(false).await;
        });
    };

    let flash = move |card: Card, text: &str| {
        flashing.set((card, text.to_string()));
        spawn_local(async move {
            TimeoutFuture::new(1800).await;
            if flashing.get_clone().0 == card {
                flashing.set((Card::None, String::new()));
            }
        });
    };
    let run_action = move |card: Card| {
        match card {
            Card::Hosts => {
                flash(Card::Hosts, "切换中");
                spawn_local(async move {
                    match api::switch_hosts().await {
                        Ok(env) => {
                            hosts_env.set(env);
                            flashing.set((Card::None, String::new()));
                        }
                        Err(e) => flash(Card::Hosts, if e.contains("取消") { "已取消" } else { "失败" }),
                    }
                });
            }
            Card::Egg => {
                if pick_egg(&ctx.global_settings.get_clone().egg_url).is_empty() {
                    flash(Card::Egg, "没配");
                    return;
                }
                spawn_local(async move {
                    if api::dock_egg(true).await.is_err() {
                        flash(Card::Egg, "失败");
                    }
                });
            }
            Card::Main => spawn_local(async move {
                let _ = api::show_main().await;
            }),
            _ => {}
        }
    };

    let press = create_signal(None::<(i32, i32)>);
    let dragged = create_signal(false);
    let on_down = move |e: web_sys::MouseEvent| {
        if e.button() == 0 {
            press.set(Some((e.screen_x(), e.screen_y())));
        }
    };
    let on_move = move |e: web_sys::MouseEvent| {
        let Some((x, y)) = press.get_clone() else {
            return;
        };
        if e.buttons() & 1 == 0 {
            press.set(None);
            return;
        }
        if (e.screen_x() - x).abs() + (e.screen_y() - y).abs() > 4 {
            press.set(None);
            dragged.set(true);
            close_card();
            spawn_local(async move {
                let _ = api::dock_drag().await;
            });
        }
    };
    let on_up = move |_: web_sys::MouseEvent| press.set(None);

    let root_class = move || format!("dock-root {}", side.get_clone());
    let card_top = move || {
        let card = active.get_clone();
        let idx = ITEMS.iter().position(|c| *c == card).unwrap_or(0) as f64;
        let center = STRIP_PAD + idx * ITEM_H + ITEM_H / 2.0;
        (center - 46.0).clamp(8.0, DOCK_H - card.height() - 8.0)
    };
    let card_style = move || {
        let card = active.get_clone();
        format!("top:{}px;height:{}px;", card_top(), card.height())
    };
    let arrow_style = move || {
        let idx = ITEMS.iter().position(|c| *c == active.get_clone()).unwrap_or(0) as f64;
        let center = STRIP_PAD + idx * ITEM_H + ITEM_H / 2.0;
        format!("top:{}px;", center - card_top() - 7.0)
    };

    view! {
        div(
            class=root_class,
            on:mouseleave=move |_| press.set(None),
            on:mousedown=on_down,
            on:mousemove=on_move,
            on:mouseup=on_up
        ) {
            div(class="dock-strip") {
                (View::from(ITEMS.into_iter().map(move |card| {
                    let item_class = move || {
                        if active.get_clone() == card { "dock-item active" } else { "dock-item" }
                    };
                    view! {
                        div(
                            class=item_class,
                            title=action_tip(card),
                            on:click=move |_| {
                                if dragged.get() {
                                    dragged.set(false);
                                    return;
                                }
                                if card.is_action() {
                                    run_action(card);
                                } else if active.get_clone() == card {
                                    close_card();
                                } else {
                                    open_card(card);
                                }
                            }
                        ) {
                            div(class="dock-icon") { (icon_svg(card)) }
                            span(class="dock-label") { (item_label(card, hosts_env, flashing)) }
                        }
                    }
                }).collect::<Vec<View>>()))
                div(class="dock-close", title="关闭工具条（点程序坞图标回来）",
                    on:click=move |_| {
                        if dragged.get() { dragged.set(false); return; }
                        spawn_local(async move { let _ = api::hide_dock().await; });
                    }) {
                    svg(viewBox="0 0 16 16", width="11", height="11") {
                        path(d="M4 4l8 8M12 4l-8 8", fill="none", stroke="currentColor",
                             stroke-width="1.6", stroke-linecap="round")
                    }
                }
            }

            (if active.get_clone() == Card::None {
                view! {}
            } else {
                view! {
                    div(class="dock-card", style=card_style) {
                        span(class="dock-arrow", style=arrow_style) {}
                        (qr_card(ctx, is_battery, copied))
                    }
                }
            })
        }
    }
}

fn action_tip(card: Card) -> &'static str {
    match card {
        Card::Hosts => "点一下轮换本地 hosts：内网 → 外网 → 正式",
        Card::Egg => "敲个鸡蛋",
        Card::Main => "打开主窗口",
        _ => "",
    }
}

fn item_label(card: Card, hosts_env: Signal<String>, flashing: Signal<(Card, String)>) -> String {
    let (flash_card, text) = flashing.get_clone();
    if flash_card == card {
        return text;
    }
    match card {
        Card::Qr => "扫码".to_string(),
        Card::Hosts => {
            let env = hosts_env.get_clone();
            if env.is_empty() { "hosts".to_string() } else { env }
        }
        Card::Egg => "鸡蛋".to_string(),
        Card::Main => "主窗".to_string(),
        Card::None => String::new(),
    }
}

fn stroke_icon(d: &'static str) -> View {
    view! {
        svg(viewBox="0 0 16 16", width="18", height="18") {
            path(d=d, fill="none", stroke="currentColor", stroke-width="1.3",
                 stroke-linecap="round", stroke-linejoin="round")
        }
    }
}

fn icon_svg(card: Card) -> View {
    match card {
        Card::Qr => stroke_icon("M2 2h4.2v4.2H2zM9.8 2H14v4.2H9.8zM2 9.8h4.2V14H2zM9.8 9.8h1.6v1.6H9.8zM12.4 9.8H14v1.6h-1.6zM9.8 12.4h1.6V14H9.8zM12.4 12.4H14V14h-1.6z"),
        Card::Hosts => stroke_icon("M8 1.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM1.5 8h13M8 1.5c1.8 2 2.7 4.1 2.7 6.5S9.8 12.5 8 14.5c-1.8-2-2.7-4.1-2.7-6.5S6.2 3.5 8 1.5z"),
        Card::Egg => stroke_icon("M8 1.6c2.5 0 4.6 3.6 4.6 6.6a4.6 4.6 0 0 1-9.2 0c0-3 2.1-6.6 4.6-6.6z"),
        Card::Main => stroke_icon("M2 3h12v10H2zM2 6h12M4.2 4.5h.01M6.2 4.5h.01"),
        Card::None => view! {},
    }
}

fn qr_card(ctx: AppCtx, is_battery: Signal<bool>, copied: Signal<bool>) -> View {
    let display_no = move || {
        if is_battery.get() {
            let b = ctx.battery_no.get_clone().trim().to_string();
            if b.is_empty() { "CMAH030799497009".to_string() } else { b }
        } else {
            let b = ctx.bike_no.get_clone().trim().to_string();
            if b.is_empty() { "A60004000180".to_string() } else { b }
        }
    };
    let link = move || {
        let sets = ctx.global_settings.get_clone();
        if is_battery.get() {
            qr_url(&sets.battery.qr_url_template, DEFAULT_BATTERY_QR, "{battery_no}", &display_no())
        } else {
            qr_url(&sets.client.qr_url_template, DEFAULT_BIKE_QR, "{bike_no}", &display_no())
        }
    };
    let qr_data_url = create_memo(move || {
        let svg = render_qr_svg(&link()).unwrap_or_default();
        format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
    });
    let copy_link = move |_| {
        let url = link();
        spawn_local(async move {
            let _ = api::copy_to_clipboard(&url).await;
            copied.set(true);
            TimeoutFuture::new(1500).await;
            copied.set(false);
        });
    };

    view! {
        div(class="dock-tabs") {
            button(
                class=move || if is_battery.get() { "dock-tab" } else { "dock-tab active" },
                on:click=move |_| is_battery.set(false)
            ) { "车辆" }
            button(
                class=move || if is_battery.get() { "dock-tab active" } else { "dock-tab" },
                on:click=move |_| is_battery.set(true)
            ) { "电池" }
        }
        div(class="dock-qr") {
            img(src=qr_data_url, alt="二维码", style="width:164px;height:164px;display:block;")
        }
        div(class="dock-no") {
            span { (display_no()) }
            button(class="dock-copy", title="复制链接", on:click=copy_link) {
                (if copied.get() { "已复制" } else { "复制" })
            }
        }
    }
}

async fn refresh(ctx: AppCtx, hosts_env: Signal<String>) {
    if let Ok(cfg) = api::get_config().await {
        if ctx.cfg.get_clone() != cfg {
            ctx.adopt_config(cfg);
        }
        if ctx.show_whats_new.get() {
            ctx.show_whats_new.set(false);
        }
    }
    if let Ok(selection) = api::get_dock_qr_selection().await {
        ctx.bike_no.set(selection.bike_no);
        ctx.battery_no.set(selection.battery_no);
    }
    if let Ok(env) = api::hosts_current().await {
        if hosts_env.get_clone() != env {
            hosts_env.set(env);
        }
    }
}
