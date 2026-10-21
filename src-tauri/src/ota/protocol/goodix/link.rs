//! 建立在 [`Transport`] 之上的分帧链路。
//!
//! 基础（`0x4744`）协议会把一帧拆到多个 BLE 通知里，
//! 由 [`NotificationReader`] 按精确字节数重组。
//! 快速（`DOOG`）协议每个通知即一整帧。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::error::{DfuError, Result};
use super::frame::basic::{BasicFrame, HEADER_LEN, TRAILER_LEN};
use super::frame::fast::FastFrame;
use super::transport::{Channel, NotificationStream, Transport, WriteMode};

/// 把通知载荷重组成精确字节流。
pub struct NotificationReader {
    stream: NotificationStream,
    buffer: VecDeque<u8>,
}

impl NotificationReader {
    /// 包装一个通知流。
    pub fn new(stream: NotificationStream) -> Self {
        Self {
            stream,
            buffer: VecDeque::new(),
        }
    }

    /// 已接收但尚未消费的字节数。
    pub fn pending(&self) -> usize {
        self.buffer.len()
    }

    /// 精确读取 `len` 字节，必要时跨多个通知等待。
    pub async fn read_exact(
        &mut self,
        len: usize,
        timeout: Duration,
        context: &str,
    ) -> Result<Vec<u8>> {
        let deadline = Instant::now() + timeout;
        while self.buffer.len() < len {
            let now = Instant::now();
            if now >= deadline {
                return Err(DfuError::timeout(context, timeout));
            }
            let remaining = deadline.saturating_duration_since(now);
            match tokio::time::timeout(remaining, self.stream.recv()).await {
                Ok(Some(chunk)) => self.buffer.extend(chunk),
                Ok(None) => {
                    return Err(DfuError::Transport(
                        "notification stream was closed by the transport".to_string(),
                    ))
                }
                Err(_) => return Err(DfuError::timeout(context, timeout)),
            }
        }

        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            if let Some(byte) = self.buffer.pop_front() {
                out.push(byte);
            }
        }
        Ok(out)
    }
}

/// 基础 `0x4744` 协议的命令链路。
pub struct BasicLink {
    transport: Arc<dyn Transport>,
    reader: NotificationReader,
    timeout: Duration,
    verify_checksum: bool,
}

impl BasicLink {
    /// 订阅 DFU 通知通道并创建链路。
    pub async fn new(
        transport: Arc<dyn Transport>,
        timeout: Duration,
        verify_checksum: bool,
    ) -> Result<Self> {
        let stream = transport.subscribe(Channel::DfuNotify).await?;
        Ok(Self {
            transport,
            reader: NotificationReader::new(stream),
            timeout,
            verify_checksum,
        })
    }

    /// 默认命令超时。
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// 传输重连后重新订阅。
    pub async fn resubscribe(&mut self) -> Result<()> {
        let stream = self.transport.subscribe(Channel::DfuNotify).await?;
        self.reader = NotificationReader::new(stream);
        Ok(())
    }

    /// 底层传输。
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }

    /// 发送一帧，不等待响应。
    pub async fn send(&self, opcode: u16, payload: &[u8]) -> Result<()> {
        let frame = BasicFrame::new(opcode, payload.to_vec());
        tracing::debug!(
            opcode = format_args!("0x{opcode:04X}"),
            len = payload.len(),
            "send basic frame"
        );
        self.transport
            .write(Channel::DfuWrite, &frame.encode(), WriteMode::Auto)
            .await
    }

    /// 在命令特征上写原始字节（快速模式固件数据）。
    pub async fn write_raw(&self, data: &[u8]) -> Result<()> {
        self.transport
            .write(Channel::DfuWrite, data, WriteMode::Auto)
            .await
    }

    /// 写控制点（切换 DFU 模式）。
    pub async fn write_control(&self, data: &[u8]) -> Result<()> {
        tracing::info!(len = data.len(), "write DFU control point");
        self.transport
            .write(Channel::DfuControl, data, WriteMode::Auto)
            .await
    }

    /// 等待指定 opcode 的响应帧。
    pub async fn receive(&mut self, expected_opcode: u16, timeout: Duration) -> Result<Vec<u8>> {
        let header = self
            .reader
            .read_exact(HEADER_LEN, timeout, "basic frame header")
            .await?;
        let (opcode, length) = BasicFrame::parse_header(&header)?;
        if opcode != expected_opcode {
            return Err(DfuError::Protocol(format!(
                "expected opcode 0x{expected_opcode:04X}, received 0x{opcode:04X}"
            )));
        }

        let rest_len = usize::from(length) + TRAILER_LEN;
        let rest = self
            .reader
            .read_exact(rest_len, timeout, "basic frame body")
            .await?;

        if self.verify_checksum {
            let payload = rest[..usize::from(length)].to_vec();
            let frame = BasicFrame::from_parts(opcode, payload, &rest[usize::from(length)..])?;
            Ok(frame.payload)
        } else {
            Ok(rest[..usize::from(length)].to_vec())
        }
    }

    /// 发送命令并等待其响应载荷。
    pub async fn request(&mut self, opcode: u16, payload: &[u8]) -> Result<Vec<u8>> {
        self.send(opcode, payload).await?;
        self.receive(opcode, self.timeout).await
    }
}

/// 专用快速（`DOOG`）协议的命令链路。
pub struct FastLink {
    transport: Arc<dyn Transport>,
    stream: NotificationStream,
    timeout: Duration,
}

impl FastLink {
    /// 订阅快速命令通道。
    pub async fn new(transport: Arc<dyn Transport>, timeout: Duration) -> Result<Self> {
        let stream = transport.subscribe(Channel::FastCommand).await?;
        Ok(Self {
            transport,
            stream,
            timeout,
        })
    }

    /// 底层传输。
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }

    /// 发送快速命令，不等待应答。
    pub async fn send(&self, frame: &FastFrame) -> Result<()> {
        tracing::debug!(
            opcode = format_args!("0x{:02X}", frame.opcode),
            "send fast frame"
        );
        self.transport
            .write(Channel::FastCommand, &frame.encode(), WriteMode::Auto)
            .await
    }

    /// 在快速数据通道写批量数据。
    pub async fn write_data(&self, data: &[u8]) -> Result<()> {
        self.transport
            .write(Channel::FastData, data, WriteMode::Auto)
            .await
    }

    /// 等待下一个应答帧。
    pub async fn next_ack(&mut self, timeout: Duration, context: &str) -> Result<FastFrame> {
        match tokio::time::timeout(timeout, self.stream.recv()).await {
            Ok(Some(chunk)) => FastFrame::decode(&chunk),
            Ok(None) => Err(DfuError::Transport(
                "fast command notification stream was closed".to_string(),
            )),
            Err(_) => Err(DfuError::timeout(context, timeout)),
        }
    }

    /// 发送命令并等待具有相同 opcode 的应答。
    pub async fn request(&mut self, frame: FastFrame) -> Result<FastFrame> {
        let opcode = frame.opcode;
        self.send(&frame).await?;
        let ack = self
            .next_ack(self.timeout, "fast command acknowledgement")
            .await?;
        if ack.opcode != opcode {
            return Err(DfuError::Protocol(format!(
                "expected fast ack 0x{opcode:02X}, received 0x{:02X}",
                ack.opcode
            )));
        }
        Ok(ack)
    }
}
