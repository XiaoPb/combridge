//! `dfu_img_info_t`，40 字节，位于 DFU 镜像末尾附近。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};
use super::BootInfo;

/// [`ImgInfo`] 的序列化长度。
pub const IMG_INFO_LEN: usize = 40;
/// 有效镜像信息块的魔术字。
pub const VALID_PATTERN: u16 = 0x4744;
/// 注释字段长度。
pub const COMMENTS_LEN: usize = 12;

/// 镜像信息块。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImgInfo {
    /// 必须为 [`VALID_PATTERN`]。
    pub pattern: u16,
    /// 镜像版本。
    pub version: u16,
    /// Boot 信息。
    pub boot_info: BootInfo,
    /// 人类可读注释，零填充。
    pub comments: [u8; COMMENTS_LEN],
}

impl Default for ImgInfo {
    fn default() -> Self {
        Self {
            pattern: VALID_PATTERN,
            version: 0,
            boot_info: BootInfo::default(),
            comments: [0_u8; COMMENTS_LEN],
        }
    }
}

impl ImgInfo {
    /// 由各部分构造镜像信息块。
    pub fn new(version: u16, boot_info: BootInfo, comments: &str) -> Self {
        let mut buffer = [0_u8; COMMENTS_LEN];
        for (slot, byte) in buffer.iter_mut().zip(comments.as_bytes()) {
            *slot = *byte;
        }
        Self {
            pattern: VALID_PATTERN,
            version,
            boot_info,
            comments: buffer,
        }
    }

    /// 解析 40 字节。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < IMG_INFO_LEN {
            return Err(DfuError::Firmware(format!(
                "image info needs {IMG_INFO_LEN} bytes, got {}",
                bytes.len()
            )));
        }
        let mut comments = [0_u8; COMMENTS_LEN];
        comments.copy_from_slice(&bytes[28..40]);
        Ok(Self {
            pattern: u16::from_le_bytes([bytes[0], bytes[1]]),
            version: u16::from_le_bytes([bytes[2], bytes[3]]),
            boot_info: BootInfo::from_bytes(&bytes[4..28])?,
            comments,
        })
    }

    /// 序列化信息块。
    pub fn to_bytes(&self) -> [u8; IMG_INFO_LEN] {
        let mut out = [0_u8; IMG_INFO_LEN];
        out[0..2].copy_from_slice(&self.pattern.to_le_bytes());
        out[2..4].copy_from_slice(&self.version.to_le_bytes());
        out[4..28].copy_from_slice(&self.boot_info.to_bytes());
        out[28..40].copy_from_slice(&self.comments);
        out
    }

    /// 魔术字是否有效。
    pub fn is_valid(&self) -> bool {
        self.pattern == VALID_PATTERN
    }

    /// 注释字符串，在第一个 NUL 字节处截断。
    pub fn comments_str(&self) -> String {
        let end = self
            .comments
            .iter()
            .position(|byte| *byte == 0)
            .unwrap_or(COMMENTS_LEN);
        String::from_utf8_lossy(&self.comments[..end]).into_owned()
    }
}
