//! RPC Type Definitions

pub const FRAME_HEADER: [u8; 2] = [0xAA, 0x11];

pub const GHRPC_FRAME_SIZE: usize = 240;

pub const MAX_SUPPORT_KEY_SIZE: usize = 32;

pub const DYNAMIC_NODE_SIZE: usize = 3;

pub const COMM_RETRY_TIME: u64 = 500;

pub const COMM_RETRY_ROUND: u32 = 100;

pub const DEFAULT_TIMEOUT_MS: u64 = 200;

pub const MAX_RETRY_COUNT: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProPackType {
    Double = 0,
    Unsigned = 1,
    Signed = 2,
    Pack = 3,
}

impl Default for ProPackType {
    fn default() -> Self {
        Self::Unsigned
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TypeKey {
    pub pack_type: u8,
    pub is_array: bool,
    pub width: u8,
    pub secure: bool,
    pub fin: bool,
}

impl TypeKey {
    pub fn from_byte(byte: u8) -> Self {
        Self {
            pack_type: (byte & 0x03) as u8,
            is_array: ((byte >> 2) & 0x01) != 0,
            width: ((byte >> 3) & 0x07) as u8,
            secure: ((byte >> 6) & 0x01) != 0,
            fin: ((byte >> 7) & 0x01) != 0,
        }
    }

    pub fn to_byte(&self) -> u8 {
        let mut byte = self.pack_type & 0x03;
        if self.is_array {
            byte |= 0x04;
        }
        byte |= (self.width & 0x07) << 3;
        if self.secure {
            byte |= 0x40;
        }
        if self.fin {
            byte |= 0x80;
        }
        byte
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FrameIndex {
    pub invoke_idx: u8,
    pub frame_idx: u8,
}

pub type DeviceAddr = u8;
pub type CommandId = u8;
pub type SequenceId = u8;
pub type Payload = Vec<u8>;

#[derive(Debug, Clone)]
pub struct RpcRequest {
    pub device_addr: DeviceAddr,
    pub command_id: CommandId,
    pub sequence_id: SequenceId,
    pub payload: Payload,
}

#[derive(Debug, Clone)]
pub struct RpcResponse {
    pub device_addr: DeviceAddr,
    pub command_id: CommandId,
    pub sequence_id: SequenceId,
    pub payload: Payload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PackHeader: u32 {
        const RAWDATA_EN = 1 << 0;
        const PHY_VALUE_EN = 1 << 1;
        const GS_DATA_EN = 1 << 2;
        const FLAGS_EN = 1 << 3;
        const ALG_DATA_EN = 1 << 4;
        const AGC_INFO_EN = 1 << 5;
        const TIMESTAMP_EN = 1 << 6;
        const FRAMEID_EN = 1 << 7;
        const FUNC_ID_EN = 1 << 8;
        const SLOT_CFG_EN = 1 << 9;
    }
}

impl Default for PackHeader {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_key_conversion() {
        let tk = TypeKey {
            pack_type: ProPackType::Unsigned as u8,
            is_array: false,
            width: 1,
            secure: true,
            fin: true,
        };
        let byte = tk.to_byte();
        let tk2 = TypeKey::from_byte(byte);
        assert_eq!(tk.pack_type, tk2.pack_type);
        assert_eq!(tk.is_array, tk2.is_array);
        assert_eq!(tk.width, tk2.width);
        assert_eq!(tk.secure, tk2.secure);
        assert_eq!(tk.fin, tk2.fin);
    }

    #[test]
    fn test_constants() {
        assert_eq!(FRAME_HEADER, [0xAA, 0x11]);
        assert_eq!(GHRPC_FRAME_SIZE, 240);
        assert_eq!(MAX_SUPPORT_KEY_SIZE, 32);
        assert_eq!(DEFAULT_TIMEOUT_MS, 200);
        assert_eq!(MAX_RETRY_COUNT, 3);
    }
}
