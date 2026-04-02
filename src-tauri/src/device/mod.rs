pub mod ble;
pub mod serial;

pub use ble::{
    AtConfig, BleBackend, BleCharacteristic, BleCharacteristicProperties, BleConnection,
    BleDevice, BleManager, BleManagerRef, BleMode, BleService, NotifyCallback,
};
pub use serial::{
    BaudRate, DataBits, FlowControl, Parity, PortInfo, SerialManager, SerialManagerRef,
    SerialPortConfig, StopBits,
};
