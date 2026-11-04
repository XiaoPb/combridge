//! 串口数据订阅 hub。
//!
//! `serial:data` 事件经 EventBus 以 **msgpack** 编码广播（`serial_manager.rs` 的
//! `publish_msgpack`），MCP hub 通过 broadcast 通道订阅、解码后按端口维护：
//! - 累计字节游标（`total_written`），供 `serial_read` 的 `since` 参数使用
//! - 最近接收缓冲（按端口上限裁剪，超限丢弃最旧数据并置 `truncated`）
//! - `Notify` 唤醒源，供 `serial_expect` 在服务端阻塞等待匹配
//!
//! GUI 与 MCP 在同一进程内共享 EventBus，无论端口由谁打开，
//! 数据都会流经 hub —— 无需替换或注册额外回调。

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::Serialize;
use tokio::sync::Notify;

use crate::service::event_bus::{topics, Event, EventBus, SerialDataEvent, SerialDisconnectedEvent};

/// 单端口接收缓冲上限（字节）
const PORT_BUFFER_LIMIT: usize = 1024 * 1024;
/// 单次 read 返回上限（字节），超出截断
pub const READ_MAX_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone)]
struct Chunk {
    data: Vec<u8>,
}

#[derive(Default)]
struct PortStream {
    chunks: VecDeque<Chunk>,
    /// 已缓冲的字节数
    buffered: usize,
    /// 累计写入的游标（全局单调递增）
    total_written: u64,
    /// 因缓冲上限被丢弃的字节数（用于判断 truncated）
    dropped: u64,
    /// broadcast 消费滞后标记（Lagged 表示中间有数据丢失）
    lagged: bool,
    /// 端口已关闭（收到 SERIAL_DISCONNECTED）；有数据到达时自动复位
    closed: bool,
    notify: Arc<Notify>,
}

impl PortStream {
    fn push(&mut self, data: &[u8]) {
        self.chunks.push_back(Chunk {
            data: data.to_vec(),
        });
        self.buffered += data.len();
        self.total_written += data.len() as u64;
        while self.buffered > PORT_BUFFER_LIMIT {
            if let Some(front) = self.chunks.pop_front() {
                self.buffered -= front.data.len();
                self.dropped += front.data.len() as u64;
            } else {
                break;
            }
        }
    }
}

/// `serial_read` / `serial_expect` 的快照输出。
#[derive(Debug, Clone, Serialize)]
pub struct ReadOut {
    /// 自 `since` 起累计的数据
    pub data: Vec<u8>,
    /// 下一次读取应使用的游标
    pub next_cursor: u64,
    /// 请求区间内已有部分数据被丢弃（缓冲上限）
    pub truncated: bool,
    /// EventBus 消费滞后，区间内可能存在缺口
    pub lagged: bool,
    /// 当前游标总量（= 自打开以来累计接收字节数）
    pub total_written: u64,
}

pub struct SerialHub {
    ports: RwLock<HashMap<String, PortStream>>,
}

impl SerialHub {
    pub fn new() -> Self {
        Self {
            ports: RwLock::new(HashMap::new()),
        }
    }

    fn with_stream<R>(&self, port: &str, f: impl FnOnce(&mut PortStream) -> R) -> R {
        let mut guard = self
            .ports
            .write()
            .unwrap_or_else(|e| e.into_inner());
        f(guard.entry(port.to_string()).or_default())
    }

    /// 写入一条接收数据（由订阅任务调用，也可测试直用）。
    pub fn feed(&self, port: &str, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        self.with_stream(port, |s| {
            s.closed = false;
            s.push(data);
            // 多等待者：逐个注册者唤醒；无等待者时 notify_waiters 是 no-op
            s.notify.notify_waiters();
        });
    }

    /// 标记消费滞后（broadcast Lagged）。
    pub fn mark_lagged(&self, port: &str) {
        self.with_stream(port, |s| s.lagged = true);
    }

    /// 快照：返回 `since` 游标之后的数据。端口不存在（未打开或已移除）时返回空。
    pub fn snapshot_since(&self, port: &str, since: u64) -> ReadOut {
        let guard = rw_read(&self.ports);
        let Some(stream) = guard.get(port) else {
            return ReadOut {
                data: Vec::new(),
                next_cursor: since,
                truncated: false,
                lagged: false,
                total_written: 0,
            };
        };

        let total = stream.total_written;
        let clamped_since = since.min(total);
        // 若 since 早于缓冲区内最旧数据起点，则只有部分可读
        let oldest_cursor = stream.dropped;
        let truncated = clamped_since < oldest_cursor;

        let mut data = Vec::new();
        let mut consumed = oldest_cursor;
        for chunk in &stream.chunks {
            let chunk_start = consumed;
            consumed += chunk.data.len() as u64;
            if consumed <= clamped_since {
                continue;
            }
            let skip = clamped_since.saturating_sub(chunk_start) as usize;
            data.extend_from_slice(&chunk.data[skip.min(chunk.data.len())..]);
            if data.len() >= READ_MAX_BYTES {
                data.truncate(READ_MAX_BYTES);
                break;
            }
        }

        ReadOut {
            next_cursor: total,
            data,
            truncated,
            lagged: stream.lagged,
            total_written: total,
        }
    }

    /// 当前游标（端口从未收过数据则为 0）。
    pub fn current_cursor(&self, port: &str) -> u64 {
        let guard = rw_read(&self.ports);
        guard.get(port).map(|s| s.total_written).unwrap_or(0)
    }

    /// 丢弃缓冲并把游标推进到当前值（"从此刻开始读" 语义）。
    pub fn reset_to_now(&self, port: &str) -> u64 {
        self.with_stream(port, |s| {
            s.chunks.clear();
            s.buffered = 0;
            s.dropped = s.total_written;
            s.lagged = false;
            s.total_written
        })
    }

    /// 端口关闭时标记并清理数据（保留游标基线，重开后继续单调递增）。
    /// 同时唤醒所有等待者以便其感知连接断开。
    pub fn remove_port(&self, port: &str) {
        let mut guard = self
            .ports
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let stream = guard.entry(port.to_string()).or_default();
        stream.closed = true;
        stream.chunks.clear();
        stream.buffered = 0;
        stream.dropped = stream.total_written;
        stream.lagged = false;
        stream.notify.notify_waiters();
    }

    /// 是否已收到该端口的断开通知。
    pub fn is_closed(&self, port: &str) -> bool {
        rw_read(&self.ports)
            .get(port)
            .map(|s| s.closed)
            .unwrap_or(false)
    }

    /// 端口打开时复位断开标记（避免重开后的首个 expect 被误判为已断开）。
    pub fn mark_open(&self, port: &str) {
        self.with_stream(port, |s| s.closed = false);
    }

    /// 获取端口的唤醒源。端口不存在时不创建（返回独立 Notify，
    /// 永不会被唤醒，等待方仅靠超时退出并感知 `disconnected`）。
    fn notify_for(&self, port: &str) -> Arc<Notify> {
        if let Some(n) = rw_read(&self.ports).get(port).map(|s| s.notify.clone()) {
            return n;
        }
        Arc::new(Notify::new())
    }

    /// 阻塞等待直到出现匹配。`matcher` 对累计文本执行，返回 Some(捕获) 即命中。
    ///
    /// 等待期间每次新数据到达都会重新求值；连接断开（`remove_port` 唤醒）时
    /// 以未命中结束并带 `disconnected: true`（见 `ExpectOut`）。
    pub async fn wait_for_match(
        &self,
        port: &str,
        since: u64,
        timeout: Duration,
        matcher: impl Fn(&str) -> Option<String> + Send,
    ) -> ExpectOut {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            // 先注册唤醒再快照，避免 "快照后、等待前" 到达的数据丢失唤醒
            let notify = self.notify_for(port);
            let notified = notify.notified();
            let out = self.snapshot_since(port, since);
            let disconnected = self.is_closed(port);
            let text = String::from_utf8_lossy(&out.data).to_string();
            if let Some(matched) = matcher(&text) {
                return ExpectOut {
                    matched: true,
                    matched_text: matched,
                    buffer: text,
                    next_cursor: out.next_cursor,
                    elapsed_ms: 0,
                    truncated: out.truncated,
                    lagged: out.lagged,
                    disconnected: false,
                };
            }
            if disconnected {
                return ExpectOut {
                    matched: false,
                    matched_text: String::new(),
                    buffer: text,
                    next_cursor: out.next_cursor,
                    elapsed_ms: 0,
                    truncated: out.truncated,
                    lagged: out.lagged,
                    disconnected: true,
                };
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return ExpectOut {
                    matched: false,
                    matched_text: String::new(),
                    buffer: text,
                    next_cursor: out.next_cursor,
                    elapsed_ms: timeout.as_millis() as u64,
                    truncated: out.truncated,
                    lagged: out.lagged,
                    disconnected: false,
                };
            }
            if tokio::time::timeout(remaining, notified).await.is_err() {
                // 超时：最后一轮重新快照，可能数据刚好在超时瞬间到达
                let out = self.snapshot_since(port, since);
                let text = String::from_utf8_lossy(&out.data).to_string();
                if let Some(matched) = matcher(&text) {
                    return ExpectOut {
                        matched: true,
                        matched_text: matched,
                        buffer: text,
                        next_cursor: out.next_cursor,
                        elapsed_ms: timeout.as_millis() as u64,
                        truncated: out.truncated,
                        lagged: out.lagged,
                        disconnected: false,
                    };
                }
                return ExpectOut {
                    matched: false,
                    matched_text: String::new(),
                    buffer: text,
                    next_cursor: out.next_cursor,
                    elapsed_ms: timeout.as_millis() as u64,
                    truncated: out.truncated,
                    lagged: out.lagged,
                    disconnected: false,
                };
            }
        }
    }

    /// 订阅 EventBus 广播并持续喂给 hub。应在 MCP 启动时调用一次。
    ///
    /// 返回任务句柄：服务重启时必须 abort，否则每重启一次就多一个消费者。
    pub fn spawn_subscription(self: &Arc<Self>, bus: Arc<EventBus>) -> tokio::task::JoinHandle<()> {
        let hub = Arc::clone(self);
        let mut rx = bus.subscribe_channel();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(event) => hub.handle_event(&event),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("[MCP-SerialHub] 广播消费滞后，丢失 {} 条事件", n);
                        // 具体哪个端口无法得知，全部标记
                        let ports: Vec<String> = {
                            hub.ports
                                .read()
                                .unwrap_or_else(|e| e.into_inner())
                                .keys()
                                .cloned()
                                .collect()
                        };
                        for p in ports {
                            hub.mark_lagged(&p);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::warn!("[MCP-SerialHub] EventBus 已关闭，订阅退出");
                        break;
                    }
                }
            }
        })
    }

    fn handle_event(&self, event: &Event) {
        match event.topic.as_str() {
            topics::SERIAL_DATA => {
                match rmp_serde::from_slice::<SerialDataEvent>(&event.payload) {
                    Ok(data_event) => self.feed(&data_event.device_id, &data_event.data),
                    Err(e) => tracing::error!("[MCP-SerialHub] msgpack 解码失败: {}", e),
                }
            }
            topics::SERIAL_DISCONNECTED => {
                if let Ok(disconnected) =
                    serde_json::from_slice::<SerialDisconnectedEvent>(&event.payload)
                {
                    tracing::info!(
                        "[MCP-SerialHub] 串口 {} 断开，清理 hub 状态",
                        disconnected.port_name
                    );
                    self.remove_port(&disconnected.port_name);
                }
            }
            _ => {}
        }
    }
}

impl Default for SerialHub {
    fn default() -> Self {
        Self::new()
    }
}

fn rw_read<T>(lock: &RwLock<T>) -> std::sync::RwLockReadGuard<'_, T> {
    lock.read().unwrap_or_else(|e| e.into_inner())
}

/// `serial_expect` 的返回。
#[derive(Debug, Clone, Serialize)]
pub struct ExpectOut {
    pub matched: bool,
    /// 命中的片段
    pub matched_text: String,
    /// 等待期间累计的全部文本（便于模型看到上下文 / 错误信息）
    pub buffer: String,
    pub next_cursor: u64,
    pub elapsed_ms: u64,
    pub truncated: bool,
    pub lagged: bool,
    /// 等待期间端口断开
    pub disconnected: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feed_and_snapshot() {
        let hub = SerialHub::new();
        hub.feed("COM3", b"AT\r\n");
        hub.feed("COM3", b"OK\r\n");
        let out = hub.snapshot_since("COM3", 0);
        assert_eq!(out.data, b"AT\r\nOK\r\n");
        assert_eq!(out.next_cursor, 8);
        assert!(!out.truncated);
    }

    #[test]
    fn test_cursor_semantics() {
        let hub = SerialHub::new();
        hub.feed("COM3", b"abc"); // cursor 3
        let out1 = hub.snapshot_since("COM3", 0);
        assert_eq!(out1.data, b"abc");
        hub.feed("COM3", b"def"); // cursor 6
        let out2 = hub.snapshot_since("COM3", out1.next_cursor);
        assert_eq!(out2.data, b"def");
        // 悬空游标被钳制
        let out3 = hub.snapshot_since("COM3", 999);
        assert_eq!(out3.data, b"");
        assert_eq!(out3.next_cursor, 6);
    }

    #[test]
    fn test_reset_to_now() {
        let hub = SerialHub::new();
        hub.feed("COM3", b"old-data");
        hub.reset_to_now("COM3");
        let out = hub.snapshot_since("COM3", 0);
        assert_eq!(out.data, b"");
        assert!(out.truncated); // 旧数据已被语义上丢弃
        hub.feed("COM3", b"new");
        let out2 = hub.snapshot_since("COM3", 8);
        assert_eq!(out2.data, b"new");
    }

    #[test]
    fn test_buffer_limit_truncation() {
        let hub = SerialHub::new();
        // 超过 PORT_BUFFER_LIMIT 的一半，写两次触发驱逐
        let big = vec![b'a'; PORT_BUFFER_LIMIT];
        hub.feed("COM3", &big);
        hub.feed("COM3", b"tail");
        let out = hub.snapshot_since("COM3", 0);
        assert!(out.truncated);
        assert_eq!(out.data, b"tail");
    }

    #[test]
    fn test_remove_port_wakes() {
        let hub = Arc::new(SerialHub::new());
        hub.feed("COM3", b"x");
        hub.remove_port("COM3");
        assert!(hub.is_closed("COM3"));
        // 数据已清空但游标基线保留
        let out = hub.snapshot_since("COM3", 0);
        assert_eq!(out.data, b"");
        assert_eq!(out.total_written, 1);
        assert!(out.truncated);
        // 重新有数据到达则复位 closed
        hub.feed("COM3", b"y");
        assert!(!hub.is_closed("COM3"));
        let out2 = hub.snapshot_since("COM3", 1);
        assert_eq!(out2.data, b"y");
    }

    #[tokio::test]
    async fn test_wait_for_match_hit() {
        let hub = Arc::new(SerialHub::new());
        let hub2 = Arc::clone(&hub);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            hub2.feed("COM3", b"ERROR: timeout\r\n");
        });
        let out = hub
            .wait_for_match(
                "COM3",
                0,
                Duration::from_secs(2),
                |text| {
                    if text.contains("ERROR") {
                        Some("ERROR".to_string())
                    } else {
                        None
                    }
                },
            )
            .await;
        assert!(out.matched);
        assert_eq!(out.matched_text, "ERROR");
        assert!(out.buffer.contains("timeout"));
    }

    #[tokio::test]
    async fn test_wait_for_match_timeout() {
        let hub = Arc::new(SerialHub::new());
        let out = hub
            .wait_for_match("COM3", 0, Duration::from_millis(100), |_text| None)
            .await;
        assert!(!out.matched);
        assert!(!out.disconnected);
        assert_eq!(out.buffer, "");
    }

    #[tokio::test]
    async fn test_wait_for_match_immediate() {
        let hub = Arc::new(SerialHub::new());
        hub.feed("COM3", b"OK");
        let out = hub
            .wait_for_match("COM3", 0, Duration::from_millis(100), |text| {
                text.contains("OK").then(|| "OK".to_string())
            })
            .await;
        assert!(out.matched);
    }
}
