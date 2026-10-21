use std::sync::Arc;

use crate::error::ComBridgeError;

use super::protocol::OtaProtocol;
use super::types::OtaProtocolDescriptor;

/// 协议注册表：管理接口按 id 解析协议实现，未指定时使用默认协议。
pub struct ProtocolRegistry {
    protocols: Vec<Arc<dyn OtaProtocol>>,
    default_id: &'static str,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            protocols: Vec::new(),
            default_id: "smp",
        }
    }

    /// 同 id 重复注册时覆盖旧实现，便于测试或运行时替换。
    pub fn register(&mut self, protocol: Arc<dyn OtaProtocol>) {
        let id = protocol.id();
        self.protocols.retain(|existing| existing.id() != id);
        self.protocols.push(protocol);
    }

    pub fn resolve(&self, id: Option<&str>) -> Result<Arc<dyn OtaProtocol>, ComBridgeError> {
        let id = id.unwrap_or(self.default_id);
        self.protocols
            .iter()
            .find(|protocol| protocol.id() == id)
            .cloned()
            .ok_or_else(|| ComBridgeError::ota(format!("不支持的 OTA 协议: {id}")))
    }

    pub fn descriptors(&self) -> Vec<OtaProtocolDescriptor> {
        self.protocols
            .iter()
            .map(|protocol| OtaProtocolDescriptor {
                id: protocol.id().to_string(),
                name: protocol.display_name().to_string(),
                description: protocol.description().to_string(),
            })
            .collect()
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ComBridgeError;
    use crate::ota::package::ZipPackage;
    use crate::ota::protocol::{FirmwareImage, OtaProgressFn, OtaProtocol};
    use crate::ota::types::{OtaDeviceInfo, OtaStartOptions};
    use async_trait::async_trait;
    use std::sync::atomic::AtomicBool;

    struct TestProtocol {
        id: &'static str,
    }

    #[async_trait]
    impl OtaProtocol for TestProtocol {
        fn id(&self) -> &'static str {
            self.id
        }

        fn display_name(&self) -> &'static str {
            "测试协议"
        }

        fn description(&self) -> &'static str {
            "仅用于注册表测试"
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
            _progress: OtaProgressFn,
        ) -> Result<(), ComBridgeError> {
            Err(ComBridgeError::ota("测试协议不支持升级"))
        }
    }

    fn registry() -> ProtocolRegistry {
        let mut registry = ProtocolRegistry::new();
        registry.register(Arc::new(TestProtocol { id: "smp" }));
        registry
    }

    #[test]
    fn resolves_default_protocol() {
        assert_eq!(registry().resolve(None).expect("default").id(), "smp");
    }

    #[test]
    fn resolves_explicit_protocol() {
        assert_eq!(
            registry().resolve(Some("smp")).expect("explicit").id(),
            "smp"
        );
    }

    #[test]
    fn rejects_unknown_protocol() {
        assert!(registry().resolve(Some("unknown")).is_err());
    }

    #[test]
    fn same_id_registration_replaces_previous() {
        let mut registry = ProtocolRegistry::new();
        let first: Arc<dyn OtaProtocol> = Arc::new(TestProtocol { id: "smp" });
        let second: Arc<dyn OtaProtocol> = Arc::new(TestProtocol { id: "smp" });
        registry.register(first);
        registry.register(Arc::clone(&second));

        let resolved = registry.resolve(Some("smp")).expect("resolved");
        assert!(Arc::ptr_eq(&resolved, &second));
        assert_eq!(registry.descriptors().len(), 1);
    }
}
