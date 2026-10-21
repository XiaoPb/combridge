//! `0x4744` 基础命令帧。
//!
//! 线格式（小端）：
//!
//! ```text
//! +--------+--------+--------+--------+--------+---...---+--------+--------+
//! | 0x44   | 0x47   | opcode u16      | length u16      | payload       | cksum  |
//! +--------+--------+--------+--------+--------+---...---+--------+--------+
//! ```
//!
//! 校验和为从 opcode 字段开始到（不含）校验和本身的 16 位回绕和。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};

/// 帧魔术字（`0x4744`，线序为 `44 47`）。
pub const MAGIC: u16 = 0x4744;
/// 头部字节数：magic + opcode + length。
pub const HEADER_LEN: usize = 6;
/// 尾部校验和字节数。
pub const TRAILER_LEN: usize = 2;

/// 解码后的基础 DFU 命令帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicFrame {
    /// 命令 opcode。
    pub opcode: u16,
    /// 命令参数。
    pub payload: Vec<u8>,
}

impl BasicFrame {
    /// 由 opcode 与载荷构造帧。
    pub fn new(opcode: u16, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            opcode,
            payload: payload.into(),
        }
    }

    /// `bytes` 的 16 位回绕校验和。
    pub fn checksum(bytes: &[u8]) -> u16 {
        let mut sum: u16 = 0;
        for byte in bytes {
            sum = sum.wrapping_add(u16::from(*byte));
        }
        sum
    }

    /// 序列化帧并追加校验和。
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.payload.len() + TRAILER_LEN);
        out.extend_from_slice(&MAGIC.to_le_bytes());
        out.extend_from_slice(&self.opcode.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.payload);
        let checksum = Self::checksum(&out[2..]);
        out.extend_from_slice(&checksum.to_le_bytes());
        out
    }

    /// 解析 6 字节头部为 `(opcode, 载荷长度)`。
    pub fn parse_header(header: &[u8]) -> Result<(u16, u16)> {
        if header.len() < HEADER_LEN {
            return Err(DfuError::Protocol(format!(
                "basic frame header needs {HEADER_LEN} bytes, got {}",
                header.len()
            )));
        }
        let magic = u16::from_le_bytes([header[0], header[1]]);
        if magic != MAGIC {
            return Err(DfuError::Protocol(format!(
                "unexpected basic frame magic 0x{magic:04X}"
            )));
        }
        let opcode = u16::from_le_bytes([header[2], header[3]]);
        let length = u16::from_le_bytes([header[4], header[5]]);
        Ok((opcode, length))
    }

    /// 解码包含头部与校验和的完整帧。
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let (opcode, length) = Self::parse_header(bytes)?;
        let expected = HEADER_LEN + usize::from(length) + TRAILER_LEN;
        if bytes.len() != expected {
            return Err(DfuError::Protocol(format!(
                "basic frame length mismatch: declared {expected}, got {}",
                bytes.len()
            )));
        }
        let payload = bytes[HEADER_LEN..HEADER_LEN + usize::from(length)].to_vec();
        Self::from_parts(opcode, payload, &bytes[HEADER_LEN + usize::from(length)..])
    }

    /// 由已拆分的载荷与校验和字节重建帧。
    pub fn from_parts(opcode: u16, payload: Vec<u8>, checksum_bytes: &[u8]) -> Result<Self> {
        if checksum_bytes.len() < TRAILER_LEN {
            return Err(DfuError::Protocol(
                "basic frame checksum is truncated".to_string(),
            ));
        }
        let frame = Self { opcode, payload };
        let received = u16::from_le_bytes([checksum_bytes[0], checksum_bytes[1]]);
        let computed = Self::checksum(&frame.checksummed_bytes());
        if received != computed {
            return Err(DfuError::Protocol(format!(
                "basic frame checksum mismatch: 0x{received:04X} != 0x{computed:04X}"
            )));
        }
        Ok(frame)
    }

    /// 返回 `opcode || length || payload`，即校验和覆盖的区域。
    pub fn checksummed_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.payload.len());
        out.extend_from_slice(&self.opcode.to_le_bytes());
        out.extend_from_slice(&(self.payload.len() as u16).to_le_bytes());
        out.extend_from_slice(&self.payload);
        out
    }
}

/// 基础（`0x4744`）DFU 协议的 opcode。
pub mod opcode {
    /// 读取芯片信息。
    pub const GET_INFO: u16 = 0x0001;
    /// 复位设备。
    pub const RESET: u16 = 0x0002;
    /// 写入 RAM。
    pub const WRITE_RAM: u16 = 0x0011;
    /// 读取 RAM。
    pub const READ_RAM: u16 = 0x0012;
    /// 导出 Flash。
    pub const DUMP_FLASH: u16 = 0x0021;
    /// 擦除 Flash。
    pub const ERASE_FLASH: u16 = 0x0022;
    /// 准备固件或资源传输。
    pub const PROGRAM_START: u16 = 0x0023;
    /// 传输一个固件分段。
    pub const PROGRAM_FLASH: u16 = 0x0024;
    /// 结束传输。
    pub const PROGRAM_END: u16 = 0x0025;
    /// 更新 Flash。
    pub const UPDATE_FLASH: u16 = 0x0026;
    /// 读写系统配置区（SCA）。
    pub const SYSTEM_CONFIG: u16 = 0x0027;
    /// NVDS 操作。
    pub const OPERATION_NVDS: u16 = 0x0028;
    /// 读写 eFuse。
    pub const RW_EFUSE: u16 = 0x0029;
    /// 配置外部 Flash。
    pub const CONFIG_EXT_FLASH: u16 = 0x002A;
    /// 读取 Flash 信息。
    pub const GET_FLASH_INFO: u16 = 0x002B;
    /// 读写寄存器。
    pub const RW_REG: u16 = 0x002C;
    /// 选择 DFU 模式（单 Bank / 双 Bank）。
    pub const SET_DFU_MODE: u16 = 0x0041;
    /// 读取 App Bootloader 附加信息。
    pub const GET_FW_INFO: u16 = 0x0042;
    /// 快速模式完成确认。
    pub const PROGRAM_FLASH_FAST: u16 = 0x00FF;
}

/// 请求固件进入 DFU 模式的控制点模式字。
pub const CTRL_POINT_DFU_PATTERN: [u8; 4] = [0x44, 0x4F, 0x4F, 0x47];
