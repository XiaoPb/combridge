use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

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

pub struct BleManager {
    mode: RwLock<BleMode>,
    native_backend: RwLock<Option<Arc<RwLock<NativeBleBackend>>>>,
    at_backend: RwLock<Option<Arc<RwLock<AtBleBackend>>>>,
}

impl BleManager {
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(BleMode::Native),
            native_backend: RwLock::new(None),
            at_backend: RwLock::new(None),
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
        let mut backend_guard = self.native_backend.write().await;
        
        let mut backend = NativeBleBackend::new();
        backend.configure().await?;
        
        *backend_guard = Some(Arc::new(RwLock::new(backend)));
        
        self.set_mode(BleMode::Native).await?;
        info!("原生BLE后端配置完成");
        Ok(())
    }

    pub async fn configure_at(&self, config: AtConfig) -> Result<()> {
        let transport = AtTransport::new(&config.port_name, config.baud_rate, config.timeout_ms)?;
        let backend = AtBleBackend::with_transport(transport);
        
        let mut backend_guard = self.at_backend.write().await;
        let mut backend = backend;
        backend.configure().await?;
        
        *backend_guard = Some(Arc::new(RwLock::new(backend)));
        
        self.set_mode(BleMode::At).await?;
        info!("AT BLE后端配置完成: {}", config.port_name);
        Ok(())
    }

    pub async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.scan(duration_ms).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.scan(duration_ms).await
            }
        }
    }

    pub async fn connect(&self, address: &str) -> Result<BleConnection> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.connect(address).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.connect(address).await
            }
        }
    }

    pub async fn disconnect(&self, address: &str) -> Result<()> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.disconnect(address).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.disconnect(address).await
            }
        }
    }

    pub async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.get_connections().await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.get_connections().await
            }
        }
    }

    pub async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.discover_services(address).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.discover_services(address).await
            }
        }
    }

    pub async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.discover_characteristics(address, service_uuid).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.discover_characteristics(address, service_uuid).await
            }
        }
    }

    pub async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.read_characteristic(address, char_uuid).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.read_characteristic(address, char_uuid).await
            }
        }
    }

    pub async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.write_characteristic(address, char_uuid, data).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.write_characteristic(address, char_uuid, data).await
            }
        }
    }

    pub async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.subscribe_notify(address, char_uuid, callback).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.subscribe_notify(address, char_uuid, callback).await
            }
        }
    }

    pub async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.unsubscribe_notify(address, char_uuid).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.unsubscribe_notify(address, char_uuid).await
            }
        }
    }

    pub async fn get_rssi(&self, address: &str) -> Result<i16> {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("原生BLE后端未配置")
                })?;
                backend.read().await.get_rssi(address).await
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                let backend = backend_guard.as_ref().ok_or_else(|| {
                    ComBridgeError::ble("AT BLE后端未配置")
                })?;
                backend.read().await.get_rssi(address).await
            }
        }
    }

    pub async fn is_configured(&self) -> bool {
        let mode = self.mode.read().await;
        
        match *mode {
            BleMode::Native => {
                let backend_guard = self.native_backend.read().await;
                backend_guard.is_some()
            }
            BleMode::At => {
                let backend_guard = self.at_backend.read().await;
                backend_guard.is_some()
            }
        }
    }
}

impl Default for BleManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type BleManagerRef = Arc<BleManager>;
