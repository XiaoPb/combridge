pub mod smp;

use crate::ota::types::OtaPackageInfo;

/// 校验并提取后的升级镜像。
pub struct FirmwareImage {
    pub info: OtaPackageInfo,
    pub image: Vec<u8>,
}
