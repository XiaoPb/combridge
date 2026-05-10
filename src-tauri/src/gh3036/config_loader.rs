//! GH3036 配置文件加载模块
//!
//! 本模块提供配置文件的加载和解析功能：
//! - 解析 [Register_List] 段落中的寄存器列表
//! - 提供公共接口供其他模块使用

use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct RegisterItem {
    pub addr: u16,
    pub value: u16,
}

impl RegisterItem {
    pub fn new(addr: u16, value: u16) -> Self {
        Self { addr, value }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigLoader {
    register_list: Vec<RegisterItem>,
}

impl ConfigLoader {
    pub fn new() -> Self {
        Self {
            register_list: Vec::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut loader = Self::new();
        loader.load(path)?;
        Ok(loader)
    }

    pub fn from_content(content: &str) -> Result<Self, String> {
        let mut loader = Self::new();
        loader.parse_register_list(content)?;
        Ok(loader)
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(format!("配置文件不存在: {}", path.display()));
        }

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("读取配置文件失败: {}", e))?;

        self.parse_register_list(&content)?;

        info!(
            "[ConfigLoader] 从 {} 加载了 {} 个寄存器",
            path.display(),
            self.register_list.len()
        );

        Ok(())
    }

    fn parse_register_list(&mut self, content: &str) -> Result<(), String> {
        self.register_list.clear();

        let mut in_register_list = false;
        let mut line_num = 0;

        for line in content.lines() {
            line_num += 1;
            let trimmed = line.trim();

            if trimmed.starts_with("[Register_List]") {
                in_register_list = true;
                continue;
            }

            if !in_register_list {
                continue;
            }

            if trimmed.starts_with('[')
                && trimmed.ends_with(']')
                && !trimmed.starts_with("[Register_List]")
            {
                break;
            }

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
                continue;
            }

            if trimmed.starts_with("addr") || trimmed.starts_with("address") {
                continue;
            }

            if let Some(item) = self.parse_register_line(trimmed, line_num) {
                self.register_list.push(item);
            }
        }

        if self.register_list.is_empty() {
            warn!("[ConfigLoader] 未解析到任何寄存器");
        }

        Ok(())
    }

    fn parse_register_line(&self, line: &str, line_num: usize) -> Option<RegisterItem> {
        let line = line.split("//").next()?.trim();

        let line = line.trim_end_matches(',');

        if !line.starts_with('{') || !line.ends_with('}') {
            return None;
        }

        let inner = &line[1..line.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();

        if parts.len() < 2 {
            warn!("[ConfigLoader] 第 {} 行格式错误: {}", line_num, line);
            return None;
        }

        let addr_str = parts[0]
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");
        let val_str = parts[1]
            .trim()
            .trim_start_matches("0x")
            .trim_start_matches("0X");

        let addr = u16::from_str_radix(addr_str, 16).ok()?;
        let value = u16::from_str_radix(val_str, 16).ok()?;

        Some(RegisterItem::new(addr, value))
    }

    pub fn get_register_list(&self) -> &[RegisterItem] {
        &self.register_list
    }

    pub fn get_values(&self) -> Vec<u16> {
        self.register_list.iter().map(|r| r.value).collect()
    }

    pub fn get_addr_value_pairs(&self) -> Vec<(u16, u16)> {
        self.register_list
            .iter()
            .map(|r| (r.addr, r.value))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.register_list.is_empty()
    }

    pub fn len(&self) -> usize {
        self.register_list.len()
    }

    pub fn format_for_download(&self) -> Vec<String> {
        let mut result = Vec::with_capacity(self.register_list.len() * 2);
        for r in &self.register_list {
            result.push(format!("0x{:04X}", r.addr));
            result.push(format!("0x{:04X}", r.value));
        }
        result
    }

    pub fn format_values_only(&self) -> Vec<String> {
        self.register_list
            .iter()
            .map(|r| format!("0x{:04X}", r.value))
            .collect()
    }

    pub fn format_for_display(&self) -> String {
        self.register_list
            .iter()
            .map(|r| format!("{{0x{:04X}, 0x{:04X}}}", r.addr, r.value))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_register_line() {
        let loader = ConfigLoader::new();

        let item = loader.parse_register_line("{0x0016,0x001f},// comment", 1);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.addr, 0x0016);
        assert_eq!(item.value, 0x001f);

        let item = loader.parse_register_line("{0x0020,0x2919}", 2);
        assert!(item.is_some());
        let item = item.unwrap();
        assert_eq!(item.addr, 0x0020);
        assert_eq!(item.value, 0x2919);
    }

    #[test]
    fn test_parse_register_list() {
        let content = r#"
[Register_List]
addr, value, default
{0x0016,0x001f},// comment
{0x0020,0x2919},// another comment
"#;
        let mut loader = ConfigLoader::new();
        loader.parse_register_list(content).unwrap();
        assert_eq!(loader.len(), 2);
    }

    #[test]
    fn test_format_for_download() {
        let content = r#"
[Register_List]
{0x0016,0x001f},
{0x0020,0x2919},
"#;
        let loader = ConfigLoader::from_content(content).unwrap();
        let params = loader.format_for_download();
        assert_eq!(params, vec!["0x0016", "0x001F", "0x0020", "0x2919"]);
    }
}
