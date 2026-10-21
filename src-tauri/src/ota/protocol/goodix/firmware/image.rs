//! Goodix `.bin` DFU 固件解析。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::path::Path;

use super::super::error::{DfuError, Result};
use super::{BootInfo, ImgInfo, IMG_INFO_LEN, VALID_PATTERN};

/// 保留区签名，表示“已签名但未加密”。
const NOT_ENCRYPTED_SIGNATURE: u32 = 0x4E47_4953;
/// 普通布局中镜像信息块相对文件末尾的偏移
/// （40 字节镜像信息 + 8 字节保留区）。
const PLAIN_BLOCK_OFFSET: usize = IMG_INFO_LEN + 8;
/// 签名/加密布局中镜像信息块相对文件末尾的偏移。
const SIGNED_BLOCK_OFFSET: usize = 904;
/// 保留区相对文件末尾的偏移。
const RESERVED_OFFSET_FROM_END: usize = 784;

/// 已解析的 DFU 固件文件。
///
/// 传输时需要发送**整个**文件（含尾部信息块），
/// 因此 [`DfuImage::data`] 原样保留。
#[derive(Debug, Clone)]
pub struct DfuImage {
    /// 原始文件字节。
    pub data: Vec<u8>,
    /// 从尾部解析出的镜像信息。
    pub img_info: ImgInfo,
    /// 全部文件字节的 32 位和。
    pub checksum: u32,
    /// 文件带有签名块。
    pub signed: bool,
    /// 文件已加密。
    pub encrypted: bool,
}

impl DfuImage {
    /// 解析固件镜像。
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        if data.is_empty() {
            return Err(DfuError::Firmware("firmware file is empty".to_string()));
        }
        let checksum = sum_checksum(&data);
        let len = data.len();

        // 先尝试普通布局，再尝试签名/加密布局。
        let (offset, signed) = if len >= PLAIN_BLOCK_OFFSET
            && pattern_at(&data, len - PLAIN_BLOCK_OFFSET).is_some_and(|p| p == VALID_PATTERN)
        {
            (len - PLAIN_BLOCK_OFFSET, false)
        } else if len >= SIGNED_BLOCK_OFFSET + IMG_INFO_LEN
            && pattern_at(&data, len - SIGNED_BLOCK_OFFSET).is_some_and(|p| p == VALID_PATTERN)
        {
            (len - SIGNED_BLOCK_OFFSET, true)
        } else {
            return Err(DfuError::Firmware(
                "can't find a valid image info block".to_string(),
            ));
        };

        let encrypted = if signed {
            if len < RESERVED_OFFSET_FROM_END {
                return Err(DfuError::Firmware(
                    "signed image is too short for the reserved area".to_string(),
                ));
            }
            let start = len - RESERVED_OFFSET_FROM_END;
            let reserved = u32::from_le_bytes([
                data[start],
                data[start + 1],
                data[start + 2],
                data[start + 3],
            ]);
            reserved != NOT_ENCRYPTED_SIGNATURE
        } else {
            false
        };

        let img_info = ImgInfo::from_bytes(&data[offset..offset + IMG_INFO_LEN])?;
        if !img_info.is_valid() {
            return Err(DfuError::Firmware(
                "image info pattern is not 0x4744".to_string(),
            ));
        }

        Ok(Self {
            data,
            img_info,
            checksum,
            signed,
            encrypted,
        })
    }

    /// 从磁盘读取并解析固件文件。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let data = std::fs::read(path).map_err(|err| {
            DfuError::Io(std::io::Error::new(
                err.kind(),
                format!("failed to read {}: {err}", path.display()),
            ))
        })?;
        Self::parse(data)
    }

    /// 文件字节数。
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// 文件是否为空。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// 镜像的 boot 信息。
    pub fn boot_info(&self) -> &BootInfo {
        &self.img_info.boot_info
    }

    /// 覆盖加载地址后的镜像信息。
    pub fn img_info_for_address(&self, write_address: u32) -> ImgInfo {
        let mut info = self.img_info.clone();
        if info.boot_info.load_addr != write_address {
            info.boot_info.load_addr = write_address;
        }
        info
    }
}

/// 全部字节的 32 位回绕和，`PROGRAM_END` 期望该值。
pub fn sum_checksum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0_u32, |sum, byte| sum.wrapping_add(u32::from(*byte)))
}

fn pattern_at(data: &[u8], offset: usize) -> Option<u16> {
    let bytes = data.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}
