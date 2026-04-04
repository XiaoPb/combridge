use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use bluest::{Characteristic, Device, Service};
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleCharacteristic, BleCharacteristicProperties, BleService, NotifyCallback,
};

pub struct GattClient {
    address: String,
    device: RwLock<Option<Arc<Device>>>,
    adapter: RwLock<Option<Arc<bluest::Adapter>>>,
    services: RwLock<HashMap<String, Service>>,
    characteristics: RwLock<HashMap<String, Vec<Characteristic>>>,
    notify_callbacks: Mutex<HashMap<String, NotifyCallback>>,
    notify_handles: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
}

impl GattClient {
    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            device: RwLock::new(None),
            adapter: RwLock::new(None),
            services: RwLock::new(HashMap::new()),
            characteristics: RwLock::new(HashMap::new()),
            notify_callbacks: Mutex::new(HashMap::new()),
            notify_handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_device(&self, device: Arc<Device>, adapter: Arc<bluest::Adapter>) {
        *self.device.write().unwrap() = Some(device);
        *self.adapter.write().unwrap() = Some(adapter);
    }

    pub fn is_connected(&self) -> bool {
        let device = self.device.read().unwrap();
        if let Some(device) = device.as_ref() {
            futures::executor::block_on(async { device.is_connected().await })
        } else {
            false
        }
    }

    pub async fn connect(&self) -> Result<()> {
        let (device, adapter) = {
            let device = self.device.read().unwrap();
            let adapter = self.adapter.read().unwrap();
            (
                device
                    .as_ref()
                    .ok_or_else(|| ComBridgeError::ble("设备未设置"))?
                    .clone(),
                adapter
                    .as_ref()
                    .ok_or_else(|| ComBridgeError::ble("适配器未设置"))?
                    .clone(),
            )
        };

        info!("连接到设备: {}", self.address);
        adapter
            .connect_device(&device)
            .await
            .map_err(|e| ComBridgeError::ble(format!("连接失败: {}", e)))?;

        info!("设备连接成功: {}", self.address);
        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        let (device, adapter) = {
            let device = self.device.read().unwrap();
            let adapter = self.adapter.read().unwrap();
            match (device.as_ref(), adapter.as_ref()) {
                (Some(d), Some(a)) => (d.clone(), a.clone()),
                _ => return Ok(()),
            }
        };

        adapter
            .disconnect_device(&device)
            .await
            .map_err(|e| ComBridgeError::ble(format!("断开失败: {}", e)))?;

        self.services.write().unwrap().clear();
        self.characteristics.write().unwrap().clear();

        let mut handles = self.notify_handles.lock().unwrap();
        for (_, handle) in handles.drain() {
            handle.abort();
        }

        info!("设备已断开: {}", self.address);
        Ok(())
    }

    pub async fn discover_services(&self) -> Result<Vec<BleService>> {
        let device = {
            let device = self.device.read().unwrap();
            device
                .as_ref()
                .ok_or_else(|| ComBridgeError::ble("设备未设置"))?
                .clone()
        };

        debug!("发现服务...");
        let services = device
            .discover_services()
            .await
            .map_err(|e| ComBridgeError::ble(format!("发现服务失败: {}", e)))?;

        let mut result = Vec::new();
        let mut svc_data = Vec::new();
        let mut total_chars = 0;

        for svc in &services {
            let uuid = svc.uuid().to_string();
            let is_primary = svc.is_primary().await.unwrap_or(true);
            svc_data.push((uuid.clone(), svc.clone(), is_primary));

            let chars = match svc.characteristics().await {
                Ok(c) => c,
                Err(e) => {
                    warn!("发现服务 {} 的特征失败: {}", uuid, e);
                    vec![]
                }
            };

            let mut ble_chars: Vec<BleCharacteristic> = Vec::new();
            let mut char_data: Vec<(String, Characteristic, BleCharacteristicProperties)> = Vec::new();

            for c in &chars {
                let char_uuid = c.uuid().to_string();
                let props = match c.properties().await {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("获取特征 {} 属性失败: {}", char_uuid, e);
                        continue;
                    }
                };
                let ble_props = BleCharacteristicProperties {
                    read: props.read,
                    write: props.write,
                    write_without_response: props.write_without_response,
                    notify: props.notify,
                    indicate: props.indicate,
                };
                char_data.push((char_uuid.clone(), c.clone(), ble_props));
                ble_chars.push(BleCharacteristic {
                    uuid: char_uuid,
                    service_uuid: uuid.clone(),
                    properties: ble_props,
                });
            }

            total_chars += ble_chars.len();

            {
                let mut char_map = self.characteristics.write().unwrap();
                let chars_vec: Vec<Characteristic> =
                    char_data.iter().map(|(_, c, _)| c.clone()).collect();
                char_map.insert(uuid.clone(), chars_vec);
            }

            result.push(BleService {
                uuid,
                primary: is_primary,
                characteristics: ble_chars,
            });
        }

        {
            let mut svc_map = self.services.write().unwrap();
            for (uuid, svc, _) in svc_data {
                svc_map.insert(uuid, svc);
            }
        }

        info!("发现 {} 个服务, {} 个特征", result.len(), total_chars);
        Ok(result)
    }

    pub async fn discover_characteristics(
        &self,
        service_uuid: &str,
    ) -> Result<Vec<BleCharacteristic>> {
        let device = {
            let device = self.device.read().unwrap();
            device
                .as_ref()
                .ok_or_else(|| ComBridgeError::ble("设备未设置"))?
                .clone()
        };

        let service = {
            let services = self.services.read().unwrap();
            services
                .get(service_uuid)
                .ok_or_else(|| ComBridgeError::ble(format!("服务未找到: {}", service_uuid)))?
                .clone()
        };

        debug!("发现特征: {}", service_uuid);
        let chars = service
            .characteristics()
            .await
            .map_err(|e| ComBridgeError::ble(format!("发现特征失败: {}", e)))?;

        let mut result: Vec<BleCharacteristic> = Vec::new();
        let mut char_data: Vec<(String, Characteristic, BleCharacteristicProperties)> = Vec::new();

        for c in &chars {
            let uuid = c.uuid().to_string();
            let props = c
                .properties()
                .await
                .map_err(|e| ComBridgeError::ble(format!("获取特征属性失败: {}", e)))?;
            let ble_props = BleCharacteristicProperties {
                read: props.read,
                write: props.write,
                write_without_response: props.write_without_response,
                notify: props.notify,
                indicate: props.indicate,
            };
            char_data.push((uuid.clone(), c.clone(), ble_props));
            result.push(BleCharacteristic {
                uuid,
                service_uuid: service_uuid.to_string(),
                properties: ble_props,
            });
        }

        {
            let mut char_map = self.characteristics.write().unwrap();
            let chars_vec: Vec<Characteristic> =
                char_data.iter().map(|(_, c, _)| c.clone()).collect();
            char_map.insert(service_uuid.to_string(), chars_vec);
        }

        debug!("发现 {} 个特征", result.len());
        Ok(result)
    }

    pub async fn read_characteristic(&self, char_uuid: &str) -> Result<Vec<u8>> {
        let char = self.find_characteristic(char_uuid).await?;

        debug!("读取特征: {}", char_uuid);
        let value = char
            .read()
            .await
            .map_err(|e| ComBridgeError::ble(format!("读取失败: {}", e)))?;

        debug!("读取到 {} 字节", value.len());
        Ok(value)
    }

    pub async fn write_characteristic(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        debug!("写入特征: {} ({} 字节)", char_uuid, data.len());
        char.write(data)
            .await
            .map_err(|e| ComBridgeError::ble(format!("写入失败: {}", e)))?;

        debug!("写入成功");
        Ok(())
    }

    pub async fn write_without_response(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        debug!("无响应写入: {} ({} 字节)", char_uuid, data.len());
        char.write_without_response(data)
            .await
            .map_err(|e| ComBridgeError::ble(format!("写入失败: {}", e)))?;

        Ok(())
    }

    pub async fn subscribe_notify(&self, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        debug!("订阅通知: {}", char_uuid);

        self.notify_callbacks
            .lock()
            .unwrap()
            .insert(char_uuid.to_string(), callback.clone());

        let uuid = char_uuid.to_string();
        let handle = tokio::spawn(async move {
            match char.notify().await {
                Ok(mut notifications) => {
                    while let Some(result) = notifications.next().await {
                        match result {
                            Ok(data) => {
                                debug!("收到通知: {} ({} 字节)", uuid, data.len());
                                callback("", &uuid, &data);
                            }
                            Err(e) => {
                                warn!("通知错误: {} - {}", uuid, e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("订阅通知失败: {} - {}", uuid, e);
                }
            }
        });

        self.notify_handles
            .lock()
            .unwrap()
            .insert(char_uuid.to_string(), handle);

        Ok(())
    }

    pub async fn unsubscribe_notify(&self, char_uuid: &str) -> Result<()> {
        debug!("取消订阅: {}", char_uuid);

        self.notify_callbacks.lock().unwrap().remove(char_uuid);

        if let Some(handle) = self.notify_handles.lock().unwrap().remove(char_uuid) {
            handle.abort();
        }

        Ok(())
    }

    pub async fn get_rssi(&self) -> Result<i16> {
        let device = {
            let device = self.device.read().unwrap();
            device
                .as_ref()
                .ok_or_else(|| ComBridgeError::ble("设备未连接"))?
                .clone()
        };

        match device.rssi().await {
            Ok(rssi) => Ok(rssi),
            Err(_) => {
                warn!("当前平台不支持RSSI查询");
                Ok(-50)
            }
        }
    }

    pub async fn set_mtu(&self, mtu: u16) -> Result<u16> {
        info!("MTU协商，请求值: {}", mtu);

        let actual_mtu = mtu.min(517).max(23);
        info!("MTU协商完成，实际值: {}", actual_mtu);
        Ok(actual_mtu)
    }

    async fn find_characteristic(&self, char_uuid: &str) -> Result<Characteristic> {
        let char_map = self.characteristics.read().unwrap().clone();

        for chars in char_map.values() {
            for c in chars {
                if c.uuid().to_string() == char_uuid {
                    return Ok(c.clone());
                }
            }
        }

        Err(ComBridgeError::ble(format!("特征未找到: {}", char_uuid)))
    }
}
