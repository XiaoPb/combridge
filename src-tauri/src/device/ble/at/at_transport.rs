use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serialport::{SerialPort as SerialPortImpl, SerialPortType};
use tracing::{debug, error, info};

use crate::error::{ComBridgeError, Result};
use super::at_commands::AtCommand;
use super::at_parser::AtParser;

pub struct AtTransport {
    port: Option<Box<dyn SerialPortImpl>>,
    parser: AtParser,
    port_name: String,
    timeout_ms: u64,
}

impl AtTransport {
    pub fn new(port_name: &str, baud_rate: u32, timeout_ms: u64) -> Result<Self> {
        let port = serialport::new(port_name, baud_rate)
            .timeout(Duration::from_millis(timeout_ms))
            .open()
            .map_err(|e| ComBridgeError::ble(format!("无法打开串口 {}: {}", port_name, e)))?;

        info!("AT传输层已连接到串口: {}", port_name);

        Ok(Self {
            port: Some(port),
            parser: AtParser::new(),
            port_name: port_name.to_string(),
            timeout_ms,
        })
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }

    pub fn close(&mut self) -> Result<()> {
        if let Some(port) = self.port.take() {
            drop(port);
            info!("AT传输层已关闭串口: {}", self.port_name);
        }
        Ok(())
    }

    pub fn send_command(&mut self, command: &AtCommand) -> Result<()> {
        let port = self.port.as_mut().ok_or_else(|| {
            ComBridgeError::ble("串口未打开")
        })?;

        let data = command.to_bytes();
        debug!("发送AT指令: {:?}", String::from_utf8_lossy(&data));

        port.write_all(&data)
            .map_err(|e| ComBridgeError::ble(format!("写入串口失败: {}", e)))?;

        port.flush()
            .map_err(|e| ComBridgeError::ble(format!("刷新串口失败: {}", e)))?;

        Ok(())
    }

    pub fn read_response(&mut self, timeout_ms: Option<u64>) -> Result<Vec<String>> {
        let port = self.port.as_mut().ok_or_else(|| {
            ComBridgeError::ble("串口未打开")
        })?;

        let timeout = Duration::from_millis(timeout_ms.unwrap_or(self.timeout_ms));
        let start = std::time::Instant::now();
        let mut responses = Vec::new();
        let mut temp_buffer = [0u8; 1024];

        loop {
            if start.elapsed() > timeout {
                break;
            }

            match port.read(&mut temp_buffer) {
                Ok(n) => {
                    self.parser.feed(&temp_buffer[..n]);
                    while let Some(line) = self.parser.read_line() {
                        debug!("收到AT响应: {}", line);
                        responses.push(line);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    continue;
                }
                Err(e) => {
                    error!("读取串口失败: {}", e);
                    return Err(ComBridgeError::ble(format!("读取串口失败: {}", e)));
                }
            }

            if !responses.is_empty() {
                let last = responses.last().unwrap();
                if last == "OK" || last.starts_with("ERROR") {
                    break;
                }
            }
        }

        Ok(responses)
    }

    pub fn read_notify(&mut self) -> Result<Option<String>> {
        let port = self.port.as_mut().ok_or_else(|| {
            ComBridgeError::ble("串口未打开")
        })?;

        let mut temp_buffer = [0u8; 256];
        
        match port.read(&mut temp_buffer) {
            Ok(n) => {
                self.parser.feed(&temp_buffer[..n]);
                if let Some(line) = self.parser.read_line() {
                    if line.starts_with("+NOTIFY:") {
                        return Ok(Some(line));
                    }
                }
                Ok(None)
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                Ok(None)
            }
            Err(e) => {
                Err(ComBridgeError::ble(format!("读取通知失败: {}", e)))
            }
        }
    }

    pub fn port_name(&self) -> &str {
        &self.port_name
    }
}

impl Drop for AtTransport {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

pub type AtTransportRef = Arc<Mutex<AtTransport>>;

pub fn scan_at_ports() -> Result<Vec<String>> {
    let ports = serialport::available_ports()
        .map_err(|e| ComBridgeError::ble(format!("扫描串口失败: {}", e)))?;

    let at_ports: Vec<String> = ports
        .into_iter()
        .filter_map(|p| {
            match p.port_type {
                SerialPortType::UsbPort(_) | SerialPortType::BluetoothPort => {
                    Some(p.port_name)
                }
                _ => None,
            }
        })
        .collect();

    debug!("扫描到 {} 个可能的AT端口", at_ports.len());
    Ok(at_ports)
}
