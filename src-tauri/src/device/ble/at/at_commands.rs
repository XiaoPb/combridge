use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AtCommand {
    Test,
    GetInfo,
    GetName,
    SetName(String),
    GetMac,
    SetRole(u8),
    GetMtu,
    SetMtu(u16),
    GetTxUuid,
    SetTxUuid(String),
    GetRxUuid,
    SetRxUuid(String),
    GetSrvUuid,
    SetSrvUuid(String),
    ScanStart,
    ScanStop,
    Connect(String),
    Disconnect(String),
    SendData(Vec<u8>),
    GetRssi(u32),
    ExitToTransparent,
    SetAutoExit(u8),
    Reset,
    Restore,
}

impl fmt::Display for AtCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtCommand::Test => write!(f, "AT"),
            AtCommand::GetInfo => write!(f, "AT+INFO?"),
            AtCommand::GetName => write!(f, "AT+NAME?"),
            AtCommand::SetName(name) => write!(f, "AT+NAME={}", name),
            AtCommand::GetMac => write!(f, "AT+MAC?"),
            AtCommand::SetRole(role) => write!(f, "AT+ROLE={}", role),
            AtCommand::GetMtu => write!(f, "AT+MTU?"),
            AtCommand::SetMtu(mtu) => write!(f, "AT+MTU={}", mtu),
            AtCommand::GetTxUuid => write!(f, "AT+TXUUID?"),
            AtCommand::SetTxUuid(uuid) => write!(f, "AT+TXUUID={}", uuid),
            AtCommand::GetRxUuid => write!(f, "AT+RXUUID?"),
            AtCommand::SetRxUuid(uuid) => write!(f, "AT+RXUUID={}", uuid),
            AtCommand::GetSrvUuid => write!(f, "AT+SVRUUD?"),
            AtCommand::SetSrvUuid(uuid) => write!(f, "AT+SVRUUD={}", uuid),
            AtCommand::ScanStart => write!(f, "AT+SCAN=1"),
            AtCommand::ScanStop => write!(f, "AT+SCAN=0"),
            AtCommand::Connect(address) => write!(f, "AT+CONN={}", address),
            AtCommand::Disconnect(address) => write!(f, "AT+DISC={}", address),
            AtCommand::SendData(data) => {
                let hex_data: String = data.iter().map(|b| format!("{:02X}", b)).collect();
                write!(f, "AT+BLESEND={}", hex_data)
            }
            AtCommand::GetRssi(interval_ms) => write!(f, "AT+RSSI={:04X}", interval_ms),
            AtCommand::ExitToTransparent => write!(f, "AT+EXIT"),
            AtCommand::SetAutoExit(mode) => write!(f, "AT+EXIT={}", mode),
            AtCommand::Reset => write!(f, "AT+RESET"),
            AtCommand::Restore => write!(f, "AT+RESTORE"),
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
    Info { info: String },
    Name { name: String },
    Mac { address: String },
    Mtu { mtu: u16 },
    TxUuid { uuid: String },
    RxUuid { uuid: String },
    SrvUuid { uuid: String },
    Role { role: u8 },
    ScanResult { devices: Vec<ScanDevice> },
    Connected { address: String },
    Disconnected { address: String },
    Data { data: Vec<u8> },
    Rssi { rssi: i16 },
    Notify { data: Vec<u8> },
    SleepEntry,
    SleepExit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanDevice {
    pub address: String,
    pub name: Option<String>,
    pub rssi: i16,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AtConnectionConfig {
    pub tx_uuid: Option<String>,
    pub rx_uuid: Option<String>,
    pub srv_uuid: Option<String>,
    pub mtu: Option<u16>,
}

impl AtConnectionConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_uuid(tx_uuid: impl Into<String>, rx_uuid: impl Into<String>) -> Self {
        Self {
            tx_uuid: Some(tx_uuid.into()),
            rx_uuid: Some(rx_uuid.into()),
            srv_uuid: None,
            mtu: None,
        }
    }
}
