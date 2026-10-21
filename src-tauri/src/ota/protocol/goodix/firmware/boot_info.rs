//! `dfu_boot_info_t`，24 字节，定义见 `gr55xx_dfu.h`。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};

/// 序列化 boot 信息块的字节数。
pub const BOOT_INFO_LEN: usize = 24;

/// 存放于 [`ImgInfo`](super::ImgInfo) 中的 boot 信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BootInfo {
    /// 固件字节数。
    pub bin_size: u32,
    /// 固件校验和。
    pub checksum: u32,
    /// 镜像加载地址。
    pub load_addr: u32,
    /// 镜像入口地址。
    pub run_addr: u32,
    /// XIP 读命令（`0x03`、`0x0B`、`0x3B`、`0xBB`、`0x6B`、`0xEB`）。
    pub xqspi_xip_cmd: u32,
    /// 时钟频率，0 = 64 MHz ... 4 = 16 MHz。
    pub xqspi_speed: u32,
    /// 0 = XIP，1 = QSPI。
    pub code_copy_mode: u32,
    /// 系统时钟选择。
    pub system_clk: u32,
    /// 启动后校验镜像。
    pub check_image: u32,
    /// 启动延迟标志。
    pub boot_delay: u32,
    /// DAP 启动模式标志。
    pub is_dap_boot: u32,
}

impl BootInfo {
    /// 解析 24 字节。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < BOOT_INFO_LEN {
            return Err(DfuError::Firmware(format!(
                "boot info needs {BOOT_INFO_LEN} bytes, got {}",
                bytes.len()
            )));
        }
        let config = read_u32(&bytes[20..24]);
        Ok(Self {
            bin_size: read_u32(&bytes[0..4]),
            checksum: read_u32(&bytes[4..8]),
            load_addr: read_u32(&bytes[8..12]),
            run_addr: read_u32(&bytes[12..16]),
            xqspi_xip_cmd: read_u32(&bytes[16..20]),
            xqspi_speed: config & 0xF,
            code_copy_mode: (config >> 4) & 0x1,
            system_clk: (config >> 5) & 0x7,
            check_image: (config >> 8) & 0x1,
            boot_delay: (config >> 9) & 0x1,
            is_dap_boot: (config >> 10) & 0x1,
        })
    }

    /// 序列化信息块。
    pub fn to_bytes(&self) -> [u8; BOOT_INFO_LEN] {
        let config = ((self.is_dap_boot & 0x1) << 10)
            | ((self.boot_delay & 0x1) << 9)
            | ((self.check_image & 0x1) << 8)
            | ((self.system_clk & 0x7) << 5)
            | ((self.code_copy_mode & 0x1) << 4)
            | (self.xqspi_speed & 0xF);
        let mut out = [0_u8; BOOT_INFO_LEN];
        out[0..4].copy_from_slice(&self.bin_size.to_le_bytes());
        out[4..8].copy_from_slice(&self.checksum.to_le_bytes());
        out[8..12].copy_from_slice(&self.load_addr.to_le_bytes());
        out[12..16].copy_from_slice(&self.run_addr.to_le_bytes());
        out[16..20].copy_from_slice(&self.xqspi_xip_cmd.to_le_bytes());
        out[20..24].copy_from_slice(&config.to_le_bytes());
        out
    }

    /// `[addr, addr + size)` 是否与本镜像相交。
    pub fn has_overlap(&self, addr: u32, size: u32) -> bool {
        regions_overlap(self.load_addr, self.bin_size, addr, size)
    }

    /// 两个区域是否相交。
    pub fn overlaps(&self, other: &Self) -> bool {
        regions_overlap(
            self.load_addr,
            self.bin_size,
            other.load_addr,
            other.bin_size,
        )
    }
}

/// 使用 64 位算术避免溢出的半开区间相交判断。
pub fn regions_overlap(addr1: u32, size1: u32, addr2: u32, size2: u32) -> bool {
    let start1 = u64::from(addr1);
    let end1 = start1 + u64::from(size1);
    let start2 = u64::from(addr2);
    let end2 = start2 + u64::from(size2);
    end1 > start2 && start1 < end2
}

fn read_u32(bytes: &[u8]) -> u32 {
    let mut value = [0_u8; 4];
    value.copy_from_slice(&bytes[..4]);
    u32::from_le_bytes(value)
}
