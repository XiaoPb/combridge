use std::collections::VecDeque;

use crate::error::{ComBridgeError, Result};
use super::at_commands::{AtResponse, ScanDevice};

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

        if line == "+SLEEP ENTRY" || line == "+ENTRY SLEEP" {
            return Ok(AtResponse::SleepEntry);
        }

        if line == "+SLEEP EXIT" {
            return Ok(AtResponse::SleepExit);
        }

        if line.starts_with("ERROR") {
            let parts: Vec<&str> = line.splitn(3, '=').collect();
            let (code, message) = if parts.len() >= 3 {
                (parts[1].trim().parse().unwrap_or(-1), parts[2].trim().to_string())
            } else if parts.len() == 2 {
                (parts[1].trim().parse().unwrap_or(-1), String::new())
            } else {
                (-1, line.to_string())
            };
            return Ok(AtResponse::Error { code, message });
        }

        if line.starts_with("+INFO:") {
            let info = line.strip_prefix("+INFO:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::Info { info });
        }

        if line.starts_with("+NAME:") {
            let name = line.strip_prefix("+NAME:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::Name { name });
        }

        if line.starts_with("+MAC:") {
            let address = line.strip_prefix("+MAC:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::Mac { address });
        }

        if line.starts_with("+MTU:") {
            let mtu_str = line.strip_prefix("+MTU:").unwrap_or("23");
            let mtu = mtu_str.trim().parse::<u16>().unwrap_or(23);
            return Ok(AtResponse::Mtu { mtu });
        }

        if line.starts_with("+TXUUID:") {
            let uuid = line.strip_prefix("+TXUUID:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::TxUuid { uuid });
        }

        if line.starts_with("+RXUUID:") {
            let uuid = line.strip_prefix("+RXUUID:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::RxUuid { uuid });
        }

        if line.starts_with("+SVRUUD:") {
            let uuid = line.strip_prefix("+SVRUUD:").unwrap_or("").trim().to_string();
            return Ok(AtResponse::SrvUuid { uuid });
        }

        if line.starts_with("+ROLE:") {
            let role_str = line.strip_prefix("+ROLE:").unwrap_or("0");
            let role = role_str.trim().parse::<u8>().unwrap_or(0);
            return Ok(AtResponse::Role { role });
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

        if line.starts_with("+BLESEND:") {
            let hex_data = line.strip_prefix("+BLESEND:").unwrap_or("").trim();
            let data = self.parse_hex_data(hex_data)?;
            return Ok(AtResponse::Data { data });
        }

        if line.starts_with("+RSSI:") {
            return self.parse_rssi_response(line);
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
                });
            }
        }
        Ok(devices)
    }

    fn parse_rssi_response(&self, line: &str) -> Result<AtResponse> {
        let content = line.strip_prefix("+RSSI:").unwrap_or("");
        let parts: Vec<&str> = content.split(',').collect();
        
        if parts.len() >= 1 {
            let rssi_hex = parts[0].trim();
            if let Ok(rssi_byte) = u8::from_str_radix(rssi_hex, 16) {
                let rssi = rssi_byte as i8 as i16;
                return Ok(AtResponse::Rssi { rssi });
            }
        }

        Err(ComBridgeError::ble(format!("无效的RSSI响应: {}", line)))
    }

    pub fn parse_hex_data(&self, hex: &str) -> Result<Vec<u8>> {
        if hex.is_empty() {
            return Ok(Vec::new());
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| {
                if i + 2 <= hex.len() {
                    u8::from_str_radix(&hex[i..i + 2], 16)
                        .map_err(|e| ComBridgeError::parse(format!("十六进制解析失败: {}", e)))
                } else {
                    Err(ComBridgeError::parse("十六进制数据不完整"))
                }
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }

    pub fn read_all(&mut self) -> Vec<u8> {
        self.buffer.drain(..).collect()
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
        let response = parser.parse_response("ERROR=1:Timeout").unwrap();
        assert_eq!(response, AtResponse::Error { code: 1, message: "Timeout".to_string() });
    }

    #[test]
    fn test_parse_hex_data() {
        let parser = AtParser::new();
        let data = parser.parse_hex_data("48656C6C6F").unwrap();
        assert_eq!(data, b"Hello");
    }

    #[test]
    fn test_parse_scan_result() {
        let parser = AtParser::new();
        let response = parser.parse_response("+SCAN:112233445566,Device1,-50|778899AABBCC,Device2,-60").unwrap();
        if let AtResponse::ScanResult { devices } = response {
            assert_eq!(devices.len(), 2);
            assert_eq!(devices[0].address, "112233445566");
            assert_eq!(devices[0].name, Some("Device1".to_string()));
            assert_eq!(devices[0].rssi, -50);
        } else {
            panic!("Expected ScanResult");
        }
    }

    #[test]
    fn test_parse_connected() {
        let parser = AtParser::new();
        let response = parser.parse_response("+CONN:112233445566").unwrap();
        assert_eq!(response, AtResponse::Connected { address: "112233445566".to_string() });
    }

    #[test]
    fn test_parse_rssi() {
        let parser = AtParser::new();
        let response = parser.parse_response("+RSSI:C8").unwrap();
        if let AtResponse::Rssi { rssi } = response {
            assert_eq!(rssi, -56);
        } else {
            panic!("Expected Rssi");
        }
    }
}
