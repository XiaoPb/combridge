//! 通用小工具。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

/// 递增 MAC 地址的最低字节。
///
/// App Bootloader 在单 Bank 跳转后以 `MAC + 1` 广播。
pub fn bump_last_mac_byte(address: &str) -> Option<String> {
    let parts: Vec<&str> = address.split(':').collect();
    if parts.len() != 6 {
        return None;
    }
    let mut bytes = [0_u8; 6];
    for (slot, part) in bytes.iter_mut().zip(parts) {
        *slot = u8::from_str_radix(part, 16).ok()?;
    }
    bytes[5] = bytes[5].wrapping_add(1);
    Some(
        bytes
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}

/// 解析可能带 `0x` 前缀的十六进制地址。
pub fn parse_hex_u32(value: &str) -> Option<u32> {
    let trimmed = value
        .trim()
        .strip_prefix("0x")
        .or_else(|| value.trim().strip_prefix("0X"))
        .unwrap_or_else(|| value.trim());
    u32::from_str_radix(trimmed, 16).ok()
}
