pub mod manager;
pub mod package;
pub mod protocol;
pub mod registry;
pub mod types;

pub use manager::{OtaManager, OtaManagerRef};
pub use package::ZipPackage;
pub use protocol::{FirmwareImage, OtaProgressFn, OtaProtocol};
pub use registry::ProtocolRegistry;
pub use types::{
    OtaDeviceInfo, OtaKeyValue, OtaPackageInfo, OtaProtocolDescriptor, OtaStage, OtaStartOptions,
    OtaState, OtaStatusDto,
};
