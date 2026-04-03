use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use tracing::{info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleBackend, BleDevice, BleConnection, BleService, BleCharacteristic, NotifyCallback,
};
use super::adapter::BleAdapter;
use super::gatt_client::GattClient;

pub struct NativeBleBackend {
    adapter: Option<Arc<BleAdapter>>,
    clients: RwLock<HashMap<String, Arc<GattClient>>>,
    configured: bool,
}

impl NativeBleBackend {
    pub fn new() -> Self {
        Self {
            adapter: None,
            clients: RwLock::new(HashMap::new()),
            configured: false,
        }
    }

    fn get_or_create_client(&self, address: &str) -> Arc<GattClient> {
        let mut clients = self.clients.write().unwrap();
        clients
            .entry(address.to_string())
            .or_insert_with(|| Arc::new(GattClient::new(address)))
            .clone()
    }

    fn check_platform_support() -> Result<()> {
        #[cfg(target_os = "windows")]
        {
            warn!("Windows 平台的原生 BLE 支持尚未完全实现，建议使用 AT 模式");
            return Err(ComBridgeError::ble(
                "Windows 平台原生 BLE 支持尚未完全实现。请使用 AT 模式通过串口连接 BLE 模块，或等待后续版本更新。"
            ));
        }
        
        #[cfg(target_os = "linux")]
        {
            warn!("Linux 平台的原生 BLE 支持尚未完全实现，建议使用 AT 模式");
            return Err(ComBridgeError::ble(
                "Linux 平台原生 BLE 支持尚未完全实现。请使用 AT 模式通过串口连接 BLE 模块，或等待后续版本更新。"
            ));
        }
        
        #[cfg(target_os = "macos")]
        {
            warn!("macOS 平台的原生 BLE 支持尚未完全实现，建议使用 AT 模式");
            return Err(ComBridgeError::ble(
                "macOS 平台原生 BLE 支持尚未完全实现。请使用 AT 模式通过串口连接 BLE 模块，或等待后续版本更新。"
            ));
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            Err(ComBridgeError::ble("当前平台不支持原生 BLE 功能"))
        }
    }
}

impl Default for NativeBleBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BleBackend for NativeBleBackend {
    async fn configure(&mut self) -> Result<()> {
        info!("配置原生BLE后端");
        
        Self::check_platform_support()?;
        
        let adapter = BleAdapter::new()?;
        adapter.power_on().await?;
        
        self.adapter = Some(Arc::new(adapter));
        self.configured = true;
        
        info!("原生BLE后端配置成功");
        Ok(())
    }

    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        adapter.start_scan().await?;
        
        let duration = std::time::Duration::from_millis(duration_ms);
        tokio::time::sleep(duration).await;

        adapter.stop_scan().await?;
        
        let devices = adapter.get_scanned_devices().await?;
        info!("扫描完成，发现 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn stop_scan(&self) -> Result<Vec<BleDevice>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;
        
        adapter.stop_scan().await?;
        let devices = adapter.get_scanned_devices().await?;
        info!("停止扫描，返回 {} 个设备", devices.len());
        Ok(devices)
    }

    async fn connect(&self, address: &str) -> Result<BleConnection> {
        let client = self.get_or_create_client(address);
        client.connect().await?;

        let connection = BleConnection {
            address: address.to_string(),
            name: None,
            connected: true,
        };

        info!("已连接到设备: {}", address);
        Ok(connection)
    }

    async fn disconnect(&self, address: &str) -> Result<()> {
        let client = {
            let clients = self.clients.read().unwrap();
            clients.get(address).cloned()
        };

        if let Some(client) = client {
            client.disconnect().await?;
        }

        info!("已断开设备: {}", address);
        Ok(())
    }

    async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let clients = self.clients.read().unwrap();
        let connections: Vec<BleConnection> = clients
            .iter()
            .filter(|(_, client)| client.is_connected())
            .map(|(addr, _)| BleConnection {
                address: addr.clone(),
                name: None,
                connected: true,
            })
            .collect();

        Ok(connections)
    }

    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let client = self.get_or_create_client(address);
        client.discover_services().await
    }

    async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>> {
        let client = self.get_or_create_client(address);
        client.discover_characteristics(service_uuid).await
    }

    async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>> {
        let client = self.get_or_create_client(address);
        client.read_characteristic(char_uuid).await
    }

    async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let client = self.get_or_create_client(address);
        client.write_characteristic(char_uuid, data).await
    }

    async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()> {
        let client = self.get_or_create_client(address);
        client.write_without_response(char_uuid, data).await
    }

    async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let client = self.get_or_create_client(address);
        client.subscribe_notify(char_uuid, callback).await
    }

    async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()> {
        let client = self.get_or_create_client(address);
        client.unsubscribe_notify(char_uuid).await
    }

    async fn get_rssi(&self, address: &str) -> Result<i16> {
        let client = self.get_or_create_client(address);
        client.get_rssi().await
    }

    async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16> {
        let client = self.get_or_create_client(address);
        client.set_mtu(mtu).await
    }
}
