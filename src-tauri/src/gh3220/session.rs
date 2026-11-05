//! 命令会话：UProtocol 响应没有序列号，只能串行请求。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use gh_uprotocol::Frame;
use tokio::sync::oneshot;

use super::link::Gh3220Link;
use super::types::{Gh3220Error, DEFAULT_COMMAND_TIMEOUT_MS};

struct Pending {
    cmd: u8,
    tx: oneshot::Sender<Frame>,
}

pub struct Session {
    link: Arc<Gh3220Link>,
    pending: Mutex<Option<Pending>>,
    /// 保证同一时刻只有一条命令在等待响应
    queue: tokio::sync::Mutex<()>,
    closed: AtomicBool,
}

impl Session {
    pub fn new(link: Arc<Gh3220Link>) -> Self {
        Self {
            link,
            pending: Mutex::new(None),
            queue: tokio::sync::Mutex::new(()),
            closed: AtomicBool::new(false),
        }
    }

    pub fn link(&self) -> &Arc<Gh3220Link> {
        &self.link
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// 下发命令并等待同 CMD 的响应。
    pub async fn request(&self, cmd: u8, payload: Vec<u8>) -> Result<Frame, Gh3220Error> {
        self.request_with_timeout(
            cmd,
            payload,
            Duration::from_millis(DEFAULT_COMMAND_TIMEOUT_MS),
        )
        .await
    }

    pub async fn request_with_timeout(
        &self,
        cmd: u8,
        payload: Vec<u8>,
        timeout: Duration,
    ) -> Result<Frame, Gh3220Error> {
        let _guard = self.queue.lock().await;

        if self.is_closed() {
            return Err(Gh3220Error::Disconnected);
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
            if pending.is_some() {
                return Err(Gh3220Error::Busy);
            }
            *pending = Some(Pending { cmd, tx });
        }

        let frame = Frame::new(cmd, payload);
        tracing::debug!(
            "[GH3220] request cmd=0x{cmd:02X} len={}",
            frame.payload.len()
        );
        if let Err(e) = self.link.send_frame(&frame).await {
            self.clear_pending(cmd);
            return Err(e);
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(frame)) => Ok(frame),
            Ok(Err(_)) => Err(Gh3220Error::Disconnected),
            Err(_) => {
                self.clear_pending(cmd);
                Err(Gh3220Error::Timeout { cmd })
            }
        }
    }

    /// 只发送，不等待响应（事件 ACK、手工 raw）。
    pub async fn send(&self, frame: &Frame) -> Result<(), Gh3220Error> {
        if self.is_closed() {
            return Err(Gh3220Error::Disconnected);
        }
        self.link.send_frame(frame).await
    }

    /// 把响应交给等待者；返回 true 表示已匹配。
    pub fn dispatch(&self, frame: Frame) -> bool {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        let matches = pending.as_ref().is_some_and(|p| p.cmd == frame.cmd);
        if !matches {
            return false;
        }
        if let Some(p) = pending.take() {
            let _ = p.tx.send(frame);
        }
        true
    }

    /// 关闭会话：等待中的请求会立即收到 Disconnected。
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        *pending = None;
    }

    fn clear_pending(&self, cmd: u8) {
        let mut pending = self.pending.lock().unwrap_or_else(|e| e.into_inner());
        if pending.as_ref().is_some_and(|p| p.cmd == cmd) {
            *pending = None;
        }
    }
}
