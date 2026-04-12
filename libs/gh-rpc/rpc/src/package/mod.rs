//! # 帧解析器模块
//!
//! 本模块提供 RPC 帧的解析功能，实现状态机逐字节解析帧。
//!
//! ## 主要组件
//!
//! - [`ParseState`]: 帧解析状态枚举，定义解析过程中的各个阶段
//! - [`FrameIndex`]: 帧索引信息，用于多帧传输和确认机制
//! - [`ParseResult`]: 解析结果，包含解析完成后的帧数据
//! - [`FrameParser`]: 帧解析状态机，核心解析器实现
//!
//! ## 解析流程
//!
//! 帧解析器采用状态机模式，逐字节处理输入数据：
//!
//! ```text
//! FrameHeader -> CheckKey -> CheckIndex -> CheckParam -> Complete
//! ```
//!
//! 1. **FrameHeader**: 检测帧头 `[0xAA, 0x11]`
//! 2. **CheckKey**: 解析 TypeKey 和键数据
//! 3. **CheckIndex**: 解析帧索引（安全帧模式）
//! 4. **CheckParam**: 解析参数数据和 CRC 校验
//! 5. **Complete**: 解析完成
//!
//! ## 示例
//!
//! ### 基本使用
//!
//! ```rust
//! use rpc::{FrameParser, ParseState};
//!
//! let mut parser = FrameParser::new();
//!
//! // 逐字节处理数据
//! for byte in &[0xAA, 0x11] {
//!     let result = parser.process_byte(*byte);
//! }
//! ```
//!
//! ### 处理解析结果
//!
//! ```rust
//! use rpc::{FrameParser, FrameError};
//!
//! let mut parser = FrameParser::new();
//!
//! // 假设 received_data 包含完整的 RPC 帧
//! let received_data: &[u8] = &[0xAA, 0x11, /* ... */];
//!
//! for &byte in received_data {
//!     match parser.process_byte(byte) {
//!         Ok(Some(result)) => {
//!             // 帧解析完成
//!             println!("Key: {}", result.key_str());
//!             println!("Secure: {}", result.is_secure);
//!             println!("Param length: {}", result.param.len());
//!         }
//!         Ok(None) => {
//!             // 需要更多数据，继续处理
//!         }
//!         Err(e) => {
//!             // 解析错误，重置解析器
//!             println!("Error: {:?}", e);
//!             parser.reset();
//!         }
//!     }
//! }
//! ```

use crate::types::{FrameError, TypeKey, FRAME_HEADER, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};
use heapless::Vec as HeaplessVec;

/// 帧解析状态
///
/// 定义帧解析状态机的各个状态。
///
/// # 状态转换
///
/// ```text
/// FrameHeader -> CheckKey -> CheckIndex -> CheckParam -> Complete
///                    |                      |
///                    +----------------------+
///                    (当 is_fin && !is_secure 时跳过 CheckIndex)
/// ```
///
/// # 示例
///
/// ```rust
/// use rpc::ParseState;
///
/// let state = ParseState::FrameHeader;
/// assert_eq!(state, ParseState::FrameHeader);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseState {
    /// 解析帧头
    ///
    /// 检测帧头标识 `[0xAA, 0x11]`。
    FrameHeader,

    /// 解析键数据
    ///
    /// 解析 TypeKey 字节和键数据。
    CheckKey,

    /// 解析索引
    ///
    /// 解析调用索引和帧索引（安全帧模式）。
    CheckIndex,

    /// 解析参数
    ///
    /// 解析参数数据和 CRC 校验。
    CheckParam,

    /// 校验 CRC
    ///
    /// 验证 CRC 校验和。
    CheckCrc,

    /// 解析完成
    ///
    /// 帧解析成功完成。
    Complete,
}

/// 帧索引信息
///
/// 用于多帧传输和确认机制。
///
/// # 字段说明
///
/// - `invoke_idx`: 调用索引，用于安全帧模式的请求-响应匹配
/// - `frame_idx`: 帧索引，用于多帧传输时的帧序号
///
/// # 示例
///
/// ```rust
/// use rpc::FrameIndex;
///
/// let index = FrameIndex {
///     invoke_idx: 1,
///     frame_idx: 0,
/// };
/// assert_eq!(index.invoke_idx, 1);
/// assert_eq!(index.frame_idx, 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameIndex {
    /// 调用索引（安全帧模式）
    ///
    /// 用于匹配请求和响应。发送方设置此值，
    /// 接收方在响应中返回相同的值以确认。
    pub invoke_idx: u8,

    /// 帧索引
    ///
    /// 多帧传输时的帧序号。最后一帧的索引为 255。
    pub frame_idx: u8,
}

impl Default for FrameIndex {
    fn default() -> Self {
        Self {
            invoke_idx: 0,
            frame_idx: 0,
        }
    }
}

/// 帧解析结果
///
/// 包含解析完成后的帧数据。
///
/// # 字段说明
///
/// - `key`: 命令名称（键）
/// - `key_len`: 键的有效长度
/// - `is_secure`: 是否为安全帧（需要确认）
/// - `is_fin`: 是否为最后一帧
/// - `frame_index`: 帧索引信息
/// - `param`: 参数数据
///
/// # 示例
///
/// ```rust
/// use rpc::{ParseResult, FrameIndex, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};
/// use heapless::Vec as HeaplessVec;
///
/// let result = ParseResult {
///     key: [0u8; MAX_SUPPORT_KEY_SIZE],
///     key_len: 0,
///     is_secure: false,
///     is_fin: true,
///     frame_index: FrameIndex::default(),
///     param: HeaplessVec::new(),
/// };
///
/// // 获取键字符串
/// assert_eq!(result.key_str(), "");
/// ```
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// 键数据
    ///
    /// 固定大小的缓冲区，存储命令名称。
    pub key: [u8; MAX_SUPPORT_KEY_SIZE],

    /// 键数据长度
    ///
    /// 键在 `key` 缓冲区中的有效长度。
    pub key_len: usize,

    /// 是否为安全帧
    ///
    /// 当为 `true` 时，帧需要接收方确认（ACK）。
    pub is_secure: bool,

    /// 是否为最后一帧
    ///
    /// 当为 `true` 时，表示这是最后一帧或单帧传输。
    pub is_fin: bool,

    /// 帧索引信息
    ///
    /// 包含调用索引和帧索引。
    pub frame_index: FrameIndex,

    /// 参数数据
    ///
    /// 帧中携带的参数数据。
    pub param: HeaplessVec<u8, GHRPC_FRAME_SIZE>,
}

impl PartialEq for ParseResult {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.key_len == other.key_len
            && self.is_secure == other.is_secure
            && self.is_fin == other.is_fin
            && self.frame_index == other.frame_index
            && self.param == other.param
    }
}

impl ParseResult {
    /// 获取键字符串
    ///
    /// 将键数据转换为 UTF-8 字符串。
    ///
    /// # 返回值
    ///
    /// - 如果键数据是有效的 UTF-8，返回对应的字符串
    /// - 如果键数据无效或为空，返回空字符串
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{ParseResult, FrameIndex, GHRPC_FRAME_SIZE, MAX_SUPPORT_KEY_SIZE};
    /// use heapless::Vec as HeaplessVec;
    ///
    /// let mut result = ParseResult {
    ///     key: [0u8; MAX_SUPPORT_KEY_SIZE],
    ///     key_len: 0,
    ///     is_secure: false,
    ///     is_fin: true,
    ///     frame_index: FrameIndex::default(),
    ///     param: HeaplessVec::new(),
    /// };
    ///
    /// // 设置键数据
    /// result.key[0] = b'g';
    /// result.key[1] = b'e';
    /// result.key[2] = b't';
    ///
    /// assert_eq!(result.key_str(), "get");
    /// ```
    pub fn key_str(&self) -> &str {
        if self.key_len == 0 {
            return "";
        }
        core::str::from_utf8(&self.key[..self.key_len]).unwrap_or("")
    }
}

/// 帧解析器
#[derive(Debug, Clone)]
pub struct FrameParser {
    /// 当前解析状态
    pub state: ParseState,

    /// 帧头索引
    pub header_index: usize,

    /// 帧大小
    pub frame_size: u8,

    /// CRC 校验值
    pub crc: u8,

    /// 键数据缓冲区
    pub key_buffer: [u8; MAX_SUPPORT_KEY_SIZE],

    /// 键数据总长度
    pub key_total_len: usize,

    /// 键数据已读取长度
    pub key_read_pos: usize,

    /// 是否为安全帧
    pub is_secure: bool,

    /// 是否为最后一帧
    pub is_fin: bool,

    /// 帧索引信息
    pub frame_index: FrameIndex,

    /// 参数数据缓冲区
    pub param_buffer: HeaplessVec<u8, GHRPC_FRAME_SIZE>,

    /// 是否已读取索引第二字节
    pub index_second_byte: bool,
}

impl Default for FrameParser {
    fn default() -> Self {
        Self {
            state: ParseState::FrameHeader,
            header_index: 0,
            frame_size: 0,
            crc: 0,
            key_buffer: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_total_len: 0,
            key_read_pos: 0,
            is_secure: false,
            is_fin: false,
            frame_index: FrameIndex::default(),
            param_buffer: HeaplessVec::new(),
            index_second_byte: false,
        }
    }
}

impl FrameParser {
    /// 创建新的帧解析器
    ///
    /// 返回初始状态的解析器。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{FrameParser, ParseState};
    ///
    /// let parser = FrameParser::new();
    /// assert_eq!(parser.state, ParseState::FrameHeader);
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 重置解析器状态
    ///
    /// 将解析器恢复到初始状态，清空所有缓冲区。
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{FrameParser, ParseState};
    ///
    /// let mut parser = FrameParser::new();
    /// parser.state = ParseState::CheckKey;
    /// parser.frame_size = 100;
    ///
    /// parser.reset();
    ///
    /// assert_eq!(parser.state, ParseState::FrameHeader);
    /// assert_eq!(parser.frame_size, 0);
    /// ```
    pub fn reset(&mut self) {
        self.state = ParseState::FrameHeader;
        self.header_index = 0;
        self.frame_size = 0;
        self.crc = 0;
        self.key_buffer = [0u8; MAX_SUPPORT_KEY_SIZE];
        self.key_total_len = 0;
        self.key_read_pos = 0;
        self.is_secure = false;
        self.is_fin = false;
        self.frame_index = FrameIndex::default();
        self.param_buffer.clear();
        self.index_second_byte = false;
    }

    /// 处理单个字节
    ///
    /// 逐字节处理输入数据，实现状态机解析。
    ///
    /// # 返回值
    ///
    /// - `Ok(Some(result))`: 帧解析完成，返回解析结果
    /// - `Ok(None)`: 需要更多数据，继续处理
    /// - `Err(e)`: 解析错误
    ///
    /// # 错误
    ///
    /// - [`FrameError::CrcMismatch`]: CRC 校验失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use rpc::{FrameParser, ParseState};
    ///
    /// let mut parser = FrameParser::new();
    ///
    /// // 处理帧头
    /// assert_eq!(parser.process_byte(0xAA).unwrap(), None);
    /// assert_eq!(parser.state, ParseState::FrameHeader);
    ///
    /// assert_eq!(parser.process_byte(0x11).unwrap(), None);
    /// assert_eq!(parser.state, ParseState::FrameHeader);
    ///
    /// // 处理长度字段
    /// assert_eq!(parser.process_byte(10).unwrap(), None);
    /// assert_eq!(parser.state, ParseState::CheckKey);
    /// ```
    pub fn process_byte(&mut self, byte: u8) -> Result<Option<ParseResult>, FrameError> {
        match self.state {
            ParseState::FrameHeader => {
                if self.header_index < FRAME_HEADER.len() {
                    if byte == FRAME_HEADER[self.header_index] {
                        self.header_index += 1;
                    } else {
                        self.header_index = 0;
                    }
                } else {
                    self.frame_size = byte;
                    self.state = ParseState::CheckKey;
                    self.key_total_len = 0;
                    self.key_read_pos = 0;
                }
                Ok(None)
            }
            ParseState::CheckKey => {
                if self.key_total_len == 0 {
                    let type_key = TypeKey::from_byte(byte);
                    self.is_secure = type_key.secure;
                    self.is_fin = type_key.fin;
                    self.crc = self.crc.wrapping_add(byte);
                    self.frame_size -= 1;

                    if type_key.is_array {
                        self.key_total_len = 255;
                    } else {
                        self.key_total_len = 1;
                    }
                } else if self.key_total_len == 255 {
                    self.key_total_len = byte as usize;
                    self.crc = self.crc.wrapping_add(byte);
                    self.frame_size -= 1;
                } else {
                    self.key_buffer[self.key_read_pos] = byte;
                    self.key_read_pos += 1;
                    self.crc = self.crc.wrapping_add(byte);
                    self.frame_size -= 1;

                    if self.key_read_pos >= self.key_total_len {
                        if self.is_fin && !self.is_secure {
                            self.state = ParseState::CheckParam;
                        } else {
                            self.state = ParseState::CheckIndex;
                        }
                    }
                }
                Ok(None)
            }
            ParseState::CheckIndex => {
                self.crc = self.crc.wrapping_add(byte);
                self.frame_size -= 1;

                let check = (self.is_secure as u8) << 1 | (self.is_fin as u8);
                match check {
                    0b00 => {
                        self.frame_index.frame_idx = byte;
                        self.state = ParseState::CheckParam;
                    }
                    0b01 => {
                        self.frame_index.frame_idx = 255;
                        self.state = ParseState::CheckParam;
                    }
                    0b10 => {
                        if !self.index_second_byte {
                            self.frame_index.invoke_idx = byte;
                            self.index_second_byte = true;
                        } else {
                            self.frame_index.frame_idx = byte;
                            self.state = ParseState::CheckParam;
                        }
                    }
                    0b11 => {
                        self.frame_index.invoke_idx = byte;
                        self.frame_index.frame_idx = 255;
                        self.state = ParseState::CheckParam;
                    }
                    _ => {}
                }
                Ok(None)
            }
            ParseState::CheckParam => {
                if self.frame_size == 0 {
                    let crc_ok = byte == self.crc;
                    let result = if crc_ok {
                        Some(ParseResult {
                            key: self.key_buffer,
                            key_len: self.key_total_len,
                            is_secure: self.is_secure,
                            is_fin: self.is_fin,
                            frame_index: self.frame_index,
                            param: self.param_buffer.clone(),
                        })
                    } else {
                        None
                    };
                    self.reset();
                    if crc_ok {
                        Ok(result)
                    } else {
                        Err(FrameError::CrcMismatch)
                    }
                } else {
                    let _ = self.param_buffer.push(byte);
                    self.crc = self.crc.wrapping_add(byte);
                    self.frame_size -= 1;
                    Ok(None)
                }
            }
            ParseState::CheckCrc => {
                let crc_ok = byte == self.crc;
                let result = if crc_ok {
                    Some(ParseResult {
                        key: self.key_buffer,
                        key_len: self.key_total_len,
                        is_secure: self.is_secure,
                        is_fin: self.is_fin,
                        frame_index: self.frame_index,
                        param: self.param_buffer.clone(),
                    })
                } else {
                    None
                };
                self.reset();
                if crc_ok {
                    Ok(result)
                } else {
                    Err(FrameError::CrcMismatch)
                }
            }
            ParseState::Complete => {
                self.reset();
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_parser_header() {
        let mut parser = FrameParser::new();

        assert_eq!(parser.process_byte(0xAA).unwrap(), None);
        assert_eq!(parser.state, ParseState::FrameHeader);

        assert_eq!(parser.process_byte(0x11).unwrap(), None);
        assert_eq!(parser.state, ParseState::FrameHeader);

        assert_eq!(parser.process_byte(10).unwrap(), None);
        assert_eq!(parser.state, ParseState::CheckKey);
        assert_eq!(parser.frame_size, 10);
    }

    #[test]
    fn test_frame_parser_reset() {
        let mut parser = FrameParser::new();
        parser.state = ParseState::CheckKey;
        parser.frame_size = 100;
        parser.crc = 50;

        parser.reset();

        assert_eq!(parser.state, ParseState::FrameHeader);
        assert_eq!(parser.frame_size, 0);
        assert_eq!(parser.crc, 0);
    }

    #[test]
    fn test_parse_result_key_str() {
        let mut result = ParseResult {
            key: [0u8; MAX_SUPPORT_KEY_SIZE],
            key_len: 0,
            is_secure: false,
            is_fin: true,
            frame_index: FrameIndex::default(),
            param: HeaplessVec::new(),
        };
        result.key[0] = b't';
        result.key[1] = b'e';
        result.key[2] = b's';
        result.key[3] = b't';

        assert_eq!(result.key_str(), "test");
    }
}
