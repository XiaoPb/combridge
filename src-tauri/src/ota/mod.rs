pub mod package;
pub mod protocol;
pub mod types;

pub use package::ZipPackage;

pub use types::{
    OtaDeviceInfo, OtaKeyValue, OtaPackageInfo, OtaProtocolDescriptor, OtaStage, OtaStartOptions,
    OtaState, OtaStatusDto,
};
