use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::serial_config::{PortInfo, SerialPortConfig};
use super::serial_port::{scan_ports, SerialPort};

pub type DataCallback = Arc<dyn Fn(&str, &[u8]) + Send + Sync>;

pub struct SerialManager {
    ports: RwLock<HashMap<String, Arc<Mutex<SerialPort>>>>,
    callbacks: RwLock<Vec<DataCallback>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            ports: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(Vec::new()),
        }
    }

    pub fn scan_ports(&self) -> Result<Vec<PortInfo>> {
        scan_ports()
    }

    pub fn open_port(&self, config: SerialPortConfig) -> Result<()> {
        let port_name = config.port_name.clone();

        {
            let ports = self.ports.read().unwrap();
            if ports.contains_key(&port_name) {
                return Err(ComBridgeError::serial(format!(
                    "串口 {} 已经打开",
                    port_name
                )));
            }
        }

        let mut port = SerialPort::open(config)?;

        let callbacks = self.callbacks.read().unwrap().clone();
        port.start_read_loop(move |name, data| {
            for callback in callbacks.iter() {
                callback(name, data);
            }
        });

        let port = Arc::new(Mutex::new(port));

        {
            let mut ports = self.ports.write().unwrap();
            ports.insert(port_name.clone(), Arc::clone(&port));
        }

        info!("串口 {} 已打开并添加到管理器", port_name);
        Ok(())
    }

    pub fn close_port(&self, port_name: &str) -> Result<()> {
        let port = {
            let mut ports = self.ports.write().unwrap();
            ports
                .remove(port_name)
                .ok_or_else(|| ComBridgeError::serial(format!("串口 {} 未打开", port_name)))?
        };

        let mut port = port.lock().unwrap();
        port.close()?;

        info!("串口 {} 已关闭并从管理器移除", port_name);
        Ok(())
    }

    pub fn close_all_ports(&self) -> Result<()> {
        let port_names: Vec<String> = {
            let ports = self.ports.read().unwrap();
            ports.keys().cloned().collect()
        };

        for name in port_names {
            if let Err(e) = self.close_port(&name) {
                warn!("关闭串口 {} 失败: {}", name, e);
            }
        }

        info!("所有串口已关闭");
        Ok(())
    }

    pub fn send_data(&self, port_name: &str, data: &[u8]) -> Result<usize> {
        let ports = self.ports.read().unwrap();

        let port = ports
            .get(port_name)
            .ok_or_else(|| ComBridgeError::serial(format!("串口 {} 未打开", port_name)))?;

        let mut port = port.lock().unwrap();
        port.write(data)
    }

    pub fn is_port_open(&self, port_name: &str) -> bool {
        let ports = self.ports.read().unwrap();
        ports.contains_key(port_name)
    }

    pub fn get_open_ports(&self) -> Vec<String> {
        let ports = self.ports.read().unwrap();
        ports.keys().cloned().collect()
    }

    pub fn register_callback<F>(&self, callback: F)
    where
        F: Fn(&str, &[u8]) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.push(Arc::new(callback));
        debug!("已注册数据回调，当前共 {} 个回调", callbacks.len());
    }

    pub fn clear_callbacks(&self) {
        let mut callbacks = self.callbacks.write().unwrap();
        callbacks.clear();
        debug!("已清除所有数据回调");
    }

    pub fn get_port_config(&self, port_name: &str) -> Option<SerialPortConfig> {
        let ports = self.ports.read().unwrap();
        ports.get(port_name).and_then(|p| {
            let port = p.lock().unwrap();
            Some(port.config().clone())
        })
    }
}

impl Default for SerialManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SerialManager {
    fn drop(&mut self) {
        let _ = self.close_all_ports();
    }
}

pub type SerialManagerRef = Arc<SerialManager>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_manager_new() {
        let manager = SerialManager::new();
        assert!(manager.get_open_ports().is_empty());
    }

    #[test]
    fn test_scan_ports() {
        let manager = SerialManager::new();
        let ports = manager.scan_ports().unwrap();
        println!("扫描到 {} 个串口", ports.len());
    }

    #[test]
    fn test_register_callback() {
        let manager = SerialManager::new();
        let call_count = Arc::new(Mutex::new(0));
        let count_clone = Arc::clone(&call_count);

        manager.register_callback(move |_name, _data| {
            let mut count = count_clone.lock().unwrap();
            *count += 1;
        });

        drop(manager);
    }
}
