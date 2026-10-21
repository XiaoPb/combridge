use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use tracing::{error, info};

use crate::error::ComBridgeError;
use crate::service::event_bus::{topics, BleConnectionEvent, EventBus};

use super::package::ZipPackage;
use super::protocol::{OtaProgressFn, OtaProtocol};
use super::registry::ProtocolRegistry;
use super::types::{
    now_millis, OtaDeviceInfo, OtaPackageInfo, OtaProtocolDescriptor, OtaStage, OtaStartOptions,
    OtaState, OtaStatusDto,
};

pub type OtaManagerRef = Arc<OtaManager>;

struct OtaInner {
    running: bool,
    status: OtaStatusDto,
    cancel: Arc<AtomicBool>,
}

/// OTA 管理接口：协议无关的状态机与事件发布。
pub struct OtaManager {
    protocols: ProtocolRegistry,
    event_bus: Arc<EventBus>,
    inner: Mutex<OtaInner>,
}

impl OtaManager {
    pub fn new(protocols: ProtocolRegistry, event_bus: Arc<EventBus>) -> Self {
        Self {
            protocols,
            event_bus,
            inner: Mutex::new(OtaInner {
                running: false,
                status: OtaStatusDto::idle(),
                cancel: Arc::new(AtomicBool::new(false)),
            }),
        }
    }

    /// 注册到现有 EventBus：设备断开时中止 OTA。
    pub fn subscribe_events(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_topic, event| {
                if let Some(manager) = weak.upgrade() {
                    manager.on_device_disconnected(&event.address);
                }
            },
        );
    }

    pub fn list_protocols(&self) -> Vec<OtaProtocolDescriptor> {
        self.protocols.descriptors()
    }

    pub fn status(&self) -> OtaStatusDto {
        self.inner.lock().status.clone()
    }

    pub async fn validate_package(
        &self,
        protocol: Option<&str>,
        firmware_path: &str,
    ) -> Result<OtaPackageInfo, ComBridgeError> {
        let protocol = self.protocols.resolve(protocol)?;
        let path = Path::new(firmware_path).to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut package = ZipPackage::open(&path)?;
            protocol
                .validate_package(&mut package)
                .map(|image| image.info)
        })
        .await
        .map_err(|e| ComBridgeError::ota(format!("固件包校验线程异常: {e}")))?
    }

    pub async fn preflight(
        &self,
        protocol: Option<&str>,
        address: &str,
    ) -> Result<OtaDeviceInfo, ComBridgeError> {
        let protocol = self.protocols.resolve(protocol)?;
        protocol.preflight(address).await
    }

    /// 启动升级：先用所选协议校验 zip，通过后才占用状态并连接设备。
    pub async fn start(
        self: &Arc<Self>,
        protocol: Option<String>,
        address: String,
        firmware_path: String,
        options: OtaStartOptions,
    ) -> Result<OtaStatusDto, ComBridgeError> {
        {
            let inner = self.inner.lock();
            if inner.running {
                return Err(ComBridgeError::ota("OTA 升级正在进行中"));
            }
        }

        let resolved = self.protocols.resolve(protocol.as_deref())?;
        let protocol_id = resolved.id().to_string();

        let verify_path = firmware_path.clone();
        let verified = tokio::task::spawn_blocking(move || {
            let mut package = ZipPackage::open(Path::new(&verify_path))?;
            resolved.validate_package(&mut package)
        })
        .await
        .map_err(|e| ComBridgeError::ota(format!("固件包校验线程异常: {e}")))??;

        let (cancel, snapshot) = {
            let mut inner = self.inner.lock();
            if inner.running {
                return Err(ComBridgeError::ota("OTA 升级正在进行中"));
            }
            inner.running = true;
            inner.cancel = Arc::new(AtomicBool::new(false));
            inner.status = OtaStatusDto {
                state: OtaState::Running,
                stage: OtaStage::Preparing,
                protocol: Some(protocol_id.clone()),
                address: Some(address.clone()),
                firmware_path: Some(verified.info.path.clone()),
                current: 0,
                total: verified.info.image_size,
                percent: 0.0,
                message: format!(
                    "固件包校验通过（{}，{} 字节），正在准备 {} 通道",
                    verified.info.image_name, verified.info.image_size, protocol_id
                ),
                error: None,
                updated_at: now_millis(),
            };
            (Arc::clone(&inner.cancel), inner.status.clone())
        };
        self.event_bus
            .publish_msgpack(topics::OTA_PROGRESS, &snapshot);

        info!(
            address = %address,
            protocol = %protocol_id,
            package = %verified.info.file_name,
            image = %verified.info.image_name,
            sha256 = %verified.info.image_sha256,
            "OTA 固件包校验通过，开始升级"
        );

        let updater = self.protocols.resolve(Some(&protocol_id))?;
        let image = verified.image;
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            manager
                .run_update(updater, address, image, options, cancel)
                .await;
        });

        Ok(snapshot)
    }

    pub fn cancel(&self) -> Result<(), ComBridgeError> {
        let inner = self.inner.lock();
        if !inner.running {
            return Err(ComBridgeError::ota("当前没有正在进行的 OTA 升级"));
        }
        inner.cancel.store(true, Ordering::Relaxed);
        info!("收到 OTA 取消请求");
        Ok(())
    }

    async fn run_update(
        self: Arc<Self>,
        updater: Arc<dyn OtaProtocol>,
        address: String,
        image: Vec<u8>,
        options: OtaStartOptions,
        cancel: Arc<AtomicBool>,
    ) {
        let protocol_id = updater.id().to_string();
        let cancel_cb = Arc::clone(&cancel);
        let manager = Arc::clone(&self);
        let progress: OtaProgressFn = Arc::new(move |stage, bytes, message| {
            manager.report_progress(&protocol_id, stage, bytes, message);
            !cancel_cb.load(Ordering::Relaxed)
        });

        let result = updater
            .upgrade(&address, image, &options, Arc::clone(&cancel), progress)
            .await;

        let mut inner = self.inner.lock();
        inner.running = false;
        match result {
            Ok(()) => {
                inner.status.state = OtaState::Success;
                inner.status.stage = OtaStage::Completed;
                inner.status.percent = 100.0;
                inner.status.message = "升级完成，设备正在重启".to_string();
                info!(address = %address, "OTA 升级完成");
            }
            Err(err) => {
                if cancel.load(Ordering::Relaxed) {
                    inner.status.state = OtaState::Cancelled;
                    inner.status.message = format!("升级已取消: {err}");
                } else {
                    inner.status.state = OtaState::Failed;
                    inner.status.stage = OtaStage::Failed;
                    inner.status.error = Some(err.to_string());
                    inner.status.message = err.to_string();
                    error!(address = %address, error = %err, "OTA 升级失败");
                }
            }
        }
        inner.status.updated_at = now_millis();
        let snapshot = inner.status.clone();
        drop(inner);
        self.event_bus
            .publish_msgpack(topics::OTA_PROGRESS, &snapshot);
    }

    fn report_progress(
        &self,
        protocol: &str,
        stage: OtaStage,
        progress: Option<(u64, u64)>,
        message: &str,
    ) {
        let (current, total) = progress.unwrap_or((0, 0));
        let percent = if total > 0 {
            (current as f64) * 100.0 / (total as f64)
        } else {
            0.0
        };

        let mut inner = self.inner.lock();
        if !inner.running {
            return;
        }
        if inner.status.stage == stage
            && (percent - inner.status.percent).abs() < 0.5
            && current == inner.status.current
        {
            return;
        }
        inner.status.stage = stage;
        inner.status.protocol = Some(protocol.to_string());
        inner.status.current = current;
        inner.status.total = total;
        inner.status.percent = percent;
        inner.status.message = message.to_string();
        inner.status.updated_at = now_millis();
        let snapshot = inner.status.clone();
        drop(inner);
        self.event_bus
            .publish_msgpack(topics::OTA_PROGRESS, &snapshot);
    }

    fn on_device_disconnected(&self, address: &str) {
        let mut inner = self.inner.lock();
        if !inner.running || inner.status.address.as_deref() != Some(address) {
            return;
        }
        inner.cancel.store(true, Ordering::Relaxed);
        inner.status.message = "设备已断开，升级中止".to_string();
    }
}
