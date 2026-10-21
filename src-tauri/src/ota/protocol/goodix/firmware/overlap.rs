//! 与官方 Android 实现一致的 Flash 区域重叠检查。
//!
//! DFU 绝不能擦除或覆盖 SCA、App Bootloader 或正在运行的固件
//! （使用外部 Flash 时除外）。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use super::super::error::{DfuError, Result};
use super::boot_info::regions_overlap;
use super::BootInfo;

/// 系统配置区大小。
pub const SCA_SIZE: u32 = 0x2000;

/// DFU V2（App Bootloader）方案的重叠规则检查。
#[allow(clippy::too_many_arguments)]
pub fn check_overlap_v2(
    is_update_fw: bool,
    to_ext_flash: bool,
    data_size: u32,
    write_address: u32,
    address_of_sca: u32,
    app_bootloader: &BootInfo,
    running_app_fw: Option<&BootInfo>,
    new_fw: Option<&BootInfo>,
) -> Result<()> {
    ensure_no_overlap(
        "DATA",
        write_address,
        data_size,
        "SCA",
        address_of_sca,
        SCA_SIZE,
    )?;
    ensure_no_overlap(
        "DATA",
        write_address,
        data_size,
        "BOOTLOADER",
        app_bootloader.load_addr,
        app_bootloader.bin_size,
    )?;

    if is_update_fw {
        let new_fw = new_fw.ok_or_else(|| {
            DfuError::Overlap("firmware update requires image boot info".to_string())
        })?;
        if write_address != new_fw.load_addr {
            if let Some(running) = running_app_fw {
                ensure_no_overlap(
                    "DATA",
                    write_address,
                    data_size,
                    "APP_OLD",
                    running.load_addr,
                    running.bin_size,
                )?;
            }
            ensure_no_overlap(
                "DATA",
                write_address,
                data_size,
                "APP_NEW",
                new_fw.load_addr,
                data_size,
            )?;
            ensure_no_overlap(
                "APP_NEW",
                new_fw.load_addr,
                data_size,
                "SCA",
                address_of_sca,
                SCA_SIZE,
            )?;
            ensure_no_overlap(
                "APP_NEW",
                new_fw.load_addr,
                data_size,
                "BOOTLOADER",
                app_bootloader.load_addr,
                app_bootloader.bin_size,
            )?;
        }
    } else if !to_ext_flash {
        if let Some(running) = running_app_fw {
            ensure_no_overlap(
                "DATA",
                write_address,
                data_size,
                "APP",
                running.load_addr,
                running.bin_size,
            )?;
        }
    }

    Ok(())
}

/// DFU V1 方案的重叠规则检查。
pub fn check_overlap_v1(
    is_update_fw: bool,
    to_ext_flash: bool,
    data_size: u32,
    write_address: u32,
    address_of_sca: u32,
    running_fw: &BootInfo,
    new_fw: Option<&BootInfo>,
) -> Result<()> {
    ensure_no_overlap(
        "DATA",
        write_address,
        data_size,
        "SCA",
        address_of_sca,
        SCA_SIZE,
    )?;

    if is_update_fw || !to_ext_flash {
        ensure_no_overlap(
            "DATA",
            write_address,
            data_size,
            "APP_OLD",
            running_fw.load_addr,
            running_fw.bin_size,
        )?;
    }

    if is_update_fw {
        if let Some(new_fw) = new_fw {
            if write_address != new_fw.load_addr {
                ensure_no_overlap(
                    "DATA",
                    write_address,
                    data_size,
                    "APP_NEW",
                    new_fw.load_addr,
                    data_size,
                )?;
                ensure_no_overlap(
                    "APP_NEW",
                    new_fw.load_addr,
                    data_size,
                    "SCA",
                    address_of_sca,
                    SCA_SIZE,
                )?;
            }
        }
    }

    Ok(())
}

/// 两个半开区间相交时返回错误。
pub fn ensure_no_overlap(
    name: &str,
    address: u32,
    size: u32,
    other_name: &str,
    other_address: u32,
    other_size: u32,
) -> Result<()> {
    if regions_overlap(address, size, other_address, other_size) {
        return Err(DfuError::Overlap(format!(
            "{name}{} overlaps {other_name}{}",
            format_region(address, size),
            format_region(other_address, other_size)
        )));
    }
    Ok(())
}

fn format_region(address: u32, size: u32) -> String {
    format!("<0x{address:08X} - 0x{:08X}>", address.wrapping_add(size))
}
