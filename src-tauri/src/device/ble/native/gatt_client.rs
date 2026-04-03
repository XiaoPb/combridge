use std::collections::HashMap;
use std::sync::{Mutex, RwLock};

use bluest::{Characteristic, Device, Service};
use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleCharacteristic, BleCharacteristicProperties, BleService, NotifyCallback,
};

pub struct GattClient {
    address: String,
    device: RwLock<Option<Device>>,
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
            services: RwLock::new(HashMap::new()),
            characteristics: RwLock::new(HashMap::new()),
            notify_callbacks: Mutex::new(HashMap::new()),
            notify_handles: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_device(&self, device: Device) {
        *self.device.write().unwrap() = Some(device);
    }

    pub fn get_device(&self) -> Option<Device> {
        self.device.read().unwrap().clone()
    }

    pub fn clear_device(&self) {
        *self.device.write().unwrap() = None;
        self.services.write().unwrap().clear();
        self.characteristics.write().unwrap().clear();

        let mut handles = self.notify_handles.lock().unwrap();
        for (_, handle) in handles.drain() {
            handle.abort();
        }
    }

    pub async fn is_connected(&self) -> bool {
        let device = self.device.read().unwrap().clone();
        if let Some(d) = device {
            d.is_connected().await
        } else {
            false
        }
    }

    pub async fn discover_services(&self) -> Result<Vec<BleService>> {
        let device = self.device.read().unwrap().clone();
        let device = device
            .ok_or_else(|| ComBridgeError::ble("设备未连接"))?;

        if !device.is_connected().await {
            return Err(ComBridgeError::ble("设备未连接"));
        }

        debug!("发现服务...");
        let services = device
            .services()
            .await
            .map_err(|e| ComBridgeError::ble(format!("发现服务失败: {}", e)))?;

        let mut result = Vec::new();
        let mut svc_map = HashMap::new();
        
        for svc in &services {
            let uuid = svc.uuid().to_string();
            result.push(BleService {
                uuid: uuid.clone(),
                primary: true,
            });
            svc_map.insert(uuid, svc.clone());
        }

        *self.services.write().unwrap() = svc_map;

        info!("发现 {} 个服务", result.len());
        Ok(result)
    }

    pub async fn discover_characteristics(
        &self,
        service_uuid: &str,
    ) -> Result<Vec<BleCharacteristic>> {
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

        let mut result = Vec::new();
        let mut char_list = Vec::new();
        
        for c in &chars {
            let uuid = c.uuid().to_string();
            let props = c.properties()
                .await
                .map_err(|e| ComBridgeError::ble(format!("获取特征属性失败: {}", e)))?;
            
            let ble_props = BleCharacteristicProperties {
                read: props.read,
                write: props.write,
                write_without_response: props.write_without_response,
                notify: props.notify,
                indicate: props.indicate,
            };
            
            result.push(BleCharacteristic {
                uuid: uuid.clone(),
                service_uuid: service_uuid.to_string(),
                properties: ble_props,
            });
            
            char_list.push(c.clone());
        }

        self.characteristics.write().unwrap().insert(service_uuid.to_string(), char_list);

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
        let device = self.device.read().unwrap().clone();
        let device = device
            .ok_or_else(|| ComBridgeError::ble("设备未连接"))?;

        #[cfg(target_os = "macos")]
        {
            device
                .rssi()
                .await
                .map_err(|e| ComBridgeError::ble(format!("获取RSSI失败: {}", e)))
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = device;
            warn!("当前平台不支持RSSI查询");
            Ok(-50)
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
