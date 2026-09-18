use crate::api;
use crate::state::{AppCtx, LogLevel, SendResult};
use std::future::Future;
use wasm_bindgen_futures::spawn_local;

pub fn run_client_named<F>(ctx: AppCtx, name: &'static str, fut: F)
where
    F: Future<Output = Result<(), String>> + 'static,
{
    spawn_local(async move {
        match fut.await {
            Ok(()) => {
                ctx.log_client(format!("【已触发】{name}"), LogLevel::Info);
            }
            Err(e) => {
                ctx.log_client(format!("【错误】{name}失败: {e}"), LogLevel::Error);
            }
        }
    });
}

pub fn run_battery_named<F>(ctx: AppCtx, name: &'static str, fut: F)
where
    F: Future<Output = Result<(), String>> + 'static,
{
    spawn_local(async move {
        match fut.await {
            Ok(()) => {
                ctx.log_battery(format!("【已触发】{name}"), LogLevel::Info);
            }
            Err(e) => {
                ctx.log_battery(format!("【错误】{name}失败: {e}"), LogLevel::Error);
            }
        }
    });
}

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
