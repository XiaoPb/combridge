use std::error::Error;
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
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum ComBridgeError {
    SerialError(String),
    BleError(String),
    ProtocolError(String),
    WebSocketError(String),
    ConfigError(String),
    IoError(String),
    ParseError(String),
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
}

impl fmt::Display for ComBridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = self.error_code();
        let msg = self.message();
        write!(f, "[{}] {}", code, msg)
    }
}

impl Error for ComBridgeError {}

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

impl From<Box<dyn Error>> for ComBridgeError {
    fn from(err: Box<dyn Error>) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_display() {
        assert_eq!(ErrorCode::SerialError.to_string(), "E1000");
        assert_eq!(ErrorCode::BleError.to_string(), "E2000");
        assert_eq!(ErrorCode::ProtocolError.to_string(), "E3000");
    }

    #[test]
    fn test_combridge_error_display() {
        let err = ComBridgeError::serial("串口打开失败");
        assert_eq!(format!("{}", err), "[E1000] 串口打开失败");
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
