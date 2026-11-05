//! 传输层：BLE（正式）与 TCP（调试，默认关闭）。
//!
//! 约束：
//! - BLE 写特征不会自动分片，必须按 MTU 切片并串行写；
//! - `BleManager::connect` 不填充特征缓存，必须先 discover 再 subscribe；
//! - 重复订阅会重复投递通知，订阅前检查 `get_subscriptions`。

use std::sync::Arc;

use gh_uprotocol::Frame;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::{mpsc, Mutex, Notify};
use tracing::info;

use crate::device::ble::{BleManagerRef, BleMode};

use super::types::{
    Gh3220Error, Gh3220LinkConfig, Gh3220LinkKind, DEFAULT_CHUNK_SIZE, MAX_FRAME_BYTES,
    NOTIFY_UUID, SERVICE_UUID, WRITE_UUID,
};

enum LinkInner {
    Ble {
        ble: BleManagerRef,
        address: String,
        write_uuid: String,
    },
    Tcp {
        write: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    },
}

pub struct Gh3220Link {
    kind: Gh3220LinkKind,
    chunk_size: usize,
    write_lock: Mutex<()>,
    inner: LinkInner,
}

pub struct LinkConnection {
    pub link: Gh3220Link,
    /// TCP 链路需要把读循环的数据转交给解码任务；BLE 走 EventBus
    pub rx: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    pub name: Option<String>,
}

impl Gh3220Link {
    pub async fn connect(
        config: &Gh3220LinkConfig,
        ble: BleManagerRef,
        tcp_enabled: bool,
    ) -> Result<LinkConnection, Gh3220Error> {
        if config.address.trim().is_empty() {
            return Err(Gh3220Error::Invalid("设备地址不能为空".to_string()));
        }

        match config.kind {
            Gh3220LinkKind::Ble => Self::connect_ble(config, ble).await,
            Gh3220LinkKind::Tcp => Self::connect_tcp(config, tcp_enabled).await,
        }
    }

    async fn connect_ble(
        config: &Gh3220LinkConfig,
        ble: BleManagerRef,
    ) -> Result<LinkConnection, Gh3220Error> {
        if ble.mode().await != BleMode::Native {
            return Err(Gh3220Error::Ble(
                "GH3220 仅支持 BLE Native 模式，请先在蓝牙页面切换模式".to_string(),
            ));
        }

        let address = config.address.trim().to_string();
        let write_uuid = config
            .write_uuid
            .clone()
            .unwrap_or_else(|| WRITE_UUID.to_string());
        let notify_uuid = config
            .notify_uuid
            .clone()
            .unwrap_or_else(|| NOTIFY_UUID.to_string());

        let existing = ble
            .get_connections()
            .await
            .map_err(|e| Gh3220Error::Ble(e.to_string()))?
            .into_iter()
            .find(|c| c.address.eq_ignore_ascii_case(&address) && c.is_connected);

        let name = match existing {
            Some(conn) => conn.name,
            None => {
                let conn = ble
                    .connect(&address)
                    .await
                    .map_err(|e| Gh3220Error::Ble(e.to_string()))?;
                conn.name
            }
        };

        ble.discover_services(&address)
            .await
            .map_err(|e| Gh3220Error::Ble(format!("发现服务失败: {e}")))?;

        let characteristics = ble
            .discover_characteristics(&address, SERVICE_UUID)
            .await
            .map_err(|e| Gh3220Error::Ble(format!("发现特征失败: {e}")))?;

        let write_char = characteristics
            .iter()
            .find(|c| c.uuid.eq_ignore_ascii_case(&write_uuid))
            .ok_or_else(|| {
                Gh3220Error::Ble(format!(
                    "未找到写特征 {write_uuid}，请确认设备固件与 Service UUID"
                ))
            })?;

        let notify_char = characteristics
            .iter()
            .find(|c| c.uuid.eq_ignore_ascii_case(&notify_uuid))
            .ok_or_else(|| Gh3220Error::Ble(format!("未找到通知特征 {notify_uuid}")))?;

        info!(
            "[GH3220] write char props: write={} wnr={}; notify char props: notify={} indicate={}",
            write_char.properties.write,
            write_char.properties.write_without_response,
            notify_char.properties.notify,
            notify_char.properties.indicate
        );
        if !write_char.properties.write && !write_char.properties.write_without_response {
            return Err(Gh3220Error::Ble(format!("特征 {write_uuid} 不可写")));
        }
        if !notify_char.properties.notify && !notify_char.properties.indicate {
            return Err(Gh3220Error::Ble(format!(
                "特征 {notify_uuid} 不支持 notify/indicate"
            )));
        }

        let subscribed = ble
            .get_subscriptions(&address)
            .await
            .iter()
            .any(|u| u.eq_ignore_ascii_case(&notify_uuid));
        if !subscribed {
            // `BleManager::subscribe_notify` 只负责启动通知任务，返回时 CCCD 可能
            // 尚未生效。若立即下发命令，设备的第一条响应会在订阅生效前到达而丢失。
            // 这里等第一条通知（或超时）再返回，确保命令通道可用。
            let ready = Arc::new(Notify::new());
            let ready_cb = Arc::clone(&ready);
            ble.subscribe_notify(
                &address,
                &notify_uuid,
                Arc::new(move |_, _, _| ready_cb.notify_one()),
            )
            .await
            .map_err(|e| Gh3220Error::Ble(format!("订阅通知失败: {e}")))?;

            let _ = tokio::time::timeout(std::time::Duration::from_millis(1500), ready.notified())
                .await;
        }

        let chunk_size = match config.mtu {
            Some(mtu) => {
                (mtu.saturating_sub(3) as usize).clamp(DEFAULT_CHUNK_SIZE, MAX_FRAME_BYTES)
            }
            None => ble
                .max_write_len(&address, &write_uuid)
                .await
                .map(|n| n.clamp(DEFAULT_CHUNK_SIZE, MAX_FRAME_BYTES))
                .unwrap_or(DEFAULT_CHUNK_SIZE),
        };

        Ok(LinkConnection {
            link: Self {
                kind: Gh3220LinkKind::Ble,
                chunk_size,
                write_lock: Mutex::new(()),
                inner: LinkInner::Ble {
                    ble,
                    address,
                    write_uuid,
                },
            },
            rx: None,
            name,
        })
    }

    async fn connect_tcp(
        config: &Gh3220LinkConfig,
        tcp_enabled: bool,
    ) -> Result<LinkConnection, Gh3220Error> {
        if !tcp_enabled {
            return Err(Gh3220Error::DebugTcpDisabled);
        }
        let address = config.address.trim().to_string();
        if !is_loopback_endpoint(&address) {
            return Err(Gh3220Error::Invalid(
                "调试 TCP 通道只允许回环地址（127.0.0.1 / localhost / ::1）".to_string(),
            ));
        }

        let stream = tokio::net::TcpStream::connect(&address)
            .await
            .map_err(|e| Gh3220Error::Io(format!("连接 {address} 失败: {e}")))?;
        stream
            .set_nodelay(true)
            .map_err(|e| Gh3220Error::Io(format!("设置 TCP_NODELAY 失败: {e}")))?;

        let (mut read, write) = stream.into_split();
        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            loop {
                match read.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(LinkConnection {
            link: Self {
                kind: Gh3220LinkKind::Tcp,
                chunk_size: MAX_FRAME_BYTES,
                write_lock: Mutex::new(()),
                inner: LinkInner::Tcp {
                    write: Arc::new(Mutex::new(write)),
                },
            },
            rx: Some(rx),
            name: None,
        })
    }

    pub fn kind(&self) -> Gh3220LinkKind {
        self.kind
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// 编码并按链路能力发送一帧。整帧持有写锁，避免分片被其他命令穿插。
    pub async fn send_frame(&self, frame: &Frame) -> Result<(), Gh3220Error> {
        let bytes = frame.encode()?;
        let _guard = self.write_lock.lock().await;

        match &self.inner {
            LinkInner::Ble {
                ble,
                address,
                write_uuid,
            } => {
                for chunk in bytes.chunks(self.chunk_size.max(1)) {
                    ble.write_characteristic(address, write_uuid, chunk)
                        .await
                        .map_err(|e| Gh3220Error::Ble(format!("写入失败: {e}")))?;
                }
            }
            LinkInner::Tcp { write } => {
                write
                    .lock()
                    .await
                    .write_all(&bytes)
                    .await
                    .map_err(|e| Gh3220Error::Io(format!("TCP 写入失败: {e}")))?;
            }
        }

        Ok(())
    }

    /// 主动关闭链路（被动断开时不要调用，避免和系统断开流程互相干扰）
    pub async fn close(&self) {
        match &self.inner {
            LinkInner::Ble { ble, address, .. } => {
                let _ = ble.disconnect(address).await;
            }
            LinkInner::Tcp { write } => {
                let mut guard = write.lock().await;
                let _ = guard.shutdown().await;
            }
        }
    }
}

/// 调试 TCP 只允许回环地址
fn is_loopback_endpoint(addr: &str) -> bool {
    let host = if let Some(rest) = addr.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        addr.rsplit_once(':').map(|(h, _)| h).unwrap_or(addr)
    };
    matches!(host, "127.0.0.1" | "localhost" | "::1") || host.starts_with("127.")
}

/// 计算一次写入的分片范围（BLE 单次写上限）
#[cfg(test)]
fn chunk_ranges(len: usize, chunk: usize) -> Vec<(usize, usize)> {
    let chunk = chunk.max(1);
    (0..len)
        .step_by(chunk)
        .map(|start| (start, (start + chunk).min(len)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcp_endpoint_requires_loopback() {
        assert!(is_loopback_endpoint("127.0.0.1:9000"));
        assert!(is_loopback_endpoint("localhost:9000"));
        assert!(is_loopback_endpoint("[::1]:9000"));
        assert!(is_loopback_endpoint("127.1.2.3:9000"));
        assert!(!is_loopback_endpoint("192.168.1.10:9000"));
        assert!(!is_loopback_endpoint("example.com:9000"));
    }

    #[test]
    fn chunk_ranges_cover_frame_once() {
        assert_eq!(chunk_ranges(243, 20).len(), 13);
        assert_eq!(chunk_ranges(243, 20).last().copied(), Some((240, 243)));
        assert_eq!(chunk_ranges(20, 20), vec![(0, 20)]);
        assert_eq!(chunk_ranges(0, 20), Vec::new());
    }
}
