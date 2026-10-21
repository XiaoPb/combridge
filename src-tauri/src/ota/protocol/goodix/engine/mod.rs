//! 与传输无关的 DFU 引擎。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

pub mod events;
pub mod plan;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use super::driver::fast_service::FastServiceDriver;
use super::driver::normal::NormalDriver;
use super::error::{DfuError, Result};
use super::firmware::{sum_checksum, DfuImage};
use super::transport::{Service, Transport};

pub use events::{DfuReport, ProgressEvent, ProgressSink, Stage};
pub use plan::{
    DfuPlan, Operation, ResourceImage, Route, TargetAddress, TransferMode, TransferOptions,
};

/// 命令往返的重试策略。
///
/// 官方 SDK 不做应用层重传，因此默认零重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// 首次尝试之后的额外尝试次数。
    pub max_retries: u32,
    /// 两次尝试之间的延迟。
    pub retry_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 0,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// 引擎的超时与调优参数。
#[derive(Debug, Clone)]
pub struct EngineConfig {
    /// 基础服务命令响应超时。
    pub command_timeout: Duration,
    /// 快速服务应答超时。
    pub fast_command_timeout: Duration,
    /// 校验收到的基础帧校验和。
    pub verify_frame_checksum: bool,
    /// 命令往返的重试策略。
    pub retry: RetryPolicy,
    /// `SET_DFU_MODE` 无响应，发送后的等待时间。
    pub set_dfu_mode_wait: Duration,
    /// 重连重启后的 Bootloader 之前的延迟。
    pub reconnect_delay: Duration,
    /// 重连后额外等待，让 Bootloader 稳定。
    pub post_reconnect_wait: Duration,
    /// 校验快速服务校验和之前的等待。
    pub fast_settle: Duration,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            command_timeout: Duration::from_secs(10),
            fast_command_timeout: Duration::from_secs(3),
            verify_frame_checksum: true,
            retry: RetryPolicy::default(),
            set_dfu_mode_wait: Duration::from_millis(500),
            reconnect_delay: Duration::from_millis(500),
            post_reconnect_wait: Duration::from_secs(2),
            fast_settle: Duration::from_millis(200),
        }
    }
}

/// 在给定 [`Transport`] 上执行 [`DfuPlan`]。
pub struct DfuEngine {
    transport: Arc<dyn Transport>,
    config: EngineConfig,
}

impl DfuEngine {
    /// 由现有传输创建引擎。
    pub fn new(transport: Arc<dyn Transport>, config: EngineConfig) -> Self {
        Self { transport, config }
    }

    /// 访问传输。
    pub fn transport(&self) -> &Arc<dyn Transport> {
        &self.transport
    }

    /// 引擎配置。
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// 执行会话，通过 `progress` 上报进度。
    pub async fn execute(
        &self,
        plan: DfuPlan,
        progress: ProgressSink,
        cancel: Arc<AtomicBool>,
    ) -> Result<DfuReport> {
        if cancel.load(Ordering::Relaxed) {
            return Err(DfuError::Cancelled);
        }

        progress.stage(Stage::Connect);
        tracing::info!(address = ?self.transport.address(), "connecting to device");
        self.transport.connect().await?;

        let result = self.execute_connected(plan, &progress, &cancel).await;
        if let Err(error) = self.transport.disconnect().await {
            tracing::warn!(%error, "failed to disconnect cleanly");
        }
        result
    }

    /// 不监听进度执行会话。
    pub async fn run(&self, plan: DfuPlan) -> Result<DfuReport> {
        self.execute(plan, ProgressSink::none(), Arc::new(AtomicBool::new(false)))
            .await
    }

    async fn execute_connected(
        &self,
        plan: DfuPlan,
        progress: &ProgressSink,
        cancel: &Arc<AtomicBool>,
    ) -> Result<DfuReport> {
        progress.stage(Stage::Probe);
        let caps = self.transport.capabilities();
        let route = resolve_route(plan.options.route, &caps)?;
        tracing::info!(?route, mtu = caps.mtu, "DFU route selected");

        let report = match route {
            Route::Normal => {
                let is_v2 = caps.indicates(super::transport::Channel::DfuControl);
                let mut driver = NormalDriver::new(
                    Arc::clone(&self.transport),
                    self.config.clone(),
                    progress.clone(),
                    cancel.clone(),
                    is_v2,
                )
                .await?;
                match &plan.operation {
                    Operation::Firmware(image) => {
                        driver.update_firmware(image, &plan.options).await?
                    }
                    Operation::Resource(resource) => {
                        driver
                            .update_resource(
                                &resource.data,
                                resource.address,
                                resource.ext_flash,
                                &plan.options,
                            )
                            .await?
                    }
                }
            }
            Route::FastService => {
                let mut driver = FastServiceDriver::new(
                    Arc::clone(&self.transport),
                    self.config.clone(),
                    progress.clone(),
                    cancel.clone(),
                )
                .await?;
                match &plan.operation {
                    Operation::Firmware(image) => {
                        driver.update_firmware(image, &plan.options).await?
                    }
                    Operation::Resource(resource) => {
                        driver
                            .update_resource(resource.address, &resource.data, resource.ext_flash)
                            .await?
                    }
                }
            }
            Route::Auto => {
                return Err(DfuError::Unsupported(
                    "route resolution returned Auto".to_string(),
                ))
            }
        };

        progress.stage(Stage::Complete);
        progress.emit(ProgressEvent::Completed(report.clone()));
        Ok(report)
    }

    /// 固件传输的便捷入口。
    pub async fn run_firmware(&self, image: DfuImage) -> Result<DfuReport> {
        self.run(DfuPlan::firmware(image, TransferOptions::in_place()))
            .await
    }

    /// 供手工构造资源镜像的调用方使用的校验和工具。
    pub fn checksum(data: &[u8]) -> u32 {
        sum_checksum(data)
    }
}

fn resolve_route(
    requested: Route,
    caps: &super::transport::TransportCapabilities,
) -> Result<Route> {
    let resolved = match requested {
        Route::Auto => {
            if caps.has_service(Service::Dfu) {
                Route::Normal
            } else if caps.has_service(Service::FastDfu) {
                Route::FastService
            } else {
                return Err(DfuError::Unsupported(
                    "device exposes no Goodix DFU service".to_string(),
                ));
            }
        }
        Route::Normal => {
            if !caps.has_service(Service::Dfu) {
                return Err(DfuError::Unsupported(
                    "basic DFU service a6ed0401 is not present".to_string(),
                ));
            }
            Route::Normal
        }
        Route::FastService => {
            if !caps.has_service(Service::FastDfu) {
                return Err(DfuError::Unsupported(
                    "fast DFU service A6ED0701 is not present".to_string(),
                ));
            }
            Route::FastService
        }
    };
    Ok(resolved)
}

/// 基础服务上是否应使用快速模式。
pub(crate) fn use_fast_mode(mode: TransferMode, is_v2: bool) -> bool {
    match mode {
        TransferMode::Fast => is_v2,
        TransferMode::Normal => false,
        TransferMode::Auto => is_v2,
    }
}
