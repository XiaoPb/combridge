use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serialport::{SerialPort as SerialPortTrait, SerialPortType};
use tracing::{debug, error, info, warn};

use crate::error::{ComBridgeError, Result};
use super::serial_config::{DataBits, FlowControl, Parity, PortInfo, SerialPortConfig, StopBits};

pub struct SerialPort {
    port: Arc<Mutex<Box<dyn SerialPortTrait>>>,
    config: SerialPortConfig,
    is_open: Arc<AtomicBool>,
    read_thread: RefCell<Option<JoinHandle<()>>>,
}

impl SerialPort {
    pub fn open(config: SerialPortConfig) -> Result<Self> {
        let baud_rate: u32 = config.baud_rate.into();
        let data_bits = match config.data_bits {
            DataBits::Five => serialport::DataBits::Five,
            DataBits::Six => serialport::DataBits::Six,
            DataBits::Seven => serialport::DataBits::Seven,
            DataBits::Eight => serialport::DataBits::Eight,
        };
        let parity = match config.parity {
            Parity::None => serialport::Parity::None,
            Parity::Odd => serialport::Parity::Odd,
            Parity::Even => serialport::Parity::Even,
        };
        let stop_bits = match config.stop_bits {
            StopBits::One => serialport::StopBits::One,
            StopBits::Two => serialport::StopBits::Two,
        };
        let flow_control = match config.flow_control {
            FlowControl::None => serialport::FlowControl::None,
            FlowControl::Software => serialport::FlowControl::Software,
            FlowControl::Hardware => serialport::FlowControl::Hardware,
        };

        let port = serialport::new(&config.port_name, baud_rate)
            .data_bits(data_bits)
            .parity(parity)
            .stop_bits(stop_bits)
            .flow_control(flow_control)
            .timeout(Duration::from_millis(config.timeout_ms))
            .open()
            .map_err(|e| ComBridgeError::serial(format!("无法打开串口 {}: {}", config.port_name, e)))?;

        info!("串口 {} 已打开", config.port_name);

        Ok(Self {
            port: Arc::new(Mutex::new(port)),
            config,
            is_open: Arc::new(AtomicBool::new(true)),
            read_thread: RefCell::new(None),
        })
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }

    pub fn port_name(&self) -> &str {
        &self.config.port_name
    }

    pub fn config(&self) -> &SerialPortConfig {
        &self.config
    }

    pub fn write(&self, data: &[u8]) -> Result<usize> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let mut port = self.port.lock().unwrap();
        port.write_all(data)
            .map_err(|e| ComBridgeError::serial(format!("写入数据失败: {}", e)))?;

        port.flush()
            .map_err(|e| ComBridgeError::serial(format!("刷新缓冲区失败: {}", e)))?;

        debug!("串口 {} 写入 {} 字节", self.config.port_name, data.len());
        Ok(data.len())
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let mut port = self.port.lock().unwrap();
        let size = port
            .read(buf)
            .map_err(|e| ComBridgeError::serial(format!("读取数据失败: {}", e)))?;

        debug!("串口 {} 读取 {} 字节", self.config.port_name, size);
        Ok(size)
    }

    pub fn start_read_loop<F>(&self, mut callback: F)
    where
        F: FnMut(&str, &[u8]) + Send + 'static,
    {
        let is_open = Arc::clone(&self.is_open);
        let port = Arc::clone(&self.port);
        let port_name = self.config.port_name.clone();

        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            info!("串口 {} 读取线程已启动", port_name);

            while is_open.load(Ordering::SeqCst) {
                let result = {
                    let mut port_guard = match port.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            warn!("串口 {} 锁被占用: {}", port_name, e);
                            continue;
                        }
                    };
                    port_guard.read(&mut buffer)
                };

                match result {
                    Ok(size) if size > 0 => {
                        debug!("串口 {} 收到 {} 字节", port_name, size);
                        callback(&port_name, &buffer[..size]);
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        continue;
                    }
                    Err(e) => {
                        if is_open.load(Ordering::SeqCst) {
                            warn!("串口 {} 读取错误: {}", port_name, e);
                        }
                        break;
                    }
                }
            }

            info!("串口 {} 读取线程已退出", port_name);
        });

        *self.read_thread.borrow_mut() = Some(handle);
    }

    pub fn close(&self) -> Result<()> {
        if !self.is_open() {
            return Ok(());
        }

        self.is_open.store(false, Ordering::SeqCst);
        info!("串口 {} 正在关闭...", self.config.port_name);

        if let Some(handle) = self.read_thread.borrow_mut().take() {
            let _ = handle.join();
        }

        info!("串口 {} 已关闭", self.config.port_name);
        Ok(())
    }

    pub fn clear_buffer(&self) -> Result<()> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let port = self.port.lock().unwrap();
        port.clear(serialport::ClearBuffer::All)
            .map_err(|e| ComBridgeError::serial(format!("清除缓冲区失败: {}", e)))?;

        debug!("串口 {} 缓冲区已清除", self.config.port_name);
        Ok(())
    }

    pub fn set_timeout(&self, timeout_ms: u64) -> Result<()> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let mut port = self.port.lock().unwrap();
        port.set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|e| ComBridgeError::serial(format!("设置超时失败: {}", e)))?;

        Ok(())
    }
}

impl Drop for SerialPort {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

pub fn scan_ports() -> Result<Vec<PortInfo>> {
    let ports = serialport::available_ports()
        .map_err(|e| ComBridgeError::serial(format!("扫描串口失败: {}", e)))?;

    let port_infos: Vec<PortInfo> = ports
        .into_iter()
        .map(|port| {
            let port_type = match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    let mut type_str = "USB".to_string();
                    type_str.push_str(&format!(" VID:{:04x}", info.vid));
                    type_str.push_str(&format!(" PID:{:04x}", info.pid));
                    PortInfo {
                        name: port.port_name,
                        port_type: type_str,
                        manufacturer: info.manufacturer.clone(),
                        product: info.product.clone(),
                        serial_number: info.serial_number.clone(),
                    }
                }
                SerialPortType::BluetoothPort => PortInfo {
                    name: port.port_name,
                    port_type: "Bluetooth".to_string(),
                    manufacturer: None,
                    product: None,
                    serial_number: None,
                },
                SerialPortType::PciPort => PortInfo {
                    name: port.port_name,
                    port_type: "PCI".to_string(),
                    manufacturer: None,
                    product: None,
                    serial_number: None,
                },
                SerialPortType::Unknown => PortInfo {
                    name: port.port_name,
                    port_type: "Unknown".to_string(),
                    manufacturer: None,
                    product: None,
                    serial_number: None,
                }
            };
            port_type
        })
        .collect();

    Ok(port_infos)
}
