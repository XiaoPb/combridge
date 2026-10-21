pub mod goodix;
pub mod smp;

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
