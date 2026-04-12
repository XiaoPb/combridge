use std::sync::Arc;

use async_trait::async_trait;
use tracing::info;

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleBackend, BleCharacteristic, BleConnection, BleDevice, BleService, NotifyCallback,
};
use super::adapter::BleAdapter;

pub struct NativeBleBackend {
    adapter: Option<Arc<BleAdapter>>,
    configured: bool,
}

impl NativeBleBackend {
    pub fn new() -> Self {
        Self {
            adapter: None,
            configured: false,
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

        let adapter = BleAdapter::new().await?;
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
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        adapter.connect_device(address).await?;

        let name = adapter.get_device_name(address);

        let connection = BleConnection {
            address: address.to_string(),
            name,
            is_connected: true,
            services: vec![],
        };

        info!("已连接到设备: {}", address);
        Ok(connection)
    }

    async fn disconnect(&self, address: &str) -> Result<()> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        if let Some(client) = adapter.get_client(address) {
            client.disconnect().await?;
            adapter.remove_client(address);
            adapter.clear_scanned_device(address);
            info!("已断开设备: {}", address);
        } else {
            info!("设备未连接或已断开: {}", address);
        }

        Ok(())
    }

    async fn get_connections(&self) -> Result<Vec<BleConnection>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let clients = adapter.list_clients();
        let mut connections = Vec::new();

        for (address, client) in clients {
            let is_connected = client.is_connected()?;
            if is_connected {
                let name = adapter.get_device_name(&address);
                let services = client.get_discovered_services()?;
                connections.push(BleConnection {
                    address,
                    name,
                    is_connected: true,
                    services,
                });
            }
        }

        info!("当前 {} 个活跃连接", connections.len());
        Ok(connections)
    }

    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.discover_services().await
    }

    async fn discover_characteristics(
        &self,
        address: &str,
        service_uuid: &str,
    ) -> Result<Vec<BleCharacteristic>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.discover_characteristics(service_uuid).await
    }

    async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.read_characteristic(char_uuid).await
    }

    async fn write_characteristic(
        &self,
        address: &str,
        char_uuid: &str,
        data: &[u8],
    ) -> Result<()> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.write_characteristic(char_uuid, data).await
    }

    async fn write_without_response(
        &self,
        address: &str,
        char_uuid: &str,
        data: &[u8],
    ) -> Result<()> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.write_without_response(char_uuid, data).await
    }

    async fn subscribe_notify(
        &self,
        address: &str,
        char_uuid: &str,
        callback: NotifyCallback,
    ) -> Result<()> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.subscribe_notify(char_uuid, callback).await
    }

    async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.unsubscribe_notify(char_uuid).await
    }

    async fn get_rssi(&self, address: &str) -> Result<i16> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.get_rssi().await
    }

    async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16> {
        let adapter = self.adapter.as_ref().ok_or_else(|| {
            ComBridgeError::ble("蓝牙适配器未初始化")
        })?;

        let client = adapter.get_or_create_client(address);
        client.set_mtu(mtu).await
    }
}
