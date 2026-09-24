use super::widgets::{render_broken_qr_svg, render_qr_svg};
use crate::api;
use crate::state::{
    elide_middle, host_env_tag, normalize_ecu_no, AppCtx, Page,
};

/// 设备列宽度有限，编号超过这个长度就中间省略，完整值留在 title 里
const NO_MAX: usize = 18;
use gloo_timers::future::TimeoutFuture;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, PartialEq)]
pub struct DeviceRow {
    pub no: String,
    pub host: String,
    pub connected: bool,
    pub coordinates: String,
    /// 二维码卡片底部文字与链接内容，None 表示这台设备暂时没有二维码
    pub qr_label: Option<String>,
    pub qr_url: Option<String>,
}

/// 设备清单与二维码墙的数据源：中控、电池各提供一份，页面之间只差这些取值与动作
#[derive(Clone)]
pub struct DeviceSource {
    pub page: Page,
    /// 「设备」「电池」，用于新增占位符与复制提示
    pub noun: &'static str,
    /// 二维码复制提示里的称呼，如「车辆」「电池」
    pub qr_noun: &'static str,
    pub rows: Rc<dyn Fn() -> Vec<DeviceRow>>,
    pub selected: Signal<String>,
    pub is_selected: Rc<dyn Fn(&str) -> bool>,
    pub selected_list: Rc<dyn Fn() -> Vec<String>>,
    pub select: Rc<dyn Fn(&str)>,
    pub toggle_select: Rc<dyn Fn(&str)>,
    pub range_select: Rc<dyn Fn(&str)>,
    pub add: Rc<dyn Fn(String)>,
    pub remove: Rc<dyn Fn(String)>,
    pub rename: Option<Rc<dyn Fn(String, String)>>,
    pub toggle_connect: Rc<dyn Fn(String)>,
}

impl DeviceSource {
    fn click_select(&self, no: &str, ev: &web_sys::MouseEvent) {
        if ev.meta_key() || ev.ctrl_key() {
            (self.toggle_select)(no);
        } else if ev.shift_key() {
            (self.range_select)(no);
        } else {
            (self.select)(no);
        }
    }
}

const ICON_COPY: &str = "M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z";
const ICON_CHECK: &str = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z";

/// 二维码墙：一台设备一张卡，单击选中、双击连接/断开
pub fn qr_panel(src: DeviceSource) -> View {
    let ctx = use_context::<AppCtx>();
    let rows = src.rows.clone();
    let items = create_memo(move || rows());

    view! {
        div(class="client-qr-panel") {
            div(class="client-qr-grid") {
                Indexed(
                    list=items,
                    view=move |row: DeviceRow| {
                        let src = src.clone();
                        let no = row.no.clone();
                        let connected = row.connected;
                        let (env_label, env) = host_env_tag(&ctx.global_settings.get_clone(), &row.host);
                        let label = row.qr_label.clone().unwrap_or_default();
                        let url = row.qr_url.clone().unwrap_or_default();
                        let is_damaged = url.trim().is_empty();
                        let shown_label = if is_damaged {
                            format!("{} (未查到车辆编号)", elide_middle(&no, 8))
                        } else {
                            elide_middle(&label, NO_MAX)
                        };
                        let tooltip_title = if is_damaged {
                            format!("设备序列号: {no}\n提示: 网关/数据库未查询到该 ECU 关联的车辆编号\n单击选中，双击{}", if connected { "断开连接" } else { "连接并登录" })
                        } else {
                            format!("车辆编号: {label}\n设备序列号: {no}\n单击选中，双击{}", if connected { "断开连接" } else { "连接并登录" })
                        };
                        let cls = {
                            let src = src.clone();
                            let no = no.clone();
                            move || {
                                let mut res = String::from("client-qr-card");
                                res.push_str(if connected { " online" } else { " offline" });
                                if is_damaged {
                                    res.push_str(" damaged");
                                }
                                if (src.is_selected)(&no) {
                                    res.push_str(" active");
                                }
                                res
                            }
                        };
                        let qr_data_url = {
                            let svg = if is_damaged {
                                render_broken_qr_svg()
                            } else {
                                render_qr_svg(&url).unwrap_or_else(render_broken_qr_svg)
                            };
                            format!("data:image/svg+xml;utf8,{}", js_sys::encode_uri_component(&svg))
                        };
                        let copied = create_signal(false);
                        let copy_link = {
                            let noun = src.qr_noun;
                            let label = label.clone();
                            let no = no.clone();
                            let url = url.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                let url = url.clone();
                                let label = label.clone();
                                let no = no.clone();
                                spawn_local(async move {
                                    if is_damaged {
                                        let _ = api::copy_to_clipboard(&no).await;
                                        copied.set(true);
                                        ctx.toast(format!("未查到车辆编号，已复制设备序列号 {no}"));
                                    } else {
                                        let _ = api::copy_to_clipboard(&url).await;
                                        copied.set(true);
                                        ctx.toast(format!("已复制{noun} {label} 二维码链接"));
                                    }
                                    TimeoutFuture::new(1500).await;
                                    copied.set(false);
                                });
                            }
                        };
                        let pick = {
                            let src = src.clone();
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| src.click_select(&no, &ev)
                        };
                        let dblpick = {
                            let src = src.clone();
                            let no = no.clone();
                            move |_| (src.toggle_connect)(no.clone())
                        };
                        view! {
                            div(
                                class=cls,
                                title=tooltip_title,
                                on:click=pick,
                                on:dblclick=dblpick
                            ) {
                                span(class=format!("qr-env-badge badge-env-{env}")) { (env_label) }
                                div(class="qr-svg-container") {
                                    img(src=qr_data_url, alt="二维码", style="width:100%;height:100%;display:block;")
                                }
                                div(class="nav-qr-foot") {
                                    span(class="qr-bike-no", title=if is_damaged { format!("设备 {no} (未查到关联车辆编号)") } else { label.clone() }) { (shown_label) }
                                    button(
                                        class=move || if copied.get() { "qr-copy-btn copied" } else { "qr-copy-btn" },
                                        title=move || if copied.get() { "已复制" } else if is_damaged { "复制设备序列号" } else { "复制链接" },
                                        on:click=copy_link
                                    ) {
                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="currentColor") {
                                            path(d=move || if copied.get() { ICON_CHECK } else { ICON_COPY }) {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                )
            }
        }
    }
}

/// 左侧设备清单：新增、移除、多选，选中后 Cmd+C 复制序列号
pub fn device_list(src: DeviceSource) -> View {
    let root_ctx = use_context::<AppCtx>();
    let adding = create_signal(false);
    let new_no = create_signal(String::new());

    let confirm_add: Rc<dyn Fn()> = {
        let src = src.clone();
        Rc::new(move || {
            let no = new_no.get_clone().trim().to_string();
            adding.set(false);
            new_no.set(String::new());
            if !no.is_empty() {
                (src.add)(no);
            }
        })
    };

    let head_text = {
        let src = src.clone();
        create_memo(move || {
            let total = (src.rows)().len();
            let selected_cnt = (src.selected_list)().len();
            if selected_cnt > 1 {
                format!("设备 (已选 {}/{})", selected_cnt, total)
            } else {
                format!("设备 ({})", total)
            }
        })
    };

    let copied = create_signal(false);
    let copy_selected = {
        let src = src.clone();
        let ctx = use_context::<AppCtx>();
        move || {
            let noun = src.noun;
            let list = (src.selected_list)();
            let text = if list.is_empty() {
                let sel = src.selected.get_clone();
                if sel.trim().is_empty() {
                    return;
                }
                sel
            } else {
                list.join("\n")
            };
            let count = if list.is_empty() { 1 } else { list.len() };
            spawn_local(async move {
                let _ = api::copy_to_clipboard(&text).await;
                copied.set(true);
                if count == 1 {
                    ctx.toast(format!("已复制{noun}序列号：{text}"));
                } else {
                    ctx.toast(format!("已复制 {count} {noun}序列号到剪贴板"));
                }
                TimeoutFuture::new(1000).await;
                copied.set(false);
            });
        }
    };

    let on_list_keydown = {
        let copy_selected = copy_selected.clone();
        move |ev: web_sys::KeyboardEvent| {
            if (ev.meta_key() || ev.ctrl_key()) && (ev.key() == "c" || ev.key() == "C") && !api::has_active_selection() {
                ev.prevent_default();
                copy_selected();
            }
        }
    };

    {
        let ctx = use_context::<AppCtx>();
        let page = src.page;
        let copy_selected = copy_selected.clone();
        let cb = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::wrap(Box::new(move |ev: web_sys::KeyboardEvent| {
            if (ev.meta_key() || ev.ctrl_key()) && (ev.key() == "c" || ev.key() == "C") {
                if api::has_active_selection() {
                    return;
                }
                if let Some(target) = ev.target() {
                    if let Ok(el) = target.dyn_into::<web_sys::Element>() {
                        let tag = el.tag_name().to_lowercase();
                        if tag == "input" || tag == "textarea" || el.closest(".log-pane").ok().flatten().is_some() {
                            return;
                        }
                    }
                }
                if ctx.page.get() == page {
                    ev.prevent_default();
                    copy_selected();
                }
            }
        }));
        if let Some(w) = web_sys::window() {
            let _ = w.add_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            // 换页会把这份设备列表拆掉，监听留着就是对已销毁的作用域取信号
            on_cleanup(move || {
                let _ = w.remove_event_listener_with_callback("keydown", cb.as_ref().unchecked_ref());
            });
        }
    }

    let noun = src.noun;
    let rows = src.rows.clone();
    let items = create_memo(move || rows());

    let editing_item = create_signal(Option::<String>::None);
    let editing_val = create_signal(String::new());

    view! {
        div(class="device-list", tabindex="0", on:keydown=on_list_keydown) {
            div(class="device-list-head") {
                span { (head_text.get_clone()) }
                button(class="icon-btn ok", title="新增设备", on:click=move |_| {
                    adding.set(true);
                    new_no.set(String::new());
                    spawn_local(async {
                        TimeoutFuture::new(20).await;
                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                            if let Ok(Some(el)) = doc.query_selector(".device-list-head + .device-add input") {
                                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                    let _ = input.focus();
                                }
                            }
                        }
                    });
                }) {
                    svg(viewBox="0 0 24 24", width="15", height="15", fill="none", stroke="currentColor", stroke-width="2.4", stroke-linecap="round", stroke-linejoin="round") {
                        line(x1="12", y1="5", x2="12", y2="19") {}
                        line(x1="5", y1="12", x2="19", y2="12") {}
                    }
                }
            }

            (move || if adding.get() {
                let confirm_add = confirm_add.clone();
                let on_blur = confirm_add.clone();
                view! {
                    div(class="device-add") {
                        input(
                            r#type="text",
                            placeholder=format!("{noun}序列号，回车确认"),
                            bind:value=new_no,
                            on:blur=move |_| on_blur(),
                            on:keydown=move |ev: web_sys::KeyboardEvent| {
                                match ev.key().as_str() {
                                    "Enter" => confirm_add(),
                                    "Escape" => { adding.set(false); new_no.set(String::new()); }
                                    _ => {}
                                }
                            }
                        )
                    }
                }
            } else {
                view! {}
            })

            div(class="device-items") {
                Indexed(
                    list=items,
                    view=move |row: DeviceRow| {
                        let src = src.clone();
                        let no = row.no.clone();
                        let connected = row.connected;
                        let (env_label, env) = host_env_tag(&root_ctx.global_settings.get_clone(), &row.host);
                        let pick = {
                            let src = src.clone();
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| src.click_select(&no, &ev)
                        };
                        let dblpick = {
                            let src = src.clone();
                            let no = no.clone();
                            move |_| (src.toggle_connect)(no.clone())
                        };
                        let remove = {
                            let src = src.clone();
                            let no = no.clone();
                            move |ev: web_sys::MouseEvent| {
                                ev.stop_propagation();
                                (src.remove)(no.clone());
                            }
                        };
                        let cls = {
                            let src = src.clone();
                            let no = no.clone();
                            move || {
                                let mut res = format!("device-item env-{env}");
                                if (src.is_selected)(&no) {
                                    res.push_str(" active");
                                    if copied.get() {
                                        res.push_str(" copied-flash");
                                    }
                                }
                                res
                            }
                        };
                        let shown_no = elide_middle(&no, NO_MAX);
                        let sub = if !connected {
                            format!("{env_label} · 未连接")
                        } else if row.coordinates.is_empty() {
                            format!("{env_label} · 无坐标")
                        } else {
                            format!("{env_label} · {}", row.coordinates)
                        };

                        let is_editing = {
                            let no = no.clone();
                            move || editing_item.get_clone() == Some(no.clone())
                        };

                        let finish_edit = {
                            let src = src.clone();
                            let old_no = no.clone();
                            move || {
                                let mut val = editing_val.get_clone().trim().to_string();
                                if src.page == Page::Client {
                                    val = normalize_ecu_no(&val);
                                }
                                editing_item.set(None);
                                if !val.is_empty() && val != old_no {
                                    if let Some(rename_fn) = &src.rename {
                                        rename_fn(old_no.clone(), val);
                                    }
                                }
                            }
                        };

                        let cancel_edit = move || {
                            editing_item.set(None);
                        };

                        view! {
                            div(
                                class=cls,
                                title=if connected { "单击切换当前配置，双击断开连接 (按住 Cmd/Shift 可多选)" } else { "单击切换当前配置，双击连接并登录 (按住 Cmd/Shift 可多选)" },
                                on:click=pick,
                                on:dblclick=dblpick
                            ) {
                                (move || {
                                    if is_editing() {
                                        let on_blur = finish_edit.clone();
                                        let on_keydown = {
                                            let finish_edit = finish_edit.clone();
                                            move |ev: web_sys::KeyboardEvent| {
                                                match ev.key().as_str() {
                                                    "Enter" => finish_edit(),
                                                    "Escape" => cancel_edit(),
                                                    _ => {}
                                                }
                                            }
                                        };
                                        view! {
                                            div(class="device-add", on:click=move |ev: web_sys::MouseEvent| ev.stop_propagation()) {
                                                input(
                                                    r#type="text",
                                                    placeholder=format!("{noun}序列号，回车保存"),
                                                    bind:value=editing_val,
                                                    on:blur=move |_| on_blur(),
                                                    on:keydown=on_keydown
                                                )
                                            }
                                        }
                                    } else {
                                        let edit_no = no.clone();
                                        let can_rename = src.rename.is_some();
                                        let remove = remove.clone();
                                        let title_no = no.clone();
                                        let shown = shown_no.clone();
                                        let sub = sub.clone();
                                        view! {
                                            div(class="device-item-top") {
                                                span(
                                                    class=if connected { "dot on" } else { "dot off" },
                                                    title=format!("{env_label} · {}", if connected { "已连接" } else { "未连接" })
                                                ) {}
                                                span(class="device-no", title=title_no) { (shown) }
                                                div(class="device-actions") {
                                                    (if can_rename {
                                                        let edit_no = edit_no.clone();
                                                        view! {
                                                            button(
                                                                class="device-btn edit",
                                                                title="编辑序列号",
                                                                on:click=move |ev: web_sys::MouseEvent| {
                                                                    ev.stop_propagation();
                                                                    editing_val.set(edit_no.clone());
                                                                    editing_item.set(Some(edit_no.clone()));
                                                                    spawn_local(async {
                                                                        TimeoutFuture::new(20).await;
                                                                        if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
                                                                            if let Ok(Some(el)) = doc.query_selector(".device-item .device-add input") {
                                                                                if let Ok(input) = el.dyn_into::<web_sys::HtmlInputElement>() {
                                                                                    let _ = input.focus();
                                                                                    input.select();
                                                                                }
                                                                            }
                                                                        }
                                                                    });
                                                                }
                                                            ) {
                                                                svg(viewBox="0 0 24 24", width="12", height="12", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                                                                    path(d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7") {}
                                                                    path(d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z") {}
                                                                }
                                                            }
                                                        }
                                                    } else {
                                                        view! {}
                                                    })
                                                    button(class="device-btn del", title="移除", on:click=remove) {
                                                        svg(viewBox="0 0 24 24", width="12", height="12", fill="none", stroke="currentColor", stroke-width="2.4", stroke-linecap="round", stroke-linejoin="round") {
                                                            line(x1="18", y1="6", x2="6", y2="18") {}
                                                            line(x1="6", y1="6", x2="18", y2="18") {}
                                                        }
                                                    }
                                                }
                                            }
                                            div(class="device-sub") { (sub) }
                                        }
                                    }
                                })
                            }
                        }
                    }
                )
            }
        }
    }
}
