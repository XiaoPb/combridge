pub mod commands;
pub mod device;
pub mod error;
pub mod service;

use std::sync::Arc;

use device::SerialManager;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let serial_manager = Arc::new(SerialManager::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(serial_manager)
        .invoke_handler(tauri::generate_handler![
            greet,
            commands::serial::scan_serial_ports,
            commands::serial::open_serial_port,
            commands::serial::close_serial_port,
            commands::serial::send_serial_data,
            commands::serial::get_open_ports,
            commands::serial::is_port_open,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
