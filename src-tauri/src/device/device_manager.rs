use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

use super::ble::{AtConfig, BleConnection, BleManager, BleManagerRef};
use super::serial::{SerialManager, SerialManagerRef, SerialPortConfig};
use crate::error::{ComBridgeError, Result};
use crate::service::event_bus::EventBus;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    Serial,
    Ble,
}

impl std::fmt::Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceType::Serial => write!(f, "serial"),
            DeviceType::Ble => write!(f, "ble"),
        }
    }
}

#[derive(Clone)]
pub struct DeviceManager {
    pub serial_manager: SerialManagerRef,
    pub ble_manager: BleManagerRef,
}

impl DeviceManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            serial_manager: Arc::new(SerialManager::new(Arc::clone(&event_bus))),
            ble_manager: Arc::new(BleManager::new(event_bus)),
        }
    }

    pub async fn send_direct(
        &self,
        device_type: DeviceType,
        device_name: &str,
        char_uuid: Option<&str>,
        data: &[u8],
    ) -> Result<()> {
        match device_type {
            DeviceType::Serial => {
                self.serial_manager.send_data(device_name, data)?;
            }
            DeviceType::Ble => {
                let uuid = char_uuid.ok_or_else(|| ComBridgeError::ble("缺少特征UUID"))?;
                self.ble_manager
                    .write_characteristic(device_name, uuid, data)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn open_serial(&self, config: SerialPortConfig) -> Result<()> {
        self.serial_manager
            .open_port(config.clone(), move |_name, _data| {})?;
        info!("串口已打开: {}", config.port_name);
        Ok(())
    }

    pub async fn close_serial(&self, port_name: &str) -> Result<()> {
        self.serial_manager.close_port(port_name)?;
        info!("串口已关闭: {}", port_name);
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
        info!("BLE 设备已连接: {}", address);
        Ok(connection)
    }

    pub async fn disconnect_ble(&self, address: &str) -> Result<()> {
        self.ble_manager.disconnect(address).await?;
        info!("BLE 设备已断开: {}", address);
        Ok(())
    }
}

pub type DeviceManagerRef = Arc<DeviceManager>;
