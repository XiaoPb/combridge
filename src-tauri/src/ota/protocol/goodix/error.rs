//! Goodix DFU 错误类型。
//!
//! Portions derived from gr5xxx_dfu, MIT OR Apache-2.0.

use std::time::Duration;

use crate::error::ComBridgeError;

/// Goodix DFU 内部使用的结果别名。
pub type Result<T> = std::result::Result<T, DfuError>;

/// DFU 栈中所有可恢复失败。
#[derive(Debug, thiserror::Error)]
pub enum DfuError {
    /// 底层传输（BLE、mock、自定义）失败。
    #[error("transport error: {0}")]
    Transport(String),

    /// 响应未在超时时间内到达。
    #[error("timeout while waiting for {context} ({timeout:?})")]
    Timeout {
        /// 等待内容的可读描述。
        context: String,
        /// 配置的超时时间。
        timeout: Duration,
    },

    /// 帧或协议字段非法。
    #[error("protocol error: {0}")]
    Protocol(String),

    /// 设备返回否定状态。
    #[error("device rejected the operation: {0}")]
    Device(String),

    /// 固件解析失败。
    #[error("firmware error: {0}")]
    Firmware(String),

    /// Flash 区域重叠。
    #[error("overlap: {0}")]
    Overlap(String),

    /// 传输层或设备不支持该特性。
    #[error("unsupported: {0}")]
    Unsupported(String),

    /// 调用方取消了会话。
    #[error("cancelled")]
    Cancelled,

    /// 本地 IO 失败。
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl DfuError {
    /// 构造超时错误。
    pub fn timeout(context: impl Into<String>, timeout: Duration) -> Self {
        Self::Timeout {
            context: context.into(),
            timeout,
        }
    }

    /// 是否为超时错误。
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Timeout { .. })
    }
}

impl From<DfuError> for ComBridgeError {
    fn from(error: DfuError) -> Self {
        ComBridgeError::ota(error.to_string())
    }
}
