use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use bluest::{Adapter, Device, DeviceId};
use futures::StreamExt;
use tracing::{debug, info};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::BleDevice;
use super::gatt_client::GattClient;

pub struct BleAdapter {
    adapter: Arc<Adapter>,
    scanned_devices: Arc<RwLock<HashMap<DeviceId, Device>>>,
    clients: RwLock<HashMap<String, Arc<GattClient>>>,
    scan_handle: RwLock<Option<tokio::task::JoinHandle<()>>>,
}

impl BleAdapter {
    pub async fn new() -> Result<Self> {
        info!("初始化蓝牙适配器");

        let adapter = Adapter::default()
            .await
            .ok_or_else(|| ComBridgeError::ble("无法获取蓝牙适配器"))?;

        info!("蓝牙适配器初始化成功");
        Ok(Self {
            adapter: Arc::new(adapter),
            scanned_devices: Arc::new(RwLock::new(HashMap::new())),
            clients: RwLock::new(HashMap::new()),
            scan_handle: RwLock::new(None),
        })
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub async fn power_on(&self) -> Result<()> {
        info!("等待蓝牙适配器可用");
        self.adapter.wait_available().await
            .map_err(|e| ComBridgeError::ble(format!("适配器不可用: {}", e)))?;
        info!("蓝牙适配器已就绪");
        Ok(())
    }

    pub async fn start_scan(&self) -> Result<()> {
        info!("开始扫描BLE设备");

        {
            let mut handle_guard = self.scan_handle.write().unwrap();
            if let Some(handle) = handle_guard.take() {
                handle.abort();
            }
        }

        self.scanned_devices.write().unwrap().clear();

        let adapter = self.adapter.clone();
        let devices = self.scanned_devices.clone();

        let handle = tokio::spawn(async move {
            let mut scan = match adapter.scan(&[]).await {
                Ok(s) => s,
                Err(e) => {
                    debug!("扫描启动失败: {}", e);
                    return;
                }
            };
            
            while let Some(discovered) = scan.next().await {
                let device = discovered.device;
                let device_id = device.id();
                let name = device.name().ok();
                let rssi = discovered.rssi;

                debug!("发现设备: {:?} RSSI: {:?}", name, rssi);

                let mut devices_guard = devices.write().unwrap();
                devices_guard.insert(device_id, device);
            }
        });

        *self.scan_handle.write().unwrap() = Some(handle);

        Ok(())
    }

    pub async fn stop_scan(&self) -> Result<()> {
        info!("停止扫描BLE设备");
        
        let mut handle_guard = self.scan_handle.write().unwrap();
        if let Some(handle) = handle_guard.take() {
            handle.abort();
        }
        
        Ok(())
    }

    pub async fn get_scanned_devices(&self) -> Result<Vec<BleDevice>> {
        let devices = self.scanned_devices.read().unwrap();
        let result: Vec<BleDevice> = devices
            .iter()
            .map(|(id, device)| BleDevice {
                address: id.to_string(),
                name: device.name().ok(),
                rssi: None,
                is_connectable: true,
            })
            .collect();

        info!("返回 {} 个扫描到的设备", result.len());
        Ok(result)
    }

    fn find_device(&self, device_id: &str) -> Option<Device> {
        let devices = self.scanned_devices.read().unwrap();
        devices.iter()
            .find(|(id, _)| id.to_string() == device_id)
            .map(|(_, device)| device.clone())
    }

    pub fn get_or_create_client(&self, address: &str) -> Arc<GattClient> {
        let mut clients = self.clients.write().unwrap();
        clients
            .entry(address.to_string())
            .or_insert_with(|| Arc::new(GattClient::new(address)))
            .clone()
    }

    pub async fn connect_device(&self, address: &str) -> Result<Arc<GattClient>> {
        let device = self
            .find_device(address)
            .ok_or_else(|| ComBridgeError::ble(format!("设备未找到: {}", address)))?;

        info!("连接到设备: {}", address);
        self.adapter
            .connect_device(&device)
            .await
            .map_err(|e| ComBridgeError::ble(format!("连接失败: {}", e)))?;

        let client = self.get_or_create_client(address);
        client.set_device(device);

        info!("设备连接成功: {}", address);
        Ok(client)
    }

    pub async fn disconnect_device(&self, address: &str) -> Result<()> {
        let client = self.get_or_create_client(address);
        if let Some(device) = client.get_device() {
            self.adapter
                .disconnect_device(&device)
                .await
                .map_err(|e| ComBridgeError::ble(format!("断开失败: {}", e)))?;
            client.clear_device();
            info!("设备已断开: {}", address);
        }
        Ok(())
    }
}
