use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use tracing::{debug, error, info, warn};

use crate::error::{ComBridgeError, Result};
use super::serial_config::{PortInfo, SerialPortConfig};
use super::serial_port::{scan_ports, SerialPort};
use crate::device::cache::{ChannelCache, RingBufferRef, create_ring_buffer};

pub type DataCallback = Arc<dyn Fn(&str, &[u8]) + Send + Sync>;

struct SerialPortCache {
    tx_buffer: RingBufferRef,
    rx_buffer: RingBufferRef,
}

impl SerialPortCache {
    fn new() -> Self {
        Self {
            tx_buffer: create_ring_buffer(),
            rx_buffer: create_ring_buffer(),
        }
    }
}

pub struct SerialManager {
    ports: RwLock<HashMap<String, Arc<Mutex<SerialPort>>>>,
    callbacks: RwLock<HashMap<String, DataCallback>>,
    caches: RwLock<HashMap<String, SerialPortCache>>,
}

impl SerialManager {
    pub fn new() -> Self {
        Self {
            ports: RwLock::new(HashMap::new()),
            callbacks: RwLock::new(HashMap::new()),
            caches: RwLock::new(HashMap::new()),
        }
    }

    pub fn scan_ports(&self) -> Result<Vec<PortInfo>> {
        scan_ports()
    }

    pub fn open_port<F>(&self, config: SerialPortConfig, callback: F) -> Result<()>
    where
        F: Fn(&str, &[u8]) + Send + Sync + 'static,
    {
        let port_name = config.port_name.clone();

        {
            let ports = self.ports.read()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            if ports.contains_key(&port_name) {
                return Err(ComBridgeError::serial(format!(
                    "串口 {} 已经打开",
                    port_name
                )));
            }
        }

        let callback_arc: DataCallback = Arc::new(callback);
        {
            let mut callbacks = self.callbacks.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            callbacks.insert(port_name.clone(), Arc::clone(&callback_arc));
        }

        let cache = SerialPortCache::new();
        let rx_buffer = Arc::clone(&cache.rx_buffer);
        {
            let mut caches = self.caches.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            caches.insert(port_name.clone(), cache);
        }

        let port = SerialPort::open(config)?;
        
        let callback_clone = Arc::clone(&callback_arc);
        port.start_read_loop(move |name, data| {
            if let Err(e) = rx_buffer.write(data) {
                error!("写入接收缓存失败: {}", e);
            }
            callback_clone(name, data);
        });

        let port = Arc::new(Mutex::new(port));

        {
            let mut ports = self.ports.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            ports.insert(port_name.clone(), Arc::clone(&port));
        }

        info!("串口 {} 已打开并添加到管理器", port_name);
        Ok(())
    }

    pub fn close_port(&self, port_name: &str) -> Result<()> {
        let port = {
            let mut ports = self.ports.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            ports
                .remove(port_name)
                .ok_or_else(|| ComBridgeError::serial(format!("串口 {} 未打开", port_name)))?
        };

        {
            let mut callbacks = self.callbacks.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            callbacks.remove(port_name);
            debug!("已移除串口 {} 的回调", port_name);
        }

        {
            let mut caches = self.caches.write()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            caches.remove(port_name);
            debug!("已清除串口 {} 的缓存", port_name);
        }

        port.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?
            .close()?;

        info!("串口 {} 已关闭并从管理器移除", port_name);
        Ok(())
    }

    pub fn close_all_ports(&self) -> Result<()> {
        let port_names: Vec<String> = {
            let ports = self.ports.read()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
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
        let port = {
            let ports = self.ports.read()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            ports.get(port_name).map(|p| Arc::clone(p)).ok_or_else(|| ComBridgeError::serial(format!("串口 {} 未打开", port_name)))?
        };

        {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(port_name) {
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
            }
        }

        let result = {
            let port_guard = port.lock()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            port_guard.write(data)
        };
        result
    }

    pub fn is_port_open(&self, port_name: &str) -> Result<bool> {
        let ports = self.ports.read()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(ports.contains_key(port_name))
    }

    pub fn get_open_ports(&self) -> Result<Vec<String>> {
        let ports = self.ports.read()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        Ok(ports.keys().cloned().collect())
    }

    pub fn register_callback<F>(&self, port_name: &str, callback: F) -> Result<()>
    where
        F: Fn(&str, &[u8]) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        callbacks.insert(port_name.to_string(), Arc::new(callback));
        debug!("已为串口 {} 注册数据回调", port_name);
        Ok(())
    }

    pub fn unregister_callback(&self, port_name: &str) -> Result<()> {
        let mut callbacks = self.callbacks.write()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        if callbacks.remove(port_name).is_some() {
            debug!("已移除串口 {} 的数据回调", port_name);
        }
        Ok(())
    }

    pub fn clear_callbacks(&self) -> Result<()> {
        let mut callbacks = self.callbacks.write()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        callbacks.clear();
        debug!("已清除所有数据回调");
        Ok(())
    }

    pub fn get_port_config(&self, port_name: &str) -> Result<SerialPortConfig> {
        let port = {
            let ports = self.ports.read()
                .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
            ports.get(port_name).map(|p| Arc::clone(p))
                .ok_or_else(|| ComBridgeError::serial(format!("串口 {} 未打开", port_name)))?
        };
        let config = port.lock()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?
            .config()
            .clone();
        Ok(config)
    }

    pub fn get_cache(&self, port_name: &str) -> Result<ChannelCache> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        let cache = caches.get(port_name)
            .ok_or_else(|| ComBridgeError::serial(format!("串口 {} 缓存未找到", port_name)))?;
        Ok(ChannelCache {
            tx_cache: cache.tx_buffer.get_cache_data()?,
            rx_cache: cache.rx_buffer.get_cache_data()?,
        })
    }

    pub fn clear_cache(&self, port_name: &str) -> Result<bool> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        if let Some(cache) = caches.get(port_name) {
            cache.tx_buffer.clear()?;
            cache.rx_buffer.clear()?;
            debug!("已清除串口 {} 的缓存", port_name);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_cache_size(&self, port_name: &str) -> Result<Option<(usize, usize)>> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::serial(format!("锁获取失败: {}", e)))?;
        match caches.get(port_name) {
            Some(cache) => Ok(Some((cache.tx_buffer.len()?, cache.rx_buffer.len()?))),
            None => Ok(None),
        }
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
        assert!(manager.get_open_ports().unwrap().is_empty());
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

        manager.register_callback("test_port", move |_name, _data| {
            let mut count = count_clone.lock().unwrap();
            *count += 1;
        }).unwrap();

        manager.unregister_callback("test_port").unwrap();
    }
}
