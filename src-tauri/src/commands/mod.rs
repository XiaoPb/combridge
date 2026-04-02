pub mod protocol;
pub mod serial;
pub mod websocket;

pub use protocol::{
    bind_protocol, disable_protocol, enable_protocol, get_bound_protocols, get_protocol,
    list_protocols, load_protocol, unbind_protocol, unload_protocol,
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
