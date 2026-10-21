//! 传输抽象：核心逻辑不依赖具体 BLE 栈。
//!
//! ComBridge 中由 `GoodixBleTransport` 桥接现有 `BleManager`。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::error::{DfuError, Result};

/// 写入或通知所属的逻辑通道。
///
/// 到 GATT 特征的映射由传输实现完成。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    /// `a6ed0402`：基础 DFU 服务的通知。
    DfuNotify,
    /// `a6ed0403`：基础 DFU 服务的命令与数据写入。
    DfuWrite,
    /// `a6ed0404`：基础 DFU 服务的控制点。
    DfuControl,
    /// `a6ed0702`：快速 DFU 服务的命令与应答通道。
    FastCommand,
    /// `a6ed0703`：快速 DFU 服务的批量数据通道。
    FastData,
}

/// 设备暴露的 GATT 服务。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Service {
    /// 基础 DFU 服务（`a6ed0401`），DFU V1 / V2 使用。
    Dfu,
    /// 专用快速 DFU 服务（`a6ed0701`）。
    FastDfu,
}

/// BLE 写入语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    /// 由传输依据特征属性自动选择。
    Auto,
    /// 有响应写入。
    WithResponse,
    /// 无响应写入。
    WithoutResponse,
}

/// 连接后发现的能力集合。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportCapabilities {
    /// 设备上存在的服务。
    pub services: Vec<Service>,
    /// 支持 indicate（而非仅 notify）的通道。
    pub indicate_channels: Vec<Channel>,
    /// 协商出的 MTU，未知时为 `0`。
    pub mtu: u16,
}

impl TransportCapabilities {
    /// 服务是否存在。
    pub fn has_service(&self, service: Service) -> bool {
        self.services.contains(&service)
    }

    /// 通道是否支持 indicate。
    pub fn indicates(&self, channel: Channel) -> bool {
        self.indicate_channels.contains(&channel)
    }

    /// 单次无响应写入可承载的载荷长度。
    pub fn write_chunk_size(&self) -> usize {
        if self.mtu > 3 {
            usize::from(self.mtu - 3)
        } else {
            20
        }
    }
}

/// 通知载荷流（每个 BLE 通知一个条目）。
pub type NotificationStream = mpsc::UnboundedReceiver<Vec<u8>>;

/// 传给 [`Transport::reconnect`] 的提示。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconnectHint {
    /// 重连后的首选地址，通常是 MAC 加一。
    pub address: Option<String>,
    /// 设备重新广播前的等待时间。
    pub delay: Duration,
}

/// 到 DFU 设备的双向字节通道。
///
/// 核心只需要分帧写入与通知流；扫描、GATT 发现、MTU、配对等
/// BLE 细节全部留在实现中。
#[async_trait]
pub trait Transport: Send + Sync {
    /// 连接设备并发现 DFU 服务。
    async fn connect(&self) -> Result<()>;

    /// 关闭连接。
    async fn disconnect(&self) -> Result<()>;

    /// 返回 [`Transport::connect`] 发现的能力。
    fn capabilities(&self) -> TransportCapabilities;

    /// 订阅 `channel` 的通知。
    async fn subscribe(&self, channel: Channel) -> Result<NotificationStream>;

    /// 向 `channel` 写入 `data`。
    async fn write(&self, channel: Channel, data: &[u8], mode: WriteMode) -> Result<()>;

    /// 读取最多 `buf.len()` 字节（若传输支持直接读取）。
    ///
    /// DFU 驱动只使用通知，默认实现返回不支持。
    async fn read(&self, _channel: Channel, _buf: &mut [u8], _timeout: Duration) -> Result<usize> {
        Err(DfuError::Unsupported(
            "this transport does not support direct reads".to_string(),
        ))
    }

    /// 重连，可选切换到别的地址。
    ///
    /// 仅单 Bank V2 场景需要：Bootloader 重启后 MAC 地址加一。
    async fn reconnect(&self, _hint: ReconnectHint) -> Result<()> {
        Err(DfuError::Unsupported(
            "this transport does not support reconnecting".to_string(),
        ))
    }

    /// 当前连接设备地址（已知时）。
    fn address(&self) -> Option<String> {
        None
    }

    /// 登记设备即将因升级而重启断开。
    ///
    /// 默认实现为空；BLE 传输会把它转给 `OtaManager` 的预期断开标记，
    /// 避免复位过程中的链路丢失被误判为升级失败。
    fn mark_expected_disconnect(&self) {}
}
