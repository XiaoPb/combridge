use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use zip::ZipArchive;

use crate::error::ComBridgeError;

/// zip 文件本身的上限，防止误选超大压缩包。
pub const MAX_PACKAGE_BYTES: u64 = 32 * 1024 * 1024;

/// 协议无关的 zip 固件包容器：只做容器级校验与条目读取，不解释包语义。
pub struct ZipPackage {
    path: PathBuf,
    file_name: String,
    file_size: u64,
    archive: ZipArchive<File>,
}

impl ZipPackage {
    pub fn open(path: &Path) -> Result<Self, ComBridgeError> {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("firmware.zip")
            .to_string();

        if !file_name.to_ascii_lowercase().ends_with(".zip") {
            return Err(ComBridgeError::ota("只支持 .zip 固件包，请先打包再升级"));
        }

        let metadata = std::fs::metadata(path)
            .map_err(|e| ComBridgeError::ota(format!("读取固件包失败 {}: {e}", path.display())))?;
        if !metadata.is_file() {
            return Err(ComBridgeError::ota("固件包路径不是文件"));
        }
        if metadata.len() > MAX_PACKAGE_BYTES {
            return Err(ComBridgeError::ota(format!(
                "固件包过大: {} 字节（上限 {} 字节）",
                metadata.len(),
                MAX_PACKAGE_BYTES
            )));
        }

        let file =
            File::open(path).map_err(|e| ComBridgeError::ota(format!("打开固件包失败: {e}")))?;
        let archive = ZipArchive::new(file)
            .map_err(|e| ComBridgeError::ota(format!("固件包不是有效的 zip: {e}")))?;

        Ok(Self {
            path: path.to_path_buf(),
            file_name,
            file_size: metadata.len(),
            archive,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    /// 返回所有文件条目名（忽略目录、`__MACOSX/` 与隐藏文件）。
    pub fn entries(&self) -> Vec<String> {
        self.archive
            .file_names()
            .filter(|name| is_visible_entry(name))
            .map(str::to_string)
            .collect()
    }

    /// 读取条目内容；`max_bytes` 为允许的最大解压尺寸（防 zip 炸弹）。
    pub fn read_entry(&mut self, name: &str, max_bytes: u64) -> Result<Vec<u8>, ComBridgeError> {
        let entry = self
            .archive
            .by_name(name)
            .map_err(|e| ComBridgeError::ota(format!("读取 zip 条目 {name} 失败: {e}")))?;

        if entry.size() > max_bytes {
            return Err(ComBridgeError::ota(format!(
                "条目 {name} 解压后超过 {max_bytes} 字节"
            )));
        }

        let mut data = Vec::with_capacity(entry.size() as usize);
        entry
            .take(max_bytes + 1)
            .read_to_end(&mut data)
            .map_err(|e| ComBridgeError::ota(format!("解压 {name} 失败: {e}")))?;
        if data.len() as u64 > max_bytes {
            return Err(ComBridgeError::ota(format!(
                "条目 {name} 解压后超过 {max_bytes} 字节"
            )));
        }

        Ok(data)
    }
}

fn is_visible_entry(name: &str) -> bool {
    !name.ends_with('/')
        && !name.starts_with("__MACOSX/")
        && !name.contains("/.")
        && !name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[test]
    fn rejects_non_zip_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("firmware.bin");
        std::fs::write(&path, [0u8; 8]).expect("write bin");
        assert!(ZipPackage::open(&path).is_err());
    }

    #[test]
    fn rejects_invalid_zip_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fake.zip");
        std::fs::write(&path, b"not a zip").expect("write");
        assert!(ZipPackage::open(&path).is_err());
    }

    #[test]
    fn lists_and_reads_entries() {
        let zip = make_zip(&[("manifest.json", b"{}"), ("app.bin", &[1, 2, 3])]);
        let mut package = ZipPackage::open(zip.path()).expect("open");
        assert_eq!(
            package.entries(),
            vec!["manifest.json".to_string(), "app.bin".to_string()]
        );
        assert_eq!(
            package.read_entry("app.bin", 16).expect("read"),
            vec![1, 2, 3]
        );
        assert!(package.read_entry("app.bin", 2).is_err());
    }
}
