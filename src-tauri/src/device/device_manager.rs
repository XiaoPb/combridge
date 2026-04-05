use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::serial::{SerialManager, SerialManagerRef, SerialPortConfig};
use super::ble::{BleManager, BleManagerRef, BleConnection, AtConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Serial,
    Ble,
    WebSocket,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Serial => write!(f, "serial"),
            DeviceType::Ble => write!(f, "ble"),
            DeviceType::WebSocket => write!(f, "websocket"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: DeviceType,
    pub is_connected: bool,
    pub connected_at: Option<u64>,
    pub bytes_received: u64,
    pub bytes_sent: u64,
    pub last_activity: Option<u64>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataRoute {
    pub source_id: String,
    pub target_id: String,
    pub enabled: bool,
    pub filter: Option<DataFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFilter {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub start_byte: Option<u8>,
    pub end_byte: Option<u8>,
    pub pattern: Option<Vec<u8>>,
}

pub type DataCallback = Arc<dyn Fn(&str, DeviceType, &[u8]) + Send + Sync>;

#[derive(Clone)]
pub struct DeviceManager {
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
    devices: Arc<RwLock<HashMap<String, DeviceInfo>>>,
    routes: Arc<RwLock<Vec<DataRoute>>>,
    callbacks: Arc<RwLock<Vec<DataCallback>>>,
}

impl DeviceManager {
    pub fn new(serial_manager: SerialManagerRef, ble_manager: BleManagerRef) -> Self {
        Self {
            serial_manager,
            ble_manager,
            devices: Arc::new(RwLock::new(HashMap::new())),
            routes: Arc::new(RwLock::new(Vec::new())),
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn register_device(&self, device: DeviceInfo) {
        let mut devices = self.devices.write().await;
        devices.insert(device.id.clone(), device.clone());
        info!("设备已注册: {} ({})", device.name, device.device_type);
    }

    pub async fn unregister_device(&self, device_id: &str) {
        let mut devices = self.devices.write().await;
        if devices.remove(device_id).is_some() {
            info!("设备已注销: {}", device_id);
        }
    }

    pub async fn get_device(&self, device_id: &str) -> Option<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.get(device_id).cloned()
    }

    pub async fn get_all_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        devices.values().cloned().collect()
    }

    pub async fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        let devices = self.devices.read().await;
        devices
            .values()
            .filter(|d| d.device_type == device_type)
            .cloned()
            .collect()
    }

    pub async fn update_device_stats(
        &self,
        device_id: &str,
        bytes_received: u64,
        bytes_sent: u64,
    ) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(device_id)
            .ok_or_else(|| ComBridgeError::config(format!("设备不存在: {}", device_id)))?;

        device.bytes_received += bytes_received;
        device.bytes_sent += bytes_sent;
        device.last_activity = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        );

        Ok(())
    }

    pub async fn set_device_connected(&self, device_id: &str, connected: bool) -> Result<()> {
        let mut devices = self.devices.write().await;
        let device = devices
            .get_mut(device_id)
            .ok_or_else(|| ComBridgeError::config(format!("设备不存在: {}", device_id)))?;

        device.is_connected = connected;
        if connected {
            device.connected_at = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            );
        }

        Ok(())
    }

    pub async fn add_route(&self, route: DataRoute) {
        let mut routes = self.routes.write().await;
        routes.push(route.clone());
        info!("数据路由已添加: {} -> {}", route.source_id, route.target_id);
    }

    pub async fn remove_route(&self, source_id: &str, target_id: &str) {
        let mut routes = self.routes.write().await;
        routes.retain(|r| !(r.source_id == source_id && r.target_id == target_id));
        info!("数据路由已移除: {} -> {}", source_id, target_id);
    }

    pub async fn get_routes(&self) -> Vec<DataRoute> {
        let routes = self.routes.read().await;
        routes.clone()
    }

    pub async fn route_data(&self, source_id: &str, data: &[u8]) -> Result<Vec<String>> {
        let routes = self.routes.read().await;
        let mut routed_to = Vec::new();

        for route in routes.iter().filter(|r| r.source_id == source_id && r.enabled) {
            if let Some(ref filter) = route.filter {
                if !self.apply_filter(data, filter) {
                    debug!("数据被过滤器拦截: {} -> {}", source_id, route.target_id);
                    continue;
                }
            }

            if let Err(e) = self.send_to_device(&route.target_id, data).await {
                warn!("路由数据失败: {} -> {}: {}", source_id, route.target_id, e);
            } else {
                routed_to.push(route.target_id.clone());
            }
        }

        Ok(routed_to)
    }

    fn apply_filter(&self, data: &[u8], filter: &DataFilter) -> bool {
        if let Some(min) = filter.min_length {
            if data.len() < min {
                return false;
            }
        }

        if let Some(max) = filter.max_length {
            if data.len() > max {
                return false;
            }
        }

        if let Some(start) = filter.start_byte {
            if data.first() != Some(&start) {
                return false;
            }
        }

        if let Some(end) = filter.end_byte {
            if data.last() != Some(&end) {
                return false;
            }
        }

        if let Some(ref pattern) = filter.pattern {
            if !data.windows(pattern.len()).any(|w| w == pattern.as_slice()) {
                return false;
            }
        }

        true
    }

    /// 直接发送数据到设备（不需要预先注册）
    ///
    /// # 参数
    /// - `device_type`: 设备类型
    /// - `device_name`: 设备名称（串口名或蓝牙地址）
    /// - `char_uuid`: 蓝牙特征 UUID（蓝牙设备必需）
    /// - `data`: 待发送的数据
    pub async fn send_direct(
        &self,
        device_type: DeviceType,
        device_name: &str,
        char_uuid: Option<&str>,
        data: &[u8],
    ) -> Result<()> {
        info!("DeviceManager send_direct: type={:?}, name={}, {} bytes", device_type, device_name, data.len());
        
        match device_type {
            DeviceType::Serial => {
                self.serial_manager.send_data(device_name, data)?;
                info!("DeviceManager send_direct 串口发送成功: {} bytes", data.len());
            }
            DeviceType::Ble => {
                let uuid = char_uuid
                    .ok_or_else(|| ComBridgeError::ble("缺少特征UUID"))?;
                self.ble_manager
                    .write_characteristic(device_name, uuid, data)
                    .await?;
                info!("DeviceManager send_direct 蓝牙发送成功: {} bytes", data.len());
            }
            DeviceType::WebSocket => {
                debug!("WebSocket发送数据到 {}: {} bytes", device_name, data.len());
            }
        }

        Ok(())
    }

    async fn send_to_device(&self, device_id: &str, data: &[u8]) -> Result<()> {
        let devices = self.devices.read().await;
        let device = devices
            .get(device_id)
            .ok_or_else(|| ComBridgeError::config(format!("设备不存在: {}", device_id)))?;

        match device.device_type {
            DeviceType::Serial => {
                self.serial_manager.send_data(&device.name, data)?;
            }
            DeviceType::Ble => {
                let char_uuid = device
                    .metadata
                    .get("characteristic_uuid")
                    .ok_or_else(|| ComBridgeError::ble("缺少特征UUID"))?;
                self.ble_manager
                    .write_characteristic(&device.name, char_uuid, data)
                    .await?;
            }
            DeviceType::WebSocket => {
                debug!("WebSocket发送数据到 {}: {} bytes", device_id, data.len());
            }
        }

        Ok(())
    }

    pub fn register_callback<F>(&self, callback: F)
    where
        F: Fn(&str, DeviceType, &[u8]) + Send + Sync + 'static,
    {
        let rt = tokio::runtime::Handle::current();
        let mut callbacks = rt.block_on(self.callbacks.write());
        callbacks.push(Arc::new(callback));
        debug!("已注册设备数据回调，当前共 {} 个回调", callbacks.len());
    }

    pub async fn notify_callbacks(&self, device_id: &str, device_type: DeviceType, data: &[u8]) {
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback(device_id, device_type, data);
        }
    }

    pub async fn open_serial(&self, config: SerialPortConfig) -> Result<()> {
        let device_id = format!("serial-{}", config.port_name);
        let device_id_clone = device_id.clone();
        let manager = self.clone();
        
        self.serial_manager.open_port(config.clone(), move |_name, data| {
            let data = data.to_vec();
            let device_id_clone = device_id_clone.clone();
            let manager = manager.clone();
            
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    manager.notify_callbacks(&device_id_clone, DeviceType::Serial, &data).await;
                });
            }
        })?;

        let device = DeviceInfo {
            id: device_id.clone(),
            name: config.port_name.clone(),
            device_type: DeviceType::Serial,
            is_connected: true,
            connected_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            bytes_received: 0,
            bytes_sent: 0,
            last_activity: None,
            metadata: HashMap::new(),
        };

        self.register_device(device).await;
        Ok(())
    }

    pub async fn close_serial(&self, port_name: &str) -> Result<()> {
        let device_id = format!("serial-{}", port_name);
        self.serial_manager.close_port(port_name)?;
        self.unregister_device(&device_id).await;
        Ok(())
    }

    pub async fn configure_ble_at(&self, config: AtConfig) -> Result<()> {
        self.ble_manager.configure_at(config).await
    }

    pub async fn configure_ble_native(&self) -> Result<()> {
        self.ble_manager.configure_native().await
    }

    pub async fn connect_ble(&self, address: &str) -> Result<BleConnection> {
        let connection = self.ble_manager.connect(address).await?;

        let device_id = format!("ble-{}", address);
        let mut metadata = HashMap::new();
        metadata.insert("address".to_string(), address.to_string());

        let device = DeviceInfo {
            id: device_id.clone(),
            name: connection.name.clone().unwrap_or_else(|| address.to_string()),
            device_type: DeviceType::Ble,
            is_connected: true,
            connected_at: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
            ),
            bytes_received: 0,
            bytes_sent: 0,
            last_activity: None,
            metadata,
        };

        self.register_device(device).await;
        Ok(connection)
    }

    pub async fn disconnect_ble(&self, address: &str) -> Result<()> {
        self.ble_manager.disconnect(address).await?;

        let device_id = format!("ble-{}", address);
        self.unregister_device(&device_id).await;
        Ok(())
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new(
            Arc::new(SerialManager::new()),
            Arc::new(BleManager::new()),
        )
    }
}

pub type DeviceManagerRef = Arc<DeviceManager>;
