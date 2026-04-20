use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use bluest::{Characteristic, Device, Service};
use futures::StreamExt;
use tracing::{debug, error, info, warn};

use crate::error::{ComBridgeError, Result};
use super::super::ble_traits::{
    BleCharacteristic, BleCharacteristicProperties, BleService, NotifyCallback,
};
use crate::device::cache::{ChannelCache, RingBufferRef, create_ring_buffer};

fn extract_short_mac(address: &str) -> String {
    if let Some(pos) = address.rfind('-') {
        let mac = &address[pos + 1..];
        return mac.to_uppercase();
    }
    address.to_string()
}

fn extract_short_uuid(uuid: &str) -> String {
    if uuid.len() >= 8 {
        uuid[..8].to_uppercase()
    } else {
        uuid.to_uppercase()
    }
}

fn format_ble_log(mac: &str, uuid: &str, direction: &str, data: &[u8]) -> String {
    format!(
        "[{}][{}][{}] {} bytes",
        extract_short_mac(mac),
        extract_short_uuid(uuid),
        direction,
        data.len()
    )
}

struct CharacteristicCache {
    tx_buffer: RingBufferRef,
    rx_buffer: RingBufferRef,
}

impl CharacteristicCache {
    fn new() -> Self {
        Self {
            tx_buffer: create_ring_buffer(),
            rx_buffer: create_ring_buffer(),
        }
    }
}

pub struct GattClient {
    address: String,
    device: RwLock<Option<Arc<Device>>>,
    adapter: RwLock<Option<Arc<bluest::Adapter>>>,
    services: RwLock<HashMap<String, Service>>,
    characteristics: RwLock<HashMap<String, Vec<Characteristic>>>,
    notify_callbacks: Mutex<HashMap<String, NotifyCallback>>,
    notify_handles: Mutex<HashMap<String, tokio::task::JoinHandle<()>>>,
    caches: RwLock<HashMap<String, CharacteristicCache>>,
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
            caches: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_device(&self, device: Arc<Device>, adapter: Arc<bluest::Adapter>) -> Result<()> {
        *self.device.write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))? = Some(device);
        *self.adapter.write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))? = Some(adapter);
        Ok(())
    }

    pub fn is_connected(&self) -> Result<bool> {
        let device = self.device.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        if let Some(device) = device.as_ref() {
            Ok(futures::executor::block_on(async { device.is_connected().await }))
        } else {
            Ok(false)
        }
    }

    pub async fn connect(&self) -> Result<()> {
        let (device, adapter) = {
            let device = self.device.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            let adapter = self.adapter.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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
            let device = self.device.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            let adapter = self.adapter.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            match (device.as_ref(), adapter.as_ref()) {
                (Some(d), Some(a)) => (Some(d.clone()), Some(a.clone())),
                _ => (None, None),
            }
        };

        if let (Some(device), Some(adapter)) = (device, adapter) {
            adapter
                .disconnect_device(&device)
                .await
                .map_err(|e| ComBridgeError::ble(format!("断开失败: {}", e)))?;
        }

        {
            let mut device_guard = self.device.write()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            *device_guard = None;
        }

        self.services.write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();
        self.characteristics.write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();
        self.caches.write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();

        let mut handles = self.notify_handles.lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        for (_, handle) in handles.drain() {
            handle.abort();
        }

        info!("设备已断开: {}", self.address);
        Ok(())
    }

    pub async fn discover_services(&self) -> Result<Vec<BleService>> {
        let device = {
            let device = self.device.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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
                    subscribed: false,
                });
            }

            total_chars += ble_chars.len();

            {
                let mut char_map = self.characteristics.write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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
            let mut svc_map = self.services.write()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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
        let service = {
            let services = self.services.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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
                subscribed: false,
            });
        }

        {
            let mut char_map = self.characteristics.write()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            let chars_vec: Vec<Characteristic> =
                char_data.iter().map(|(_, c, _)| c.clone()).collect();
            char_map.insert(service_uuid.to_string(), chars_vec);
        }

        debug!("发现 {} 个特征", result.len());
        Ok(result)
    }

    pub async fn read_characteristic(&self, char_uuid: &str) -> Result<Vec<u8>> {
        let char = self.find_characteristic(char_uuid).await?;

        let value = char
            .read()
            .await
            .map_err(|e| ComBridgeError::ble(format!("读取失败: {}", e)))?;

        info!("{}", format_ble_log(&self.address, char_uuid, "R", &value));

        {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.rx_buffer.write(&value) {
                    error!("写入接收缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self.caches.write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                let cache = CharacteristicCache::new();
                if let Err(e) = cache.rx_buffer.write(&value) {
                    error!("写入接收缓存失败: {}", e);
                }
                caches.insert(char_uuid.to_string(), cache);
            }
        }

        Ok(value)
    }

    pub async fn write_characteristic(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        info!("{}", format_ble_log(&self.address, char_uuid, "W", data));

        {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self.caches.write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                let cache = CharacteristicCache::new();
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
                caches.insert(char_uuid.to_string(), cache);
            }
        }

        if let Err(e) = char.write(data).await {
            let error_str = format!("{}", e);
            if error_str.contains("已关闭") || error_str.contains("closed") || error_str.contains("disconnected") {
                warn!("[{}] 设备已断开，写入失败: {}", extract_short_mac(&self.address), e);
            } else {
                error!("[{}] 写入特征失败: {}", extract_short_mac(&self.address), e);
            }
            return Err(ComBridgeError::ble(format!("写入失败: {}", e)));
        }

        Ok(())
    }

    pub async fn write_without_response(&self, char_uuid: &str, data: &[u8]) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        info!("{}", format_ble_log(&self.address, char_uuid, "W", data));

        {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self.caches.write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                let cache = CharacteristicCache::new();
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
                caches.insert(char_uuid.to_string(), cache);
            }
        }

        if let Err(e) = char.write_without_response(data).await {
            let error_str = format!("{}", e);
            if error_str.contains("已关闭") || error_str.contains("closed") || error_str.contains("disconnected") {
                warn!("[{}] 设备已断开，写入失败: {}", extract_short_mac(&self.address), e);
            } else {
                error!("[{}] 写入特征失败: {}", extract_short_mac(&self.address), e);
            }
            return Err(ComBridgeError::ble(format!("写入失败: {}", e)));
        }

        Ok(())
    }

    pub async fn subscribe_notify(&self, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        info!("[{}][{}][SUB] 订阅通知", extract_short_mac(&self.address), extract_short_uuid(char_uuid));

        self.notify_callbacks
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .insert(char_uuid.to_string(), callback.clone());

        {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if caches.get(char_uuid).is_none() {
                drop(caches);
                let mut caches = self.caches.write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                caches.insert(char_uuid.to_string(), CharacteristicCache::new());
            }
        }

        let rx_buffer = {
            let caches = self.caches.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            caches.get(char_uuid)
                .map(|c| Arc::clone(&c.rx_buffer))
                .ok_or_else(|| ComBridgeError::ble(format!("特征 {} 缓存未找到", char_uuid)))?
        };

        let uuid = char_uuid.to_string();
        let device_id = self.address.clone();
        let handle = tokio::spawn(async move {
            match char.notify().await {
                Ok(mut notifications) => {
                    while let Some(result) = notifications.next().await {
                        match result {
                            Ok(data) => {
                                if let Err(e) = rx_buffer.write(&data) {
                                    error!("写入通知缓存失败: {}", e);
                                }
                                info!("{}", format_ble_log(&device_id, &uuid, "N", &data));
                                callback(&device_id, &uuid, &data);
                            }
                            Err(e) => {
                                warn!("[{}][{}][N] 通知错误: {}", extract_short_mac(&device_id), extract_short_uuid(&uuid), e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("[{}][{}][SUB] 订阅失败: {}", extract_short_mac(&device_id), extract_short_uuid(&uuid), e);
                }
            }
        });

        self.notify_handles
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .insert(char_uuid.to_string(), handle);

        Ok(())
    }

    pub async fn unsubscribe_notify(&self, char_uuid: &str) -> Result<()> {
        info!("[{}][{}][UNSUB] 取消订阅", extract_short_mac(&self.address), extract_short_uuid(char_uuid));

        self.notify_callbacks.lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .remove(char_uuid);

        if let Some(handle) = self.notify_handles.lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .remove(char_uuid) {
            handle.abort();
        }

        Ok(())
    }

    pub async fn get_rssi(&self) -> Result<i16> {
        let device = {
            let device = self.device.read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
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

    pub fn get_discovered_services(&self) -> Result<Vec<BleService>> {
        let services = self.services.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        let characteristics = self.characteristics.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;

        Ok(services
            .iter()
            .map(|(uuid, svc)| {
                let is_primary = futures::executor::block_on(async { svc.is_primary().await.unwrap_or(true) });

                let ble_chars: Vec<BleCharacteristic> = characteristics
                    .get(uuid)
                    .map(|chars| {
                        chars
                            .iter()
                            .filter_map(|c| {
                                let char_uuid = c.uuid().to_string();
                                let props = futures::executor::block_on(async { c.properties().await.ok() })?;
                                Some(BleCharacteristic {
                                    uuid: char_uuid,
                                    service_uuid: uuid.clone(),
                                    properties: BleCharacteristicProperties {
                                        read: props.read,
                                        write: props.write,
                                        write_without_response: props.write_without_response,
                                        notify: props.notify,
                                        indicate: props.indicate,
                                    },
                                    subscribed: false,
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();

                BleService {
                    uuid: uuid.clone(),
                    primary: is_primary,
                    characteristics: ble_chars,
                }
            })
            .collect())
    }

    pub fn get_cache(&self, char_uuid: &str) -> Result<ChannelCache> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        let cache = caches.get(char_uuid)
            .ok_or_else(|| ComBridgeError::ble(format!("特征 {} 缓存未找到", char_uuid)))?;
        Ok(ChannelCache {
            tx_cache: cache.tx_buffer.get_cache_data()?,
            rx_cache: cache.rx_buffer.get_cache_data()?,
        })
    }

    pub fn clear_cache(&self, char_uuid: &str) -> Result<bool> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        if let Some(cache) = caches.get(char_uuid) {
            cache.tx_buffer.clear()?;
            cache.rx_buffer.clear()?;
            debug!("已清除特征 {} 的缓存", char_uuid);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn get_cache_size(&self, char_uuid: &str) -> Result<Option<(usize, usize)>> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        match caches.get(char_uuid) {
            Some(cache) => Ok(Some((cache.tx_buffer.len()?, cache.rx_buffer.len()?))),
            None => Ok(None),
        }
    }

    pub fn get_all_caches(&self) -> Result<HashMap<String, ChannelCache>> {
        let caches = self.caches.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        caches
            .iter()
            .map(|(uuid, cache)| {
                Ok((
                    uuid.clone(),
                    ChannelCache {
                        tx_cache: cache.tx_buffer.get_cache_data()?,
                        rx_cache: cache.rx_buffer.get_cache_data()?,
                    },
                ))
            })
            .collect()
    }

    async fn find_characteristic(&self, char_uuid: &str) -> Result<Characteristic> {
        let char_map = self.characteristics.read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clone();

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
