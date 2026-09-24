use crate::api;
use gloo_timers::future::TimeoutFuture;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;

const CITY_PRESETS: &[(&str, f64, f64)] = &[
    ("南宁良庆区", 108.375256, 22.767133),
    ("汕头澄海区", 116.765101, 23.461051),
    ("北京海淀区", 116.298056, 39.959912),
];

#[component(inline_props)]
pub fn InlineMapPicker(
    container_id: &'static str,
    target_coord: Signal<String>,
) -> View {
    let custom_input = create_signal(String::new());
    let locating = create_signal(false);

    let initial_val = target_coord.get_clone();
    let initial_val = if initial_val.trim().is_empty() {
        "108.375256,22.767133".to_string()
    } else {
        initial_val
    };
    custom_input.set(initial_val.clone());

    spawn_local(async move {
        TimeoutFuture::new(50).await;
        let custom_input_init = custom_input;
        api::init_map_picker(container_id, &initial_val, move |picked| {
            target_coord.set(picked.clone());
            custom_input_init.set(picked);
        });
    });

    create_effect(move || {
        let coord_val = target_coord.get_clone();
        if !coord_val.trim().is_empty() {
            custom_input.set(coord_val.clone());
            if coord_val.contains(',') {
                let parts: Vec<&str> = coord_val.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(lng), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                        api::jump_map_coord(lng, lat);
                    }
                }
            }
        }
    });

    let jump_custom = move || {
        let input_val = custom_input.get_clone();
        if input_val.contains(',') {
            let parts: Vec<&str> = input_val.split(',').collect();
            if parts.len() == 2 {
                if let (Ok(lng), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                    let str_val = format!("{:.6},{:.6}", lng, lat);
                    api::jump_map_coord(lng, lat);
                    target_coord.set(str_val.clone());
                    custom_input.set(str_val);
                }
            }
        }
    };

    let locate_my_pos = move |_| {
        locating.set(true);
        let target_coord_clone = target_coord;
        let custom_input_clone = custom_input;
        let locating_clone = locating;

        api::locate_current_position(move |picked| {
            locating_clone.set(false);
            target_coord_clone.set(picked.clone());
            custom_input_clone.set(picked.clone());
            if picked.contains(',') {
                let parts: Vec<&str> = picked.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(lng), Ok(lat)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
                        api::jump_map_coord(lng, lat);
                    }
                }
            }
        });
    };

    view! {
        div(class="field inline-map-field") {
            label { "位置" }
            div(class="inline-map-card") {
                div(class="inline-map-head") {
                    div(class="inline-map-input-group") {
                        input(
                            r#type="text",
                            class="inline-map-input",
                            placeholder="经度,纬度 (回车或失焦跳转)",
                            bind:value=custom_input,
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                if ev.key() == "Enter" {
                                    jump_custom();
                                }
                            },
                            on:blur=move |_| jump_custom()
                        )
                    }
                    div(class="inline-map-toolbar") {
                        div(class="map-preset-bar") {
                            span(class="map-preset-title") { "快捷城市:" }
                            Indexed(
                                list=CITY_PRESETS.to_vec(),
                                view=move |(name, lng, lat): (&'static str, f64, f64)| {
                                    view! {
                                        button(class="map-preset-btn", on:click=move |_| {
                                            let str_val = format!("{:.6},{:.6}", lng, lat);
                                            api::jump_map_coord(lng, lat);
                                            target_coord.set(str_val.clone());
                                            custom_input.set(str_val);
                                        }) { (name) }
                                    }
                                }
                            )
                        }
                        button(
                            class="env-chip inner active",
                            title="点击获取当前物理位置或IP定位",
                            disabled=locating.get(),
                            on:click=locate_my_pos
                        ) {
                            (if locating.get() { "定位中..." } else { "当前定位" })
                        }
                    }
                }

                div(id=container_id, class="inline-map-container") {}
            }
        }
    }
}
