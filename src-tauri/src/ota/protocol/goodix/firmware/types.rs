//! App Bootloader 返回的辅助结构。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};
use super::{BootInfo, ImgInfo, IMG_INFO_LEN, VALID_PATTERN};

/// 从 SCA 读取的启动 boot 信息（`SYSTEM_CONFIG`，opcode `0`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StartupBootInfo {
    /// SCA 中的固件已加密。
    pub encrypted: bool,
    /// 当前运行应用的 boot 信息。
    pub boot_info: BootInfo,
}

impl StartupBootInfo {
    /// 解析 `SYSTEM_CONFIG` 响应载荷。
    ///
    /// 布局：status (1)、op (1)、address (4)、length (2)、24 字节 boot 信息。
    pub fn from_response(payload: &[u8], expected_address: u32) -> Result<Self> {
        let body = ConfigResponse::parse(payload)?;
        body.ensure_status()?;
        body.ensure_address(expected_address)?;
        if usize::from(body.length) != super::BOOT_INFO_LEN {
            return Err(DfuError::Protocol(format!(
                "unexpected boot info length {}",
                body.length
            )));
        }
        Ok(Self {
            encrypted: body.op & 0xF0 != 0,
            boot_info: BootInfo::from_bytes(body.data)?,
        })
    }
}

/// `GET_FW_INFO`（`0x42`）返回的 App Bootloader 附加信息。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AppBootloaderExtraInfo {
    /// 后台（双 Bank）下载推荐使用的地址。
    pub recommend_save_address: u32,
    /// `0` 表示 Bootloader 在运行，`1` 表示应用在运行。
    pub position: u8,
    /// 正在运行的应用固件镜像信息。
    pub app_fw_img_info: ImgInfo,
}

impl AppBootloaderExtraInfo {
    /// 设备当前运行应用固件。
    pub const CURRENT_FW_IS_APP: u8 = 1;

    /// 解析 `GET_FW_INFO` 响应载荷。
    ///
    /// 布局：status (1)、推荐地址 (4)、position (1)、40 字节镜像信息。
    pub fn from_response(payload: &[u8]) -> Result<Self> {
        if payload.len() < 1 + 4 + 1 + IMG_INFO_LEN {
            return Err(DfuError::Protocol(format!(
                "GET_FW_INFO response is too short: {} bytes",
                payload.len()
            )));
        }
        ensure_status(payload[0])?;
        let recommend_save_address =
            u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
        Ok(Self {
            recommend_save_address,
            position: payload[5],
            app_fw_img_info: ImgInfo::from_bytes(&payload[6..6 + IMG_INFO_LEN])?,
        })
    }
}

/// 从 SCA 读取的镜像信息列表（仅 DFU V1）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImgInfoList {
    /// 列表以加密形式存放。
    pub encrypted: bool,
    /// 有效镜像信息条目。
    pub images: Vec<ImgInfo>,
}

impl ImgInfoList {
    /// Bootloader 存放的镜像信息槽位数。
    pub const SLOTS: usize = 10;

    /// 解析包含十个镜像信息条目的 `SYSTEM_CONFIG` 响应。
    pub fn from_response(payload: &[u8], expected_address: u32) -> Result<Self> {
        let body = ConfigResponse::parse(payload)?;
        body.ensure_status()?;
        body.ensure_address(expected_address)?;
        if usize::from(body.length) != IMG_INFO_LEN * Self::SLOTS {
            return Err(DfuError::Protocol(format!(
                "unexpected image list length {}",
                body.length
            )));
        }
        let mut images = Vec::with_capacity(Self::SLOTS);
        for chunk in body.data.chunks_exact(IMG_INFO_LEN) {
            let info = ImgInfo::from_bytes(chunk)?;
            if info.pattern == VALID_PATTERN {
                images.push(info);
            }
        }
        Ok(Self {
            encrypted: body.op & 0xF0 != 0,
            images,
        })
    }
}

/// `SYSTEM_CONFIG` 响应的公共形态。
struct ConfigResponse<'a> {
    status: u8,
    op: u8,
    address: u32,
    length: u16,
    data: &'a [u8],
}

impl<'a> ConfigResponse<'a> {
    fn parse(payload: &'a [u8]) -> Result<Self> {
        if payload.len() < 9 {
            return Err(DfuError::Protocol(format!(
                "SYSTEM_CONFIG response is too short: {} bytes",
                payload.len()
            )));
        }
        let length = u16::from_le_bytes([payload[6], payload[7]]);
        let data_start = 8;
        let data_end = data_start + usize::from(length);
        if payload.len() < data_end {
            return Err(DfuError::Protocol(format!(
                "SYSTEM_CONFIG declares {length} data bytes but only {} are present",
                payload.len() - data_start
            )));
        }
        Ok(Self {
            status: payload[0],
            op: payload[1],
            address: u32::from_le_bytes([payload[2], payload[3], payload[4], payload[5]]),
            length,
            data: &payload[data_start..data_end],
        })
    }

    fn ensure_status(&self) -> Result<()> {
        ensure_status(self.status)
    }

    fn ensure_address(&self, expected: u32) -> Result<()> {
        if self.address != expected {
            return Err(DfuError::Protocol(format!(
                "SYSTEM_CONFIG returned address 0x{:08X}, expected 0x{expected:08X}",
                self.address
            )));
        }
        Ok(())
    }
}

/// 校验响应载荷的状态字节。
pub fn ensure_status(status: u8) -> Result<()> {
    if status == 1 {
        Ok(())
    } else {
        Err(DfuError::Device(format!("device status = {status}")))
    }
}
