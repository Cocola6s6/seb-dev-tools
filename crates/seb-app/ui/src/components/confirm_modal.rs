use crate::state::AppCtx;
use sycamore::prelude::*;

#[component]
pub fn ConfirmModal() -> View {
    let ctx = use_context::<AppCtx>();

    let on_cancel = move |_| {
        ctx.close_confirm();
    };

    let on_ok = move |_| {
        if let Some(st) = ctx.confirm_modal.get_clone() {
            ctx.close_confirm();
            (st.on_confirm)();
        }
    };

    view! {
        (move || if let Some(st) = ctx.confirm_modal.get_clone() {
            view! {
                div(class="confirm-modal-backdrop", on:click=on_cancel)
                div(class="confirm-modal-card") {
                    div(class="confirm-modal-head") {
                        div(class="confirm-modal-icon-wrap") {
                            svg(viewBox="0 0 24 24", width="18", height="18", fill="none", stroke="currentColor", stroke-width="2.2", stroke-linecap="round", stroke-linejoin="round") {
                                path(d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z") {}
                                line(x1="12", y1="9", x2="12", y2="13") {}
                                line(x1="12", y1="17", x2="12.01", y2="17") {}
                            }
                        }
                        span(class="confirm-modal-title") { (st.title) }
                    }
                    div(class="confirm-modal-body") {
                        p(class="confirm-modal-msg") { (st.message) }
                    }
                    div(class="confirm-modal-foot") {
                        button(class="confirm-modal-btn cancel", on:click=on_cancel) { "取消" }
                        button(class="confirm-modal-btn ok primary", on:click=on_ok) { "确认连接" }
                    }
                }
            }
        } else {
            view! {}
        })
    }
}
