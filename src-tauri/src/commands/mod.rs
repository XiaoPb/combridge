pub mod protocol;
pub mod serial;
pub mod websocket;

pub use protocol::{
    bind_protocol, disable_protocol, enable_protocol, get_bound_protocols, get_protocol,
    list_protocols, load_protocol, unbind_protocol, unload_protocol,
};
pub mod ble;
pub mod serial;

pub use ble::{
    configure_ble, connect_ble, disconnect_ble, discover_ble_characteristics,
    discover_ble_services, get_ble_connections, get_ble_mode, get_ble_rssi, is_ble_configured,
    read_ble_characteristic, scan_ble_devices, subscribe_ble_notify, unsubscribe_ble_notify,
    write_ble_characteristic, BleConfigDto, BleNotifyEvent,
};
pub use serial::{
    close_serial_port, get_open_ports, is_port_open, open_serial_port, scan_serial_ports,
    send_serial_data, SerialDataEvent, SerialPortConfigDto,
};
pub use websocket::{
    connect_websocket, disconnect_websocket, get_all_websocket_connections,
    get_all_websocket_status, get_websocket_status, send_websocket_message,
    WebSocketConnectionConfig, WebSocketMessageEvent, WebSocketStatusEvent,
};
