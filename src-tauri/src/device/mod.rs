pub mod serial;

pub use serial::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialManager, SerialManagerRef,
    SerialPortConfig, StopBits,
};
