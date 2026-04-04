use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serialport::{SerialPort as SerialPortTrait, SerialPortType};
use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::serial_config::{DataBits, FlowControl, Parity, PortInfo, SerialPortConfig, StopBits};

const READ_TIMEOUT_MS: u64 = 10;

pub struct SerialPort {
    port: Arc<Mutex<Box<dyn SerialPortTrait>>>,
    config: RwLock<SerialPortConfig>,
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
            .timeout(Duration::from_millis(READ_TIMEOUT_MS))
            .open()
            .map_err(|e| ComBridgeError::serial(format!("无法打开串口 {}: {}", config.port_name, e)))?;

        info!("串口 {} 已打开 (超时: {}ms)", config.port_name, READ_TIMEOUT_MS);

        Ok(Self {
            port: Arc::new(Mutex::new(port)),
            config: RwLock::new(config),
            is_open: Arc::new(AtomicBool::new(true)),
            read_thread: RefCell::new(None),
        })
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::SeqCst)
    }

    pub fn port_name(&self) -> String {
        self.config.read().unwrap().port_name.clone()
    }

    pub fn config(&self) -> SerialPortConfig {
        self.config.read().unwrap().clone()
    }

    pub fn write(&self, data: &[u8]) -> Result<usize> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let data_hex: String = data.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let port_name = self.port_name();
        
        match self.port.try_lock() {
            Ok(mut port) => {
                port.write_all(data)
                    .map_err(|e| ComBridgeError::serial(format!("写入数据失败: {}", e)))?;

                port.flush()
                    .map_err(|e| ComBridgeError::serial(format!("刷新缓冲区失败: {}", e)))?;

                debug!("[WRITE] 串口 {} 发送 {} 字节: {}", port_name, data.len(), data_hex);
                Ok(data.len())
            }
            Err(_) => {
                warn!("[WRITE] 串口 {} 锁被占用，等待释放...", port_name);
                let mut port = self.port.lock().unwrap();
                
                port.write_all(data)
                    .map_err(|e| ComBridgeError::serial(format!("写入数据失败: {}", e)))?;
                    
                port.flush()
                    .map_err(|e| ComBridgeError::serial(format!("刷新缓冲区失败: {}", e)))?;

                debug!("[WRITE] 串口 {} 发送 {} 字节 (等待后): {}", port_name, data.len(), data_hex);
                Ok(data.len())
            }
        }
    }

    pub fn start_read_loop<F>(&self, mut callback: F)
    where
        F: FnMut(&str, &[u8]) + Send + 'static,
    {
        let is_open = Arc::clone(&self.is_open);
        let port = Arc::clone(&self.port);
        let port_name = self.port_name();
        let pack_timeout_ms = self.config.read().unwrap().pack_timeout_ms;

        let handle = thread::spawn(move || {
            let mut buffer = [0u8; 4096];
            let mut data_buffer: Vec<u8> = Vec::new();
            let mut last_data_time: Option<std::time::Instant> = None;
            
            info!("[RECV] 串口 {} 读取线程已启动 (组包超时: {}ms)", port_name, pack_timeout_ms);

            while is_open.load(Ordering::SeqCst) {
                let result = {
                    let mut port_guard = match port.lock() {
                        Ok(guard) => guard,
                        Err(e) => {
                            warn!("[RECV] 串口 {} 锁被占用: {}", port_name, e);
                            std::thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                    };
                    port_guard.read(&mut buffer)
                };

                match result {
                    Ok(size) if size > 0 => {
                        data_buffer.extend_from_slice(&buffer[..size]);
                        last_data_time = Some(std::time::Instant::now());
                        
                        let data_hex: String = buffer[..size]
                            .iter()
                            .map(|b| format!("{:02X}", b))
                            .collect::<Vec<_>>()
                            .join(" ");
                        debug!("[RECV] 串口 {} 收到 {} 字节: {}", port_name, size, data_hex);
                    }
                    Ok(_) => {}
                    Err(ref e) if e.kind() == io::ErrorKind::TimedOut => {
                        if let Some(last_time) = last_data_time {
                            if last_time.elapsed() >= Duration::from_millis(pack_timeout_ms) && !data_buffer.is_empty() {
                                let data_hex: String = data_buffer
                                    .iter()
                                    .map(|b| format!("{:02X}", b))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                info!("[RECV] 串口 {} 组包完成，共 {} 字节: {}", port_name, data_buffer.len(), data_hex);
                                callback(&port_name, &data_buffer);
                                data_buffer.clear();
                                last_data_time = None;
                            }
                        }
                        continue;
                    }
                    Err(e) => {
                        if is_open.load(Ordering::SeqCst) {
                            warn!("[RECV] 串口 {} 读取错误: {}", port_name, e);
                        }
                        break;
                    }
                }
            }

            if !data_buffer.is_empty() {
                info!("[RECV] 串口 {} 刷新剩余缓冲数据 {} 字节", port_name, data_buffer.len());
                callback(&port_name, &data_buffer);
            }

            info!("[RECV] 串口 {} 读取线程已退出", port_name);
        });

        *self.read_thread.borrow_mut() = Some(handle);
    }

    pub fn close(&self) -> Result<()> {
        if !self.is_open() {
            return Ok(());
        }

        let port_name = self.port_name();
        self.is_open.store(false, Ordering::SeqCst);
        info!("{} 正在关闭...", port_name);

        if let Some(handle) = self.read_thread.borrow_mut().take() {
            let _ = handle.join();
        }

        info!("{} 已关闭", port_name);
        Ok(())
    }

    pub fn clear_buffer(&self) -> Result<()> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let port = self.port.lock().unwrap();
        port.clear(serialport::ClearBuffer::All)
            .map_err(|e| ComBridgeError::serial(format!("清除缓冲区失败: {}", e)))?;

        debug!("串口 {} 缓冲区已清除", self.port_name());
        Ok(())
    }

    pub fn set_timeout(&self, timeout_ms: u64) -> Result<()> {
        if !self.is_open() {
            return Err(ComBridgeError::serial("串口未打开"));
        }

        let mut port = self.port.lock().unwrap();
        port.set_timeout(Duration::from_millis(timeout_ms))
            .map_err(|e| ComBridgeError::serial(format!("设置超时失败: {}", e)))?;

        self.config.write().unwrap().timeout_ms = timeout_ms;
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
