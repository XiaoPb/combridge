//! 专用快速 DFU 服务使用的 `DOOG` 快速命令帧。
//!
//! 线格式：
//!
//! ```text
//! +--------+--------+--------+--------+--------+---...---+
//! | 'D'    | 'O'    | 'O'    | 'G'    | opcode | params  |
//! +--------+--------+--------+--------+--------+---...---+
//! ```
//!
//! 魔术字为 `0x474F4F44` 的小端编码。无长度字段、无校验和：
//! 整个通知即为应答。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};

/// 快速帧魔术字，`0x474F4F44` 的小端表示。
pub const MAGIC: [u8; 4] = [0x44, 0x4F, 0x4F, 0x47];
/// 最小帧长度（magic + opcode）。
pub const MIN_LEN: usize = 5;

/// 解码后的快速 DFU 帧。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FastFrame {
    /// 命令 opcode。
    pub opcode: u8,
    /// 命令参数。
    pub params: Vec<u8>,
}

impl FastFrame {
    /// 构造帧。
    pub fn new(opcode: u8, params: impl Into<Vec<u8>>) -> Self {
        Self {
            opcode,
            params: params.into(),
        }
    }

    /// 序列化帧。
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(MIN_LEN + self.params.len());
        out.extend_from_slice(&MAGIC);
        out.push(self.opcode);
        out.extend_from_slice(&self.params);
        out
    }

    /// 解码一个完整通知。
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < MIN_LEN {
            return Err(DfuError::Protocol(format!(
                "fast frame needs at least {MIN_LEN} bytes, got {}",
                bytes.len()
            )));
        }
        if bytes[..4] != MAGIC {
            return Err(DfuError::Protocol(format!(
                "unexpected fast frame magic {:02X?}",
                &bytes[..4]
            )));
        }
        Ok(Self {
            opcode: bytes[4],
            params: bytes[MIN_LEN..].to_vec(),
        })
    }
}

/// 专用快速 DFU 服务的 opcode。
pub mod opcode {
    /// 擦除 Flash；阶段通过重复应答上报。
    pub const ERASE_FLASH: u8 = 0x01;
    /// 刷新写缓冲。
    pub const FLUSH_FLASH: u8 = 0x02;
    /// 校验 Flash 校验和。
    pub const VERIFY_CHECKSUM: u8 = 0x03;
    /// 写入 boot 信息。
    pub const WRITE_BOOT: u8 = 0x04;
    /// 选择内部或外部 Flash。
    pub const SELECT_FLASH_TYPE: u8 = 0x05;
    /// 流控：暂停。
    pub const FLOW_CTRL_PAUSE: u8 = 0x06;
    /// 流控：恢复。
    pub const FLOW_CTRL_RESUME: u8 = 0x07;
    /// 开始拷贝已下载镜像。
    pub const START_COPY: u8 = 0x08;
    /// 读取缓冲区大小。
    pub const GET_BUFFER_SIZE: u8 = 0x09;
    /// 一个数据块的应答。
    pub const NEXT_BUFFER: u8 = 0x0A;
    /// 读取快速 DFU 协议版本。
    pub const GET_VERSION: u8 = 0x0B;
}

/// 擦除应答阶段。
pub mod erase_stage {
    /// 地址未按 4 KiB 对齐。
    pub const UNALIGNED: u8 = 0x00;
    /// 总扇区数。
    pub const TOTAL: u8 = 0x01;
    /// 又擦除一个扇区。
    pub const ERASED: u8 = 0x02;
    /// 擦除完成。
    pub const DONE: u8 = 0x03;
    /// 区域与运行中固件重叠。
    pub const OVERLAP: u8 = 0x04;
    /// 擦除失败。
    pub const FAILED: u8 = 0x05;
    /// 无外部 Flash。
    pub const NO_EXT_FLASH: u8 = 0x06;
}
