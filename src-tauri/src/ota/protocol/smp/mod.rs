pub mod package;
pub mod transport;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use mcumgr_toolkit::client::{FirmwareUpdateParams, FirmwareUpdateStep};
use mcumgr_toolkit::MCUmgrClient;
use tracing::{info, warn};

use crate::device::ble::{BleManagerRef, BleMode};
use crate::error::ComBridgeError;
use crate::ota::package::ZipPackage;
use crate::ota::protocol::{FirmwareImage, OtaProgressFn, OtaProtocol};
use crate::ota::types::{OtaDeviceInfo, OtaStage, OtaStartOptions};

use self::transport::{SmpBleTransport, SMP_CHAR_UUID, SMP_SERVICE_UUID};

const SMP_TIMEOUT: Duration = Duration::from_secs(10);
/// 未协商出写入长度时的最小 ATT MTU。
const DEFAULT_MTU: u16 = 23;
const PROTOCOL_ID: &str = "smp";

/// Nordic NCS（Zephyr MCUmgr）OTA 协议实现。
pub struct SmpProtocol {
    ble: BleManagerRef,
}

impl SmpProtocol {
    pub fn new(ble: BleManagerRef) -> Self {
        Self { ble }
    }
}

#[async_trait]
impl OtaProtocol for SmpProtocol {
    fn id(&self) -> &'static str {
        PROTOCOL_ID
    }

    fn display_name(&self) -> &'static str {
        "Nordic SMP"
    }

    fn description(&self) -> &'static str {
        "Zephyr MCUmgr over BLE + MCUboot"
    }

    fn validate_package(&self, package: &mut ZipPackage) -> Result<FirmwareImage, ComBridgeError> {
        package::validate(package)
    }

    async fn preflight(&self, address: &str) -> Result<OtaDeviceInfo, ComBridgeError> {
        let connections = self.ble.get_connections().await?;
        let connection = connections
            .iter()
            .find(|c| c.address.eq_ignore_ascii_case(address) && c.is_connected)
            .ok_or_else(|| ComBridgeError::ota(format!("设备未连接: {address}")))?
            .clone();

        // 原生后端只在 discover_services 后才缓存特征，
        // 后续 discover_characteristics / max_write_len / subscribe_notify 都依赖该缓存。
        self.ble.discover_services(address).await?;

        let characteristics = self
            .ble
            .discover_characteristics(address, SMP_SERVICE_UUID)
            .await?;
        if !characteristics
            .iter()
            .any(|c| c.uuid.eq_ignore_ascii_case(SMP_CHAR_UUID))
        {
            return Err(ComBridgeError::ota("设备未提供 Nordic SMP 服务"));
        }

        let mode = match self.ble.mode().await {
            BleMode::Native => "native",
            BleMode::At => "at",
        };
        // 用真实 GATT 单包写入长度推导 MTU，避免报告一个设备并不支持的协商值。
        let write_len = self
            .ble
            .max_write_len(address, SMP_CHAR_UUID)
            .await
            .unwrap_or_else(|e| {
                warn!("获取 SMP 特征写入长度失败，回退 20 字节: {e}");
                (DEFAULT_MTU - 3) as usize
            });
        let mtu = ((write_len + 3).min(u16::MAX as usize) as u16).max(DEFAULT_MTU);

        let cancel = Arc::new(AtomicBool::new(false));
        let transport = SmpBleTransport::open(Arc::clone(&self.ble), address, cancel).await?;

        let app_info = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let client = MCUmgrClient::new_from_transport(transport);
            client
                .set_timeout(SMP_TIMEOUT)
                .map_err(|e| e.to_string())?;
            client
                .use_auto_frame_size()
                .map_err(|e| e.to_string())?;
            client.os_echo("ping").map_err(|e| e.to_string())?;
            client
                .os_application_info(None)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| ComBridgeError::ota(format!("SMP 线程异常: {e}")))?
        .map_err(ComBridgeError::ota)?;

        self.ble
            .unsubscribe_notify(address, SMP_CHAR_UUID)
            .await
            .ok();

        info!(address = address, mode, mtu, "OTA 预检成功");

        Ok(OtaDeviceInfo {
            address: address.to_string(),
            protocol: PROTOCOL_ID.to_string(),
            name: connection.name,
            mode: Some(mode.to_string()),
            mtu: Some(mtu),
            version: Some(app_info),
            fields: Vec::new(),
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
        // 保证特征缓存存在，SmpBleTransport::open 的 subscribe_notify 依赖它。
        self.ble.discover_services(address).await?;

        let transport =
            SmpBleTransport::open(Arc::clone(&self.ble), address, Arc::clone(&cancel)).await?;

        let cancel_cb = Arc::clone(&cancel);
        let mut progress_cb = move |step: FirmwareUpdateStep, p: Option<(u64, u64)>| -> bool {
            if cancel_cb.load(Ordering::Relaxed) {
                return false;
            }
            let (stage, message) = map_step(step);
            progress(stage, p, &message)
        };

        // 闭包需 'static，先把 Copy 的选项拷出来。
        let force_confirm = options.force_confirm;
        let upgrade_only = options.upgrade_only;

        let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
            let client = MCUmgrClient::new_from_transport(transport);
            client
                .set_timeout(SMP_TIMEOUT)
                .map_err(|e| e.to_string())?;
            client
                .use_auto_frame_size()
                .map_err(|e| e.to_string())?;

            client
                .firmware_update(
                    &image,
                    None,
                    FirmwareUpdateParams {
                        bootloader_type: None,
                        skip_reboot: false,
                        force_confirm,
                        upgrade_only,
                    },
                    Some(&mut progress_cb),
                )
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| ComBridgeError::ota(format!("SMP 线程异常: {e}")))?;

        // transport 的 Drop 也会退订；这里再显式退订一次，取消订阅是幂等操作。
        self.ble
            .unsubscribe_notify(address, SMP_CHAR_UUID)
            .await
            .ok();

        result.map_err(|message| {
            if cancel.load(Ordering::Relaxed) {
                ComBridgeError::ota("升级已取消")
            } else {
                ComBridgeError::ota(message)
            }
        })
    }
}

/// 把 `mcumgr-toolkit` 的 8 个步骤映射为通用阶段；不要加 `_` 分支。
pub fn map_step(step: FirmwareUpdateStep) -> (OtaStage, String) {
    match step {
        FirmwareUpdateStep::DetectingBootloader => {
            (OtaStage::Preparing, "正在探测 Bootloader".to_string())
        }
        FirmwareUpdateStep::BootloaderFound(bootloader) => (
            OtaStage::Preparing,
            format!("已找到 Bootloader: {bootloader}"),
        ),
        FirmwareUpdateStep::ParsingFirmwareImage => {
            (OtaStage::Preparing, "正在解析固件".to_string())
        }
        FirmwareUpdateStep::QueryingDeviceState => {
            (OtaStage::Preparing, "正在查询设备镜像状态".to_string())
        }
        FirmwareUpdateStep::UpdateInfo { .. } => (
            OtaStage::Preparing,
            "已获取当前版本信息".to_string(),
        ),
        FirmwareUpdateStep::UploadingFirmware => {
            (OtaStage::Transferring, "正在上传固件".to_string())
        }
        FirmwareUpdateStep::ActivatingFirmware => {
            (OtaStage::Activating, "正在激活新固件".to_string())
        }
        FirmwareUpdateStep::TriggeringReboot => {
            (OtaStage::Rebooting, "正在重启设备".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcumgr_toolkit::client::FirmwareUpdateStep;

    #[test]
    fn maps_uploading_step() {
        let (stage, _) = map_step(FirmwareUpdateStep::UploadingFirmware);
        assert_eq!(stage, OtaStage::Transferring);
    }

    #[test]
    fn maps_reboot_step() {
        let (stage, _) = map_step(FirmwareUpdateStep::TriggeringReboot);
        assert_eq!(stage, OtaStage::Rebooting);
    }
}
