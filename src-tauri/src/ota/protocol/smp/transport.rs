use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::Arc;
use std::time::Duration;

use mcumgr_toolkit::transport::{
    ReceiveError, SendError, Transport, SMP_HEADER_SIZE, SMP_TRANSFER_BUFFER_SIZE,
};
use parking_lot::Mutex;
use tokio::runtime::Handle;
use tracing::{debug, error, info, warn};

use crate::device::ble::BleManagerRef;
use crate::error::ComBridgeError;

/// Zephyr/NCS SMP 服务 UUID。
pub const SMP_SERVICE_UUID: &str = "8D53DC1D-1DB7-4CD3-868B-8A527460AA84";
/// Zephyr/NCS SMP 特征 UUID（读写 SMP 帧）。
pub const SMP_CHAR_UUID: &str = "DA2E7828-FBCE-4E01-AE9E-261174997C48";

const DEFAULT_MTU: u16 = 23;
const DEFAULT_WRITE_DELAY_MS: u64 = 2;

/// 从接收缓冲中切出所有完整 SMP 帧，未完成的尾部保留。
pub fn take_frames(pending: &mut Vec<u8>) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    loop {
        if pending.len() < SMP_HEADER_SIZE {
            break;
        }
        let payload_len = u16::from_be_bytes([pending[2], pending[3]]) as usize;
        let total = SMP_HEADER_SIZE + payload_len;
        if total > SMP_TRANSFER_BUFFER_SIZE {
            warn!(total, "SMP 帧长度非法，清空接收缓冲");
            pending.clear();
            break;
        }
        if pending.len() < total {
            break;
        }
        frames.push(pending.drain(..total).collect());
    }
    frames
}

/// 基于 [`crate::device::ble::BleManager`] 的 SMP 传输层。
///
/// 该类型实现同步的 [`Transport`]，内部通过 [`Handle::block_on`] 调用异步 BLE API，
/// 因此 `MCUmgrClient` 的调用需放在阻塞线程（如 `spawn_blocking`）中执行。
pub struct SmpBleTransport {
    ble: BleManagerRef,
    address: String,
    handle: Handle,
    rx: Receiver<Vec<u8>>,
    timeout: Duration,
    chunk_size: usize,
    cancel: Arc<AtomicBool>,
}

impl SmpBleTransport {
    pub async fn open(
        ble: BleManagerRef,
        address: impl Into<String>,
        cancel: Arc<AtomicBool>,
    ) -> Result<Self, ComBridgeError> {
        let address = address.into();
        let handle = Handle::try_current().map_err(|e| {
            ComBridgeError::ota(format!("SMP Transport 需在 Tokio 运行时内创建: {}", e))
        })?;

        let mtu = ble
            .set_mtu(&address, 247)
            .await
            .unwrap_or(DEFAULT_MTU)
            .max(DEFAULT_MTU);

        let (tx, rx): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = std::sync::mpsc::channel();
        let pending = Arc::new(Mutex::new(Vec::<u8>::new()));
        let pending_in_cb = Arc::clone(&pending);

        let callback: crate::device::ble::NotifyCallback =
            Arc::new(move |_addr, _char_uuid, data| {
                let frames = {
                    let mut buf = pending_in_cb.lock();
                    buf.extend_from_slice(data);
                    take_frames(&mut buf)
                };
                for frame in frames {
                    if tx.send(frame).is_err() {
                        debug!("SMP 接收通道已关闭");
                        break;
                    }
                }
            });

        ble.subscribe_notify(&address, SMP_CHAR_UUID, callback)
            .await?;

        let chunk_size = (mtu as usize).saturating_sub(3).max(20);
        info!(
            "SMP BLE Transport 已打开: 设备 {}, MTU {}, 分片 {}",
            address, mtu, chunk_size
        );

        Ok(Self {
            ble,
            address,
            handle,
            rx,
            timeout: Duration::from_secs(10),
            chunk_size,
            cancel,
        })
    }

    pub fn mtu(&self) -> u16 {
        (self.chunk_size + 3) as u16
    }
}

impl Transport for SmpBleTransport {
    fn send_raw_frame(
        &mut self,
        header: [u8; SMP_HEADER_SIZE],
        data: &[u8],
    ) -> Result<(), SendError> {
        let mut frame = Vec::with_capacity(SMP_HEADER_SIZE + data.len());
        frame.extend_from_slice(&header);
        frame.extend_from_slice(data);

        let ble = Arc::clone(&self.ble);
        let address = self.address.clone();
        let handle = self.handle.clone();

        for (index, part) in frame.chunks(self.chunk_size).enumerate() {
            if self.cancel.load(Ordering::Relaxed) {
                warn!("SMP 发送已取消: 设备 {}", self.address);
                return Err(SendError::TransportError(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "OTA 已取消",
                ))));
            }

            handle
                .block_on(ble.write_without_response(&address, SMP_CHAR_UUID, part))
                .map_err(|e| {
                    error!("SMP 发送失败: 设备 {}, 错误 {}", self.address, e);
                    SendError::TransportError(Box::new(e))
                })?;

            if index % 8 == 7 {
                std::thread::sleep(Duration::from_millis(DEFAULT_WRITE_DELAY_MS));
            }
        }
        Ok(())
    }

    fn recv_raw_frame<'a>(
        &mut self,
        buffer: &'a mut [u8; SMP_TRANSFER_BUFFER_SIZE],
    ) -> Result<&'a [u8], ReceiveError> {
        let frame = self.rx.recv_timeout(self.timeout).map_err(|e| match e {
            RecvTimeoutError::Timeout => ReceiveError::Timeout,
            RecvTimeoutError::Disconnected => ReceiveError::TransportError(Box::new(
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "SMP 通知通道已关闭"),
            )),
        })?;

        if frame.len() > buffer.len() {
            return Err(ReceiveError::FrameTooBig);
        }
        buffer[..frame.len()].copy_from_slice(&frame);
        Ok(&buffer[..frame.len()])
    }

    fn set_timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.timeout = timeout;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(payload: &[u8]) -> Vec<u8> {
        let mut f = vec![0u8, 0, 0, 0, 0, 1, 0, 0];
        f[2..4].copy_from_slice(&(payload.len() as u16).to_be_bytes());
        f.extend_from_slice(payload);
        f
    }

    #[test]
    fn assembles_fragmented_frame() {
        let expected = frame(&[10, 20, 30]);
        let mut pending = Vec::new();

        pending.extend_from_slice(&expected[..4]);
        assert!(take_frames(&mut pending).is_empty());

        pending.extend_from_slice(&expected[4..9]);
        assert!(take_frames(&mut pending).is_empty());

        pending.extend_from_slice(&expected[9..]);
        let frames = take_frames(&mut pending);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0], expected);
        assert!(pending.is_empty());
    }

    #[test]
    fn extracts_two_complete_frames() {
        let a = frame(&[1, 2, 3]);
        let b = frame(&[4, 5]);
        let mut pending = [a.clone(), b.clone()].concat();

        let frames = take_frames(&mut pending);
        assert_eq!(frames, vec![a, b]);
        assert!(pending.is_empty());
    }

    #[test]
    fn keeps_incomplete_tail() {
        let a = frame(&[1, 2, 3]);
        let b = frame(&[4, 5]);
        let mut pending = [a.clone(), b[..5].to_vec()].concat();

        let frames = take_frames(&mut pending);
        assert_eq!(frames, vec![a]);
        assert_eq!(pending, b[..5].to_vec());
    }
}
