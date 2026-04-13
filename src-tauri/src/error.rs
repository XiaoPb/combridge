use std::fmt;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    SerialError = 1000,
    BleError = 2000,
    ProtocolError = 3000,
    WebSocketError = 4000,
    ConfigError = 5000,
    IoError = 6000,
    ParseError = 7000,
    DeviceError = 8000,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::SerialError => write!(f, "E1000"),
            ErrorCode::BleError => write!(f, "E2000"),
            ErrorCode::ProtocolError => write!(f, "E3000"),
            ErrorCode::WebSocketError => write!(f, "E4000"),
            ErrorCode::ConfigError => write!(f, "E5000"),
            ErrorCode::IoError => write!(f, "E6000"),
            ErrorCode::ParseError => write!(f, "E7000"),
            ErrorCode::DeviceError => write!(f, "E8000"),
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ComBridgeError {
    #[error("[E1000] {0}")]
    SerialError(String),
    #[error("[E2000] {0}")]
    BleError(String),
    #[error("[E3000] {0}")]
    ProtocolError(String),
    #[error("[E4000] {0}")]
    WebSocketError(String),
    #[error("[E5000] {0}")]
    ConfigError(String),
    #[error("[E6000] {0}")]
    IoError(String),
    #[error("[E7000] {0}")]
    ParseError(String),
    #[error("[E8000] {message}")]
    DeviceError { code: ErrorCode, message: String },
}

impl serde::Serialize for ComBridgeError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            ComBridgeError::SerialError(msg) => map.serialize_entry("SerialError", msg)?,
            ComBridgeError::BleError(msg) => map.serialize_entry("BleError", msg)?,
            ComBridgeError::ProtocolError(msg) => map.serialize_entry("ProtocolError", msg)?,
            ComBridgeError::WebSocketError(msg) => map.serialize_entry("WebSocketError", msg)?,
            ComBridgeError::ConfigError(msg) => map.serialize_entry("ConfigError", msg)?,
            ComBridgeError::IoError(msg) => map.serialize_entry("IoError", msg)?,
            ComBridgeError::ParseError(msg) => map.serialize_entry("ParseError", msg)?,
            ComBridgeError::DeviceError { code, message } => {
                map.serialize_entry("DeviceError", &serde_json::json!({"code": *code as i32, "message": message}))?;
            }
        }
        map.end()
    }
}

impl ComBridgeError {
    pub fn error_code(&self) -> ErrorCode {
        match self {
            ComBridgeError::SerialError(_) => ErrorCode::SerialError,
            ComBridgeError::BleError(_) => ErrorCode::BleError,
            ComBridgeError::ProtocolError(_) => ErrorCode::ProtocolError,
            ComBridgeError::WebSocketError(_) => ErrorCode::WebSocketError,
            ComBridgeError::ConfigError(_) => ErrorCode::ConfigError,
            ComBridgeError::IoError(_) => ErrorCode::IoError,
            ComBridgeError::ParseError(_) => ErrorCode::ParseError,
            ComBridgeError::DeviceError { code, .. } => *code,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            ComBridgeError::SerialError(msg) => msg,
            ComBridgeError::BleError(msg) => msg,
            ComBridgeError::ProtocolError(msg) => msg,
            ComBridgeError::WebSocketError(msg) => msg,
            ComBridgeError::ConfigError(msg) => msg,
            ComBridgeError::IoError(msg) => msg,
            ComBridgeError::ParseError(msg) => msg,
            ComBridgeError::DeviceError { message, .. } => message,
        }
    }

    pub fn to_error_response(&self) -> ErrorResponse {
        ErrorResponse {
            code: self.error_code() as i32,
            error_code: self.error_code().to_string(),
            message: self.message().to_string(),
        }
    }

    pub fn serial<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::SerialError(msg.into())
    }

    pub fn ble<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::BleError(msg.into())
    }

    pub fn protocol<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ProtocolError(msg.into())
    }

    pub fn websocket<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::WebSocketError(msg.into())
    }

    pub fn config<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ConfigError(msg.into())
    }

    pub fn io<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::IoError(msg.into())
    }

    pub fn parse<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ParseError(msg.into())
    }

    pub fn device<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::DeviceError {
            code: ErrorCode::DeviceError,
            message: msg.into(),
        }
    }
}

impl From<io::Error> for ComBridgeError {
    fn from(err: io::Error) -> Self {
        ComBridgeError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ComBridgeError {
    fn from(err: serde_json::Error) -> Self {
        ComBridgeError::ParseError(err.to_string())
    }
}

impl From<Box<dyn std::error::Error>> for ComBridgeError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ComBridgeError::ParseError(err.to_string())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub error_code: String,
    pub message: String,
}

impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error_code, self.message)
    }
}

pub type Result<T> = std::result::Result<T, ComBridgeError>;

pub trait LockResultExt<T> {
    fn lock_err(self, context: &str) -> Result<T>;
}

impl<T> LockResultExt<T> for std::result::Result<T, std::sync::PoisonError<T>> {
    fn lock_err(self, context: &str) -> Result<T> {
        self.map_err(|e| ComBridgeError::DeviceError {
            code: ErrorCode::DeviceError,
            message: format!("{}锁获取失败: {}", context, e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::SerialError.to_string(), "E1000");
        assert_eq!(ErrorCode::BleError.to_string(), "E2000");
        assert_eq!(ErrorCode::ProtocolError.to_string(), "E3000");
        assert_eq!(ErrorCode::DeviceError.to_string(), "E8000");
    }

    #[test]
    fn test_combridge_error_display() {
        let err = ComBridgeError::serial("串口打开失败");
        assert_eq!(format!("{}", err), "[E1000] 串口打开失败");
    }

    #[test]
    fn test_device_error() {
        let err = ComBridgeError::device("设备未找到");
        assert_eq!(format!("{}", err), "[E8000] 设备未找到");
        assert_eq!(err.error_code(), ErrorCode::DeviceError);
    }

    #[test]
    fn test_error_response() {
        let err = ComBridgeError::protocol("协议解析错误");
        let resp = err.to_error_response();
        assert_eq!(resp.code, 3000);
        assert_eq!(resp.error_code, "E3000");
        assert_eq!(resp.message, "协议解析错误");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "文件未找到");
        let err: ComBridgeError = io_err.into();
        assert!(matches!(err, ComBridgeError::IoError(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: ComBridgeError = json_err.into();
        assert!(matches!(err, ComBridgeError::ParseError(_)));
    }
}
