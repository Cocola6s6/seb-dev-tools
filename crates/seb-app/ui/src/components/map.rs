use crate::api;
use gloo_timers::future::TimeoutFuture;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

const CITY_PRESETS: &[(&str, f64, f64)] = &[
    ("南宁良庆区", 108.38, 22.77),
    ("汕头澄海区", 116.765101, 23.461051),
    ("北京海淀区", 116.298056, 39.959912),
];

#[component(inline_props)]
pub fn MapPickerModal(
    open: Signal<bool>,
    target_coord: Signal<String>,
) -> View {
    let current_pick = create_signal(String::new());
    let custom_input = create_signal(String::new());

    create_effect(move || {
        if open.get() {
            let init_val = target_coord.get_clone();
            let init_val = if init_val.trim().is_empty() {
                "108.38,22.77".to_string()
            } else {
                init_val
            };
            current_pick.set(init_val.clone());
            custom_input.set(init_val.clone());

            spawn_local(async move {
                TimeoutFuture::new(60).await;
                api::init_map_picker("map-picker-container", &init_val, move |picked| {
                    current_pick.set(picked.clone());
                    custom_input.set(picked);
                });
            });
        }
    });

    let confirm = move |_| {
        let val = current_pick.get_clone();
        if !val.trim().is_empty() {
            target_coord.set(val);
        }
        open.set(false);
    };

    let cancel = move |_| {
        open.set(false);
    };

    let jump_custom = move |_| {
        let input_val = custom_input.get_clone();
        if input_val.contains(',') {
            let parts: Vec<&str> = input_val.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(lng), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    api::jump_map_coord(lng, lat);
                    current_pick.set(format!("{:.6},{:.6}", lng, lat));
                }
            }
        }
    };

    let locating = create_signal(false);

    let locate_my_pos = move |_| {
        locating.set(true);
        let current_pick_clone = current_pick;
        let custom_input_clone = custom_input;
        let locating_clone = locating;

        api::locate_current_position(move |picked| {
            locating_clone.set(false);
            current_pick_clone.set(picked.clone());
            custom_input_clone.set(picked);
        });
    };

    view! {
        (if open.get() {
            view! {
                div(class="modal-backdrop", on:click=move |ev: web_sys::MouseEvent| {
                    if let Some(target) = ev.target() {
                        if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                            if el.class_name().contains("modal-backdrop") {
                                open.set(false);
                            }
                        }
                    }
                }) {
                    div(class="modal-dialog") {
                        div(class="modal-head") {
                            div(class="modal-title") { "高德地图坐标拾取 (GCJ-02)" }
                            button(class="link", on:click=cancel, title="关闭") {
                                svg(viewBox="0 0 24 24", width="16", height="16", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                                    line(x1="18", y1="6", x2="6", y2="18") {}
                                    line(x1="6", y1="6", x2="18", y2="18") {}
                                }
                            }
                        }
                        div(class="modal-body") {
                            div(class="row", style="gap:10px;justify-content:space-between;align-items:center;") {
                                div(class="field-inline", style="gap:6px;flex:none;") {
                                    input(
                                        r#type="text",
                                        placeholder="经度,纬度 (如 108.38,22.77)",
                                        bind:value=custom_input,
                                        style="width:230px;min-width:210px;height:32px;font-family:monospace;font-size:12px;padding:4px 10px;"
                                    )
                                    button(on:click=jump_custom, style="height:32px;padding:4px 12px;font-size:12px;") { "定位" }
                                    button(
                                        on:click=locate_my_pos,
                                        disabled=locating.get(),
                                        style="height:32px;padding:4px 12px;font-size:12px;background:#f0fdf4;color:#15803d;border:1px solid #bbf7d0;font-weight:500;"
                                    ) {
                                        (if locating.get() { "定位中..." } else { "当前位置" })
                                    }
                                }
                                div(class="map-preset-bar") {
                                    span(style="font-size:11px;color:var(--muted);") { "快捷城市:" }
                                    Indexed(
                                        list=CITY_PRESETS.to_vec(),
                                        view=move |(name, lng, lat): (&'static str, f64, f64)| view! {
                                            button(class="map-preset-btn", on:click=move |_| {
                                                api::jump_map_coord(lng, lat);
                                                let str_val = format!("{:.6},{:.6}", lng, lat);
                                                current_pick.set(str_val.clone());
                                                custom_input.set(str_val);
                                            }) { (name) }
                                        }
                                    )
                                }
                            }
                            div(id="map-picker-container", class="modal-map-container") {}
                            div(class="row", style="font-size:12px;color:var(--muted);justify-content:space-between;") {
                                span { "点击地图任意位置或拖拽红点即可选取精确坐标。" }
                                span(style="font-family:monospace;color:var(--text);font-weight:600;") {
                                    (format!("当前选中: {}", current_pick.get_clone()))
                                }
                            }
                        }
                        div(class="modal-footer") {
                            button(on:click=cancel) { "取消" }
                            button(class="primary", on:click=confirm) { "确定使用该坐标" }
                        }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}
