use std::sync::{Arc, RwLock};

use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::BleDevice;

pub struct BleAdapter {
    adapter_name: String,
    is_powered: RwLock<bool>,
}

impl BleAdapter {
    pub fn new() -> Result<Self> {
        info!("初始化蓝牙适配器");
        
        Ok(Self {
            adapter_name: "default".to_string(),
            is_powered: RwLock::new(false),
        })
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub fn is_powered(&self) -> bool {
        *self.is_powered.read().unwrap()
    }

    pub async fn power_on(&self) -> Result<()> {
        info!("打开蓝牙适配器电源");
        *self.is_powered.write().unwrap() = true;
        Ok(())
    }

    pub async fn power_off(&self) -> Result<()> {
        info!("关闭蓝牙适配器电源");
        *self.is_powered.write().unwrap() = false;
        Ok(())
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub async fn start_scan(&self) -> Result<()> {
        if !self.is_powered() {
            return Err(ComBridgeError::ble("蓝牙适配器未开启"));
        }
        debug!("开始扫描BLE设备");
        Ok(())
    }

    pub async fn stop_scan(&self) -> Result<()> {
        debug!("停止扫描BLE设备");
        Ok(())
    }

    pub async fn get_scanned_devices(&self) -> Result<Vec<BleDevice>> {
        Ok(Vec::new())
    }
}

impl Default for BleAdapter {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            adapter_name: "none".to_string(),
            is_powered: RwLock::new(false),
        })
    }
}
