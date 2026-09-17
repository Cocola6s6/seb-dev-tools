use crate::actions::run_send;
use crate::api;
use crate::components::{select_value, Check, DeviceBar};
use crate::state::{AppCtx, BorrowOptions, ControlType};
use sycamore::prelude::*;

const QUICK_ACTIONS: &[(&str, u16)] = &[
    ("开头盔", 0x3E),
    ("开尾箱", 0x55),
    ("断电", 0x1B),
    ("上电", 0x1C),
    ("开电池仓", 0x0F),
];

#[component]
pub fn ControlPage() -> View {
    let ctx = use_context::<AppCtx>();

    let open_helmet_lock = create_signal(false);
    let open_trunk_lock = create_signal(false);
    let helmet_taken = create_signal(false);
    let helmet_worn = create_signal(false);
    let trunk_lock_close = create_signal(false);

    let selected_control = create_signal(String::new());
    let voice_id = create_signal("1".to_string());

    create_effect(move || {
        let list = ctx.control_types.get_clone();
        if selected_control.get_clone().is_empty() {
            if let Some(first) = list.first() {
                selected_control.set(first.name.clone());
            }
        }
    });

    let borrow = move |_| {
        let options = BorrowOptions {
            open_helmet_lock: open_helmet_lock.get(),
            open_trunk_lock: open_trunk_lock.get(),
            helmet_taken: helmet_taken.get(),
            helmet_worn: helmet_worn.get(),
            trunk_lock_close: trunk_lock_close.get(),
        };
        run_send(ctx, api::send_borrow(options));
    };

    let send_selected_control = move |_| {
        let name = selected_control.get_clone();
        match ctx
            .control_types
            .get_clone()
            .into_iter()
            .find(|c| c.name == name)
        {
            Some(c) => run_send(ctx, async move { api::send_control(c.code, &c.name).await }),
            None => ctx.log_warn("【警告】请先选择控制命令"),
        }
    };

    let send_voice = move |_| match voice_id.get_clone().trim().parse::<i64>() {
        Ok(id) => run_send(ctx, api::send_voice(id)),
        Err(_) => ctx.log_warn("【警告】语音 ID 必须是整数"),
    };

    view! {
        div {
            div(class="page-head") {
                div(class="page-title") { "中控指令" }
                div(class="page-desc") {
                    "向中控网关投递开锁、还车、供电使能、头盔/尾箱锁控制及语音播报等中控指令。"
                }
            }

            DeviceBar {}

            div(class="section") {
                div(class="section-title") { "借车参数（安全骑行与硬件）" }
                div(class="row checks") {
                    Check(label="开头盔锁", checked=open_helmet_lock)
                    Check(label="开尾箱锁", checked=open_trunk_lock)
                    Check(label="取头盔上电", checked=helmet_taken)
                    Check(label="戴头盔上电", checked=helmet_worn)
                    Check(label="关尾箱上电", checked=trunk_lock_close)
                }
                div(class="row", style="margin-top:10px") {
                    button(class="primary", on:click=borrow) { "借车" }
                    button(on:click=move |_| run_send(ctx, api::send_control(0x01, "还车"))) { "还车" }
                }
                div(class="hint", style="margin-top:6px") {
                    "任一勾选即 safetyRiding = true；未勾选时下发传统借车流程。"
                }
            }

            div(class="section") {
                div(class="section-title") { "快捷操作" }
                div(class="row") {
                    Indexed(
                        list=QUICK_ACTIONS.to_vec(),
                        view=move |(label, code): (&'static str, u16)| view! {
                            button(on:click=move |_| run_send(ctx, api::send_control(code, label))) { (label) }
                        }
                    )
                }
            }

            div(class="section") {
                div(class="section-title") { "控制命令" }
                div(class="row") {
                    select(
                        class="w-lg",
                        on:change=move |ev| selected_control.set(select_value(ev))
                    ) {
                        Indexed(
                            list=ctx.control_types,
                            view=move |c: ControlType| {
                                let is_selected = selected_control.get_clone() == c.name;
                                let value = c.name.clone();
                                let text = format!("{}  ({})", c.name, c.hex);
                                view! {
                                    option(value=value, selected=is_selected) { (text) }
                                }
                            }
                        )
                    }
                    button(class="primary", on:click=send_selected_control) { "发送控制命令" }
                }
            }

            div(class="section") {
                div(class="section-title") { "语音播报" }
                div(class="row") {
                    input(r#type="number", min="0", max="255", bind:value=voice_id, class="w-sm")
                    button(class="primary", on:click=send_voice) { "发送语音" }
                }
            }
        }
    }
}
