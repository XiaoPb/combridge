//! RPC Error Types

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcError {
    MemoryNotEnough,
    FormatError,
    KeyOverMaxSize,
    NotUnderInvoke,
    SendFail,
    SendStatus,
    LoseFrame,
    CrcMismatch,
    InvalidHeader,
    Timeout,
    ChannelClosed,
    MaxRetryExceeded,
    CommandNotFound,
    InvalidParameter,
    UnpackageError,
    ParamTooMuch,
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::MemoryNotEnough => write!(f, "内存不足"),
            RpcError::FormatError => write!(f, "格式错误"),
            RpcError::KeyOverMaxSize => write!(f, "键超过最大大小"),
            RpcError::NotUnderInvoke => write!(f, "不在调用上下文中"),
            RpcError::SendFail => write!(f, "发送失败"),
            RpcError::SendStatus => write!(f, "发送状态错误"),
            RpcError::LoseFrame => write!(f, "丢帧"),
            RpcError::CrcMismatch => write!(f, "CRC校验失败"),
            RpcError::InvalidHeader => write!(f, "无效帧头"),
            RpcError::Timeout => write!(f, "超时"),
            RpcError::ChannelClosed => write!(f, "通道已关闭"),
            RpcError::MaxRetryExceeded => write!(f, "超过最大重试次数"),
            RpcError::CommandNotFound => write!(f, "命令未找到"),
            RpcError::InvalidParameter => write!(f, "参数错误"),
            RpcError::UnpackageError => write!(f, "解包错误"),
            RpcError::ParamTooMuch => write!(f, "参数过多"),
        }
    }
}

impl std::error::Error for RpcError {}

pub type Result<T> = std::result::Result<T, RpcError>;
