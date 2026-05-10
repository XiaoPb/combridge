# 错误处理模块

## 概述

错误处理模块定义了统一的错误类型和错误码体系，提供一致的错误处理和响应格式。当前实现采用 **`thiserror` 库** 简化错误类型定义。

## 模块位置

- 源码路径：`src-tauri/src/error.rs`

## 重要说明

> **当前实现使用 `thiserror` 库**。`ComBridgeError` 使用 `#[derive(thiserror::Error)]` 自动实现 `Error` trait 和 `Display` trait。

## 核心组件

### ErrorCode

错误码枚举，每个变体对应一个数值范围：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    SerialError = 1000,     // 串口错误 (1000-1999)
    BleError = 2000,        // BLE 错误 (2000-2999)
    ProtocolError = 3000,   // 协议错误 (3000-3999)
    ConfigError = 5000,     // 配置错误 (5000-5999)
    IoError = 6000,         // IO 错误 (6000-6999)
    ParseError = 7000,      // 解析错误 (7000-7999)
}
```

`Display` 实现输出格式为 `E1000`~`E7000`：

```rust
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::SerialError => write!(f, "E1000"),
            ErrorCode::BleError => write!(f, "E2000"),
            ErrorCode::ProtocolError => write!(f, "E3000"),
            ErrorCode::ConfigError => write!(f, "E5000"),
            ErrorCode::IoError => write!(f, "E6000"),
            ErrorCode::ParseError => write!(f, "E7000"),
        }
    }
}
```

### ComBridgeError

统一错误类型，使用 `thiserror` 库自动实现 `Display` 和 `Error` trait：

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum ComBridgeError {
    #[error("[E1000] {0}")]
    SerialError(String),
    #[error("[E2000] {0}")]
    BleError(String),
    #[error("[E3000] {0}")]
    ProtocolError(String),
    #[error("[E4000] {0}")]
    #[error("[E5000] {0}")]
    ConfigError(String),
    #[error("[E6000] {0}")]
    IoError(String),
    #[error("[E7000] {0}")]
    ParseError(String),
    #[error("[E8000] {message}")]
    DeviceError { code: ErrorCode, message: String },
}
```

### ErrorResponse

错误响应结构，用于 Tauri 命令返回给前端：

```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct ErrorResponse {
    pub code: i32,            // 错误码数值
    pub error_code: String,   // 错误码字符串（如 "E1000"）
    pub message: String,      // 错误消息
}
```

`Display` 实现输出格式为 `[Exxxx] 错误消息`：

```rust
impl fmt::Display for ErrorResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.error_code, self.message)
    }
}
```

### Result

统一结果类型：

```rust
pub type Result<T> = std::result::Result<T, ComBridgeError>;
```

## 错误码体系

| 错误码 | 名称 | 数值范围 | 说明 |
|--------|------|----------|------|
| E1000 | SerialError | 1000-1999 | 串口相关错误 |
| E2000 | BleError | 2000-2999 | BLE 相关错误 |
| E3000 | ProtocolError | 3000-3999 | 协议相关错误 |
| E5000 | ConfigError | 5000-5999 | 配置相关错误 |
| E6000 | IoError | 6000-6999 | IO 相关错误 |
| E7000 | ParseError | 7000-7999 | 解析相关错误 |
| E8000 | DeviceError | 8000-8999 | 设备管理错误 |

## 核心功能

### 错误创建便捷方法

```rust
impl ComBridgeError {
    pub fn serial<T: Into<String>>(msg: T) -> Self { ComBridgeError::SerialError(msg.into()) }
    pub fn ble<T: Into<String>>(msg: T) -> Self { ComBridgeError::BleError(msg.into()) }
    pub fn protocol<T: Into<String>>(msg: T) -> Self { ComBridgeError::ProtocolError(msg.into()) }
    pub fn config<T: Into<String>>(msg: T) -> Self { ComBridgeError::ConfigError(msg.into()) }
    pub fn io<T: Into<String>>(msg: T) -> Self { ComBridgeError::IoError(msg.into()) }
    pub fn parse<T: Into<String>>(msg: T) -> Self { ComBridgeError::ParseError(msg.into()) }
    pub fn device<T: Into<String>>(msg: T) -> Self { ComBridgeError::DeviceError { code: ErrorCode::DeviceError, message: msg.into() } }
}
```

### 错误信息获取

```rust
impl ComBridgeError {
    pub fn error_code(&self) -> ErrorCode
    pub fn message(&self) -> &str
    pub fn to_error_response(&self) -> ErrorResponse
}
```

## 错误转换

### From 实现

```rust
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
```

## Dashboard 模块错误

Dashboard 模块（`src-tauri/src/dashboard/`）当前使用 `Result<T, String>` 作为错误类型，未集成 `ComBridgeError`。这意味着 Dashboard 命令的错误响应格式与使用 `ComBridgeError` 的命令不同。

### 当前 Dashboard 错误处理

```rust
#[tauri::command]
pub async fn save_parser_script(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
    content: String,
) -> Result<(), String> {
    manager.save_script(&name, &content)
}
```

### 未来改进方向

Dashboard 模块应迁移至使用 `ComBridgeError`，以保持统一的错误响应格式：

```rust
#[tauri::command]
pub async fn save_parser_script(
    manager: State<'_, ParserScriptManagerRef>,
    name: String,
    content: String,
) -> Result<(), ComBridgeError> {
    manager.save_script(&name, &content)
        .map_err(|e| ComBridgeError::config(format!("保存解析脚本失败: {}", e)))
}
```

## DeviceError 变体说明

`DeviceError` 已实现，用于统一设备管理层面的错误：

```rust
#[error("[E8000] {message}")]
DeviceError { code: ErrorCode, message: String },
```

适用场景：
- 设备不存在
- 设备已连接/已断开
- 设备类型不匹配
- 锁获取失败（如 `PoisonError`）

`LockResultExt` trait 提供了锁错误的便捷转换：

```rust
pub trait LockResultExt<T> {
    fn lock_err(self, context: &str) -> Result<T>;
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

## 使用示例

### 创建错误

```rust
return Err(ComBridgeError::serial("串口打开失败"));
return Err(ComBridgeError::ble("设备连接超时"));
return Err(ComBridgeError::protocol("脚本解析错误"));
return Err(ComBridgeError::config("保存解析脚本失败: 权限不足"));
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

## 单元测试

错误处理模块包含以下单元测试：

| 测试 | 说明 |
|------|------|
| `test_error_code_display` | 验证 ErrorCode 的 Display 输出格式 |
| `test_combridge_error_display` | 验证 ComBridgeError 的 Display 输出格式 |
| `test_error_response` | 验证 ErrorResponse 的字段值 |
| `test_from_io_error` | 验证 io::Error 到 ComBridgeError 的转换 |
| `test_from_serde_json_error` | 验证 serde_json::Error 到 ComBridgeError 的转换 |

## 最佳实践

1. **使用便捷方法**：优先使用 `ComBridgeError::serial()` 等便捷方法创建错误
2. **包含上下文**：错误消息应包含足够的上下文信息，如设备名称、端口名、操作类型等
3. **错误转换**：使用 `?` 运算符自动转换底层错误
4. **日志记录**：在错误处理路径中记录 `error` 级别日志，包含错误上下文
5. **用户友好**：错误消息应对用户友好，避免暴露技术细节
6. **统一格式**：新模块应使用 `ComBridgeError` 而非 `String` 作为错误类型

## 相关模块

- [命令层](./commands-module.md) - 命令错误响应
- [设备管理](./device-manager.md) - 设备错误处理
- [服务层](./service-module.md) - 服务错误处理
- [Dashboard](./dashboard-module.md) - Dashboard 错误处理（当前使用 String）
