pub mod commands;
pub mod device;
pub mod error;
pub mod protocol;
pub mod service;
pub mod websocket;

use std::sync::Arc;

use device::SerialManager;
use protocol::PluginManager;
use websocket::ConnectionPool;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let serial_manager = Arc::new(SerialManager::new());
    let connection_pool = Arc::new(ConnectionPool::new());
    let plugin_manager = Arc::new(PluginManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(serial_manager)
        .manage(connection_pool)
        .manage(plugin_manager)
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::serial::scan_serial_ports,
            commands::serial::open_serial_port,
            commands::serial::close_serial_port,
            commands::serial::send_serial_data,
            commands::serial::get_open_ports,
            commands::serial::is_port_open,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
