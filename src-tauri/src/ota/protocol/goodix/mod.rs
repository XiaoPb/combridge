//! Goodix GR5xx DFU 协议实现。
//!
//! 作为 `ProtocolRegistry` 中与 `smp` 并列的协议（id: `goodix`）：
//! 复用项目 `BleManager` 与 OTA 管理接口，不新增 OTA 命令。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

pub mod ble_transport;
pub mod driver;
pub mod engine;
pub mod error;
pub mod firmware;
pub mod frame;
pub mod link;
pub mod package;
pub mod transport;
pub mod util;

#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use tracing::{error, info};

use crate::device::ble::BleManagerRef;
use crate::error::ComBridgeError;
use crate::ota::package::ZipPackage;
use crate::ota::protocol::{FirmwareImage, OtaProgressFn, OtaProtocol};
use crate::ota::types::{OtaDeviceInfo, OtaKeyValue, OtaStage, OtaStartOptions};
use crate::ota::ExpectedDisconnects;

use ble_transport::GoodixBleTransport;
use engine::events::{ProgressEvent, ProgressSink, Stage};
use engine::plan::{Route, TargetAddress, TransferMode, TransferOptions};
use engine::{DfuEngine, DfuPlan, EngineConfig};
use error::DfuError;
use firmware::{ensure_status, ChipInfo, DfuImage};
use frame::basic::opcode;
use link::BasicLink;
use transport::{Service, Transport};

/// Goodix DFU 协议。
pub struct GoodixProtocol {
    ble: BleManagerRef,
    expected_disconnects: ExpectedDisconnects,
}

impl GoodixProtocol {
    /// 创建协议实例。
    pub fn new(ble: BleManagerRef, expected_disconnects: ExpectedDisconnects) -> Self {
        Self {
            ble,
            expected_disconnects,
        }
    }

    async fn open_transport(&self, address: &str) -> Result<GoodixBleTransport, ComBridgeError> {
        GoodixBleTransport::open(
            Arc::clone(&self.ble),
            address,
            self.expected_disconnects.clone(),
        )
        .await
    }
}

#[async_trait]
impl OtaProtocol for GoodixProtocol {
    fn id(&self) -> &'static str {
        "goodix"
    }

    fn display_name(&self) -> &'static str {
        "Goodix GR5xx DFU"
    }

    fn description(&self) -> &'static str {
        "Goodix GR5xx 基础 DFU（V1/V2）与快速 DFU 服务"
    }

    fn validate_package(&self, package: &mut ZipPackage) -> Result<FirmwareImage, ComBridgeError> {
        package::validate(package)
    }

    fn validate_image(&self, path: &Path) -> Result<FirmwareImage, ComBridgeError> {
        package::validate_image(path)
    }

    async fn preflight(&self, address: &str) -> Result<OtaDeviceInfo, ComBridgeError> {
        let transport = self.open_transport(address).await?;
        let mode = transport.ble_mode().await.to_string();
        let capabilities = transport.capabilities();
        let mtu = capabilities.mtu;

        let mut fields = Vec::new();
        let mut version = String::from("Fast DFU");

        if capabilities.has_service(Service::Dfu) {
            let mut link = BasicLink::new(
                Arc::new(transport) as Arc<dyn Transport>,
                EngineConfig::default().command_timeout,
                true,
            )
            .await
            .map_err(|e| {
                ComBridgeError::ota(format!("订阅 DFU 通知失败（设备 {address}）: {e}"))
            })?;

            let payload = link.request(opcode::GET_INFO, &[]).await.map_err(|e| {
                ComBridgeError::ota(format!("读取 Goodix 芯片信息失败（设备 {address}）: {e}"))
            })?;
            if payload.is_empty() {
                return Err(ComBridgeError::ota(format!(
                    "设备 {address} 的 GET_INFO 响应为空"
                )));
            }
            ensure_status(payload[0])?;
            let chip = ChipInfo::from_bytes(&payload[1..])?;

            let is_v2 = capabilities.indicates(transport::Channel::DfuControl);
            let dfu_version = if is_v2 {
                2
            } else if chip.dfu_version != 0 {
                chip.dfu_version
            } else {
                1
            };
            version = format!(
                "DFU V{dfu_version} / SDK {}.{}.{}",
                chip.sdk_major, chip.sdk_minor, chip.sdk_build
            );

            let _ = link.transport().disconnect().await;

            fields.push(key_value("dfuVersion", format!("V{dfu_version}")));
            fields.push(key_value(
                "stackVersion",
                format!(
                    "{}.{}.{}",
                    chip.stack_major, chip.stack_minor, chip.stack_build
                ),
            ));
            fields.push(key_value(
                "sdkVersion",
                format!("{}.{}.{}", chip.sdk_major, chip.sdk_minor, chip.sdk_build),
            ));
            fields.push(key_value("stackSvn", format!("0x{:08X}", chip.stack_svn)));
            fields.push(key_value(
                "scaAddress",
                format!("0x{:08X}", chip.sca_address()),
            ));
        }

        let services = capabilities
            .services
            .iter()
            .map(|service| match service {
                Service::Dfu => "dfu",
                Service::FastDfu => "fast",
            })
            .collect::<Vec<_>>()
            .join(",");
        fields.push(key_value("services", services));

        info!(
            address = address,
            mode = mode,
            mtu = mtu,
            version = version,
            "Goodix preflight completed"
        );

        Ok(OtaDeviceInfo {
            address: address.to_string(),
            protocol: self.id().to_string(),
            name: None,
            mode: Some(mode),
            mtu: Some(mtu),
            version: Some(version),
            fields,
        })
    }

    async fn upgrade(
        &self,
        address: &str,
        image: Vec<u8>,
        options: &OtaStartOptions,
        cancel: Arc<AtomicBool>,
        progress: OtaProgressFn,
    ) -> Result<(), ComBridgeError> {
        let transfer_options = build_transfer_options(options)?;
        let parsed = DfuImage::parse(image)
            .map_err(|e| ComBridgeError::ota(format!("解析 Goodix 固件失败: {e}")))?;

        info!(
            address = address,
            route = ?transfer_options.route,
            mode = ?transfer_options.mode,
            target = ?transfer_options.target,
            run_immediately = transfer_options.run_immediately,
            size = parsed.len(),
            checksum = format_args!("0x{:08X}", parsed.checksum),
            signed = parsed.signed,
            encrypted = parsed.encrypted,
            "Goodix upgrade started"
        );

        let transport = self.open_transport(address).await?;
        let engine = DfuEngine::new(
            Arc::new(transport) as Arc<dyn Transport>,
            EngineConfig::default(),
        );

        let (sink, mut events) = ProgressSink::channel();
        let consumer_cancel = Arc::clone(&cancel);
        let consumer = tokio::spawn(async move {
            let mut current = OtaStage::Preparing;
            while let Some(event) = events.recv().await {
                let (stage, bytes, message) = match event {
                    ProgressEvent::Stage(stage) => {
                        current = map_stage(stage);
                        (current, None, stage_text(stage).to_string())
                    }
                    ProgressEvent::Message(message) => (current, None, message),
                    ProgressEvent::Erase { erased, total } => (
                        OtaStage::Preparing,
                        Some((u64::from(erased), u64::from(total))),
                        format!("擦除 Flash {erased}/{total} 扇区"),
                    ),
                    ProgressEvent::Transfer { written, total } => (
                        OtaStage::Transferring,
                        Some((written, total)),
                        format!("传输固件 {written}/{total} 字节"),
                    ),
                    ProgressEvent::Completed(report) => (
                        OtaStage::Completed,
                        None,
                        format!(
                            "升级完成（{} 服务，协议 V{}，{} 字节）",
                            route_label(report.route),
                            report.protocol_version,
                            report.bytes_transferred
                        ),
                    ),
                };
                if !progress(stage, bytes, &message) {
                    consumer_cancel.store(true, Ordering::Relaxed);
                }
            }
        });

        let result = engine
            .execute(
                DfuPlan::firmware(parsed, transfer_options),
                sink.clone(),
                Arc::clone(&cancel),
            )
            .await;
        drop(sink);
        let _ = consumer.await;
        self.expected_disconnects.clear();

        match result {
            Ok(report) => {
                info!(
                    address = address,
                    route = ?report.route,
                    protocol_version = report.protocol_version,
                    bytes = report.bytes_transferred,
                    "Goodix upgrade completed"
                );
                Ok(())
            }
            Err(DfuError::Cancelled) => {
                info!(address = address, "Goodix upgrade cancelled");
                Err(ComBridgeError::ota(format!("升级已取消（设备 {address}）")))
            }
            Err(error) => {
                error!(address = address, error = %error, "Goodix upgrade failed");
                Err(ComBridgeError::ota(format!("{error}（设备 {address}）")))
            }
        }
    }
}

fn key_value(key: &str, value: String) -> OtaKeyValue {
    OtaKeyValue {
        key: key.to_string(),
        value,
    }
}

fn build_transfer_options(options: &OtaStartOptions) -> Result<TransferOptions, ComBridgeError> {
    let route = match options
        .route
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None | Some("auto") => Route::Auto,
        Some("normal") => Route::Normal,
        Some("fastService") | Some("fast_service") | Some("fast") => Route::FastService,
        Some(other) => {
            return Err(ComBridgeError::ota(format!(
                "未知的 Goodix 路由: {other}（可选 auto / normal / fastService）"
            )))
        }
    };

    let mode = match options
        .transfer_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None | Some("auto") => TransferMode::Auto,
        Some("normal") => TransferMode::Normal,
        Some("fast") => TransferMode::Fast,
        Some(other) => {
            return Err(ComBridgeError::ota(format!(
                "未知的 Goodix 传输模式: {other}（可选 auto / normal / fast）"
            )))
        }
    };

    let target = match options
        .target_address
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => TargetAddress::LoadAddress,
        Some(value) => TargetAddress::Explicit(util::parse_hex_u32(value).ok_or_else(|| {
            ComBridgeError::ota(format!("无效的 Goodix 写入地址: {value}（需为十六进制）"))
        })?),
    };

    Ok(TransferOptions {
        route,
        mode,
        target,
        run_immediately: options.run_immediately.unwrap_or(true),
        control_command: None,
    })
}

fn map_stage(stage: Stage) -> OtaStage {
    match stage {
        Stage::Connect
        | Stage::Probe
        | Stage::LoadChipInfo
        | Stage::LoadBootInfo
        | Stage::LoadExtraInfo
        | Stage::CheckOverlap
        | Stage::SetDfuMode
        | Stage::Reconnect
        | Stage::Prepare
        | Stage::Erase => OtaStage::Preparing,
        Stage::Program => OtaStage::Transferring,
        Stage::Verify => OtaStage::Activating,
        Stage::Reboot => OtaStage::Rebooting,
        Stage::Complete => OtaStage::Completed,
    }
}

fn stage_text(stage: Stage) -> &'static str {
    match stage {
        Stage::Connect => "连接设备",
        Stage::Probe => "探测 DFU 能力",
        Stage::LoadChipInfo => "读取芯片信息",
        Stage::LoadBootInfo => "读取 Bootloader 信息",
        Stage::LoadExtraInfo => "读取附加信息",
        Stage::CheckOverlap => "校验 Flash 区域",
        Stage::SetDfuMode => "切换 DFU 模式",
        Stage::Reconnect => "等待设备重启",
        Stage::Prepare => "准备传输",
        Stage::Erase => "擦除 Flash",
        Stage::Program => "传输固件",
        Stage::Verify => "校验固件",
        Stage::Reboot => "重启设备",
        Stage::Complete => "升级完成",
    }
}

fn route_label(route: Route) -> &'static str {
    match route {
        Route::Auto => "自动",
        Route::Normal => "基础",
        Route::FastService => "快速",
    }
}
