# 错误处理模块

## 概述

错误处理模块定义了统一的错误类型和错误码体系，提供一致的错误处理和响应格式。

## 模块位置

- 源码路径：`src-tauri/src/error.rs`

## 核心组件

### ErrorCode

错误码枚举：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    SerialError = 1000,     // 串口错误
    BleError = 2000,        // BLE 错误
    ProtocolError = 3000,   // 协议错误
    WebSocketError = 4000,  // WebSocket 错误
    ConfigError = 5000,     // 配置错误
    IoError = 6000,         // IO 错误
    ParseError = 7000,      // 解析错误
}
```

### ComBridgeError

统一错误类型：

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub enum ComBridgeError {
    SerialError(String),      // 串口错误
    BleError(String),         // BLE 错误
    ProtocolError(String),    // 协议错误
    WebSocketError(String),   // WebSocket 错误
    ConfigError(String),      // 配置错误
    IoError(String),          // IO 错误
    ParseError(String),       // 解析错误
}
```

### ErrorResponse

错误响应结构：

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorResponse {
    pub code: i32,            // 错误码数值
    pub error_code: String,   // 错误码字符串
    pub message: String,      // 错误消息
}
```

### Result

统一结果类型：

```rust
pub type Result<T> = std::result::Result<T, ComBridgeError>;
```

## 错误码体系

| 错误码 | 名称 | 范围 | 说明 |
|--------|------|------|------|
| E1000 | SerialError | 1000-1999 | 串口相关错误 |
| E2000 | BleError | 2000-2999 | BLE 相关错误 |
| E3000 | ProtocolError | 3000-3999 | 协议相关错误 |
| E4000 | WebSocketError | 4000-4999 | WebSocket 相关错误 |
| E5000 | ConfigError | 5000-5999 | 配置相关错误 |
| E6000 | IoError | 6000-6999 | IO 相关错误 |
| E7000 | ParseError | 7000-7999 | 解析相关错误 |

## 核心功能

### 错误创建

```rust
impl ComBridgeError {
    // 创建串口错误
    pub fn serial<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::SerialError(msg.into())
    }
    
    // 创建 BLE 错误
    pub fn ble<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::BleError(msg.into())
    }
    
    // 创建协议错误
    pub fn protocol<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ProtocolError(msg.into())
    }
    
    // 创建 WebSocket 错误
    pub fn websocket<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::WebSocketError(msg.into())
    }
    
    // 创建配置错误
    pub fn config<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ConfigError(msg.into())
    }
    
    // 创建 IO 错误
    pub fn io<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::IoError(msg.into())
    }
    
    // 创建解析错误
    pub fn parse<T: Into<String>>(msg: T) -> Self {
        ComBridgeError::ParseError(msg.into())
    }
}
```

### 错误信息获取

```rust
impl ComBridgeError {
    // 获取错误码
    pub fn error_code(&self) -> ErrorCode
    
    // 获取错误消息
    pub fn message(&self) -> &str
    
    // 转换为错误响应
    pub fn to_error_response(&self) -> ErrorResponse
}
```

## 错误转换

### From 实现

```rust
// 从 std::io::Error 转换
impl From<io::Error> for ComBridgeError {
    fn from(err: io::Error) -> Self {
        ComBridgeError::IoError(err.to_string())
    }
}

// 从 serde_json::Error 转换
impl From<serde_json::Error> for ComBridgeError {
    fn from(err: serde_json::Error) -> Self {
        ComBridgeError::ParseError(err.to_string())
    }
}

// 从 Box<dyn Error> 转换
impl From<Box<dyn Error>> for ComBridgeError {
    fn from(err: Box<dyn Error>) -> Self {
        ComBridgeError::ParseError(err.to_string())
    }
}
```

## 使用示例

### 创建错误

```rust
// 使用便捷方法创建错误
return Err(ComBridgeError::serial("串口打开失败"));
return Err(ComBridgeError::ble("设备连接超时"));
return Err(ComBridgeError::protocol("脚本解析错误"));
```

### 错误处理

```rust
fn open_port(config: SerialPortConfig) -> Result<()> {
    let port = serialport::open(&config.port_name)
        .map_err(|e| ComBridgeError::serial(format!("无法打开串口: {}", e)))?;
    
    Ok(())
}
```

### 命令层错误响应

```rust
#[tauri::command]
pub async fn open_serial_port(
    serial_manager: State<'_, SerialManagerRef>,
    config: SerialPortConfig,
) -> Result<(), ErrorResponse> {
    serial_manager
        .open_port(config)
        .map_err(|e| e.to_error_response())
}
```

### 错误日志记录

```rust
match device.connect(&address).await {
    Ok(conn) => {
        info!("设备连接成功: {}", conn.address);
    }
    Err(e) => {
        error!("设备连接失败: [{}] {}", e.error_code(), e.message());
    }
}
```

## 错误响应格式

### JSON 格式

```json
{
    "code": 1001,
    "error_code": "E1000",
    "message": "串口 COM3 不存在"
}
```

### 显示格式

```
[E1000] 串口 COM3 不存在
```

## 最佳实践

1. **使用便捷方法**：优先使用 `ComBridgeError::serial()` 等便捷方法创建错误
2. **包含上下文**：错误消息应包含足够的上下文信息，如设备名称、操作类型等
3. **错误转换**：使用 `?` 运算符自动转换底层错误
4. **日志记录**：在错误处理路径中记录错误日志
5. **用户友好**：错误消息应对用户友好，避免暴露技术细节

## 相关模块

- [命令层](./commands-module.md) - 命令错误响应
- [设备管理](./device-manager.md) - 设备错误处理
- [服务层](./service-module.md) - 服务错误处理
