use std::fmt::Write as _;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::ComBridgeError;
use crate::ota::package::ZipPackage;
use crate::ota::protocol::FirmwareImage;
use crate::ota::types::{OtaKeyValue, OtaPackageInfo};

/// 解压后镜像的上限，同时用于防 zip 炸弹。
pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// MCUboot 镜像头 magic（小端）。
const MCUBOOT_MAGIC: [u8; 4] = [0x3D, 0xB8, 0xF3, 0x96];
const PROTOCOL_ID: &str = "smp";

#[derive(Debug, Deserialize, Default)]
struct Manifest {
    #[serde(rename = "format-version", default)]
    format_version: Option<u32>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    files: Vec<ManifestFile>,
}

#[derive(Debug, Deserialize)]
struct ManifestFile {
    #[serde(rename = "type", default)]
    file_type: Option<String>,
    #[serde(default)]
    board: Option<String>,
    #[serde(default)]
    soc: Option<String>,
    #[serde(default)]
    size: Option<u64>,
    file: String,
    #[serde(rename = "version_MCUBOOT", default)]
    version_mcuboot: Option<String>,
}

/// 校验 nrfutil 产线 zip 并提取 MCUboot 镜像：
/// 1. 容器级校验（`.zip`、≤32MB、zip 有效性）由 `ZipPackage::open` 完成
/// 2. 有 `manifest.json` 时以 `files[type=application].file` 为准，并校验 `size`
/// 3. 无 manifest 时退化为“包内唯一 `.bin`”
/// 4. 镜像必须以 MCUboot magic 开头，解压后不超过 16MB（CRC 在读满时由 zip crate 校验）
pub fn validate(package: &mut ZipPackage) -> Result<FirmwareImage, ComBridgeError> {
    let entries = package.entries();

    // 读取 manifest.json（若存在）
    let mut manifest: Option<Manifest> = None;
    if let Some(name) = entries
        .iter()
        .find(|name| name.eq_ignore_ascii_case("manifest.json"))
    {
        let bytes = package.read_entry(name, MAX_MANIFEST_BYTES)?;
        let text = String::from_utf8(bytes)
            .map_err(|e| ComBridgeError::ota(format!("manifest.json 不是有效 UTF-8: {e}")))?;
        if !text.trim().is_empty() {
            manifest =
                Some(serde_json::from_str::<Manifest>(&text).map_err(|e| {
                    ComBridgeError::ota(format!("manifest.json 不是有效 JSON: {e}"))
                })?);
        }
    }

    // 解析 manifest，确定目标镜像与展示字段
    let mut fields: Vec<OtaKeyValue> = Vec::new();
    let mut expected_file: Option<String> = None;
    let mut expected_size: Option<u64> = None;

    if let Some(manifest) = manifest {
        if manifest.format_version.is_some_and(|format| format != 1) {
            return Err(ComBridgeError::ota(format!(
                "不支持的 manifest format-version: {}",
                manifest.format_version.unwrap_or_default()
            )));
        }
        let applications: Vec<&ManifestFile> = manifest
            .files
            .iter()
            .filter(|file| file.file_type.as_deref() == Some("application"))
            .collect();
        if applications.len() > 1 {
            return Err(ComBridgeError::ota("manifest 中包含多个 application 镜像"));
        }
        let application = applications
            .first()
            .ok_or_else(|| ComBridgeError::ota("manifest 中没有 application 镜像"))?;
        expected_file = Some(application.file.clone());
        expected_size = application.size;
        push_field(&mut fields, "name", manifest.name);
        push_field(&mut fields, "board", application.board.clone());
        push_field(&mut fields, "soc", application.soc.clone());
        push_field(&mut fields, "version", application.version_mcuboot.clone());
    }

    // 确定镜像条目：manifest 指定优先，否则要求唯一 .bin
    let image_name = match &expected_file {
        Some(target) => entries
            .iter()
            .find(|name| *name == target || name.eq_ignore_ascii_case(target))
            .cloned()
            .ok_or_else(|| {
                ComBridgeError::ota(format!("manifest 指向的镜像 {target} 不在 zip 中"))
            })?,
        None => {
            let bins: Vec<&String> = entries
                .iter()
                .filter(|name| name.to_ascii_lowercase().ends_with(".bin"))
                .collect();
            match bins.as_slice() {
                [only] => (*only).clone(),
                [] => return Err(ComBridgeError::ota("固件包内没有 .bin 镜像")),
                _ => {
                    return Err(ComBridgeError::ota(
                        "固件包内存在多个 .bin，无法确定升级镜像",
                    ))
                }
            }
        }
    };

    let image = package.read_entry(&image_name, MAX_IMAGE_BYTES)?;

    if let Some(expected) = expected_size.filter(|expected| *expected != image.len() as u64) {
        return Err(ComBridgeError::ota(format!(
            "镜像大小与 manifest 不一致: manifest {expected} 字节，实际 {} 字节",
            image.len()
        )));
    }

    if !image.starts_with(&MCUBOOT_MAGIC) {
        return Err(ComBridgeError::ota(
            "镜像不是 MCUboot 签名固件（magic 不匹配），请确认使用 NCS 产物打包",
        ));
    }

    let info = OtaPackageInfo {
        path: package.path().display().to_string(),
        file_name: package.file_name().to_string(),
        package_size: package.file_size(),
        protocol: PROTOCOL_ID.to_string(),
        image_name,
        image_size: image.len() as u64,
        image_sha256: hex_lower(&Sha256::digest(&image)),
        dual_bank_address: None,
        fields,
    };

    Ok(FirmwareImage { info, image })
}

/// 协议自定义字段按 zip 中实际存在的值填充，前端按 key 动态展示。
fn push_field(fields: &mut Vec<OtaKeyValue>, key: &str, value: Option<String>) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        fields.push(OtaKeyValue {
            key: key.to_string(),
            value,
        });
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
    use crate::ota::package::ZipPackage;
    use crate::ota::protocol::FirmwareImage;
    use crate::ota::types::OtaPackageInfo;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    const MCUBOOT_IMAGE: &[u8] = &[0x3D, 0xB8, 0xF3, 0x96, 0x00, 0x00, 0x00, 0x00];

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

    /// 与真实 dfu_application.zip 的 manifest 同构
    fn manifest_json(name: &str, file: &str, size: u64) -> String {
        format!(
            r#"{{"format-version":1,"time":0,"files":[{{"type":"application","board":"l_evk_t2_nrf54l15","soc":"nrf54l15","load_address":59392,"image_index":"0","slot_index_primary":"1","slot_index_secondary":"2","version_MCUBOOT":"1.0.0+0","size":{size},"file":"{file}","modtime":0}}],"name":"{name}"}}"#
        )
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
    fn accepts_nrfutil_package() {
        let manifest = manifest_json(
            "app_chelsea_a",
            "app_chelsea_a.signed.bin",
            MCUBOOT_IMAGE.len() as u64,
        );
        let zip = make_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("app_chelsea_a.signed.bin", MCUBOOT_IMAGE),
        ]);
        let pkg = validate_zip(&zip).expect("valid package");
        assert_eq!(pkg.info.protocol, "smp");
        assert_eq!(pkg.info.image_name, "app_chelsea_a.signed.bin");
        assert_eq!(field(&pkg.info, "name"), Some("app_chelsea_a"));
        assert_eq!(field(&pkg.info, "board"), Some("l_evk_t2_nrf54l15"));
        assert_eq!(field(&pkg.info, "soc"), Some("nrf54l15"));
        assert_eq!(field(&pkg.info, "version"), Some("1.0.0+0"));
        assert_eq!(pkg.info.image_size, MCUBOOT_IMAGE.len() as u64);
        assert_eq!(pkg.image.as_slice(), MCUBOOT_IMAGE);
        assert_eq!(pkg.info.image_sha256.len(), 64);
    }

    #[test]
    fn accepts_single_bin_without_manifest() {
        let zip = make_zip(&[("zephyr.signed.bin", MCUBOOT_IMAGE)]);
        let pkg = validate_zip(&zip).expect("valid package");
        assert_eq!(pkg.info.image_name, "zephyr.signed.bin");
        assert!(field(&pkg.info, "board").is_none());
    }

    #[test]
    fn rejects_manifest_size_mismatch() {
        let manifest = manifest_json("app", "app.bin", 999_999);
        let zip = make_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("app.bin", MCUBOOT_IMAGE),
        ]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_manifest_target_missing() {
        let manifest = manifest_json("app", "missing.bin", MCUBOOT_IMAGE.len() as u64);
        let zip = make_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("other.bin", MCUBOOT_IMAGE),
        ]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_multiple_application_entries() {
        let manifest = r#"{"format-version":1,"files":[
            {"type":"application","size":8,"file":"a.bin"},
            {"type":"application","size":8,"file":"b.bin"}
        ],"name":"multi"}"#;
        let zip = make_zip(&[
            ("manifest.json", manifest.as_bytes()),
            ("a.bin", MCUBOOT_IMAGE),
            ("b.bin", MCUBOOT_IMAGE),
        ]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_missing_image() {
        let zip = make_zip(&[("readme.txt", b"hello")]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_multiple_bins_without_manifest() {
        let zip = make_zip(&[("a.bin", MCUBOOT_IMAGE), ("b.bin", MCUBOOT_IMAGE)]);
        assert!(validate_zip(&zip).is_err());
    }

    #[test]
    fn rejects_non_mcuboot_image() {
        let zip = make_zip(&[("image.bin", b"not-a-mcuboot-image")]);
        assert!(validate_zip(&zip).is_err());
    }
}
