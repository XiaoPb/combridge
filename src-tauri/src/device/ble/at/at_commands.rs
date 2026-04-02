use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtCommand {
    Test,
    Scan { duration_ms: u64 },
    Connect { address: String },
    Disconnect { address: String },
    DiscoverServices { address: String },
    DiscoverCharacteristics { address: String, service_uuid: String },
    Read { address: String, char_uuid: String },
    Write { address: String, char_uuid: String, data: Vec<u8> },
    Subscribe { address: String, char_uuid: String },
    Unsubscribe { address: String, char_uuid: String },
    GetRssi { address: String },
}

impl fmt::Display for AtCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtCommand::Test => write!(f, "AT"),
            AtCommand::Scan { duration_ms } => write!(f, "AT+SCAN={}", duration_ms),
            AtCommand::Connect { address } => write!(f, "AT+CONN={}", address),
            AtCommand::Disconnect { address } => write!(f, "AT+DISC={}", address),
            AtCommand::DiscoverServices { address } => write!(f, "AT+SRV={}", address),
            AtCommand::DiscoverCharacteristics { address, service_uuid } => {
                write!(f, "AT+CHAR={},{}", address, service_uuid)
            }
            AtCommand::Read { address, char_uuid } => {
                write!(f, "AT+READ={},{}", address, char_uuid)
            }
            AtCommand::Write { address, char_uuid, data } => {
                let hex_data: String = data.iter().map(|b| format!("{:02X}", b)).collect();
                write!(f, "AT+WRITE={},{},{}", address, char_uuid, hex_data)
            }
            AtCommand::Subscribe { address, char_uuid } => {
                write!(f, "AT+NOTIFY={},{}", address, char_uuid)
            }
            AtCommand::Unsubscribe { address, char_uuid } => {
                write!(f, "AT+UNNOTIFY={},{}", address, char_uuid)
            }
            AtCommand::GetRssi { address } => write!(f, "AT+RSSI={}", address),
        }
    }
}

impl AtCommand {
    pub fn to_bytes(&self) -> Vec<u8> {
        let cmd = format!("{}\r\n", self);
        cmd.into_bytes()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtResponse {
    Ok,
    Error { code: i32, message: String },
    ScanResult { devices: Vec<ScanDevice> },
    Connected { address: String },
    Disconnected { address: String },
    Services { services: Vec<ServiceInfo> },
    Characteristics { characteristics: Vec<CharInfo> },
    Data { address: String, char_uuid: String, data: Vec<u8> },
    Rssi { address: String, rssi: i16 },
    Notify { address: String, char_uuid: String, data: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDevice {
    pub address: String,
    pub name: Option<String>,
    pub rssi: i16,
    pub is_connectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceInfo {
    pub uuid: String,
    pub primary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharInfo {
    pub uuid: String,
    pub service_uuid: String,
    pub properties: u8,
}

impl CharInfo {
    pub fn can_read(&self) -> bool {
        (self.properties & 0x01) != 0
    }

    pub fn can_write(&self) -> bool {
        (self.properties & 0x02) != 0
    }

    pub fn can_notify(&self) -> bool {
        (self.properties & 0x04) != 0
    }

    pub fn can_indicate(&self) -> bool {
        (self.properties & 0x08) != 0
    }
}
