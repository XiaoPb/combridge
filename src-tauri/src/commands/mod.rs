pub mod serial;

pub use serial::{
    close_serial_port, get_open_ports, is_port_open, open_serial_port, scan_serial_ports,
    send_serial_data, SerialDataEvent, SerialPortConfigDto,
};
