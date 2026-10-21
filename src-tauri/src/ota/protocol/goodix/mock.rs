//! 测试与回放会话使用的内存传输。
//!
//! 仅在 `cfg(test)` 下编译，避免生产代码暴露测试专用 API。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;

use super::error::{DfuError, Result};
use super::transport::{
    Channel, NotificationStream, ReconnectHint, Service, Transport, TransportCapabilities,
    WriteMode,
};

/// 每次写入时调用的回调。
///
/// 返回应按 `(channel, payload)` 投递给订阅者的通知。
pub type MockHandler = Arc<dyn Fn(Channel, &[u8]) -> Vec<(Channel, Vec<u8>)> + Send + Sync>;

/// 一次被记录的写入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteRecord {
    /// 目标通道。
    pub channel: Channel,
    /// 原始字节。
    pub data: Vec<u8>,
}

#[derive(Default)]
struct MockState {
    connected: bool,
    subscribers: HashMap<Channel, Vec<mpsc::UnboundedSender<Vec<u8>>>>,
    writes: Vec<WriteRecord>,
    connect_count: usize,
    reconnect_count: usize,
    last_reconnect_hint: Option<ReconnectHint>,
    address: Option<String>,
}

/// 由脚本化处理器产生响应的传输。
pub struct MockTransport {
    capabilities: TransportCapabilities,
    handler: MockHandler,
    state: Mutex<MockState>,
}

impl MockTransport {
    /// 创建 mock 传输。
    pub fn new<F>(capabilities: TransportCapabilities, handler: F) -> Self
    where
        F: Fn(Channel, &[u8]) -> Vec<(Channel, Vec<u8>)> + Send + Sync + 'static,
    {
        Self {
            capabilities,
            handler: Arc::new(handler),
            state: Mutex::new(MockState::default()),
        }
    }

    /// 创建上报 `address` 的 mock 传输。
    pub fn with_address<F>(
        capabilities: TransportCapabilities,
        address: impl Into<String>,
        handler: F,
    ) -> Self
    where
        F: Fn(Channel, &[u8]) -> Vec<(Channel, Vec<u8>)> + Send + Sync + 'static,
    {
        let transport = Self::new(capabilities, handler);
        {
            let mut state = transport.lock_state();
            state.address = Some(address.into());
        }
        transport
    }

    /// 创建能力固定、无响应的 mock。
    pub fn silent(capabilities: TransportCapabilities) -> Arc<Self> {
        Arc::new(Self::new(capabilities, |_, _| Vec::new()))
    }

    /// 按顺序记录的写入。
    pub fn writes(&self) -> Vec<WriteRecord> {
        self.lock_state().writes.clone()
    }

    /// 清空写入记录。
    pub fn clear_writes(&self) {
        self.lock_state().writes.clear();
    }

    /// 成功调用 `connect` 的次数。
    pub fn connect_count(&self) -> usize {
        self.lock_state().connect_count
    }

    /// 调用 `reconnect` 的次数。
    pub fn reconnect_count(&self) -> usize {
        self.lock_state().reconnect_count
    }

    /// 最后一次重连提示。
    pub fn last_reconnect_hint(&self) -> Option<ReconnectHint> {
        self.lock_state().last_reconnect_hint.clone()
    }

    /// 向 `channel` 的所有订阅者推送通知。
    pub fn push_notification(&self, channel: Channel, payload: Vec<u8>) {
        let state = self.lock_state();
        if let Some(subscribers) = state.subscribers.get(&channel) {
            for sender in subscribers {
                let _ = sender.send(payload.clone());
            }
        }
    }

    /// 强制设置连接标志，用于测试失败路径。
    pub fn set_connected(&self, connected: bool) {
        self.lock_state().connected = connected;
    }

    fn lock_state(&self) -> MutexGuard<'_, MockState> {
        match self.state.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn connect(&self) -> Result<()> {
        {
            let mut state = self.lock_state();
            state.connected = true;
            state.connect_count += 1;
        }
        tracing::info!("mock transport connected");
        Ok(())
    }

    async fn disconnect(&self) -> Result<()> {
        self.lock_state().connected = false;
        tracing::info!("mock transport disconnected");
        Ok(())
    }

    fn capabilities(&self) -> TransportCapabilities {
        self.capabilities.clone()
    }

    async fn subscribe(&self, channel: Channel) -> Result<NotificationStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.lock_state()
            .subscribers
            .entry(channel)
            .or_default()
            .push(tx);
        Ok(rx)
    }

    async fn write(&self, channel: Channel, data: &[u8], _mode: WriteMode) -> Result<()> {
        {
            let mut state = self.lock_state();
            if !state.connected {
                return Err(DfuError::Transport(
                    "mock transport is not connected".to_string(),
                ));
            }
            state.writes.push(WriteRecord {
                channel,
                data: data.to_vec(),
            });
        }

        let responses = (self.handler)(channel, data);
        if responses.is_empty() {
            return Ok(());
        }
        let state = self.lock_state();
        for (target, payload) in responses {
            if let Some(subscribers) = state.subscribers.get(&target) {
                for sender in subscribers {
                    let _ = sender.send(payload.clone());
                }
            }
        }
        Ok(())
    }

    async fn read(&self, _channel: Channel, _buf: &mut [u8], _timeout: Duration) -> Result<usize> {
        Err(DfuError::Unsupported(
            "mock transport only delivers notifications".to_string(),
        ))
    }

    async fn reconnect(&self, hint: ReconnectHint) -> Result<()> {
        let mut state = self.lock_state();
        state.reconnect_count += 1;
        state.connected = true;
        if let Some(address) = &hint.address {
            state.address = Some(address.clone());
        }
        state.last_reconnect_hint = Some(hint);
        tracing::info!("mock transport reconnected");
        Ok(())
    }

    fn address(&self) -> Option<String> {
        self.lock_state().address.clone()
    }
}

/// DFU V2 设备的便捷能力集合。
pub fn v2_capabilities(mtu: u16) -> TransportCapabilities {
    TransportCapabilities {
        services: vec![Service::Dfu],
        indicate_channels: vec![Channel::DfuControl],
        mtu,
    }
}

/// DFU V1 设备的便捷能力集合。
pub fn v1_capabilities(mtu: u16) -> TransportCapabilities {
    TransportCapabilities {
        services: vec![Service::Dfu],
        indicate_channels: Vec::new(),
        mtu,
    }
}

/// 仅暴露快速服务的设备便捷能力集合。
pub fn fast_service_capabilities(mtu: u16) -> TransportCapabilities {
    TransportCapabilities {
        services: vec![Service::FastDfu],
        indicate_channels: Vec::new(),
        mtu,
    }
}
