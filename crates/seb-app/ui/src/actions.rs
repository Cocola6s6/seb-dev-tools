use crate::api;
use crate::state::{AppCtx, SendResult};
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

pub fn run_send<F>(ctx: AppCtx, fut: F)
where
    F: Future<Output = Result<SendResult, String>> + 'static,
{
    spawn_local(async move {
        if let Err(e) = api::save_config(&ctx.current_config()).await {
            ctx.log_error(format!("【错误】保存配置失败: {e}"));
            return;
        }
        ctx.apply_result(fut.await);
    });
}
