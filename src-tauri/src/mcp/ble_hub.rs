//! BLE 通知数据 hub。
//!
//! `ble:data` 事件以 msgpack 广播（`ble_manager.rs` 的 `subscribe_notify` 包装层
//! 在回调中 `publish_msgpack`），因此无论订阅由 GUI 还是 MCP 发起，
//! 通知数据都会流经 EventBus。本 hub 按 `(address, characteristic)` 分桶缓存，
//! 提供游标式读取，供 `combridge_ble_gatt(action="notifications")` 使用。
//!
//! 与串口 hub 的区别：GATT 通知通常是二进制小包，不做阻塞匹配（无法定界），
//! 只做「游标 + 缓冲」，由模型自行决定等待/重读节奏。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};

use serde::Serialize;

use crate::service::event_bus::{topics, BleDataEvent, Event, EventBus};

/// 单桶缓冲上限
const BUCKET_LIMIT: usize = 256 * 1024;
/// 单次读取上限
pub const READ_MAX_BYTES: usize = 64 * 1024;

/// 桶键：地址 + 特征 UUID（地址为 MAC / 设备 id，UUID 不含 '|'）
pub fn bucket_key(address: &str, char_uuid: &str) -> String {
    format!("{address}|{char_uuid}")
}

#[derive(Default)]
struct Bucket {
    chunks: VecDeque<Vec<u8>>,
    buffered: usize,
    total_written: u64,
    dropped: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NotifyOut {
    pub data: Vec<u8>,
    pub next_cursor: u64,
    pub truncated: bool,
    pub total_written: u64,
}

pub struct BleHub {
    buckets: RwLock<HashMap<String, Bucket>>,
}

impl BleHub {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
        }
    }

    /// 写入一条通知数据。
    pub fn feed(&self, address: &str, char_uuid: &str, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let key = bucket_key(address, char_uuid);
        let mut guard = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = guard.entry(key).or_default();
        bucket.chunks.push_back(data.to_vec());
        bucket.buffered += data.len();
        bucket.total_written += data.len() as u64;
        while bucket.buffered > BUCKET_LIMIT {
            if let Some(front) = bucket.chunks.pop_front() {
                bucket.buffered -= front.len();
                bucket.dropped += front.len() as u64;
            } else {
                break;
            }
        }
    }

    /// 游标读取。
    pub fn snapshot_since(&self, address: &str, char_uuid: &str, since: u64) -> NotifyOut {
        let key = bucket_key(address, char_uuid);
        let guard = self.buckets.read().unwrap_or_else(|e| e.into_inner());
        let Some(bucket) = guard.get(&key) else {
            return NotifyOut {
                data: Vec::new(),
                next_cursor: since,
                truncated: false,
                total_written: 0,
            };
        };

        let total = bucket.total_written;
        let clamped = since.min(total);
        let truncated = clamped < bucket.dropped;

        let mut data = Vec::new();
        let mut consumed = bucket.dropped;
        for chunk in &bucket.chunks {
            let start = consumed;
            consumed += chunk.len() as u64;
            if consumed <= clamped {
                continue;
            }
            let skip = clamped.saturating_sub(start) as usize;
            data.extend_from_slice(&chunk[skip.min(chunk.len())..]);
            if data.len() >= READ_MAX_BYTES {
                data.truncate(READ_MAX_BYTES);
                break;
            }
        }

        NotifyOut {
            data,
            next_cursor: total,
            truncated,
            total_written: total,
        }
    }

    /// 当前游标（用于"只看新通知"的起点）。
    pub fn current_cursor(&self, address: &str, char_uuid: &str) -> u64 {
        let key = bucket_key(address, char_uuid);
        let guard = self.buckets.read().unwrap_or_else(|e| e.into_inner());
        guard.get(&key).map(|b| b.total_written).unwrap_or(0)
    }

    /// 清空某桶并推进基线（重新订阅时调用，避免旧通知污染）。
    pub fn reset_to_now(&self, address: &str, char_uuid: &str) -> u64 {
        let key = bucket_key(address, char_uuid);
        let mut guard = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        let bucket = guard.entry(key).or_default();
        bucket.chunks.clear();
        bucket.buffered = 0;
        bucket.dropped = bucket.total_written;
        bucket.total_written
    }

    /// 断开设备时清理该地址下所有桶。
    pub fn remove_address(&self, address: &str) {
        let prefix = format!("{address}|");
        let mut guard = self.buckets.write().unwrap_or_else(|e| e.into_inner());
        guard.retain(|k, _| !k.starts_with(&prefix));
    }

    /// 订阅 EventBus 的 `ble:data`。返回任务句柄，服务重启时需 abort。
    pub fn spawn_subscription(self: &Arc<Self>, bus: Arc<EventBus>) -> tokio::task::JoinHandle<()> {
        let hub = Arc::clone(self);
        let mut rx = bus.subscribe_channel();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if event.topic == topics::BLE_DATA {
                            hub.handle_event(&event);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[MCP-BleHub] 广播消费滞后，丢失 {} 条事件", n);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        })
    }

    fn handle_event(&self, event: &Event) {
        match rmp_serde::from_slice::<BleDataEvent>(&event.payload) {
            Ok(e) => self.feed(&e.address, &e.characteristic_uuid, &e.data),
            Err(err) => tracing::error!("[MCP-BleHub] msgpack 解码失败: {}", err),
        }
    }
}

impl Default for BleHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_read() {
        let hub = BleHub::new();
        hub.feed("AA:BB", "char-1", &[0x01, 0x02]);
        hub.feed("AA:BB", "char-1", &[0x03]);
        let out = hub.snapshot_since("AA:BB", "char-1", 0);
        assert_eq!(out.data, vec![0x01, 0x02, 0x03]);
        assert_eq!(out.next_cursor, 3);
        assert!(!out.truncated);

        let out2 = hub.snapshot_since("AA:BB", "char-1", 2);
        assert_eq!(out2.data, vec![0x03]);
    }

    #[test]
    fn test_bucket_isolation() {
        let hub = BleHub::new();
        hub.feed("AA:BB", "char-1", &[1]);
        hub.feed("AA:BB", "char-2", &[2]);
        hub.feed("CC:DD", "char-1", &[3]);
        assert_eq!(hub.snapshot_since("AA:BB", "char-1", 0).data, vec![1]);
        assert_eq!(hub.snapshot_since("AA:BB", "char-2", 0).data, vec![2]);
        assert_eq!(hub.snapshot_since("CC:DD", "char-1", 0).data, vec![3]);
    }

    #[test]
    fn test_reset_and_remove() {
        let hub = BleHub::new();
        hub.feed("AA:BB", "char-1", &[1, 2]);
        hub.reset_to_now("AA:BB", "char-1");
        let out = hub.snapshot_since("AA:BB", "char-1", 0);
        assert!(out.data.is_empty());
        assert!(out.truncated);

        hub.feed("AA:BB", "char-1", &[9]);
        hub.remove_address("AA:BB");
        assert_eq!(hub.snapshot_since("AA:BB", "char-1", 0).total_written, 0);
    }

    #[test]
    fn test_truncation() {
        let hub = BleHub::new();
        let big = vec![7u8; BUCKET_LIMIT];
        hub.feed("AA:BB", "c", &big);
        hub.feed("AA:BB", "c", &[8]);
        let out = hub.snapshot_since("AA:BB", "c", 0);
        assert!(out.truncated);
        assert_eq!(out.data, vec![8]);
    }
}
