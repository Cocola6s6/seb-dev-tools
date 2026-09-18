mod commands;

use commands::{client, config as config_cmd, deploy, ecu as ecu_cmd, instance, send, terminal, AppState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg = seb_core::config::load();

    tauri::Builder::default()
        .manage(AppState::new(cfg))
        .invoke_handler(tauri::generate_handler![
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
            client::client_defaults,
            client::list_alarm_types,
            client::client_connect,
            client::client_disconnect,
            client::client_poll,
            client::client_set_profile,
            client::client_send_location,
            client::client_send_bms,
            client::client_send_alarm,
            client::client_send_ping,
            client::client_send_reply,
        ])
        .run(tauri::generate_context!())
        .expect("启动共享单车调试工具失败");
}
