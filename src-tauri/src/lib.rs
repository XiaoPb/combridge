pub mod commands;
pub mod device;
pub mod error;
pub mod protocol;
pub mod service;
pub mod state;
pub mod websocket;

use std::sync::Arc;

use device::{BleManager, SerialManager};
use protocol::PluginManager;
use service::LoggerService;
use state::{create_action_dispatcher, create_app_state, create_state_persistence};
use tracing::info;
use websocket::ConnectionPool;

fn init_logger() {
    let _guard = LoggerService::init_default();
    std::mem::forget(_guard);
    info!("ComBridge Rust 应用启动");
}

fn get_app_data_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::data_dir())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("combridge")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logger();

    let serial_manager = Arc::new(SerialManager::new());
    let ble_manager = Arc::new(BleManager::new());
    let connection_pool = Arc::new(ConnectionPool::new());
    let plugin_manager = Arc::new(PluginManager::new());

    let app_state = create_app_state();
    let app_data_dir = get_app_data_dir();
    let state_persistence = create_state_persistence(app_data_dir);
    let action_dispatcher = create_action_dispatcher(
        app_state.clone(),
        state_persistence.clone(),
        serial_manager.clone(),
        ble_manager.clone(),
    );

    info!("服务初始化完成");

    let ble_manager_clone = ble_manager.clone();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(serial_manager)
        .manage(ble_manager)
        .manage(connection_pool)
        .manage(plugin_manager)
        .manage(app_state)
        .manage(state_persistence)
        .manage(action_dispatcher)
        .setup(move |_app| {
            let ble_manager = ble_manager_clone.clone();
            tokio::spawn(async move {
                if let Err(e) = ble_manager.initialize().await {
                    tracing::error!("BLE 初始化失败: {}", e);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::serial::scan_serial_ports,
            commands::serial::open_serial_port,
            commands::serial::close_serial_port,
            commands::serial::send_serial_data,
            commands::serial::get_open_ports,
            commands::serial::is_port_open,
            commands::serial::export_serial_data,
            commands::ble::configure_ble,
            commands::ble::scan_ble_devices,
            commands::ble::stop_ble_scan,
            commands::ble::connect_ble,
            commands::ble::disconnect_ble,
            commands::ble::get_ble_connections,
            commands::ble::discover_ble_services,
            commands::ble::discover_ble_characteristics,
            commands::ble::read_ble_characteristic,
            commands::ble::write_ble_characteristic,
            commands::ble::write_ble_without_response,
            commands::ble::subscribe_ble_notify,
            commands::ble::unsubscribe_ble_notify,
            commands::ble::get_ble_rssi,
            commands::ble::get_ble_mode,
            commands::ble::is_ble_configured,
            commands::ble::set_ble_mtu,
            commands::ble::get_ble_subscriptions,
            commands::websocket::connect_websocket,
            commands::websocket::send_websocket_message,
            commands::websocket::disconnect_websocket,
            commands::websocket::get_websocket_status,
            commands::websocket::get_all_websocket_connections,
            commands::websocket::get_all_websocket_status,
            commands::protocol::load_protocol,
            commands::protocol::unload_protocol,
            commands::protocol::enable_protocol,
            commands::protocol::disable_protocol,
            commands::protocol::bind_protocol,
            commands::protocol::unbind_protocol,
            commands::protocol::list_protocols,
            commands::protocol::get_protocol,
            commands::protocol::get_bound_protocols,
            commands::system::get_system_info,
            commands::system::get_system_status,
            commands::system::configure_log,
            commands::system::get_log_config,
            commands::system::get_runtime_status,
            commands::system::get_app_version,
            commands::system::get_platform,
            commands::system::open_url,
            commands::system::show_in_folder,
            commands::state::dispatch_action,
            commands::state::get_state,
            commands::state::get_channel_data,
            commands::state::restore_state,
            commands::state::save_state,
            commands::state::get_connected_devices,
            commands::state::get_window_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
