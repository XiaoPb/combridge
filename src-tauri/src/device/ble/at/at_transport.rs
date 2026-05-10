use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serialport::{SerialPort as SerialPortImpl, SerialPortType};
use tracing::{debug, error, info, warn};

use super::at_commands::AtCommand;
use super::at_parser::AtParser;
use crate::error::{ComBridgeError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    AtCommand,
    Transparent,
}

pub type DataCallback = Arc<dyn Fn(&[u8]) + Send + Sync>;

pub struct AtTransport {
    port: Option<Box<dyn SerialPortImpl>>,
    parser: AtParser,
    port_name: String,
    timeout_ms: u64,
    mode: TransportMode,
    receive_thread_running: Arc<AtomicBool>,
    receive_thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    data_callback: Arc<Mutex<Option<DataCallback>>>,
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
            mode: TransportMode::AtCommand,
            receive_thread_running: Arc::new(AtomicBool::new(false)),
            receive_thread_handle: Arc::new(Mutex::new(None)),
            data_callback: Arc::new(Mutex::new(None)),
        })
    }

    pub fn is_open(&self) -> bool {
        self.port.is_some()
    }

    pub fn close(&mut self) -> Result<()> {
        self.stop_receive_thread();
        if let Some(port) = self.port.take() {
            drop(port);
            info!("AT传输层已关闭串口: {}", self.port_name);
        }
        Ok(())
    }

    pub fn mode(&self) -> TransportMode {
        self.mode
    }

    pub fn set_data_callback(&self, callback: DataCallback) {
        *self.data_callback.lock().unwrap_or_else(|e| e.into_inner()) = Some(callback);
    }

    pub fn clear_data_callback(&self) {
        *self.data_callback.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn send_command(&mut self, command: &AtCommand) -> Result<()> {
        if self.mode != TransportMode::AtCommand {
            return Err(ComBridgeError::ble("当前不在AT命令模式，无法发送AT指令"));
        }

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| ComBridgeError::ble("串口未打开"))?;

        let data = command.to_bytes();
        debug!("发送AT指令: {:?}", String::from_utf8_lossy(&data));

        port.write_all(&data)
            .map_err(|e| ComBridgeError::ble(format!("写入串口失败: {}", e)))?;

        port.flush()
            .map_err(|e| ComBridgeError::ble(format!("刷新串口失败: {}", e)))?;

        Ok(())
    }

    pub fn read_response(&mut self, timeout_ms: Option<u64>) -> Result<Vec<String>> {
        if self.mode != TransportMode::AtCommand {
            return Err(ComBridgeError::ble("当前不在AT命令模式，无法读取AT响应"));
        }

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| ComBridgeError::ble("串口未打开"))?;

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
                let last = responses
                    .last()
                    .ok_or_else(|| ComBridgeError::ble("AT响应为空"))?;
                if last == "OK" || last.starts_with("ERROR") || last.starts_with("+") {
                    if last == "OK" || last.starts_with("ERROR") {
                        break;
                    }
                }
            }
        }

        Ok(responses)
    }

    pub fn enter_transparent_mode(&mut self) -> Result<()> {
        if self.mode == TransportMode::Transparent {
            return Ok(());
        }

        self.send_command(&AtCommand::ExitToTransparent)?;
        let responses = self.read_response(Some(2000))?;

        let success = responses.iter().any(|r| r == "OK");
        if success {
            self.mode = TransportMode::Transparent;
            self.parser.clear();
            self.start_receive_thread();
            info!("已进入透传模式");
            Ok(())
        } else {
            Err(ComBridgeError::ble("进入透传模式失败"))
        }
    }

    pub fn exit_transparent_mode(&mut self) -> Result<()> {
        if self.mode == TransportMode::AtCommand {
            return Ok(());
        }

        self.stop_receive_thread();
        self.mode = TransportMode::AtCommand;
        info!("已退出透传模式");
        Ok(())
    }

    pub fn send_transparent_data(&mut self, data: &[u8]) -> Result<()> {
        if self.mode != TransportMode::Transparent {
            return Err(ComBridgeError::ble("当前不在透传模式，无法发送透传数据"));
        }

        let port = self
            .port
            .as_mut()
            .ok_or_else(|| ComBridgeError::ble("串口未打开"))?;

        debug!("发送透传数据: {} 字节", data.len());

        port.write_all(data)
            .map_err(|e| ComBridgeError::ble(format!("写入串口失败: {}", e)))?;

        port.flush()
            .map_err(|e| ComBridgeError::ble(format!("刷新串口失败: {}", e)))?;

        Ok(())
    }

    fn start_receive_thread(&mut self) {
        if self.receive_thread_running.load(Ordering::SeqCst) {
            return;
        }

        let port_clone = match self.port.as_ref().and_then(|p| p.try_clone().ok()) {
            Some(p) => p,
            None => {
                error!("无法克隆串口用于接收线程");
                return;
            }
        };

        self.receive_thread_running.store(true, Ordering::SeqCst);
        let running = self.receive_thread_running.clone();
        let port_name = self.port_name.clone();
        let data_callback = self.data_callback.clone();

        let handle = thread::spawn(move || {
            info!("透传数据接收线程已启动: {}", port_name);

            let mut port = port_clone;
            let mut buffer = [0u8; 1024];

            while running.load(Ordering::SeqCst) {
                match port.read(&mut buffer) {
                    Ok(n) => {
                        if n > 0 {
                            debug!("透传接收: {} 字节", n);
                            if let Some(callback) = data_callback
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .as_ref()
                            {
                                callback(&buffer[..n]);
                            }
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => {
                        continue;
                    }
                    Err(e) => {
                        warn!("接收线程读取错误: {}", e);
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }

            info!("透传数据接收线程已停止: {}", port_name);
        });

        *self
            .receive_thread_handle
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(handle);
    }

    fn stop_receive_thread(&self) {
        self.receive_thread_running.store(false, Ordering::SeqCst);

        if let Some(handle) = self
            .receive_thread_handle
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            let _ = handle.join();
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
        .filter_map(|p| match p.port_type {
            SerialPortType::UsbPort(_) | SerialPortType::BluetoothPort => Some(p.port_name),
            _ => None,
        })
        .collect();

    debug!("扫描到 {} 个可能的AT端口", at_ports.len());
    Ok(at_ports)
}
