//! Goodix 固件 zip 包校验。
//!
//! 沿用项目约束：只接受 `.zip`；包内必须存在**唯一**的 Goodix `.bin`
//! （含 image info trailer）。解压后上限 16MB。

use std::fmt::Write as _;

use sha2::{Digest, Sha256};

use crate::error::ComBridgeError;
use crate::ota::package::ZipPackage;
use crate::ota::protocol::FirmwareImage;
use crate::ota::types::{OtaKeyValue, OtaPackageInfo};

use super::error::DfuError;
use super::firmware::DfuImage;

/// 解压后镜像的上限，同时用于防 zip 炸弹。
pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const PROTOCOL_ID: &str = "goodix";

/// 校验 Goodix zip 包并提取带 trailer 的完整 `.bin`。
pub fn validate(package: &mut ZipPackage) -> Result<FirmwareImage, ComBridgeError> {
    let entries = package.entries();
    let bins: Vec<&String> = entries
        .iter()
        .filter(|name| name.to_ascii_lowercase().ends_with(".bin"))
        .collect();

    let image_name = match bins.as_slice() {
        [only] => (*only).clone(),
        [] => {
            return Err(ComBridgeError::ota(
                "固件包内没有 Goodix .bin 镜像（需包含 image info trailer）",
            ))
        }
        _ => return Err(ComBridgeError::ota("固件包内存在多个 .bin，无法确定升级镜像")),
    };

    let image = package.read_entry(&image_name, MAX_IMAGE_BYTES)?;
    let parsed = DfuImage::parse(image.clone()).map_err(|error| {
        let detail = match error {
            DfuError::Firmware(message) => message,
            other => other.to_string(),
        };
        ComBridgeError::ota(format!(
            "{} 不是有效的 Goodix DFU 镜像（缺少 image info trailer 或已损坏）: {detail}",
            image_name
        ))
    })?;

    let boot = parsed.boot_info();
    let mut fields = vec![
        key_value("loadAddr", format!("0x{:08X}", boot.load_addr)),
        key_value("runAddr", format!("0x{:08X}", boot.run_addr)),
        key_value("checksum", format!("0x{:08X}", boot.checksum)),
        key_value("binSize", boot.bin_size.to_string()),
        key_value("version", parsed.img_info.version.to_string()),
        key_value("signed", parsed.signed.to_string()),
        key_value("encrypted", parsed.encrypted.to_string()),
    ];
    let comments = parsed.img_info.comments_str();
    if !comments.is_empty() {
        fields.push(key_value("comments", comments));
    }

    let info = OtaPackageInfo {
        path: package.path().display().to_string(),
        file_name: package.file_name().to_string(),
        package_size: package.file_size(),
        protocol: PROTOCOL_ID.to_string(),
        image_name,
        image_size: image.len() as u64,
        image_sha256: hex_lower(&Sha256::digest(&image)),
        fields,
    };

    tracing::info!(
        image = info.image_name,
        size = info.image_size,
        load_addr = format_args!("0x{:08X}", boot.load_addr),
        checksum = format_args!("0x{:08X}", boot.checksum),
        signed = parsed.signed,
        encrypted = parsed.encrypted,
        "Goodix firmware package validated"
    );

    Ok(FirmwareImage { info, image })
}

fn key_value(key: &str, value: String) -> OtaKeyValue {
    OtaKeyValue {
        key: key.to_string(),
        value,
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ota::protocol::goodix::firmware::{BootInfo, ImgInfo};
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn make_zip(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let file = tempfile::Builder::new()
            .suffix(".zip")
            .tempfile()
            .expect("temp zip");
        {
            let mut writer = zip::ZipWriter::new(file.reopen().expect("reopen"));
            let options =
                SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            for (name, data) in entries {
                writer.start_file(*name, options).expect("start_file");
                writer.write_all(data).expect("write_all");
            }
            writer.finish().expect("finish zip");
        }
        file
    }

    fn goodix_bin(payload_len: usize) -> Vec<u8> {
        let boot = BootInfo {
            bin_size: (payload_len + 48) as u32,
            checksum: 0x1234_5678,
            load_addr: 0x0001_0000,
            run_addr: 0x0001_0101,
            xqspi_xip_cmd: 0x03,
            ..BootInfo::default()
        };
        let info = ImgInfo::new(7, boot, "unit");
        let mut data = vec![0xAA_u8; payload_len];
        data.extend_from_slice(&info.to_bytes());
        data.extend_from_slice(&[0_u8; 8]);
        data
    }

    fn field<'a>(info: &'a OtaPackageInfo, key: &str) -> Option<&'a str> {
        info.fields
            .iter()
            .find(|item| item.key == key)
            .map(|item| item.value.as_str())
    }

    fn validate_zip(zip: &tempfile::NamedTempFile) -> Result<FirmwareImage, ComBridgeError> {
        let mut package = ZipPackage::open(zip.path()).expect("open zip");
        validate(&mut package)
    }

    #[test]
    fn accepts_goodix_bin() {
        let data = goodix_bin(128);
        let zip = make_zip(&[("app.bin", &data)]);
        let firmware = validate_zip(&zip).expect("valid package");
        assert_eq!(firmware.info.protocol, "goodix");
        assert_eq!(firmware.info.image_name, "app.bin");
        assert_eq!(firmware.info.image_size, data.len() as u64);
        assert_eq!(firmware.image, data);
        assert_eq!(field(&firmware.info, "loadAddr"), Some("0x00010000"));
        assert_eq!(field(&firmware.info, "runAddr"), Some("0x00010101"));
        assert_eq!(field(&firmware.info, "checksum"), Some("0x12345678"));
        assert_eq!(field(&firmware.info, "binSize"), Some("176"));
        assert_eq!(field(&firmware.info, "version"), Some("7"));
        assert_eq!(field(&firmware.info, "signed"), Some("false"));
        assert_eq!(field(&firmware.info, "encrypted"), Some("false"));
        assert_eq!(field(&firmware.info, "comments"), Some("unit"));
        assert_eq!(firmware.info.image_sha256.len(), 64);
    }

    #[test]
    fn ignores_non_bin_entries() {
        let data = goodix_bin(32);
        let zip = make_zip(&[("readme.txt", b"hello"), ("fw/app.bin", &data)]);
        let firmware = validate_zip(&zip).expect("valid package");
        assert_eq!(firmware.info.image_name, "fw/app.bin");
    }

    #[test]
    fn rejects_missing_bin() {
        let zip = make_zip(&[("readme.txt", b"hello")]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_multiple_bins() {
        let first = goodix_bin(32);
        let second = goodix_bin(64);
        let zip = make_zip(&[("a.bin", &first), ("b.bin", &second)]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_invalid_trailer() {
        let zip = make_zip(&[("app.bin", &[0xAA_u8; 128])]);
        assert!(validate_zip(&zip).is_err());
    }
}
