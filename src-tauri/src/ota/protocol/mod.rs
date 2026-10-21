pub mod goodix;
pub mod smp;

use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use async_trait::async_trait;

use crate::error::ComBridgeError;

use super::package::ZipPackage;
use super::types::{OtaDeviceInfo, OtaPackageInfo, OtaStage, OtaStartOptions};

/// 协议向管理接口回报进度：阶段、可选的字节进度、文案；返回 false 表示请求中止。
pub type OtaProgressFn = Arc<dyn Fn(OtaStage, Option<(u64, u64)>, &str) -> bool + Send + Sync>;

/// 校验并提取后的升级镜像。
pub struct FirmwareImage {
    pub info: OtaPackageInfo,
    pub image: Vec<u8>,
}

/// OTA 协议实现契约；管理接口只依赖此 trait。
#[async_trait]
pub trait OtaProtocol: Send + Sync {
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn validate_package(&self, package: &mut ZipPackage) -> Result<FirmwareImage, ComBridgeError>;

    /// 校验并提取单个原始镜像文件（如 Goodix `.bin`）。
    ///
    /// 默认不支持直接镜像；只有以裸镜像为升级单元的协议才实现。
    fn validate_image(&self, _path: &Path) -> Result<FirmwareImage, ComBridgeError> {
        Err(ComBridgeError::ota(format!(
            "{} 协议只支持 .zip 固件包",
            self.display_name()
        )))
    }

    async fn preflight(&self, address: &str) -> Result<OtaDeviceInfo, ComBridgeError>;
    async fn upgrade(
        &self,
        address: &str,
        image: Vec<u8>,
        options: &OtaStartOptions,
        cancel: Arc<AtomicBool>,
        progress: OtaProgressFn,
    ) -> Result<(), ComBridgeError>;
}
