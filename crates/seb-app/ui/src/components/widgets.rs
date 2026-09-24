use sycamore::prelude::*;
use wasm_bindgen::JsCast;

pub fn select_value(ev: web_sys::Event) -> String {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

#[component(inline_props)]
pub fn Check(label: &'static str, checked: Signal<bool>) -> View {
    view! {
        label(class="check") {
            input(r#type="checkbox", bind:checked=checked)
            span(class="box") {}
            span { (label) }
        }
    }
}

#[component(inline_props)]
pub fn Field(
    label: &'static str,
    value: Signal<String>,
    #[prop(default)] placeholder: &'static str,
) -> View {
    let ph = placeholder;
    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Tab" && !ev.shift_key() && value.get_clone().trim().is_empty() && !ph.is_empty() {
            let fill_val = if let Some(stripped) = ph.strip_prefix("如 ") {
                stripped
            } else {
                ph
            };
            value.set(fill_val.to_string());
        }
    };
    view! {
        div(class="field") {
            label { (label) }
            input(r#type="text", placeholder=placeholder, bind:value=value, on:keydown=on_keydown)
        }
    }
}

pub fn render_qr_svg(content: &str) -> Option<String> {
    if content.trim().is_empty() {
        return None;
    }
    let code = qrcode::QrCode::new(content.as_bytes()).ok()?;
    let svg_str = code
        .render::<qrcode::render::svg::Color>()
        .min_dimensions(300, 300)
        .quiet_zone(false)
        .dark_color(qrcode::render::svg::Color("#16181d"))
        .light_color(qrcode::render::svg::Color("#ffffff"))
        .build();
    Some(svg_str)
}

pub fn render_broken_qr_svg() -> String {
    r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" width="100%" height="100%">
  <rect width="200" height="200" fill="#f8fafc" rx="8" stroke="#e2e8f0" stroke-width="1"/>
  <rect x="22" y="22" width="38" height="38" rx="4" fill="#94a3b8"/>
  <rect x="28" y="28" width="26" height="26" rx="2" fill="#f8fafc"/>
  <rect x="34" y="34" width="14" height="14" rx="1" fill="#94a3b8"/>
  <rect x="140" y="22" width="38" height="38" rx="4" fill="#94a3b8"/>
  <rect x="146" y="28" width="26" height="26" rx="2" fill="#f8fafc"/>
  <rect x="152" y="34" width="14" height="14" rx="1" fill="#94a3b8"/>
  <rect x="22" y="140" width="38" height="38" rx="4" fill="#94a3b8"/>
  <rect x="28" y="146" width="26" height="26" rx="2" fill="#f8fafc"/>
  <rect x="34" y="152" width="14" height="14" rx="1" fill="#94a3b8"/>
  <circle cx="100" cy="100" r="24" fill="#fee2e2"/>
  <path d="M100 86 v18 M100 112 v2.5" stroke="#ef4444" stroke-width="3.5" stroke-linecap="round"/>
</svg>"##.to_string()
}

