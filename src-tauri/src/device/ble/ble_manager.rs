use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

use crate::error::{ComBridgeError, Result};
use super::ble_traits::{
    BleBackend, BleDevice, BleConnection, BleService, BleCharacteristic, NotifyCallback,
};
use super::at::at_backend::AtBleBackend;
use super::at::at_transport::AtTransport;
use super::native::native_backend::NativeBleBackend;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BleMode {
    Native,
    At,
}

impl Default for BleMode {
    fn default() -> Self {
        BleMode::Native
    }
}

impl std::fmt::Display for BleMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BleMode::Native => write!(f, "native"),
            BleMode::At => write!(f, "at"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
}

impl Default for AtConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            timeout_ms: 1000,
        }
    }
}

enum Backend {
    Native(NativeBleBackend),
    At(AtBleBackend),
}

pub struct BleManager {
    mode: RwLock<BleMode>,
    backend: RwLock<Option<Backend>>,
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,
}

impl BleManager {
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(BleMode::Native),
            backend: RwLock::new(None),
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn mode(&self) -> BleMode {
        *self.mode.read().await
    }

    pub async fn set_mode(&self, mode: BleMode) -> Result<()> {
        let mut current_mode = self.mode.write().await;
        *current_mode = mode;
        info!("BLE模式切换为: {}", mode);
        Ok(())
    }

    pub async fn configure_native(&self) -> Result<()> {
        let mut backend = NativeBleBackend::new();
        backend.configure().await?;
        
        let mut backend_guard = self.backend.write().await;
        *backend_guard = Some(Backend::Native(backend));
        
        self.set_mode(BleMode::Native).await?;
        info!("原生BLE后端配置完成");
        Ok(())
    }

    pub async fn configure_at(&self, config: AtConfig) -> Result<()> {
        let transport = AtTransport::new(&config.port_name, config.baud_rate, config.timeout_ms)?;
        let mut backend = AtBleBackend::with_transport(transport);
        backend.configure().await?;
        
        let mut backend_guard = self.backend.write().await;
        *backend_guard = Some(Backend::At(backend));
        
        self.set_mode(BleMode::At).await?;
        info!("AT BLE后端配置完成: {}", config.port_name);
        Ok(())
    }

    pub async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.scan(duration_ms).await,
            Backend::At(b) => b.scan(duration_ms).await,
        }
    }

    pub async fn stop_scan(&self) -> Result<Vec<BleDevice>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.stop_scan().await,
            Backend::At(b) => b.stop_scan().await,
        }
    }

    pub async fn connect(&self, address: &str) -> Result<BleConnection> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.connect(address).await,
            Backend::At(b) => b.connect(address).await,
        }
    }

    pub async fn disconnect(&self, address: &str) -> Result<()> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.disconnect(address).await?,
            Backend::At(b) => b.disconnect(address).await?,
        }

        let mut subscriptions = self.subscriptions.write().await;
        if subscriptions.remove(address).is_some() {
            info!("清理设备 {} 的订阅记录", address);
        }
        
        Ok(())
    }

    pub async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.get_connections().await,
            Backend::At(b) => b.get_connections().await,
        }
    }

    pub async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.discover_services(address).await,
            Backend::At(b) => b.discover_services(address).await,
        }
    }

    pub async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.discover_characteristics(address, service_uuid).await,
            Backend::At(b) => b.discover_characteristics(address, service_uuid).await,
        }
    }

    pub async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.read_characteristic(address, char_uuid).await,
            Backend::At(b) => b.read_characteristic(address, char_uuid).await,
        }
    }

    pub async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.write_characteristic(address, char_uuid, data).await,
            Backend::At(b) => b.write_characteristic(address, char_uuid, data).await,
        }
    }

    pub async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.write_without_response(address, char_uuid, data).await,
            Backend::At(b) => b.write_without_response(address, char_uuid, data).await,
        }
    }

    pub async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.subscribe_notify(address, char_uuid, callback).await?,
            Backend::At(b) => b.subscribe_notify(address, char_uuid, callback).await?,
        }

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(address.to_string())
            .or_insert_with(HashSet::new)
            .insert(char_uuid.to_string());
        info!("记录订阅状态: 设备 {}, 特征 {}", address, char_uuid);
        
        Ok(())
    }

    pub async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.unsubscribe_notify(address, char_uuid).await?,
            Backend::At(b) => b.unsubscribe_notify(address, char_uuid).await?,
        }

        let mut subscriptions = self.subscriptions.write().await;
        if let Some(chars) = subscriptions.get_mut(address) {
            chars.remove(char_uuid);
            if chars.is_empty() {
                subscriptions.remove(address);
            }
        }
        info!("移除订阅状态: 设备 {}, 特征 {}", address, char_uuid);
        
        Ok(())
    }

    pub async fn get_rssi(&self, address: &str) -> Result<i16> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.get_rssi(address).await,
            Backend::At(b) => b.get_rssi(address).await,
        }
    }

    pub async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        match backend {
            Backend::Native(b) => b.set_mtu(address, mtu).await,
            Backend::At(b) => b.set_mtu(address, mtu).await,
        }
    }

    pub async fn get_subscriptions(&self, address: &str) -> Vec<String> {
        let subscriptions = self.subscriptions.read().await;
        subscriptions
            .get(address)
            .map(|chars| chars.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn is_configured(&self) -> bool {
        self.backend.read().await.is_some()
    }
}

impl Default for BleManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type BleManagerRef = Arc<BleManager>;
