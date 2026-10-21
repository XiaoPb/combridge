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

const DEVICE_DISCONNECTED_MESSAGE: &str = "设备已断开，升级中止";

struct OtaInner {
    running: bool,
    session: u64,
    status: OtaStatusDto,
    cancel: Arc<AtomicBool>,
}

/// OTA 管理接口：协议无关的状态机与事件发布。
pub struct OtaManager {
    protocols: ProtocolRegistry,
    event_bus: Arc<EventBus>,
    inner: Mutex<OtaInner>,
    /// 串行化设备通道：预检与升级互斥，避免复用同一 GATT 订阅时互相覆盖。
    op_lock: Arc<tokio::sync::Mutex<()>>,
}

impl OtaManager {
    pub fn new(protocols: ProtocolRegistry, event_bus: Arc<EventBus>) -> Self {
        Self {
            protocols,
            event_bus,
            inner: Mutex::new(OtaInner {
                running: false,
                session: 0,
                status: OtaStatusDto::idle(),
                cancel: Arc::new(AtomicBool::new(false)),
            }),
            op_lock: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    /// 注册到现有 EventBus：设备断开时中止 OTA。
    pub fn subscribe_events(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_topic, event| {
                if let Some(manager) = weak.upgrade() {
                    if let Some(snapshot) = manager.on_device_disconnected(&event.address) {
                        // 回调运行在 EventBus 的同步分发路径上（已持有读锁），
                        // 异步发布以避免发布时再次取锁造成重入。
                        let bus = Arc::clone(&manager.event_bus);
                        tauri::async_runtime::spawn(async move {
                            bus.publish_msgpack(topics::OTA_PROGRESS, &snapshot);
                        });
                    }
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
        let _guard = self.op_lock.clone().try_lock_owned().map_err(|_| {
            ComBridgeError::ota("设备通道正忙（可能正在预检或升级），请稍后再试")
        })?;
        if self.inner.lock().running {
            return Err(ComBridgeError::ota("OTA 升级正在进行中，无法预检设备"));
        }

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
        let updater = Arc::clone(&resolved);

        let verify_path = firmware_path.clone();
        let verified = tokio::task::spawn_blocking(move || {
            let mut package = ZipPackage::open(Path::new(&verify_path))?;
            resolved.validate_package(&mut package)
        })
        .await
        .map_err(|e| ComBridgeError::ota(format!("固件包校验线程异常: {e}")))??;

        // 校验通过后占用设备通道；guard 随升级任务移动到 spawn 块中，升级全程持有。
        let op_guard = self
            .op_lock
            .clone()
            .try_lock_owned()
            .map_err(|_| ComBridgeError::ota("设备通道正忙（可能正在预检），请稍后再试"))?;

        let (cancel, session, snapshot) = {
            let mut inner = self.inner.lock();
            if inner.running {
                return Err(ComBridgeError::ota("OTA 升级正在进行中"));
            }
            inner.running = true;
            inner.session += 1;
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
            (
                Arc::clone(&inner.cancel),
                inner.session,
                inner.status.clone(),
            )
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

        let image = verified.image;
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            let _guard = op_guard;
            manager
                .run_update(updater, address, image, options, cancel, session)
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
        session: u64,
    ) {
        let protocol_id = updater.id().to_string();
        let cancel_cb = Arc::clone(&cancel);
        let manager = Arc::clone(&self);
        let progress: OtaProgressFn = Arc::new(move |stage, bytes, message| {
            manager.report_progress(&protocol_id, stage, bytes, message, session);
            !cancel_cb.load(Ordering::Relaxed)
        });

        let result = updater
            .upgrade(&address, image, &options, Arc::clone(&cancel), progress)
            .await;

        let mut inner = self.inner.lock();
        if inner.session != session {
            return;
        }
        inner.running = false;
        match result {
            Ok(()) => {
                inner.status.state = OtaState::Success;
                inner.status.stage = OtaStage::Completed;
                inner.status.current = inner.status.total;
                inner.status.percent = 100.0;
                inner.status.message = "升级完成，设备正在重启".to_string();
                info!(address = %address, "OTA 升级完成");
            }
            Err(err) => {
                if cancel.load(Ordering::Relaxed) {
                    inner.status.state = OtaState::Cancelled;
                    if inner.status.message != DEVICE_DISCONNECTED_MESSAGE {
                        inner.status.message = format!("升级已取消: {err}");
                    }
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
        session: u64,
    ) {
        let mut inner = self.inner.lock();
        if !inner.running || inner.session != session || inner.cancel.load(Ordering::Relaxed) {
            return;
        }

        // 无字节进度时只更新阶段与文案，保留已有百分比，避免进度条回退。
        let (current, total, percent) = match progress {
            Some((current, total)) => {
                let percent = if total > 0 {
                    (current as f64) * 100.0 / (total as f64)
                } else {
                    0.0
                };
                (current, total, percent)
            }
            None => (
                inner.status.current,
                inner.status.total,
                inner.status.percent,
            ),
        };

        if inner.status.stage == stage
            && (percent - inner.status.percent).abs() < 0.5
            && current == inner.status.current
            && inner.status.message == message
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

    fn on_device_disconnected(&self, address: &str) -> Option<OtaStatusDto> {
        let mut inner = self.inner.lock();
        if !inner.running || inner.status.address.as_deref() != Some(address) {
            return None;
        }
        inner.cancel.store(true, Ordering::Relaxed);
        inner.status.message = DEVICE_DISCONNECTED_MESSAGE.to_string();
        inner.status.updated_at = now_millis();
        Some(inner.status.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ota::protocol::FirmwareImage;
    use async_trait::async_trait;

    type CapturedEvents = Arc<Mutex<Vec<OtaStatusDto>>>;
    type FakeReport = Option<(OtaStage, Option<(u64, u64)>, String)>;

    struct FakeProtocol {
        result: Result<(), String>,
        report: FakeReport,
    }

    impl FakeProtocol {
        fn ok(report: FakeReport) -> Self {
            Self {
                result: Ok(()),
                report,
            }
        }

        fn failed(message: &str) -> Self {
            Self {
                result: Err(message.to_string()),
                report: None,
            }
        }
    }

    #[async_trait]
    impl OtaProtocol for FakeProtocol {
        fn id(&self) -> &'static str {
            "fake"
        }

        fn display_name(&self) -> &'static str {
            "Fake"
        }

        fn description(&self) -> &'static str {
            "OTA 管理接口测试协议"
        }

        fn validate_package(
            &self,
            _package: &mut ZipPackage,
        ) -> Result<FirmwareImage, ComBridgeError> {
            Err(ComBridgeError::ota("测试协议不支持包校验"))
        }

        async fn preflight(&self, _address: &str) -> Result<OtaDeviceInfo, ComBridgeError> {
            Err(ComBridgeError::ota("测试协议不支持预检"))
        }

        async fn upgrade(
            &self,
            _address: &str,
            _image: Vec<u8>,
            _options: &OtaStartOptions,
            _cancel: Arc<AtomicBool>,
            progress: OtaProgressFn,
        ) -> Result<(), ComBridgeError> {
            if let Some((stage, bytes, message)) = &self.report {
                progress(*stage, *bytes, message);
            }
            self.result.clone().map_err(ComBridgeError::ota)
        }
    }

    fn manager() -> (Arc<OtaManager>, CapturedEvents) {
        let bus = Arc::new(EventBus::new(1024));
        let events: CapturedEvents = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        bus.subscribe_msgpack::<OtaStatusDto, _>(topics::OTA_PROGRESS, move |_topic, status| {
            sink.lock().push(status);
        });
        let manager = Arc::new(OtaManager::new(ProtocolRegistry::new(), bus));
        (manager, events)
    }

    fn seed_running(manager: &OtaManager, session: u64) {
        let mut inner = manager.inner.lock();
        inner.running = true;
        inner.session = session;
        inner.status = OtaStatusDto {
            state: OtaState::Running,
            stage: OtaStage::Transferring,
            protocol: Some("fake".to_string()),
            address: Some("AA:BB".to_string()),
            firmware_path: Some("firmware.zip".to_string()),
            current: 50,
            total: 100,
            percent: 50.0,
            message: String::new(),
            error: None,
            updated_at: 0,
        };
    }

    fn start_options() -> OtaStartOptions {
        OtaStartOptions {
            force_confirm: false,
            upgrade_only: false,
        }
    }

    #[test]
    fn cancel_rejects_when_not_running() {
        let (manager, _events) = manager();
        assert!(manager.cancel().is_err());
    }

    #[test]
    fn progress_without_bytes_preserves_percent() {
        let (manager, events) = manager();
        seed_running(&manager, 1);

        manager.report_progress("smp", OtaStage::Activating, None, "激活中", 1);

        let status = manager.status();
        assert_eq!(status.stage, OtaStage::Activating);
        assert_eq!(status.current, 50);
        assert_eq!(status.total, 100);
        assert!((status.percent - 50.0).abs() < f64::EPSILON);
        assert_eq!(status.message, "激活中");

        let captured = events.lock();
        assert_eq!(captured.len(), 1);
        assert!((captured[0].percent - 50.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn run_update_success_sets_completed() {
        let (manager, events) = manager();
        seed_running(&manager, 1);
        let protocol = FakeProtocol::ok(Some((
            OtaStage::Transferring,
            Some((100, 100)),
            "传输完成".to_string(),
        )));

        Arc::clone(&manager)
            .run_update(
                Arc::new(protocol),
                "AA:BB".to_string(),
                Vec::new(),
                start_options(),
                Arc::new(AtomicBool::new(false)),
                1,
            )
            .await;

        let status = manager.status();
        assert_eq!(status.state, OtaState::Success);
        assert_eq!(status.stage, OtaStage::Completed);
        assert!((status.percent - 100.0).abs() < f64::EPSILON);
        assert_eq!(status.current, status.total);
        assert!(!manager.inner.lock().running);

        let captured = events.lock();
        let last = captured.last().expect("终态事件");
        assert_eq!(last.state, OtaState::Success);
        assert!((last.percent - 100.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn run_update_failure_sets_failed() {
        let (manager, _events) = manager();
        seed_running(&manager, 1);

        Arc::clone(&manager)
            .run_update(
                Arc::new(FakeProtocol::failed("传输中断")),
                "AA:BB".to_string(),
                Vec::new(),
                start_options(),
                Arc::new(AtomicBool::new(false)),
                1,
            )
            .await;

        let status = manager.status();
        assert_eq!(status.state, OtaState::Failed);
        assert_eq!(status.stage, OtaStage::Failed);
        assert!(status.error.is_some());
        assert!(!manager.inner.lock().running);
    }

    #[test]
    fn stale_session_progress_is_ignored() {
        let (manager, events) = manager();
        seed_running(&manager, 2);

        manager.report_progress("smp", OtaStage::Activating, Some((80, 100)), "激活中", 1);

        let status = manager.status();
        assert_eq!(status.stage, OtaStage::Transferring);
        assert_eq!(status.current, 50);
        assert!((status.percent - 50.0).abs() < f64::EPSILON);
        assert!(status.message.is_empty());
        assert!(events.lock().is_empty());
    }

    #[tokio::test]
    async fn device_disconnected_publishes_and_keeps_message() {
        let (manager, events) = manager();
        seed_running(&manager, 1);

        let snapshot = manager.on_device_disconnected("AA:BB").expect("断开快照");
        manager
            .event_bus
            .publish_msgpack(topics::OTA_PROGRESS, &snapshot);

        {
            let captured = events.lock();
            assert_eq!(captured.len(), 1);
            assert_eq!(captured[0].message, DEVICE_DISCONNECTED_MESSAGE);
        }

        Arc::clone(&manager)
            .run_update(
                Arc::new(FakeProtocol::failed("传输中断")),
                "AA:BB".to_string(),
                Vec::new(),
                start_options(),
                Arc::new(AtomicBool::new(true)),
                1,
            )
            .await;

        let status = manager.status();
        assert_eq!(status.state, OtaState::Cancelled);
        assert_eq!(status.message, DEVICE_DISCONNECTED_MESSAGE);
    }

    #[tokio::test]
    async fn cancel_while_running_sets_cancelled() {
        let (manager, events) = manager();
        seed_running(&manager, 1);
        let cancel = Arc::clone(&manager.inner.lock().cancel);

        assert!(manager.cancel().is_ok());

        Arc::clone(&manager)
            .run_update(
                Arc::new(FakeProtocol::failed("已取消")),
                "AA:BB".to_string(),
                Vec::new(),
                start_options(),
                cancel,
                1,
            )
            .await;

        let status = manager.status();
        assert_eq!(status.state, OtaState::Cancelled);
        assert!(!manager.inner.lock().running);

        let captured = events.lock();
        let last = captured.last().expect("终态事件");
        assert_eq!(last.state, OtaState::Cancelled);
    }

    #[tokio::test]
    async fn stale_session_terminal_update_is_ignored() {
        let (manager, events) = manager();
        seed_running(&manager, 2);

        Arc::clone(&manager)
            .run_update(
                Arc::new(FakeProtocol::ok(None)),
                "AA:BB".to_string(),
                Vec::new(),
                start_options(),
                Arc::new(AtomicBool::new(false)),
                1,
            )
            .await;

        let status = manager.status();
        assert_eq!(status.state, OtaState::Running);
        assert_eq!(status.stage, OtaStage::Transferring);
        assert!((status.percent - 50.0).abs() < f64::EPSILON);
        assert!(manager.inner.lock().running);
        assert!(events.lock().is_empty());
    }
}
