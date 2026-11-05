use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

use bluest::{Characteristic, ConnectionEvent, Device, Service};
use futures::StreamExt;
use tracing::{debug, error, info, warn};

use super::super::ble_traits::{
    BleCharacteristic, BleCharacteristicProperties, BleService, NotifyCallback,
};
use crate::device::cache::{create_ring_buffer, ChannelCache, RingBufferRef};
use crate::error::{ComBridgeError, Result};

pub type DisconnectCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// 连接监控宽限期：连接刚建立时 GattSession 状态未稳定，不判定断开
const CONNECTION_GRACE_PERIOD: std::time::Duration = std::time::Duration::from_secs(10);

/// 断连二次确认延迟：系统推送的断开事件可能只是瞬时抖动，
/// 短暂等待后复查连接状态，仍为断开才认定真断开
const DISCONNECT_CONFIRM_DELAY: std::time::Duration = std::time::Duration::from_millis(800);

/// 事件流空闲时兜底轮询的间隔计数（每 N 次事件流超时做一次状态复查）
const IDLE_POLL_EVERY: u32 = 10;

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

fn format_hex_preview(data: &[u8]) -> String {
    const LIMIT: usize = 512;
    let shown = data.len().min(LIMIT);
    let mut output = String::new();
    for (idx, byte) in data.iter().take(shown).enumerate() {
        if idx > 0 {
            output.push(' ');
        }
        output.push_str(&format!("{:02X}", byte));
    }
    if data.len() > shown {
        output.push_str(&format!(" ...(+{} bytes)", data.len() - shown));
    }
    output
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
    disconnect_callback: Arc<RwLock<Option<DisconnectCallback>>>,
    connection_monitor_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
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
            disconnect_callback: Arc::new(RwLock::new(None)),
            connection_monitor_handle: Mutex::new(None),
        }
    }

    pub fn set_disconnect_callback(&self, callback: DisconnectCallback) {
        if let Ok(mut cb) = self.disconnect_callback.write() {
            *cb = Some(callback);
        }
    }

    pub fn reset_state(&self) {
        if let Ok(mut monitor) = self.connection_monitor_handle.lock() {
            if let Some(handle) = monitor.take() {
                handle.abort();
            }
        }

        if let Ok(mut device_guard) = self.device.write() {
            *device_guard = None;
        }

        if let Ok(mut services) = self.services.write() {
            services.clear();
        }

        if let Ok(mut characteristics) = self.characteristics.write() {
            characteristics.clear();
        }

        if let Ok(mut caches) = self.caches.write() {
            caches.clear();
        }

        if let Ok(mut handles) = self.notify_handles.lock() {
            for (_, handle) in handles.drain() {
                handle.abort();
            }
        }

        if let Ok(mut callbacks) = self.notify_callbacks.lock() {
            callbacks.clear();
        }

        info!(
            "[{}] GattClient 状态已重置",
            extract_short_mac(&self.address)
        );
    }

    /// 异步释放底层 GATT 连接（不清理业务状态）。
    ///
    /// 被动断开路径专用：Windows 的 WinRT GattSession 可能仍持有连接，设备端会一直
    /// 保持"已连接"而停止广播，表现为必须复位器件才能重新被发现。这里在清理本地
    /// 状态之前显式 disconnect，保证链路真正释放、设备恢复广播。
    pub fn spawn_release_connection(&self) {
        let device = self
            .device
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());
        let adapter = self
            .adapter
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());

        let (Some(device), Some(adapter)) = (device, adapter) else {
            debug!(
                "[{}] 无需释放底层连接（设备或适配器已释放）",
                extract_short_mac(&self.address)
            );
            return;
        };

        let address = self.address.clone();
        tokio::spawn(async move {
            match adapter.disconnect_device(&device).await {
                Ok(()) => info!(
                    "[{}] 已释放底层 GATT 连接，设备可恢复广播",
                    extract_short_mac(&address)
                ),
                Err(e) => warn!(
                    "[{}] 释放底层 GATT 连接失败: {}（设备可能仍处于假连接状态）",
                    extract_short_mac(&address),
                    e
                ),
            }
        });
    }

    pub fn set_device(&self, device: Arc<Device>, adapter: Arc<bluest::Adapter>) -> Result<()> {
        *self
            .device
            .write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))? = Some(device);
        *self
            .adapter
            .write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))? = Some(adapter);
        Ok(())
    }

    pub fn has_device(&self) -> Result<bool> {
        let device = self
            .device
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        Ok(device.is_some())
    }

    pub fn is_connected(&self) -> Result<bool> {
        let device = self
            .device
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        if let Some(device) = device.as_ref() {
            Ok(tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async { device.is_connected().await })
            }))
        } else {
            Ok(false)
        }
    }

    pub async fn connect(&self) -> Result<()> {
        let (device, adapter) = {
            let device = self
                .device
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            let adapter = self
                .adapter
                .read()
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

        self.start_connection_monitor();

        info!("设备连接成功: {}", self.address);
        Ok(())
    }

    fn start_connection_monitor(&self) {
        if let Ok(mut monitor) = self.connection_monitor_handle.lock() {
            if let Some(handle) = monitor.take() {
                handle.abort();
                info!(
                    "[{}] 已停止旧的连接监控任务",
                    extract_short_mac(&self.address)
                );
            }
        }

        let device = {
            let device_guard = self
                .device
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)));
            match device_guard {
                Ok(guard) => guard.as_ref().map(|d| d.clone()),
                Err(_) => None,
            }
        };

        let device = match device {
            Some(d) => d,
            None => return,
        };

        let adapter = self
            .adapter
            .read()
            .ok()
            .and_then(|guard| guard.as_ref().cloned());

        let address = self.address.clone();
        let callback_arc = self.disconnect_callback.clone();

        let handle = tokio::spawn(async move {
            // 连接后宽限期：bluest 在 Windows 下 connect 返回后 GattSession 状态
            // 可能短暂未就绪（is_connected 返回 false），立即判定会误报断开并触发
            // 通道清理（曾出现连接成功 6 秒后误报的案例）。宽限 10 秒后再开始判定。
            tokio::time::sleep(CONNECTION_GRACE_PERIOD).await;
            info!("[{}] 连接监控任务已启动", extract_short_mac(&address));

            // 优先使用系统推送的连接事件（Windows 由 BluetoothLEDevice 的
            // ConnectionStatusChanged 驱动），比固定间隔轮询更及时，也不会
            // 因为轮询采样点错过短暂状态变化。
            //
            // 注意：事件流借用 adapter 与 device，因此 adapter_handle 必须绑定在
            // 外层作用域，不能写在 match 分支里（否则借用提前结束）。
            let adapter_handle = adapter;
            let mut event_stream = match adapter_handle.as_ref() {
                Some(adapter_ref) => {
                    match adapter_ref.device_connection_events(&device).await {
                        Ok(stream) => {
                            info!(
                                "[{}] 已订阅系统连接事件流",
                                extract_short_mac(&address)
                            );
                            Some(Box::pin(stream))
                        }
                        Err(e) => {
                            warn!(
                                "[{}] 订阅连接事件流失败: {}，回退到轮询",
                                extract_short_mac(&address),
                                e
                            );
                            None
                        }
                    }
                }
                None => None,
            };

            let mut idle_rounds = 0u32;
            loop {
                let detected = match event_stream.as_mut() {
                    Some(stream) => {
                        match tokio::time::timeout(
                            std::time::Duration::from_secs(3),
                            stream.next(),
                        )
                        .await
                        {
                            Ok(Some(ConnectionEvent::Disconnected)) => true,
                            Ok(Some(ConnectionEvent::Connected)) => false,
                            Ok(None) => {
                                // 事件流结束（设备对象销毁等），退回轮询
                                warn!(
                                    "[{}] 连接事件流结束，回退到轮询",
                                    extract_short_mac(&address)
                                );
                                event_stream = None;
                                continue;
                            }
                            Err(_) => {
                                // 超时无事件：定期兜底复查一次，避免事件丢失导致永不判定
                                idle_rounds = idle_rounds.wrapping_add(1);
                                if idle_rounds % IDLE_POLL_EVERY == 0 {
                                    !device.is_connected().await
                                } else {
                                    false
                                }
                            }
                        }
                    }
                    None => {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        !device.is_connected().await
                    }
                };

                if !detected {
                    continue;
                }

                // 二次确认：系统连接状态存在瞬时抖动，短暂等待后复查再定性
                tokio::time::sleep(DISCONNECT_CONFIRM_DELAY).await;
                if device.is_connected().await {
                    debug!(
                        "[{}] 检测到瞬时断连抖动，复查已恢复，忽略",
                        extract_short_mac(&address)
                    );
                    continue;
                }

                info!(
                    "[{}] 确认设备断开连接（事件流+复查）",
                    extract_short_mac(&address)
                );
                if let Ok(cb_guard) = callback_arc.read() {
                    if let Some(cb) = cb_guard.as_ref() {
                        cb(&address);
                    }
                }
                break;
            }
        });

        if let Ok(mut monitor) = self.connection_monitor_handle.lock() {
            *monitor = Some(handle);
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        if let Ok(mut monitor) = self.connection_monitor_handle.lock() {
            if let Some(handle) = monitor.take() {
                handle.abort();
            }
        }

        let (device, adapter) = {
            let device = self
                .device
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            let adapter = self
                .adapter
                .read()
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
            let mut device_guard = self
                .device
                .write()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            *device_guard = None;
        }

        self.services
            .write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();
        self.characteristics
            .write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();
        self.caches
            .write()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clear();

        let mut handles = self
            .notify_handles
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        for (_, handle) in handles.drain() {
            handle.abort();
        }

        info!("设备已断开: {}", self.address);
        Ok(())
    }

    pub async fn discover_services(&self) -> Result<Vec<BleService>> {
        let device = {
            let device = self
                .device
                .read()
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
            let mut char_data: Vec<(String, Characteristic, BleCharacteristicProperties)> =
                Vec::new();

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
                let mut char_map = self
                    .characteristics
                    .write()
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
            let mut svc_map = self
                .services
                .write()
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
            let services = self
                .services
                .read()
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
            let mut char_map = self
                .characteristics
                .write()
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
            let caches = self
                .caches
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.rx_buffer.write(&value) {
                    error!("写入接收缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self
                    .caches
                    .write()
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
        tracing::debug!(
            "[BLE][TX_RAW] device={}, char={}, len={}, bytes={}",
            extract_short_mac(&self.address),
            extract_short_uuid(char_uuid),
            data.len(),
            format_hex_preview(data)
        );

        {
            let caches = self
                .caches
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self
                    .caches
                    .write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                let cache = CharacteristicCache::new();
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
                caches.insert(char_uuid.to_string(), cache);
            }
        }

        // 获取特征属性，决定使用哪种写入方式
        let props = char
            .properties()
            .await
            .map_err(|e| ComBridgeError::ble(format!("获取特征属性失败: {}", e)))?;

        // 根据特征属性选择写入方式
        let write_result = if props.write_without_response && !props.write {
            // 特征只支持 write without response
            tracing::debug!(
                "[BLE] 使用 write_without_response: device={}, char={}",
                extract_short_mac(&self.address),
                extract_short_uuid(char_uuid)
            );
            char.write_without_response(data).await
        } else {
            // 默认使用 write with response
            tracing::debug!(
                "[BLE] 使用 write_with_response: device={}, char={}",
                extract_short_mac(&self.address),
                extract_short_uuid(char_uuid)
            );
            char.write(data).await
        };

        if let Err(e) = write_result {
            let error_str = format!("{}", e);
            if error_str.contains("已关闭")
                || error_str.contains("closed")
                || error_str.contains("disconnected")
            {
                warn!(
                    "[{}] 设备已断开，写入失败: {}",
                    extract_short_mac(&self.address),
                    e
                );
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
        tracing::debug!(
            "[BLE][TX_RAW] device={}, char={}, without_response=true, len={}, bytes={}",
            extract_short_mac(&self.address),
            extract_short_uuid(char_uuid),
            data.len(),
            format_hex_preview(data)
        );

        {
            let caches = self
                .caches
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if let Some(cache) = caches.get(char_uuid) {
                if let Err(e) = cache.tx_buffer.write(data) {
                    error!("写入发送缓存失败: {}", e);
                }
            } else {
                drop(caches);
                let mut caches = self
                    .caches
                    .write()
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
            if error_str.contains("已关闭")
                || error_str.contains("closed")
                || error_str.contains("disconnected")
            {
                warn!(
                    "[{}] 设备已断开，写入失败: {}",
                    extract_short_mac(&self.address),
                    e
                );
            } else {
                error!("[{}] 写入特征失败: {}", extract_short_mac(&self.address), e);
            }
            return Err(ComBridgeError::ble(format!("写入失败: {}", e)));
        }

        Ok(())
    }

    pub async fn subscribe_notify(&self, char_uuid: &str, callback: NotifyCallback) -> Result<()> {
        let char = self.find_characteristic(char_uuid).await?;

        info!(
            "[{}][{}][SUB] 订阅通知",
            extract_short_mac(&self.address),
            extract_short_uuid(char_uuid)
        );

        self.notify_callbacks
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .insert(char_uuid.to_string(), callback.clone());

        {
            let caches = self
                .caches
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            if caches.get(char_uuid).is_none() {
                drop(caches);
                let mut caches = self
                    .caches
                    .write()
                    .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
                caches.insert(char_uuid.to_string(), CharacteristicCache::new());
            }
        }

        let rx_buffer = {
            let caches = self
                .caches
                .read()
                .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
            caches
                .get(char_uuid)
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
                                tracing::debug!(
                                    "[BLE][RX_RAW] device={}, char={}, len={}, bytes={}",
                                    extract_short_mac(&device_id),
                                    extract_short_uuid(&uuid),
                                    data.len(),
                                    format_hex_preview(&data)
                                );
                                callback(&device_id, &uuid, &data);
                            }
                            Err(e) => {
                                warn!(
                                    "[{}][{}][N] 通知错误: {}",
                                    extract_short_mac(&device_id),
                                    extract_short_uuid(&uuid),
                                    e
                                );
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "[{}][{}][SUB] 订阅失败: {}",
                        extract_short_mac(&device_id),
                        extract_short_uuid(&uuid),
                        e
                    );
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
        info!(
            "[{}][{}][UNSUB] 取消订阅",
            extract_short_mac(&self.address),
            extract_short_uuid(char_uuid)
        );

        self.notify_callbacks
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .remove(char_uuid);

        if let Some(handle) = self
            .notify_handles
            .lock()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .remove(char_uuid)
        {
            handle.abort();
        }

        Ok(())
    }

    pub async fn get_rssi(&self) -> Result<i16> {
        let device = {
            let device = self
                .device
                .read()
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
        // bluest 在 Windows 后端不暴露 GattSession.MaxPduSize，无法主动协商 MTU：
        // Windows 蓝牙栈会在连接建立时自动协商最大 MTU。这里仅记录请求值并返回
        // clamp 结果，不假装完成了协商（此前静默返回导致上层误以为协商成功）。
        let clamped = mtu.clamp(23, 517);
        warn!(
            "[{}] MTU 请求 {}：Windows 蓝牙栈自动管理 MTU，无法主动协商；返回 clamp 值 {}（实际 MTU 以栈协商结果为准）",
            extract_short_mac(&self.address),
            mtu,
            clamped
        );
        Ok(clamped)
    }

    pub async fn max_write_len(&self, char_uuid: &str) -> Result<usize> {
        let characteristic = self.find_characteristic(char_uuid).await?;
        characteristic
            .max_write_len_async()
            .await
            .map_err(|e| ComBridgeError::ble(format!("获取写入长度失败: {}", e)))
    }

    pub fn get_discovered_services(&self) -> Result<Vec<BleService>> {
        let services = self
            .services
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        let characteristics = self
            .characteristics
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;

        Ok(services
            .iter()
            .map(|(uuid, svc)| {
                let is_primary = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(async { svc.is_primary().await.unwrap_or(true) })
                });

                let ble_chars: Vec<BleCharacteristic> = characteristics
                    .get(uuid)
                    .map(|chars| {
                        chars
                            .iter()
                            .filter_map(|c| {
                                let char_uuid = c.uuid().to_string();
                                let props = tokio::task::block_in_place(|| {
                                    tokio::runtime::Handle::current()
                                        .block_on(async { c.properties().await.ok() })
                                })?;
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
        let caches = self
            .caches
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        let cache = caches
            .get(char_uuid)
            .ok_or_else(|| ComBridgeError::ble(format!("特征 {} 缓存未找到", char_uuid)))?;
        Ok(ChannelCache {
            tx_cache: cache.tx_buffer.get_cache_data()?,
            rx_cache: cache.rx_buffer.get_cache_data()?,
        })
    }

    pub fn clear_cache(&self, char_uuid: &str) -> Result<bool> {
        let caches = self
            .caches
            .read()
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
        let caches = self
            .caches
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?;
        match caches.get(char_uuid) {
            Some(cache) => Ok(Some((cache.tx_buffer.len()?, cache.rx_buffer.len()?))),
            None => Ok(None),
        }
    }

    pub fn get_all_caches(&self) -> Result<HashMap<String, ChannelCache>> {
        let caches = self
            .caches
            .read()
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
        let char_map = self
            .characteristics
            .read()
            .map_err(|e| ComBridgeError::ble(format!("锁获取失败: {}", e)))?
            .clone();

        let norm = |s: &str| s.trim().to_lowercase();
        let want = norm(char_uuid);
        let mut seen: Vec<String> = Vec::new();
        for chars in char_map.values() {
            for c in chars {
                let got = c.uuid().to_string();
                seen.push(got.clone());
                if norm(&got) == want {
                    return Ok(c.clone());
                }
            }
        }

        error!(
            "特征未找到: {} | 已发现的特征: {:?}",
            char_uuid, seen
        );
        Err(ComBridgeError::ble(format!("特征未找到: {}", char_uuid)))
    }
}
