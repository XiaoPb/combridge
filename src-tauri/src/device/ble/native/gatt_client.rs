use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use tracing::{debug, info};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleService, BleCharacteristic, NotifyCallback,
};

pub struct GattClient {
    address: String,
    services: RwLock<HashMap<String, BleService>>,
    characteristics: RwLock<HashMap<String, Vec<BleCharacteristic>>>,
    notify_callbacks: Mutex<HashMap<String, NotifyCallback>>,
    connected: RwLock<bool>,
}

impl GattClient {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            services: RwLock::new(HashMap::new()),
            characteristics: RwLock::new(HashMap::new()),
            notify_callbacks: Mutex::new(HashMap::new()),
            connected: RwLock::new(false),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    pub async fn connect(&self) -> Result<()> {
        info!("GATT客户端连接到: {}", self.address);
        *self.connected.write().unwrap() = true;
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        info!("GATT客户端断开: {}", self.address);
        *self.connected.write().unwrap() = false;
        self.services.write().unwrap().clear();
        self.characteristics.write().unwrap().clear();
        Ok(())
    }

    pub async fn discover_services(&self) -> Result<Vec<BleService>> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("发现服务: {}", self.address);
        let services: Vec<BleService> = Vec::new();
        
        let mut svc_map = self.services.write().unwrap();
        for svc in &services {
            svc_map.insert(svc.uuid.clone(), svc.clone());
        }

        Ok(services)
    }

    pub async fn discover_characteristics(&self, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("发现特征: {} / {}", self.address, service_uuid);
        let characteristics: Vec<BleCharacteristic> = Vec::new();

        let mut char_map = self.characteristics.write().unwrap();
        char_map.insert(service_uuid.to_string(), characteristics.clone());

        Ok(characteristics)
    }

    pub async fn read_characteristic(&self, char_uuid: &str) -> Result<Vec<u8>> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("读取特征: {}", char_uuid);
        Ok(Vec::new())
    }

    pub async fn write_characteristic(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("写入特征: {} ({} 字节)", char_uuid, data.len());
        Ok(())
    }

    pub async fn write_without_response(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("无响应写入特征: {} ({} 字节)", char_uuid, data.len());
        Ok(())
    }

    pub async fn subscribe_notify(&self, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("订阅通知: {}", char_uuid);
        self.notify_callbacks.lock().unwrap().insert(char_uuid.to_string(), callback);
        Ok(())
    }

    pub async fn unsubscribe_notify(&self, char_uuid: &str) -> Result<()> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("取消订阅通知: {}", char_uuid);
        self.notify_callbacks.lock().unwrap().remove(char_uuid);
        Ok(())
    }

    pub async fn get_rssi(&self) -> Result<i16> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        Ok(-50)
    }

    pub async fn set_mtu(&self, mtu: u16) -> Result<u16> {
        if !self.is_connected() {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        info!("MTU协商: {} (请求值: {})", self.address, mtu);
        let actual_mtu = mtu.min(517).max(23);
        info!("MTU协商完成，实际MTU: {}", actual_mtu);
        Ok(actual_mtu)
    }

    pub fn get_services(&self) -> Vec<BleService> {
        self.services.read().unwrap().values().cloned().collect()
    }

    pub fn get_characteristics(&self, service_uuid: &str) -> Option<Vec<BleCharacteristic>> {
        self.characteristics.read().unwrap().get(service_uuid).cloned()
    }
}
