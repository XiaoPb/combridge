//! GH3220 UProtocol host-side codec.
//!
//! 只做字节 <-> 结构体，不做任何 I/O。
//! 协议规范见 `docs/gh3220_uprotocol_reference.md`。

pub mod cmd;
pub mod crc8;
pub mod data;
pub mod error;
pub mod event;
pub mod frame;

pub use data::{DataDecoder, PacketHeader, Record};
pub use error::{DataError, FrameError};
pub use frame::{Frame, FrameDecoder, MAX_PAYLOAD};

/// 协议命令号
pub mod command {
    pub const REG_RW: u8 = 0x03;
    pub const PACKAGE_TEST: u8 = 0x05;
    pub const NEW_STRUCT_RAWDATA: u8 = 0x0B;
    pub const START_CTRL: u8 = 0x0C;
    pub const WORK_MODE: u8 = 0x10;
    pub const FIFO_THR_CONFIG: u8 = 0x12;
    pub const REPORT_EVENT_SET: u8 = 0x13;
    pub const ECG_CLASSIFICATION: u8 = 0x14;
    pub const CHIP_EVENT_REPORT: u8 = 0x16;
    pub const CHIP_CTRL: u8 = 0x17;
    pub const GET_VER: u8 = 0x19;
    pub const GET_CONN_STATUS: u8 = 0x1A;
    pub const SWITCH_SLOT_FUNC: u8 = 0x1C;
    pub const GET_OPENED_FUNC: u8 = 0x25;
    pub const SET_ECG_CASCADE: u8 = 0x26;
    pub const BP_DATA_TRANS: u8 = 0x2B;
    pub const FUNCTION_INFO_UPDATE: u8 = 0x2C;
    pub const GET_MAX_LEN: u8 = 0xA0;
    pub const LOAD_REG_LIST: u8 = 0xA1;
    pub const DEBUG_STATUS: u8 = 0xA2;
}

/// 版本类型（`0x19` 命令的 payload[0]）
pub mod ver_type {
    pub const FW: u8 = 0x01;
    pub const DEMO: u8 = 0x0A;
    pub const VIRTUAL_REG: u8 = 0x0B;
    pub const BOOTLOADER: u8 = 0x0C;
    pub const PROTOCOL: u8 = 0x0E;
    pub const FUNC_SUPPORT: u8 = 0x0F;
    pub const DRV: u8 = 0x10;
    pub const CHIP: u8 = 0x11;
    pub const ALGO: u8 = 0x12;
}

/// 芯片控制类型（`0x17` 命令的 payload[0]）
pub mod chip_ctrl {
    pub const HARD_RESET: u8 = 0x5A;
    pub const RX_RESET: u8 = 0x5B;
    pub const SOFT_RESET: u8 = 0xC2;
    pub const WAKEUP: u8 = 0xC3;
    pub const SLEEP: u8 = 0xC4;
}
