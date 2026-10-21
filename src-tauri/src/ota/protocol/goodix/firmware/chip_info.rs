//! `GET_INFO`（`0x01`）返回的芯片信息。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};

/// 芯片信息载荷（不含状态字节）的序列化长度。
pub const CHIP_INFO_LEN: usize = 17;

/// Stack、SDK 与 DFU 版本信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChipInfo {
    /// Stack 主版本。
    pub stack_major: u8,
    /// Stack 次版本。
    pub stack_minor: u8,
    /// Stack 构建号。
    pub stack_build: u16,
    /// Stack SVN，用于推导 SCA 地址。
    pub stack_svn: u32,
    /// SDK 主版本。
    pub sdk_major: u8,
    /// SDK 次版本。
    pub sdk_minor: u8,
    /// SDK 构建号。
    pub sdk_build: u16,
    /// SDK SVN。
    pub sdk_svn: u32,
    /// DFU 协议版本，DFU V1 中不存在。
    pub dfu_version: u8,
}

impl ChipInfo {
    /// 解析 17 字节载荷。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < CHIP_INFO_LEN {
            return Err(DfuError::Protocol(format!(
                "chip info needs {CHIP_INFO_LEN} bytes, got {}",
                bytes.len()
            )));
        }
        Ok(Self {
            stack_major: bytes[0],
            stack_minor: bytes[1],
            stack_build: u16::from_le_bytes([bytes[2], bytes[3]]),
            stack_svn: u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            sdk_major: bytes[8],
            sdk_minor: bytes[9],
            sdk_build: u16::from_le_bytes([bytes[10], bytes[11]]),
            sdk_svn: u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
            dfu_version: bytes[16],
        })
    }

    /// 该芯片系统配置区（SCA）的地址。
    ///
    /// App Bootloader 把启动 boot 信息存放在 SCA。较早的 GR551x 版本
    /// 位于 `0x0100_0000`，较新的芯片位于 `0x0020_0000`。
    pub fn sca_address(&self) -> u32 {
        match self.stack_svn {
            0x0000_1EA8 | 0x0000_0B88 => 0x0100_0000,
            _ => 0x0020_0000,
        }
    }
}
