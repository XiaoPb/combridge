use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleBackend, BleDevice, BleConnection, BleService, BleCharacteristic,
    BleCharacteristicProperties, NotifyCallback,
};
use super::at_commands::AtCommand;
use super::at_parser::AtParser;
use super::at_transport::AtTransport;
use super::at_cache::AtCache;

pub struct AtBleBackend {
    transport: Arc<Mutex<Option<AtTransport>>>,
    cache: Arc<AtCache>,
    notify_callbacks: Arc<Mutex<HashMap<String, NotifyCallback>>>,
    configured: Arc<Mutex<bool>>,
    notify_thread_running: Arc<AtomicBool>,
    notify_thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
}

impl AtBleBackend {
    pub fn new() -> Self {
        Self {
            transport: Arc::new(Mutex::new(None)),
            cache: Arc::new(AtCache::new()),
            notify_callbacks: Arc::new(Mutex::new(HashMap::new())),
            configured: Arc::new(Mutex::new(false)),
            notify_thread_running: Arc::new(AtomicBool::new(false)),
            notify_thread_handle: Arc::new(Mutex::new(None)),
        }
    }

    pub fn with_transport(transport: AtTransport) -> Self {
        Self {
            transport: Arc::new(Mutex::new(Some(transport))),
            cache: Arc::new(AtCache::new()),
            notify_callbacks: Arc::new(Mutex::new(HashMap::new())),
            configured: Arc::new(Mutex::new(true)),
            notify_thread_running: Arc::new(AtomicBool::new(false)),
            notify_thread_handle: Arc::new(Mutex::new(None)),
        }
    }

    fn send_and_wait(&self, command: &AtCommand, timeout_ms: u64) -> Result<Vec<String>> {
        let mut transport_guard = self.transport.lock().unwrap();
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;
        
        transport.send_command(command)?;
        transport.read_response(Some(timeout_ms))
    }

    fn parse_ok_response(responses: &[String]) -> Result<()> {
        if responses.is_empty() {
            return Err(ComBridgeError::ble("未收到响应"));
        }

        let last = responses.last().unwrap();
        if last == "OK" {
            Ok(())
        } else if last.starts_with("ERROR") {
            Err(ComBridgeError::ble(format!("AT指令错误: {}", last)))
        } else {
            Err(ComBridgeError::ble(format!("未知响应: {}", last)))
        }
    }

    fn make_callback_key(address: &str, char_uuid: &str) -> String {
        format!("{}:{}", address, char_uuid)
    }

    fn start_notify_thread(&self) {
        if self.notify_thread_running.load(Ordering::SeqCst) {
            return;
        }

        self.notify_thread_running.store(true, Ordering::SeqCst);
        let running = self.notify_thread_running.clone();
        let transport = self.transport.clone();
        let callbacks = self.notify_callbacks.clone();
        let parser = AtParser::new();

        let handle = thread::spawn(move || {
            info!("BLE通知监听线程已启动");
            let parser = parser;

            while running.load(Ordering::SeqCst) {
                let notify_result = {
                    let mut transport_guard = transport.lock().unwrap();
                    if let Some(ref mut t) = *transport_guard {
                        t.read_notify()
                    } else {
                        break;
                    }
                };

                match notify_result {
                    Ok(Some(line)) => {
                        debug!("收到通知数据: {}", line);
                        if let Ok(response) = parser.parse_response(&line) {
                            if let super::at_commands::AtResponse::Notify { address, char_uuid, data } = response {
                                let key = format!("{}:{}", address, char_uuid);
                                if let Some(callback) = callbacks.lock().unwrap().get(&key) {
                                    callback(&address, &char_uuid, &data);
                                }
                            }
                        }
                    }
                    Ok(None) => {
                        thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        warn!("读取通知失败: {}", e);
                        thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }

            info!("BLE通知监听线程已停止");
        });

        *self.notify_thread_handle.lock().unwrap() = Some(handle);
    }

    fn stop_notify_thread(&self) {
        self.notify_thread_running.store(false, Ordering::SeqCst);

        if let Some(handle) = self.notify_thread_handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

impl Default for AtBleBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AtBleBackend {
    fn drop(&mut self) {
        self.stop_notify_thread();
    }
}

#[async_trait]
impl BleBackend for AtBleBackend {
    async fn configure(&mut self) -> Result<()> {
        let mut transport_guard = self.transport.lock().unwrap();
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;
        
        transport.send_command(&AtCommand::Test)?;
        let responses = transport.read_response(Some(1000))?;

        Self::parse_ok_response(&responses)?;
        *self.configured.lock().unwrap() = true;
        drop(transport_guard);
        
        self.start_notify_thread();
        info!("AT BLE后端配置成功");
        Ok(())
    }

    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        let mut transport_guard = self.transport.lock().unwrap();
        let transport = transport_guard.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })?;
        
        transport.send_command(&AtCommand::Scan { duration_ms })?;
        let responses = transport.read_response(Some(duration_ms + 2000))?;

        let mut devices = Vec::new();
        let parser = AtParser::new();

        for line in &responses {
            if line.starts_with("+SCAN:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::ScanResult { devices: scanned } = response {
                        for dev in scanned {
                            self.cache.update_device(&dev.address, dev.name.clone(), dev.rssi);
                            devices.push(BleDevice {
                                address: dev.address,
                                name: dev.name,
                                rssi: Some(dev.rssi),
                                is_connectable: dev.is_connectable,
                            });
                        }
                    }
                }
            }
        }

        drop(transport_guard);
        Self::parse_ok_response(&responses)?;
        info!("扫描完成，发现 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn stop_scan(&self) -> Result<Vec<BleDevice>> {
        let responses = self.send_and_wait(&AtCommand::StopScan, 3000)?;
        
        let devices = self.cache.get_all_devices();
        let devices: Vec<BleDevice> = devices.into_iter().map(|(addr, cache)| {
            BleDevice {
                address: addr,
                name: cache.name,
                rssi: Some(cache.rssi),
                is_connectable: true,
            }
        }).collect();
        
        Self::parse_ok_response(&responses)?;
        info!("停止扫描，返回 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn connect(&self, address: &str) -> Result<BleConnection> {
        let responses = self.send_and_wait(
            &AtCommand::Connect { address: address.to_string() },
            10000
        )?;

        let connection = BleConnection {
            address: address.to_string(),
            name: self.cache.get_device(address).and_then(|d| d.name),
            is_connected: true,
            services: vec![],
        };

        Self::parse_ok_response(&responses)?;
        info!("已连接到设备: {}", address);
        Ok(connection)
    }

    async fn disconnect(&self, address: &str) -> Result<()> {
        let responses = self.send_and_wait(
            &AtCommand::Disconnect { address: address.to_string() },
            5000
        )?;

        self.cache.remove_device(address);
        Self::parse_ok_response(&responses)?;
        info!("已断开设备: {}", address);
        Ok(())
    }

    async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let devices = self.cache.get_all_devices();
        Ok(devices
            .into_iter()
            .map(|(addr, cache)| {
                let services = self.cache.get_ble_services(&addr);
                BleConnection {
                    address: addr,
                    name: cache.name,
                    is_connected: true,
                    services,
                }
            })
            .collect())
    }

    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let responses = self.send_and_wait(
            &AtCommand::DiscoverServices { address: address.to_string() },
            5000
        )?;

        let mut services = Vec::new();
        let parser = AtParser::new();

        for line in &responses {
            if line.starts_with("+SRV:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::Services { services: svcs } = response {
                        services = svcs.into_iter().map(|s| BleService {
                            uuid: s.uuid,
                            primary: s.primary,
                            characteristics: vec![],
                        }).collect();
                    }
                }
            }
        }

        self.cache.update_services(address, services.iter().map(|s| super::at_commands::ServiceInfo {
            uuid: s.uuid.clone(),
            primary: s.primary,
        }).collect());

        Self::parse_ok_response(&responses)?;
        debug!("发现 {} 个服务", services.len());
        Ok(services)
    }

    async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let responses = self.send_and_wait(
            &AtCommand::DiscoverCharacteristics {
                address: address.to_string(),
                service_uuid: service_uuid.to_string(),
            },
            5000
        )?;

        let mut characteristics = Vec::new();
        let parser = AtParser::new();

        for line in &responses {
            if line.starts_with("+CHAR:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::Characteristics { characteristics: chars } = response {
                        characteristics = chars.into_iter().map(|c| {
                            let props = BleCharacteristicProperties {
                                read: c.can_read(),
                                write: c.can_write(),
                                write_without_response: c.can_write(),
                                notify: c.can_notify(),
                                indicate: c.can_indicate(),
                            };
                            BleCharacteristic {
                                uuid: c.uuid,
                                service_uuid: c.service_uuid,
                                properties: props,
                                subscribed: false,
                            }
                        }).collect();
                    }
                }
            }
        }

        self.cache.update_characteristics(address, service_uuid, 
            characteristics.iter().map(|c| super::at_commands::CharInfo {
                uuid: c.uuid.clone(),
                service_uuid: c.service_uuid.clone(),
                properties: if c.properties.read { 0x01 } else { 0 } |
                    if c.properties.write { 0x02 } else { 0 } |
                    if c.properties.notify { 0x04 } else { 0 } |
                    if c.properties.indicate { 0x08 } else { 0 },
            }).collect()
        );

        Self::parse_ok_response(&responses)?;
        debug!("发现 {} 个特征", characteristics.len());
        Ok(characteristics)
    }

    async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>> {
        let responses = self.send_and_wait(
            &AtCommand::Read {
                address: address.to_string(),
                char_uuid: char_uuid.to_string(),
            },
            5000
        )?;

        let parser = AtParser::new();
        for line in &responses {
            if line.starts_with("+READ:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::Data { data, .. } = response {
                        return Ok(data);
                    }
                }
            }
        }

        Err(ComBridgeError::ble("读取特征失败：未收到数据"))
    }

    async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let responses = self.send_and_wait(
            &AtCommand::Write {
                address: address.to_string(),
                char_uuid: char_uuid.to_string(),
                data: data.to_vec(),
            },
            5000
        )?;

        Self::parse_ok_response(&responses)?;
        Ok(())
    }

    async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let responses = self.send_and_wait(
            &AtCommand::WriteWithoutResponse {
                address: address.to_string(),
                char_uuid: char_uuid.to_string(),
                data: data.to_vec(),
            },
            3000
        )?;

        Self::parse_ok_response(&responses)?;
        debug!("无响应写入成功");
        Ok(())
    }

    async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        if !self.notify_thread_running.load(Ordering::SeqCst) {
            self.start_notify_thread();
        }

        let responses = self.send_and_wait(
            &AtCommand::Subscribe {
                address: address.to_string(),
                char_uuid: char_uuid.to_string(),
            },
            5000
        )?;

        Self::parse_ok_response(&responses)?;

        let key = Self::make_callback_key(address, char_uuid);
        self.notify_callbacks.lock().unwrap().insert(key, callback);

        info!("已订阅通知: {} / {}", address, char_uuid);
        Ok(())
    }

    async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()> {
        let responses = self.send_and_wait(
            &AtCommand::Unsubscribe {
                address: address.to_string(),
                char_uuid: char_uuid.to_string(),
            },
            5000
        )?;

        Self::parse_ok_response(&responses)?;

        let key = Self::make_callback_key(address, char_uuid);
        self.notify_callbacks.lock().unwrap().remove(&key);

        info!("已取消订阅通知: {} / {}", address, char_uuid);
        Ok(())
    }

    async fn get_rssi(&self, address: &str) -> Result<i16> {
        let responses = self.send_and_wait(
            &AtCommand::GetRssi { address: address.to_string() },
            3000
        )?;

        let parser = AtParser::new();
        for line in &responses {
            if line.starts_with("+RSSI:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::Rssi { rssi, .. } = response {
                        return Ok(rssi);
                    }
                }
            }
        }

        Err(ComBridgeError::ble("获取RSSI失败"))
    }

    async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16> {
        let responses = self.send_and_wait(
            &AtCommand::SetMtu {
                address: address.to_string(),
                mtu,
            },
            5000
        )?;

        let parser = AtParser::new();
        for line in &responses {
            if line.starts_with("+MTU:") {
                if let Ok(response) = parser.parse_response(line) {
                    if let super::at_commands::AtResponse::Mtu { mtu: actual_mtu, .. } = response {
                        info!("MTU协商成功，实际MTU: {}", actual_mtu);
                        return Ok(actual_mtu);
                    }
                }
            }
        }

        Self::parse_ok_response(&responses)?;
        info!("MTU协商完成，使用请求值: {}", mtu);
        Ok(mtu)
    }
}
