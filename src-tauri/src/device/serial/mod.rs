pub mod serial_config;
pub mod serial_manager;
pub mod serial_port;

pub use serial_config::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialPortConfig, StopBits,
};
pub use serial_manager::{DataCallback, SerialManager, SerialManagerRef};
pub use serial_port::{scan_ports, SerialPort};
