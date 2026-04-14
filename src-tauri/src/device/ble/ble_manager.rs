use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::error::{ComBridgeError, Result};
use crate::service::event_bus::{EventBus, BleDataEvent, BleConnectionEvent};
use crate::service::event_bus::topics;
use super::ble_traits::{
    BleBackend, BleDevice, BleConnection, BleService, BleCharacteristic, NotifyCallback,
};
use super::at::at_backend::AtBleBackend;
use super::at::at_commands::AtConnectionConfig;
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
    pub tx_uuid: Option<String>,
    pub rx_uuid: Option<String>,
    pub srv_uuid: Option<String>,
}

impl Default for AtConfig {
    fn default() -> Self {
        Self {
            port_name: String::new(),
            baud_rate: 115200,
            timeout_ms: 1000,
            tx_uuid: None,
            rx_uuid: None,
            srv_uuid: None,
        }
    }
}

impl From<&AtConfig> for AtConnectionConfig {
    fn from(config: &AtConfig) -> Self {
        AtConnectionConfig {
            tx_uuid: config.tx_uuid.clone(),
            rx_uuid: config.rx_uuid.clone(),
            srv_uuid: config.srv_uuid.clone(),
            mtu: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtConnectionTab {
    pub id: String,
    pub address: String,
    pub name: Option<String>,
    pub tx_uuid: String,
    pub rx_uuid: String,
    pub connected_at: u64,
    pub received_data: Vec<DataEntry>,
    pub sent_data: Vec<DataEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataEntry {
    pub id: String,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub direction: String,
}

enum Backend {
    Native(NativeBleBackend),
    At(AtBleBackend),
}

pub struct BleManager {
    mode: RwLock<BleMode>,
    backend: RwLock<Option<Backend>>,
    subscriptions: RwLock<HashMap<String, HashSet<String>>>,
    at_tabs: RwLock<HashMap<String, AtConnectionTab>>,
    at_config: RwLock<AtConfig>,
    event_bus: Arc<EventBus>,
}

impl BleManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            mode: RwLock::new(BleMode::Native),
            backend: RwLock::new(None),
            subscriptions: RwLock::new(HashMap::new()),
            at_tabs: RwLock::new(HashMap::new()),
            at_config: RwLock::new(AtConfig::default()),
            event_bus,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!("初始化 BLE 后端（默认原生模式）");
        match self.configure_native().await {
            Ok(()) => {
                info!("BLE 原生后端初始化成功");
                Ok(())
            }
            Err(e) => {
                error!("BLE 原生后端初始化失败: {}", e);
                warn!("原生BLE不可用，将使用AT模式作为后备");
                Ok(())
            }
        }
    }

    pub async fn mode(&self) -> BleMode {
        *self.mode.read().await
    }

    pub async fn set_mode(&self, mode: BleMode) -> Result<()> {
        match mode {
            BleMode::Native => {
                match self.configure_native().await {
                    Ok(()) => {
                        info!("BLE模式切换为: {}", mode);
                        Ok(())
                    }
                    Err(e) => {
                        error!("切换到原生BLE模式失败: {}", e);
                        warn!("回退到AT模式");
                        let config = self.at_config.read().await.clone();
                        self.configure_at(config).await?;
                        info!("BLE模式回退为: AT (原生不可用)");
                        Ok(())
                    }
                }
            }
            BleMode::At => {
                let config = self.at_config.read().await.clone();
                self.configure_at(config).await?;
                info!("BLE模式切换为: {}", mode);
                Ok(())
            }
        }
    }

    pub async fn configure_native(&self) -> Result<()> {
        let mut backend = NativeBleBackend::new();
        backend.configure().await?;
        
        let mut backend_guard = self.backend.write().await;
        *backend_guard = Some(Backend::Native(backend));
        
        *self.mode.write().await = BleMode::Native;
        info!("原生BLE后端配置完成");
        Ok(())
    }

    pub async fn configure_at(&self, config: AtConfig) -> Result<()> {
        let transport = AtTransport::new(&config.port_name, config.baud_rate, config.timeout_ms)?;
        let connection_config = AtConnectionConfig::from(&config);
        let mut backend = AtBleBackend::with_config(transport, connection_config);
        backend.configure().await?;
        
        *self.at_config.write().await = config;
        
        let mut backend_guard = self.backend.write().await;
        *backend_guard = Some(Backend::At(backend));
        
        *self.mode.write().await = BleMode::At;
        info!("AT BLE后端配置完成: {}", self.at_config.read().await.port_name);
        Ok(())
    }

    pub async fn get_at_config(&self) -> AtConfig {
        self.at_config.read().await.clone()
    }

    pub async fn update_at_uuid_config(&self, tx_uuid: Option<String>, rx_uuid: Option<String>, srv_uuid: Option<String>) {
        let mut config = self.at_config.write().await;
        if let Some(tx) = tx_uuid {
            config.tx_uuid = Some(tx);
        }
        if let Some(rx) = rx_uuid {
            config.rx_uuid = Some(rx);
        }
        if let Some(srv) = srv_uuid {
            config.srv_uuid = Some(srv);
        }
        info!("AT UUID配置已更新");
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
        
        let connection = match backend {
            Backend::Native(b) => b.connect(address).await?,
            Backend::At(b) => b.connect(address).await?,
        };

        let mode = self.mode.read().await;
        if *mode == BleMode::At {
            let config = self.at_config.read().await;
            let tx_uuid = config.tx_uuid.clone().ok_or_else(|| {
                ComBridgeError::ble("AT模式连接需要配置tx_uuid")
            })?;
            let rx_uuid = config.rx_uuid.clone().ok_or_else(|| {
                ComBridgeError::ble("AT模式连接需要配置rx_uuid")
            })?;
            let tab = AtConnectionTab {
                id: format!("at-{}-{}", address, chrono::Utc::now().timestamp()),
                address: address.to_string(),
                name: connection.name.clone(),
                tx_uuid,
                rx_uuid,
                connected_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                received_data: Vec::new(),
                sent_data: Vec::new(),
            };
            self.at_tabs.write().await.insert(tab.id.clone(), tab);
            info!("创建AT连接TAB: {}", address);
        }

        let event = BleConnectionEvent::new(address, connection.name.clone());
        self.event_bus.publish_typed(topics::BLE_CONNECTED, &event);
        info!("BLE设备已连接: {}", address);
        
        Ok(connection)
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

        let mut tabs = self.at_tabs.write().await;
        tabs.retain(|_, tab| tab.address != address);

        let event = BleConnectionEvent::new(address, None);
        self.event_bus.publish_typed(topics::BLE_DISCONNECTED, &event);
        info!("BLE设备已断开: {}", address);
        
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
        
        let mut services = match backend {
            Backend::Native(b) => b.discover_services(address).await?,
            Backend::At(b) => b.discover_services(address).await?,
        };

        let subscriptions = self.subscriptions.read().await;
        if let Some(subscribed_chars) = subscriptions.get(address) {
            for service in &mut services {
                for char in &mut service.characteristics {
                    if subscribed_chars.contains(&char.uuid) {
                        char.subscribed = true;
                    }
                }
            }
        }
        
        Ok(services)
    }

    pub async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let backend_guard = self.backend.read().await;
        let backend = backend_guard.as_ref().ok_or_else(|| {
            ComBridgeError::ble("BLE后端未配置")
        })?;
        
        let mut characteristics = match backend {
            Backend::Native(b) => b.discover_characteristics(address, service_uuid).await?,
            Backend::At(b) => b.discover_characteristics(address, service_uuid).await?,
        };

        let subscriptions = self.subscriptions.read().await;
        if let Some(subscribed_chars) = subscriptions.get(address) {
            for char in &mut characteristics {
                if subscribed_chars.contains(&char.uuid) {
                    char.subscribed = true;
                }
            }
        }
        
        Ok(characteristics)
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

        let event_bus = self.event_bus.clone();
        let address_owned = address.to_string();
        let wrapped_callback = Arc::new(move |addr: &str, char: &str, data: &[u8]| {
            let event = BleDataEvent::new(
                &address_owned,
                addr,
                char,
                data.to_vec(),
            );
            event_bus.publish_msgpack(topics::BLE_DATA, &event);

            callback(addr, char, data);
        });
        
        match backend {
            Backend::Native(b) => b.subscribe_notify(address, char_uuid, wrapped_callback).await?,
            Backend::At(b) => b.subscribe_notify(address, char_uuid, wrapped_callback).await?,
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

    pub async fn get_at_tabs(&self) -> Vec<AtConnectionTab> {
        self.at_tabs.read().await.values().cloned().collect()
    }

    pub async fn get_at_tab(&self, tab_id: &str) -> Option<AtConnectionTab> {
        self.at_tabs.read().await.get(tab_id).cloned()
    }

    pub async fn add_at_received_data(&self, address: &str, data: Vec<u8>) {
        let mut tabs = self.at_tabs.write().await;
        for tab in tabs.values_mut() {
            if tab.address == address {
                tab.received_data.push(DataEntry {
                    id: format!("rx-{}", chrono::Utc::now().timestamp_millis()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    data,
                    direction: "receive".to_string(),
                });
                if tab.received_data.len() > 1000 {
                    tab.received_data = tab.received_data.split_off(tab.received_data.len() - 1000);
                }
                break;
            }
        }
    }

    pub async fn add_at_sent_data(&self, address: &str, data: Vec<u8>) {
        let mut tabs = self.at_tabs.write().await;
        for tab in tabs.values_mut() {
            if tab.address == address {
                tab.sent_data.push(DataEntry {
                    id: format!("tx-{}", chrono::Utc::now().timestamp_millis()),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    data,
                    direction: "send".to_string(),
                });
                if tab.sent_data.len() > 1000 {
                    tab.sent_data = tab.sent_data.split_off(tab.sent_data.len() - 1000);
                }
                break;
            }
        }
    }

    pub async fn clear_at_tab_data(&self, tab_id: &str) {
        let mut tabs = self.at_tabs.write().await;
        if let Some(tab) = tabs.get_mut(tab_id) {
            tab.received_data.clear();
            tab.sent_data.clear();
        }
    }

    pub async fn remove_at_tab(&self, tab_id: &str) {
        self.at_tabs.write().await.remove(tab_id);
    }
}

pub type BleManagerRef = Arc<BleManager>;
