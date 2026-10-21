//! 把 Goodix DFU 的 [`Transport`] 契约桥接到项目现有 `BleManager`。
//!
//! 不引入额外 BLE 栈：连接、服务发现、写入与通知全部复用
//! `BleManager`。OTA 只操作已连接的设备，连接由 BLE 页面建立。

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use parking_lot::Mutex;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::device::ble::{
    BleCharacteristic, BleCharacteristicProperties, BleManagerRef, BleMode, NotifyCallback,
};
use crate::error::ComBridgeError;
use crate::ota::ExpectedDisconnects;

use super::error::{DfuError, Result};
use super::transport::{
    Channel, NotificationStream, ReconnectHint, Service, Transport, TransportCapabilities,
    WriteMode,
};

/// Goodix 基础 DFU 服务。
pub const DFU_SERVICE_UUID: &str = "a6ed0401-d344-460a-8075-b9e8ec90d71b";
/// 基础 DFU 通知特征。
pub const DFU_NOTIFY_UUID: &str = "a6ed0402-d344-460a-8075-b9e8ec90d71b";
/// 基础 DFU 命令/数据写入特征。
pub const DFU_WRITE_UUID: &str = "a6ed0403-d344-460a-8075-b9e8ec90d71b";
/// 基础 DFU 控制点特征（DFU V2 使用 indicate）。
pub const DFU_CONTROL_UUID: &str = "a6ed0404-d344-460a-8075-b9e8ec90d71b";
/// Goodix 专用快速 DFU 服务。
pub const FAST_SERVICE_UUID: &str = "a6ed0701-d344-460a-8075-b9e8ec90d71b";
/// 快速 DFU 命令与应答特征。
pub const FAST_COMMAND_UUID: &str = "a6ed0702-d344-460a-8075-b9e8ec90d71b";
/// 快速 DFU 批量数据特征。
pub const FAST_DATA_UUID: &str = "a6ed0703-d344-460a-8075-b9e8ec90d71b";

const FALLBACK_WRITE_LEN: usize = 20;

/// 由一次服务发现得到的能力与绑定。
#[derive(Debug, Default)]
struct Bindings {
    capabilities: TransportCapabilities,
    uuids: HashMap<Channel, String>,
    properties: HashMap<Channel, BleCharacteristicProperties>,
    write_limits: HashMap<Channel, usize>,
}

#[derive(Debug, Default)]
struct TransportState {
    address: String,
    capabilities: TransportCapabilities,
    uuids: HashMap<Channel, String>,
    properties: HashMap<Channel, BleCharacteristicProperties>,
    write_limits: HashMap<Channel, usize>,
    senders: HashMap<Channel, mpsc::UnboundedSender<Vec<u8>>>,
    subscribed: HashSet<Channel>,
}

/// 基于 [`BleManagerRef`] 的 Goodix DFU 传输。
pub struct GoodixBleTransport {
    ble: BleManagerRef,
    state: Mutex<TransportState>,
    expected_disconnects: ExpectedDisconnects,
}

impl GoodixBleTransport {
    /// 打开传输：校验连接状态、发现 Goodix 服务与特征。
    ///
    /// 不主动连接设备；设备需已在 BLE 页面连接。
    pub async fn open(
        ble: BleManagerRef,
        address: impl Into<String>,
        expected_disconnects: ExpectedDisconnects,
    ) -> std::result::Result<Self, ComBridgeError> {
        let requested = address.into();
        let connections = ble
            .get_connections()
            .await
            .map_err(|e| ComBridgeError::ota(format!("读取 BLE 连接状态失败: {e}")))?;
        let connection = connections
            .iter()
            .find(|conn| conn.address.eq_ignore_ascii_case(&requested))
            .ok_or_else(|| {
                ComBridgeError::ota(format!("设备 {requested} 未连接，请先在 BLE 页面连接"))
            })?;
        if !connection.is_connected {
            return Err(ComBridgeError::ota(format!(
                "设备 {requested} 当前不在连接状态"
            )));
        }
        let address = connection.address.clone();

        let services = ble
            .discover_services(&address)
            .await
            .map_err(|e| ComBridgeError::ota(format!("发现 GATT 服务失败: {e}")))?;
        let has_dfu = services
            .iter()
            .any(|service| service.uuid.eq_ignore_ascii_case(DFU_SERVICE_UUID));
        let has_fast = services
            .iter()
            .any(|service| service.uuid.eq_ignore_ascii_case(FAST_SERVICE_UUID));
        if !has_dfu && !has_fast {
            return Err(ComBridgeError::ota(format!(
                "设备 {address} 未提供 Goodix DFU 服务（{DFU_SERVICE_UUID} / {FAST_SERVICE_UUID}）"
            )));
        }

        let dfu_characteristics = if has_dfu {
            discover(&ble, &address, DFU_SERVICE_UUID).await?
        } else {
            Vec::new()
        };
        let fast_characteristics = if has_fast {
            discover(&ble, &address, FAST_SERVICE_UUID).await?
        } else {
            Vec::new()
        };

        let bindings = build_bindings(
            &ble,
            &address,
            has_dfu,
            has_fast,
            &dfu_characteristics,
            &fast_characteristics,
        )
        .await?;

        info!(
            address = address,
            services = ?bindings.capabilities.services,
            mtu = bindings.capabilities.mtu,
            "Goodix BLE transport opened"
        );

        let state = TransportState {
            address,
            capabilities: bindings.capabilities,
            uuids: bindings.uuids,
            properties: bindings.properties,
            write_limits: bindings.write_limits,
            senders: HashMap::new(),
            subscribed: HashSet::new(),
        };
        Ok(Self {
            ble,
            state: Mutex::new(state),
            expected_disconnects,
        })
    }

    /// 当前 BLE 工作模式（原生 / AT）。
    pub async fn ble_mode(&self) -> BleMode {
        self.ble.mode().await
    }

    /// 当前设备地址。
    pub fn device_address(&self) -> String {
        self.state.lock().address.clone()
    }

    fn binding(&self, channel: Channel) -> Option<(String, BleCharacteristicProperties)> {
        let state = self.state.lock();
        let uuid = state.uuids.get(&channel)?.clone();
        let props = state.properties.get(&channel).copied()?;
        Some((uuid, props))
    }

    fn write_limit(&self, channel: Channel) -> usize {
        let state = self.state.lock();
        state
            .write_limits
            .get(&channel)
            .copied()
            .unwrap_or(FALLBACK_WRITE_LEN)
    }

    async fn apply_bindings(&self, bindings: Bindings) {
        let mut state = self.state.lock();
        state.capabilities = bindings.capabilities;
        state.uuids = bindings.uuids;
        state.properties = bindings.properties;
        state.write_limits = bindings.write_limits;
    }

    async fn resubscribe_all(&self) -> Result<()> {
        let (address, channels, senders) = {
            let state = self.state.lock();
            (
                state.address.clone(),
                state.subscribed.iter().copied().collect::<Vec<_>>(),
                state.senders.clone(),
            )
        };

        for channel in channels {
            let Some(sender) = senders.get(&channel).cloned() else {
                continue;
            };
            let Some((uuid, _)) = self.binding(channel) else {
                warn!(?channel, "channel disappeared after reconnect; skipping resubscribe");
                continue;
            };
            let callback: NotifyCallback = Arc::new(move |_addr, _char, data| {
                let _ = sender.send(data.to_vec());
            });
            self.ble
                .subscribe_notify(&address, &uuid, callback)
                .await
                .map_err(|e| {
                    error!(address = address, uuid = uuid, error = %e, "resubscribe failed");
                    DfuError::Transport(format!(
                        "重新订阅 {uuid} 失败（设备 {address}）: {e}"
                    ))
                })?;
            debug!(address = address, uuid = uuid, "resubscribed after reconnect");
        }
        Ok(())
    }
}

fn channel_uuid(channel: Channel) -> &'static str {
    match channel {
        Channel::DfuNotify => DFU_NOTIFY_UUID,
        Channel::DfuWrite => DFU_WRITE_UUID,
        Channel::DfuControl => DFU_CONTROL_UUID,
        Channel::FastCommand => FAST_COMMAND_UUID,
        Channel::FastData => FAST_DATA_UUID,
    }
}

async fn discover(
    ble: &BleManagerRef,
    address: &str,
    service_uuid: &str,
) -> std::result::Result<Vec<BleCharacteristic>, ComBridgeError> {
    ble.discover_characteristics(address, service_uuid)
        .await
        .map_err(|e| {
            ComBridgeError::ota(format!(
                "发现服务 {service_uuid} 的特征失败（设备 {address}）: {e}"
            ))
        })
}

async fn build_bindings(
    ble: &BleManagerRef,
    address: &str,
    has_dfu: bool,
    has_fast: bool,
    dfu_characteristics: &[BleCharacteristic],
    fast_characteristics: &[BleCharacteristic],
) -> std::result::Result<Bindings, ComBridgeError> {
    let mut bindings = Bindings::default();
    if has_dfu {
        bindings.capabilities.services.push(Service::Dfu);
    }
    if has_fast {
        bindings.capabilities.services.push(Service::FastDfu);
    }

    let channels = [
        Channel::DfuNotify,
        Channel::DfuWrite,
        Channel::DfuControl,
        Channel::FastCommand,
        Channel::FastData,
    ];

    for channel in channels {
        let uuid = channel_uuid(channel);
        let characteristic = if matches!(channel, Channel::FastCommand | Channel::FastData) {
            find_characteristic(fast_characteristics, uuid)
        } else {
            find_characteristic(dfu_characteristics, uuid)
        };
        let Some(characteristic) = characteristic else {
            continue;
        };

        bindings.uuids.insert(channel, uuid.to_string());
        bindings
            .properties
            .insert(channel, characteristic.properties);

        if channel == Channel::DfuControl && characteristic.properties.indicate {
            bindings.capabilities.indicate_channels.push(channel);
        }

        if matches!(
            channel,
            Channel::DfuWrite | Channel::DfuControl | Channel::FastCommand | Channel::FastData
        ) {
            let limit = ble
                .max_write_len(address, uuid)
                .await
                .map(|len| len.max(FALLBACK_WRITE_LEN))
                .unwrap_or_else(|e| {
                    warn!(
                        address = address,
                        uuid = uuid,
                        error = %e,
                        "读取特征写入长度失败，回退 {FALLBACK_WRITE_LEN} 字节"
                    );
                    FALLBACK_WRITE_LEN
                });
            bindings.write_limits.insert(channel, limit);
        }
    }

    let dfu_limit = bindings
        .write_limits
        .get(&Channel::DfuWrite)
        .copied()
        .or_else(|| bindings.write_limits.get(&Channel::FastData).copied())
        .unwrap_or(FALLBACK_WRITE_LEN);
    bindings.capabilities.mtu = (dfu_limit + 3) as u16;

    Ok(bindings)
}

fn find_characteristic<'a>(
    characteristics: &'a [BleCharacteristic],
    uuid: &str,
) -> Option<&'a BleCharacteristic> {
    characteristics
        .iter()
        .find(|characteristic| characteristic.uuid.eq_ignore_ascii_case(uuid))
}

#[async_trait]
impl Transport for GoodixBleTransport {
    async fn connect(&self) -> Result<()> {
        let address = self.device_address();
        let connections = self
            .ble
            .get_connections()
            .await
            .map_err(|e| DfuError::Transport(format!("读取连接状态失败: {e}")))?;
        match connections
            .iter()
            .find(|conn| conn.address.eq_ignore_ascii_case(&address))
        {
            Some(conn) if conn.is_connected => Ok(()),
            Some(_) => Err(DfuError::Transport(format!(
                "设备 {address} 当前不在连接状态"
            ))),
            None => Err(DfuError::Transport(format!(
                "设备 {address} 已断开连接"
            ))),
        }
    }

    async fn disconnect(&self) -> Result<()> {
        let (address, uuids) = {
            let state = self.state.lock();
            let uuids = state
                .uuids
                .iter()
                .filter(|(channel, _)| state.subscribed.contains(channel))
                .map(|(_, uuid)| uuid.clone())
                .collect::<Vec<_>>();
            (state.address.clone(), uuids)
        };

        for uuid in uuids {
            if let Err(e) = self.ble.unsubscribe_notify(&address, &uuid).await {
                warn!(address = address, uuid = uuid, error = %e, "取消订阅失败");
            }
        }
        let mut state = self.state.lock();
        state.subscribed.clear();
        state.senders.clear();
        info!(address = address, "Goodix BLE transport detached");
        Ok(())
    }

    fn capabilities(&self) -> TransportCapabilities {
        self.state.lock().capabilities.clone()
    }

    async fn subscribe(&self, channel: Channel) -> Result<NotificationStream> {
        let (uuid, _) = self.binding(channel).ok_or_else(|| {
            DfuError::Unsupported(format!(
                "设备未提供通道 {channel:?} 对应的特征 {}",
                channel_uuid(channel)
            ))
        })?;
        let address = self.device_address();

        // BleManager 每个特征只保留一个回调，重复订阅前先取消旧的。
        let already = self.state.lock().subscribed.contains(&channel);
        if already {
            if let Err(e) = self.ble.unsubscribe_notify(&address, &uuid).await {
                debug!(address = address, uuid = uuid, error = %e, "取消旧订阅失败");
            }
        }

        let (sender, receiver) = mpsc::unbounded_channel();
        let sender_in_callback = sender.clone();
        let callback: NotifyCallback = Arc::new(move |_addr, _char, data| {
            let _ = sender_in_callback.send(data.to_vec());
        });

        self.ble
            .subscribe_notify(&address, &uuid, callback)
            .await
            .map_err(|e| {
                error!(address = address, uuid = uuid, error = %e, "订阅 DFU 通知失败");
                DfuError::Transport(format!("订阅 {uuid} 失败（设备 {address}）: {e}"))
            })?;

        let mut state = self.state.lock();
        state.subscribed.insert(channel);
        state.senders.insert(channel, sender);
        debug!(address = address, uuid = uuid, ?channel, "subscribed");
        Ok(receiver)
    }

    async fn write(&self, channel: Channel, data: &[u8], mode: WriteMode) -> Result<()> {
        let (uuid, props) = self.binding(channel).ok_or_else(|| {
            DfuError::Unsupported(format!(
                "设备未提供通道 {channel:?} 对应的特征 {}",
                channel_uuid(channel)
            ))
        })?;
        let address = self.device_address();
        let chunk_size = self.write_limit(channel);

        let without_response = match mode {
            WriteMode::Auto => props.write_without_response,
            WriteMode::WithoutResponse => true,
            WriteMode::WithResponse => false,
        };
        if !without_response && !props.write {
            return Err(DfuError::Unsupported(format!(
                "特征 {uuid} 不支持有响应写入（设备 {address}）"
            )));
        }
        if without_response && !props.write_without_response && !props.write {
            return Err(DfuError::Unsupported(format!(
                "特征 {uuid} 不支持写入（设备 {address}）"
            )));
        }

        for chunk in data.chunks(chunk_size) {
            let result = if without_response && props.write_without_response {
                self.ble
                    .write_without_response(&address, &uuid, chunk)
                    .await
            } else {
                self.ble.write_characteristic(&address, &uuid, chunk).await
            };
            if let Err(e) = result {
                error!(
                    address = address,
                    uuid = uuid,
                    channel = ?channel,
                    len = chunk.len(),
                    error = %e,
                    "DFU 写入失败"
                );
                return Err(DfuError::Transport(format!(
                    "写入 {uuid} 失败（设备 {address}，通道 {channel:?}，{} 字节）: {e}",
                    chunk.len()
                )));
            }
        }
        Ok(())
    }

    async fn reconnect(&self, hint: ReconnectHint) -> Result<()> {
        let old_address = self.device_address();
        let new_address = hint.address.unwrap_or_else(|| old_address.clone());

        if !hint.delay.is_zero() {
            tokio::time::sleep(hint.delay).await;
        }

        // 主动断开旧连接以切换到 Bootloader 地址，登记为预期断开。
        self.expected_disconnects.mark(&old_address);
        if let Err(e) = self.ble.disconnect(&old_address).await {
            debug!(address = old_address, error = %e, "断开旧连接失败（设备可能已重启）");
        }

        self.ble.connect(&new_address).await.map_err(|e| {
            error!(address = new_address, error = %e, "重连 DFU 设备失败");
            DfuError::Transport(format!("重连 {new_address} 失败: {e}"))
        })?;

        let services = self
            .ble
            .discover_services(&new_address)
            .await
            .map_err(|e| DfuError::Transport(format!("重连后发现服务失败: {e}")))?;
        let has_dfu = services
            .iter()
            .any(|service| service.uuid.eq_ignore_ascii_case(DFU_SERVICE_UUID));
        let has_fast = services
            .iter()
            .any(|service| service.uuid.eq_ignore_ascii_case(FAST_SERVICE_UUID));
        let dfu_characteristics = if has_dfu {
            discover(&self.ble, &new_address, DFU_SERVICE_UUID)
                .await
                .map_err(|e| DfuError::Transport(e.to_string()))?
        } else {
            Vec::new()
        };
        let fast_characteristics = if has_fast {
            discover(&self.ble, &new_address, FAST_SERVICE_UUID)
                .await
                .map_err(|e| DfuError::Transport(e.to_string()))?
        } else {
            Vec::new()
        };
        let bindings = build_bindings(
            &self.ble,
            &new_address,
            has_dfu,
            has_fast,
            &dfu_characteristics,
            &fast_characteristics,
        )
        .await
        .map_err(|e| DfuError::Transport(e.to_string()))?;

        {
            let mut state = self.state.lock();
            state.address = new_address.clone();
        }
        self.apply_bindings(bindings).await;
        self.resubscribe_all().await?;

        info!(
            from = old_address,
            to = new_address,
            "Goodix BLE transport reconnected"
        );
        Ok(())
    }

    fn address(&self) -> Option<String> {
        Some(self.device_address())
    }

    fn mark_expected_disconnect(&self) {
        self.expected_disconnects.mark(&self.device_address());
    }
}

impl Drop for GoodixBleTransport {
    fn drop(&mut self) {
        let (address, uuids) = {
            let state = self.state.lock();
            let uuids = state
                .uuids
                .iter()
                .filter(|(channel, _)| state.subscribed.contains(channel))
                .map(|(_, uuid)| uuid.clone())
                .collect::<Vec<_>>();
            (state.address.clone(), uuids)
        };
        if uuids.is_empty() {
            return;
        }
        // Drop 不能 await：尽力在后台取消订阅，避免回调泄漏。
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let ble = Arc::clone(&self.ble);
            handle.spawn(async move {
                for uuid in uuids {
                    let _ = ble.unsubscribe_notify(&address, &uuid).await;
                }
            });
        }
    }
}

/// 供预检使用的默认命令超时。
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(10);
