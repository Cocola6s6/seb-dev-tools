mod commands;
mod dock;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

use commands::{
    battery as battery_cmd, client, config as config_cmd, deploy, ecu as ecu_cmd, hosts, instance,
    send, terminal, window as window_cmd, AppState,
};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = seb_core::config::load();

    tauri::Builder::default()
        .manage(AppState::new(cfg))
        .setup(|app| {
            dock::setup(app.handle())?;
            #[cfg(target_os = "macos")]
            if let Some(main_win) = app.get_webview_window(dock::MAIN_LABEL) {
                macos::setup_traffic_lights(app.handle(), &main_win);
            }
            #[cfg(target_os = "windows")]
            if let Some(main_win) = app.get_webview_window(dock::MAIN_LABEL) {
                windows::setup_window_minimize(app.handle(), &main_win);
            }
            Ok(())
        })
        .on_window_event(dock::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::take_ui_logs,
            config_cmd::get_config,
            config_cmd::save_config,
            config_cmd::get_conn_state,
            instance::instance_lookup,
            deploy::load_deploy_options,
            deploy::bike_deploy,
            deploy::bike_lookup,
            ecu_cmd::list_ecu_params,
            ecu_cmd::list_control_types,
            send::send_borrow,
            send::send_control,
            send::send_voice,
            send::send_ecu_query,
            send::send_ecu_set,
            send::send_preset,
            terminal::open_terminal_log,
            terminal::open_external_url,
            hosts::switch_hosts,
            hosts::hosts_current,
            hosts::hosts_writable,
            hosts::set_hosts_free,
            client::client_defaults,
            client::list_alarm_types,
            client::client_devices,
            client::client_update_device,
            client::client_rename_device,
            client::client_remove_device,
            client::client_connect,
            client::client_disconnect,
            client::client_poll,
            client::client_send_location,
            client::client_send_bms,
            client::client_send_alarm,
            client::client_send_ping,
            client::client_send_reply,
            client::client_get_bike_nos,
            client::client_load_batteries,
            battery_cmd::battery_defaults,
            battery_cmd::battery_devices,
            battery_cmd::battery_update_device,
            battery_cmd::battery_rename_device,
            battery_cmd::battery_remove_device,
            battery_cmd::battery_connect,
            battery_cmd::battery_disconnect,
            battery_cmd::battery_poll,
            battery_cmd::battery_send_login,
            battery_cmd::battery_send_location,
            battery_cmd::battery_send_alarm,
            battery_cmd::battery_send_runtime,
            battery_cmd::battery_send_ping,
            battery_cmd::battery_send_logout,
            window_cmd::collapse_to_dock,
            window_cmd::show_main,
            window_cmd::dock_expand,
            window_cmd::dock_drag,
            window_cmd::hide_dock,
            window_cmd::dock_egg,
            window_cmd::dock_anchor,
            window_cmd::set_dock_qr_selection,
            window_cmd::get_dock_qr_selection,
        ])
        .build(tauri::generate_context!())
        .expect("启动共享单车调试工具失败")
        .run(|app, event| match event {
            // 工具条和主窗可能都藏着，这是最后一条回来的路
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => dock::show_main(app),
            _ => {
                let _ = app;
            }
        });
}
