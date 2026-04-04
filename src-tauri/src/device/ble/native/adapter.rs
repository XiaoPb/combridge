use std::collections::HashMap;
use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};

use bluest::{Adapter, Device, DeviceId};
use futures::StreamExt;
use tracing::info;

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::BleDevice;
use super::gatt_client::GattClient;

pub struct BleAdapter {
    adapter: Arc<Adapter>,
    scanned_devices: Arc<RwLock<HashMap<DeviceId, (Arc<Device>, Option<i16>)>>>,
    clients: RwLock<HashMap<String, Arc<GattClient>>>,
    scan_cancelled: Arc<AtomicBool>,
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
            scan_cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn is_available(&self) -> bool {
        true
    }

    pub async fn power_on(&self) -> Result<()> {
        info!("等待蓝牙适配器可用");
        self.adapter
            .wait_available()
            .await
            .map_err(|e| ComBridgeError::ble(format!("适配器不可用: {}", e)))?;
        info!("蓝牙适配器已就绪");
        Ok(())
    }

    pub async fn start_scan(&self) -> Result<()> {
        info!("开始扫描BLE设备");

        self.scanned_devices.write().unwrap().clear();
        self.scan_cancelled.store(false, Ordering::SeqCst);

        let adapter = self.adapter.clone();
        let scanned_devices = self.scanned_devices.clone();
        let scan_cancelled = self.scan_cancelled.clone();

        info!("启动BLE扫描任务...");

        tokio::spawn(async move {
            match adapter.scan(&[]).await {
                Ok(mut scan) => {
                    info!("BLE扫描已开始，等待设备发现...");
                    while let Some(discovered) = scan.next().await {
                        if scan_cancelled.load(Ordering::SeqCst) {
                            info!("扫描被取消");
                            break;
                        }
                        let device = discovered.device;
                        let device_id = device.id();
                        let name = device.name().ok();
                        let rssi = discovered.rssi;

                        info!("发现BLE设备: {:?} RSSI: {:?}", name, rssi);

                        let mut devices_guard = scanned_devices.write().unwrap();
                        devices_guard.insert(device_id, (Arc::new(device), rssi));
                    }
                    info!("BLE扫描任务结束");
                }
                Err(e) => {
                    info!("BLE扫描失败: {}", e);
                }
            }
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        info!("BLE扫描任务已启动");
        Ok(())
    }

    pub async fn stop_scan(&self) -> Result<()> {
        info!("停止扫描BLE设备");
        self.scan_cancelled.store(true, Ordering::SeqCst);
        info!("已发送扫描取消信号");
        Ok(())
    }

    pub async fn get_scanned_devices(&self) -> Result<Vec<BleDevice>> {
        let devices = self.scanned_devices.read().unwrap();
        let result: Vec<BleDevice> = devices
            .iter()
            .map(|(id, (device, rssi))| BleDevice {
                address: id.to_string(),
                name: device.name().ok(),
                rssi: *rssi,
                is_connectable: true,
            })
            .collect();

        info!("返回 {} 个扫描到的设备", result.len());
        Ok(result)
    }

    fn get_device(&self, device_id: &str) -> Option<Arc<Device>> {
        let devices = self.scanned_devices.read().unwrap();
        devices
            .iter()
            .find(|(id, _)| id.to_string() == device_id)
            .map(|(_, (device, _))| device.clone())
    }

    pub fn get_device_name(&self, address: &str) -> Option<String> {
        self.get_device(address).and_then(|d| d.name().ok())
    }

    pub fn get_or_create_client(&self, address: &str) -> Arc<GattClient> {
        let mut clients = self.clients.write().unwrap();
        clients
            .entry(address.to_string())
            .or_insert_with(|| Arc::new(GattClient::new(address)))
            .clone()
    }

    pub fn get_client(&self, address: &str) -> Option<Arc<GattClient>> {
        let clients = self.clients.read().unwrap();
        clients.get(address).cloned()
    }

    pub fn remove_client(&self, address: &str) {
        let mut clients = self.clients.write().unwrap();
        clients.remove(address);
    }

    pub fn list_clients(&self) -> Vec<(String, Arc<GattClient>)> {
        let clients = self.clients.read().unwrap();
        clients
            .iter()
            .map(|(addr, client)| (addr.clone(), client.clone()))
            .collect()
    }

    pub async fn connect_device(&self, address: &str) -> Result<Arc<GattClient>> {
        let device = self
            .get_device(address)
            .ok_or_else(|| ComBridgeError::ble(format!("设备未找到: {}", address)))?;

        let client = self.get_or_create_client(address);
        client.set_device(device, self.adapter.clone());
        client.connect().await?;

        Ok(client)
    }
}
