pub mod commands;
pub mod compat;
pub mod dashboard;
pub mod device;
pub mod error;
pub mod gh3036;
pub mod protocol;
pub mod service;
pub mod state;
pub mod waveform;
pub mod websocket;

use std::sync::Arc;

use compat::check_compatibility;
use dashboard::{create_parser_script_manager, create_json_config_manager};
use device::DeviceManager;
use gh3036::Gh3036Manager;
use protocol::PluginManager;
use service::{EventBridge, EventBus, EventFilter};
use state::{create_action_dispatcher, create_app_state_with_event_bus, create_state_persistence};
use tauri::Manager;
use tracing::{error, info, warn};
use websocket::ConnectionPool;
use crate::commands::waveform::WaveformManager;

fn init_logger() {
    let exe_path = std::env::current_exe()
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let exe_dir = exe_path.parent()
        .unwrap_or(std::path::Path::new("."));
    
    let log_dir = exe_dir.join("log");
    
    let now = chrono::Local::now();
    let log_filename = format!(
        "combridge_system_{}.log",
        now.format("%Y-%m-%d-%H-%M-%S")
    );
    
    let log_path = log_dir.join(&log_filename);
    
    let config = service::logger::LoggerConfig {
        level: "info".to_string(),
        console_enabled: true,
        file_enabled: true,
        file_path: log_path.clone(),
        max_file_size: 10 * 1024 * 1024,
        max_files: 10,
    };
    
    if let Err(e) = service::logger::LoggerService::init(config) {
        eprintln!("日志系统初始化失败: {}", e);
    }
    info!("ComBridge Rust 应用启动");
    info!("日志文件: {}", log_path.display());
}

fn get_app_data_dir() -> std::path::PathBuf {
    dirs::data_local_dir()
        .or_else(|| dirs::data_dir())
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("combridge")
}

fn get_exe_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

#[cfg(target_os = "windows")]
fn downgrade_window_transparency(window: &tauri::WebviewWindow) -> Result<(), String> {
    window.set_decorations(true).map_err(|e| format!("设置窗口装饰失败: {}", e))?;
    
    info!("窗口已设置为有边框模式");
    
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_logger();

    let event_bus = Arc::new(EventBus::new(1024));
    let connection_pool = Arc::new(ConnectionPool::new());
    let plugin_manager = Arc::new(PluginManager::new(event_bus.clone()));
    let waveform_manager = Arc::new(WaveformManager::new());
    
    let device_manager = Arc::new(DeviceManager::new(event_bus.clone()));
    let gh3036_manager = Arc::new(Gh3036Manager::new(device_manager.clone(), event_bus.clone()));

    let app_state = create_app_state_with_event_bus(event_bus.clone());
    let app_data_dir = get_app_data_dir();
    let state_persistence = create_state_persistence(app_data_dir.clone());
    let action_dispatcher = create_action_dispatcher(
        app_state.clone(),
        state_persistence.clone(),
        device_manager.serial_manager.clone(),
        device_manager.ble_manager.clone(),
    );
    
    let parser_script_manager = create_parser_script_manager(app_data_dir.clone());
    let json_config_manager = create_json_config_manager(get_exe_dir());

    info!("服务初始化完成");

    let ble_manager_clone = device_manager.ble_manager.clone();
    let event_bus_clone = event_bus.clone();
    let plugin_manager_clone = plugin_manager.clone();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(device_manager.serial_manager.clone())
        .manage(device_manager.ble_manager.clone())
        .manage(connection_pool)
        .manage(plugin_manager)
        .manage(app_state)
        .manage(state_persistence)
        .manage(action_dispatcher)
        .manage(gh3036_manager)
        .manage(waveform_manager)
        .manage(parser_script_manager)
        .manage(json_config_manager)
        .manage(event_bus)
        .setup(move |app| {
            info!("Tauri setup hook 开始执行");
            
            let compat_info = check_compatibility();
            
            if !compat_info.webview2_installed {
                error!("WebView2 运行时未安装，应用可能无法正常运行");
                error!("请从以下地址下载并安装 WebView2 运行时: {}", compat::get_webview2_bootstrapper_url());
            }
            
            let main_window = app.get_webview_window("main");
            match main_window {
                Some(window) => {
                    info!("主窗口获取成功, label: {}", window.label());
                    
                    if !compat_info.transparent_supported {
                        warn!("系统不支持透明窗口，正在降级为非透明模式");
                        
                        #[cfg(target_os = "windows")]
                        {
                            if let Err(e) = downgrade_window_transparency(&window) {
                                error!("窗口降级失败: {}", e);
                            } else {
                                info!("窗口已成功降级为非透明模式");
                            }
                        }
                    }
                    
                    match window.is_visible() {
                        Ok(visible) => info!("主窗口可见状态: {}", visible),
                        Err(e) => error!("获取主窗口可见状态失败: {}", e),
                    }
                    
                    match window.is_maximized() {
                        Ok(maximized) => info!("主窗口最大化状态: {}", maximized),
                        Err(e) => error!("获取主窗口最大化状态失败: {}", e),
                    }
                    
                    match window.is_minimized() {
                        Ok(minimized) => info!("主窗口最小化状态: {}", minimized),
                        Err(e) => error!("获取主窗口最小化状态失败: {}", e),
                    }
                    
                    match window.is_focused() {
                        Ok(focused) => info!("主窗口焦点状态: {}", focused),
                        Err(e) => error!("获取主窗口焦点状态失败: {}", e),
                    }
                }
                None => {
                    error!("主窗口获取失败，窗口可能未正确创建");
                }
            }
            
            info!("开始初始化 BLE 管理器");
            let ble_manager = ble_manager_clone.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = ble_manager.initialize().await {
                    error!("BLE 初始化失败: {}", e);
                } else {
                    info!("BLE 初始化成功");
                }
            });
            
            info!("PluginManager 订阅 EventBus 事件");
            plugin_manager_clone.subscribe_to_events();
            
            info!("启动 EventBridge 服务");
            let app_handle = app.handle().clone();
            let filter = EventFilter::with_prefixes(vec![
                "serial:".to_string(),
                "ble:".to_string(),
                "gh3036:".to_string(),
                "protocol:".to_string(),
            ]);
            let mut event_bridge = EventBridge::new(event_bus_clone.clone(), app_handle)
                .with_filter(filter);
            event_bridge.start();
            
            app.manage(std::sync::Mutex::new(event_bridge));
            
            info!("启动系统监控");
            commands::system::start_system_monitor();
            
            info!("Tauri setup hook 执行完成");
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
            commands::ble::get_at_config,
            commands::ble::update_at_uuid_config,
            commands::ble::get_at_tabs,
            commands::ble::get_at_tab,
            commands::ble::clear_at_tab_data,
            commands::ble::remove_at_tab,
            commands::ble::send_at_data,
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
            commands::system::set_timezone_config,
            commands::system::get_timezone_config,
            commands::system::get_platform,
            commands::system::open_url,
            commands::system::show_in_folder,
            commands::system::show_main_window,
            commands::system::get_window_status,
            #[cfg(feature = "devtools")]
            commands::system::open_devtools,
            #[cfg(feature = "devtools")]
            commands::system::close_devtools,
            commands::state::dispatch_action,
            commands::state::get_state,
            commands::state::get_channel_data,
            commands::state::restore_state,
            commands::state::save_state,
            commands::state::get_connected_devices,
            commands::state::get_window_state,
            commands::preferences::get_preferences,
            commands::preferences::save_preferences,
            commands::preferences::update_serial_preferences,
            commands::preferences::update_ble_preferences,
            commands::preferences::update_waveform_preferences,
            commands::preferences::update_gh3036_channel_preferences,
            commands::gh3036::gh3036_init,
            commands::gh3036::gh3036_is_initialized,
            commands::gh3036::gh3036_configure_tx_channel,
            commands::gh3036::gh3036_configure_rx_channel,
            commands::gh3036::gh3036_get_channels,
            commands::gh3036::gh3036_send_data,
            commands::gh3036::gh3036_set_csv_config,
            commands::gh3036::gh3036_get_csv_config,
            commands::gh3036::gh3036_get_rpc_commands,
            commands::gh3036::gh3036_get_version_types,
            commands::gh3036::gh3036_execute_rpc,
            commands::gh3036::gh3036_subscribe_events,
            commands::gh3036::gh3036_get_library_status,
            commands::gh3036::gh3036_load_config_file,
            commands::waveform::waveform_create_buffer,
            commands::waveform::waveform_remove_buffer,
            commands::waveform::waveform_configure_parser,
            commands::waveform::waveform_parse_and_store,
            commands::waveform::waveform_read_data,
            commands::waveform::waveform_get_status,
            commands::waveform::waveform_clear_buffer,
            commands::waveform::waveform_list_buffers,
            dashboard::get_parser_scripts,
            dashboard::get_parser_script_content,
            dashboard::save_parser_script,
            dashboard::delete_parser_script,
            dashboard::execute_parser_script,
            dashboard::init_default_parser_scripts,
            dashboard::analyze_json_structure,
            dashboard::generate_parser_from_json,
            dashboard::get_parser_defined_fields,
            dashboard::merge_json_to_parser,
            dashboard::get_json_files,
            dashboard::save_json_file,
            dashboard::delete_json_file,
            dashboard::load_json_file,
            commands::gh3036::gh3036_factory_test_start,
            commands::gh3036::gh3036_factory_test_stop,
            commands::gh3036::gh3036_factory_test_status,
            commands::gh3036::gh3036_factory_test_continue,
            commands::gh3036::gh3036_factory_test_set_config_dir,
            commands::gh3036::gh3036_factory_test_validate_config,
            commands::gh3036::gh3036_factory_test_get_result,
            commands::gh3036::gh3036_validate_threshold_config,
            commands::gh3036::gh3036_get_threshold_config,
            commands::gh3036::gh3036_get_evaluation_result,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
