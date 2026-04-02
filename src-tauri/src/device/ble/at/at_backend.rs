use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
    transport: Option<AtTransport>,
    cache: Arc<AtCache>,
    notify_callbacks: Mutex<HashMap<String, NotifyCallback>>,
    configured: bool,
}

impl AtBleBackend {
    pub fn new() -> Self {
        Self {
            transport: None,
            cache: Arc::new(AtCache::new()),
            notify_callbacks: Mutex::new(HashMap::new()),
            configured: false,
        }
    }

    pub fn with_transport(transport: AtTransport) -> Self {
        Self {
            transport: Some(transport),
            cache: Arc::new(AtCache::new()),
            notify_callbacks: Mutex::new(HashMap::new()),
            configured: true,
        }
    }

    fn transport_mut(&mut self) -> Result<&mut AtTransport> {
        self.transport.as_mut().ok_or_else(|| {
            ComBridgeError::ble("AT传输层未初始化")
        })
    }

    fn send_and_wait(&mut self, command: &AtCommand, timeout_ms: u64) -> Result<Vec<String>> {
        let transport = self.transport_mut()?;
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
}

#[async_trait]
impl BleBackend for AtBleBackend {
    async fn configure(&mut self) -> Result<()> {
        let transport = self.transport_mut()?;
        transport.send_command(&AtCommand::Test)?;
        let responses = transport.read_response(Some(1000))?;

        Self::parse_ok_response(&responses)?;
        self.configured = true;
        info!("AT BLE后端配置成功");
        Ok(())
    }

    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        let transport = self_mut.transport_mut()?;
        
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

        Self::parse_ok_response(&responses)?;
        info!("扫描完成，发现 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn connect(&self, address: &str) -> Result<BleConnection> {
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
            &AtCommand::Connect { address: address.to_string() },
            10000
        )?;

        let connection = BleConnection {
            address: address.to_string(),
            name: self.cache.get_device(address).and_then(|d| d.name),
            connected: true,
        };

        Self::parse_ok_response(&responses)?;
        info!("已连接到设备: {}", address);
        Ok(connection)
    }

    async fn disconnect(&self, address: &str) -> Result<()> {
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
        Ok(devices.into_iter().map(|(addr, cache)| {
            BleConnection {
                address: addr,
                name: cache.name,
                connected: true,
            }
        }).collect())
    }

    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
                            BleCharacteristic {
                                uuid: c.uuid.clone(),
                                service_uuid: c.service_uuid,
                                properties: BleCharacteristicProperties {
                                    read: c.can_read(),
                                    write: c.can_write(),
                                    write_without_response: c.can_write(),
                                    notify: c.can_notify(),
                                    indicate: c.can_indicate(),
                                },
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
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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

    async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
        let mut self_mut = unsafe { &mut *(self as *const Self as *mut Self) };
        
        let responses = self_mut.send_and_wait(
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
}

impl Default for AtBleBackend {
    fn default() -> Self {
        Self::new()
    }
}
