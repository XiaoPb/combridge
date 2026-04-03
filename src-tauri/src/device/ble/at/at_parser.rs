use std::collections::VecDeque;

use crate::error::{ComBridgeError, Result};
use super::at_commands::{AtResponse, ScanDevice, ServiceInfo, CharInfo};

pub struct AtParser {
    buffer: VecDeque<u8>,
}

impl AtParser {
    pub fn new() -> Self {
        Self {
            buffer: VecDeque::new(),
        }
    }

    pub fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            self.buffer.push_back(byte);
        }
    }

    pub fn has_line(&self) -> bool {
        self.buffer.iter().any(|&b| b == b'\n')
    }

    pub fn read_line(&mut self) -> Option<String> {
        if !self.has_line() {
            return None;
        }

        let mut line = Vec::new();
        while let Some(byte) = self.buffer.pop_front() {
            if byte == b'\n' {
                break;
            }
            if byte != b'\r' {
                line.push(byte);
            }
        }

        String::from_utf8(line).ok()
    }

    pub fn parse_response(&self, line: &str) -> Result<AtResponse> {
        let line = line.trim();

        if line == "OK" {
            return Ok(AtResponse::Ok);
        }

        if line.starts_with("ERROR") {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            let (code, message) = if parts.len() >= 3 {
                (parts[1].trim().parse().unwrap_or(-1), parts[2].trim().to_string())
            } else if parts.len() == 2 {
                (parts[1].trim().parse().unwrap_or(-1), String::new())
            } else {
                (-1, line.to_string())
            };
            return Ok(AtResponse::Error { code, message });
        }

        if line.starts_with("+SCAN:") {
            let devices = self.parse_scan_result(line)?;
            return Ok(AtResponse::ScanResult { devices });
        }

        if line.starts_with("+CONN:") {
            let address = line.strip_prefix("+CONN:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::Connected { address });
        }

        if line.starts_with("+DISC:") {
            let address = line.strip_prefix("+DISC:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::Disconnected { address });
        }

        if line.starts_with("+SRV:") {
            let services = self.parse_services(line)?;
            return Ok(AtResponse::Services { services });
        }

        if line.starts_with("+CHAR:") {
            let characteristics = self.parse_characteristics(line)?;
            return Ok(AtResponse::Characteristics { characteristics });
        }

        if line.starts_with("+READ:") {
            return self.parse_read_response(line);
        }

        if line.starts_with("+RSSI:") {
            return self.parse_rssi_response(line);
        }

        if line.starts_with("+NOTIFY:") {
            return self.parse_notify_response(line);
        }

        if line.starts_with("+MTU:") {
            return self.parse_mtu_response(line);
        }

        Err(ComBridgeError::ble(format!("未知的AT响应: {}", line)))
    }

    fn parse_scan_result(&self, line: &str) -> Result<Vec<ScanDevice>> {
        let content = line.strip_prefix("+SCAN:").unwrap_or("");
        if content.is_empty() || content == "NONE" {
            return Ok(Vec::new());
        }

        let mut devices = Vec::new();
        for device_str in content.split('|') {
            let parts: Vec<&str> = device_str.split(',').collect();
            if parts.len() >= 2 {
                let address = parts[0].trim().to_string();
                let name = if parts.len() > 2 && !parts[1].is_empty() {
                    Some(parts[1].trim().to_string())
                } else {
                    None
                };
                let rssi = parts.last()
                    .and_then(|s| s.trim().strip_prefix('-'))
                    .and_then(|s| s.parse::<i16>().ok())
                    .map(|v| -v)
                    .unwrap_or(-100);
                
                devices.push(ScanDevice {
                    address,
                    name,
                    rssi,
                    is_connectable: true,
                });
            }
        }
        Ok(devices)
    }

    fn parse_services(&self, line: &str) -> Result<Vec<ServiceInfo>> {
        let content = line.strip_prefix("+SRV:").unwrap_or("");
        if content.is_empty() || content == "NONE" {
            return Ok(Vec::new());
        }

        let mut services = Vec::new();
        for svc_str in content.split('|') {
            let parts: Vec<&str> = svc_str.split(',').collect();
            if !parts.is_empty() {
                let uuid = parts[0].trim().to_string();
                let primary = parts.get(1).map(|s| *s == "1").unwrap_or(true);
                services.push(ServiceInfo { uuid, primary });
            }
        }
        Ok(services)
    }

    fn parse_characteristics(&self, line: &str) -> Result<Vec<CharInfo>> {
        let content = line.strip_prefix("+CHAR:").unwrap_or("");
        if content.is_empty() || content == "NONE" {
            return Ok(Vec::new());
        }

        let mut characteristics = Vec::new();
        for char_str in content.split('|') {
            let parts: Vec<&str> = char_str.split(',').collect();
            if parts.len() >= 3 {
                let uuid = parts[0].trim().to_string();
                let service_uuid = parts[1].trim().to_string();
                let properties = parts[2].trim().parse::<u8>().unwrap_or(0);
                characteristics.push(CharInfo { uuid, service_uuid, properties });
            }
        }
        Ok(characteristics)
    }

    fn parse_read_response(&self, line: &str) -> Result<AtResponse> {
        let content = line.strip_prefix("+READ:").unwrap_or("");
        let parts: Vec<&str> = content.split(',').collect();
        
        if parts.len() >= 3 {
            let address = parts[0].trim().to_string();
            let char_uuid = parts[1].trim().to_string();
            let hex_data = parts[2].trim();
            let data = self.parse_hex_data(hex_data)?;
            return Ok(AtResponse::Data { address, char_uuid, data });
        }

        Err(ComBridgeError::ble(format!("无效的READ响应: {}", line)))
    }

    fn parse_rssi_response(&self, line: &str) -> Result<AtResponse> {
        let content = line.strip_prefix("+RSSI:").unwrap_or("");
        let parts: Vec<&str> = content.split(',').collect();
        
        if parts.len() >= 2 {
            let address = parts[0].trim().to_string();
            let rssi = parts[1].trim().parse::<i16>().unwrap_or(-100);
            return Ok(AtResponse::Rssi { address, rssi });
        }

        Err(ComBridgeError::ble(format!("无效的RSSI响应: {}", line)))
    }

    fn parse_notify_response(&self, line: &str) -> Result<AtResponse> {
        let content = line.strip_prefix("+NOTIFY:").unwrap_or("");
        let parts: Vec<&str> = content.split(',').collect();
        
        if parts.len() >= 3 {
            let address = parts[0].trim().to_string();
            let char_uuid = parts[1].trim().to_string();
            let hex_data = parts[2].trim();
            let data = self.parse_hex_data(hex_data)?;
            return Ok(AtResponse::Notify { address, char_uuid, data });
        }

        Err(ComBridgeError::ble(format!("无效的NOTIFY响应: {}", line)))
    }

    fn parse_mtu_response(&self, line: &str) -> Result<AtResponse> {
        let content = line.strip_prefix("+MTU:").unwrap_or("");
        let parts: Vec<&str> = content.split(',').collect();
        
        if parts.len() >= 2 {
            let address = parts[0].trim().to_string();
            let mtu = parts[1].trim().parse::<u16>().unwrap_or(23);
            return Ok(AtResponse::Mtu { address, mtu });
        }

        Err(ComBridgeError::ble(format!("无效的MTU响应: {}", line)))
    }

    fn parse_hex_data(&self, hex: &str) -> Result<Vec<u8>> {
        (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .map_err(|e| ComBridgeError::parse(format!("十六进制解析失败: {}", e)))
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for AtParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ok() {
        let parser = AtParser::new();
        let response = parser.parse_response("OK").unwrap();
        assert_eq!(response, AtResponse::Ok);
    }

    #[test]
    fn test_parse_error() {
        let parser = AtParser::new();
        let response = parser.parse_response("ERROR:1:Timeout").unwrap();
        assert_eq!(response, AtResponse::Error { code: 1, message: "Timeout".to_string() });
    }

    #[test]
    fn test_parse_hex_data() {
        let parser = AtParser::new();
        let data = parser.parse_hex_data("48656C6C6F").unwrap();
        assert_eq!(data, b"Hello");
    }
}
