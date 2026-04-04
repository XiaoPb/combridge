pub mod ble;
pub mod protocol;
pub mod serial;
pub mod state;
pub mod system;
pub mod websocket;

pub use ble::{
    configure_ble, connect_ble, disconnect_ble, discover_ble_characteristics,
    discover_ble_services, get_ble_connections, get_ble_mode, get_ble_rssi, is_ble_configured,
    read_ble_characteristic, scan_ble_devices, set_ble_mtu, stop_ble_scan, subscribe_ble_notify, unsubscribe_ble_notify,
    write_ble_characteristic, write_ble_without_response, BleConfigDto, BleNotifyEvent,
};
pub use protocol::{
    bind_protocol, disable_protocol, enable_protocol, get_bound_protocols, get_protocol,
    list_protocols, load_protocol, unbind_protocol, unload_protocol,
};
pub use serial::{
    close_serial_port, get_open_ports, is_port_open, open_serial_port, scan_serial_ports,
    send_serial_data, SerialDataEvent, SerialPortConfigDto,
};
pub use state::{
    dispatch_action, get_channel_data, get_connected_channels, get_state, get_window_state,
    restore_state, save_state,
};
pub use system::{
    configure_log, get_app_version, get_log_config, get_platform, get_runtime_status,
    get_system_info, get_system_status, open_url, show_in_folder, DiskUsage, LogConfig,
    RuntimeStatus, SystemInfo, SystemStatus,
};
pub use websocket::{
    connect_websocket, disconnect_websocket, get_all_websocket_connections,
    get_all_websocket_status, get_websocket_status, send_websocket_message,
    WebSocketConnectionConfig, WebSocketMessageEvent, WebSocketStatusEvent,
};
