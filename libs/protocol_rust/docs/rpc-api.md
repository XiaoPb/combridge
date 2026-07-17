# rpc 模块 API 文档

## 概述

`rpc` 是 GH Protocol 的核心 RPC 库，提供 RPC 核心协议实现，支持异步操作、自动分帧、超时重发等功能。该模块是 `gh-rpc` 高级库的基础。

## 模块结构

| 模块 | 描述 |
|------|------|
| [`error`] | 错误类型定义 |
| [`types`] | 核心类型定义 |
| [`log`] | 日志接口 |
| [`frame`] | 帧解析与构建 |
| [`package`] | 数据打包与解包 |
| [`unpacker`] | 数据解码器 |
| [`core`] | RPC核心实现 |

---

## 核心类型

### `RpcCore`

RPC核心实现，支持命令注册与调用、异步调用与超时重发、多帧数据重组、发布/订阅模式。

```rust
pub struct RpcCore { /* ... */ }
```

**创建方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | `config: RpcConfig` | `Self` | 使用配置创建RpcCore |
| `default` | - | `Self` | 使用默认配置创建RpcCore |
| `with_logger` | `logger: Arc<dyn LogCallback>` | `Self` | 设置日志回调 |

---

### `RpcConfig`

RPC配置结构体。

```rust
#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub timeout_ms: u64,       // 超时时间（毫秒）
    pub retry_count: u8,       // 重试次数
    pub retry_delay_ms: u64,   // 重试延迟（毫秒）
    pub frame_size: usize,     // 帧大小
}
```

**默认值：**

| 字段 | 默认值 | 常量 |
|------|--------|------|
| `timeout_ms` | 200 | `DEFAULT_TIMEOUT_MS` |
| `retry_count` | 3 | `MAX_RETRY_COUNT` |
| `retry_delay_ms` | 200 | `DEFAULT_TIMEOUT_MS` |
| `frame_size` | 240 | `GHRPC_FRAME_SIZE` |

**示例：**

```rust
use rpc::{RpcCore, RpcConfig};

let config = RpcConfig {
    timeout_ms: 500,
    retry_count: 5,
    retry_delay_ms: 100,
    frame_size: 240,
};
let core = RpcCore::new(config);

let default_core = RpcCore::default();
```

---

### 核心方法

#### 命令注册

```rust
pub async fn register(&self, key: &str, handler: RpcHandler) -> Result<(), RpcError>
```

注册命令处理器。

**参数：**
- `key`: 命令键名（最大长度 `MAX_SUPPORT_KEY_SIZE` = 32）
- `handler`: 命令处理函数

**返回值：**
- `Ok(())`: 注册成功
- `Err(RpcError::KeyOverMaxSize)`: 键名过长

**示例：**

```rust
use rpc::{RpcCore, RpcHandler, InvokeContext};
use std::sync::Arc;

let core = RpcCore::default();

let handler: RpcHandler = Arc::new(|data: &[u8], size: usize, ctx: &mut InvokeContext| {
    println!("收到数据: {} 字节", size);
    ctx.set_response(vec![0x00]);  // 设置响应
});

core.register("test_cmd", handler).await?;
```

---

#### 命令注销

```rust
pub async fn unregister(&self, key: &str) -> bool
```

注销命令处理器。

**返回值：**
- `true`: 注销成功
- `false`: 命令不存在

---

#### 同步调用

```rust
pub async fn call(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<Vec<u8>, RpcError>
```

同步调用命令，等待响应。

**参数：**
- `key`: 命令键名
- `format`: 参数格式字符串
- `raw_data`: 原始参数数据

**返回值：**
- `Ok(Vec<u8>)`: 响应数据
- `Err(RpcError)`: 调用错误

---

#### 发送命令

```rust
pub async fn send(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError>
```

发送命令并等待ACK确认。

---

#### 发布命令

```rust
pub async fn publish(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError>
```

发布命令，不等待响应。

---

#### 安全调用

```rust
pub async fn sall(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<Vec<u8>, RpcError>
```

安全调用命令，使用安全帧传输。

---

#### 处理接收数据

```rust
pub async fn process(&self, data: &[u8]) -> Vec<Result<ParseResult, RpcError>>
```

处理接收到的数据，解析帧并调用相应的处理器。

**示例：**

```rust
use rpc::{RpcCore, RpcHandler, InvokeContext};
use std::sync::Arc;

let core = RpcCore::default();

let handler: RpcHandler = Arc::new(|data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
    println!("处理数据: {:02X?}", data);
});
core.register("G", handler).await?;

let received_data: &[u8] = &[/* 接收到的帧数据 */];
let results = core.process(received_data).await;

for result in results {
    match result {
        Ok(parse_result) => {
            println!("解析成功: key={}", parse_result.key);
        }
        Err(e) => {
            println!("解析错误: {:?}", e);
        }
    }
}
```

---

#### 设置发送函数

```rust
pub async fn set_send_function(&self, func: SendFunction)
```

设置数据发送函数，用于发送帧数据。

**示例：**

```rust
use rpc::{RpcCore, SendFunction, RpcError};
use std::sync::Arc;

let core = RpcCore::default();

let send_fn: SendFunction = Arc::new(|data: &[u8]| -> Result<(), RpcError> {
    println!("发送帧: {:02X?}", data);
    Ok(())
});

core.set_send_function(send_fn).await;
```

---

#### 辅助方法

```rust
pub async fn get_registered_commands(&self) -> Vec<String>
```

获取已注册的命令列表。

```rust
pub async fn has_command(&self, key: &str) -> bool
```

检查命令是否已注册。

```rust
pub fn get_config(&self) -> &RpcConfig
```

获取当前配置。

---

## 调用上下文

### `InvokeContext`

调用上下文，包含调用的元数据和响应数据。

```rust
#[derive(Debug, Clone)]
pub struct InvokeContext {
    pub topic: String,       // 主题
    pub is_secure: bool,     // 是否安全帧
    pub is_fin: bool,        // 是否最后一帧
    pub invoke_idx: u8,      // 调用索引
    pub frame_idx: u8,       // 帧索引
}
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | `topic: String` | `Self` | 创建新的调用上下文 |
| `set_response` | `data: Vec<u8>` | `()` | 设置响应数据 |
| `get_response` | - | `&[u8]` | 获取响应数据 |

---

### `InvokeNode`

调用节点，存储命令注册信息。

```rust
#[derive(Debug, Clone)]
pub struct InvokeNode {
    pub key: String,           // 命令键
    pub detail: Option<String>, // 详细描述
    pub handler: Option<RpcHandler>, // 处理器
}
```

---

## 帧解析

### `FrameParser`

帧解析器，用于解析接收到的帧数据。

```rust
pub struct FrameParser { /* ... */ }
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | - | `Self` | 创建新的帧解析器 |
| `reset` | - | `()` | 重置解析器状态 |
| `process` | `data: &[u8]` | `Vec<Result<ParseResult, RpcError>>` | 处理数据并返回解析结果 |

**帧格式：**

```
+----------+--------+---------+----------+-------+----------+--------+-----+
| Header   | Length | TypeKey | KeyData  | ComID | FrameID  | Param  | CRC |
| 2 bytes  | 1 byte | 1 byte  | N bytes  | 1 byte| 1 byte   | N bytes|1byte|
+----------+--------+---------+----------+-------+----------+--------+-----+
```

Length 是 1 字节无符号整数，表示从 TypeKey 开始到 Param 结束的帧体字节数，
不包含 Header、Length 字段和 CRC。完整帧长度为 `Length + 4`，CRC 位于
`3 + Length`，并通过 `calculate_crc(&frame[3..3 + Length])` 对帧体执行
`u8::wrapping_add` 累加校验。

Length 最大为 255，因此接收端支持的理论最大完整帧为 259 字节。
`GHRPC_FRAME_SIZE = 240` 是 FrameBuilder 当前使用的默认发送分片大小，不是
接收协议上限。FrameParser 使用最大 512 字节的环形接收缓存，以支持前导杂字节、
跨传输分片的半帧和单次输入中的多个协议帧；帧结束位置始终由 Length 确定，
帧体中出现 `AA 11` 不会触发提前切帧。

---

### `ParseResult`

帧解析结果。

```rust
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    pub key: String,        // 命令键
    pub param: Vec<u8>,     // 参数数据
    pub is_secure: bool,    // 是否安全帧
    pub is_fin: bool,       // 是否最后一帧
    pub invoke_idx: u8,     // 调用索引
    pub frame_idx: u8,      // 帧索引
}
```

---

### `ParseState`

解析状态枚举。

```rust
pub enum ParseState {
    FrameHeader,    // 帧头
    CheckLength,    // 长度检查
    CheckTypeKey,   // 类型键检查
    CheckKey,       // 键检查
    CheckIndex,     // 索引检查
    CheckParam,     // 参数检查
    CheckCrc,       // CRC检查
}
```

---

## 帧构建

### `FrameBuilder`

帧构建器，用于构建发送帧。

```rust
pub struct FrameBuilder { /* ... */ }
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | - | `Self` | 创建新的帧构建器 |
| `build_frame` | `key, param, secure, fin, invoke_idx, frame_idx` | `Vec<u8>` | 构建单个帧 |
| `build_frames` | `key, data, secure` | `Vec<Vec<u8>>` | 构建帧序列 |
| `build_frames_with_invoke_idx` | `key, data, secure, invoke_idx` | `Vec<Vec<u8>>` | 带调用索引构建帧序列 |
| `calculate_max_payload` | `key, secure, fin` | `usize` | 计算最大载荷大小 |

**示例：**

```rust
use rpc::FrameBuilder;

let mut builder = FrameBuilder::new();

let frame = builder.build_frame("G", &[0x01, 0x02, 0x03], false, true, 0, 0);
println!("单帧: {:02X?}", frame);

let data: Vec<u8> = (0..=255u8).cycle().take(300).collect();
let frames = builder.build_frames("Test", &data, false);
println!("帧数量: {}", frames.len());
```

---

### `calculate_crc`

计算CRC校验值（累加和）。

```rust
pub fn calculate_crc(data: &[u8]) -> u8
```

**示例：**

```rust
use rpc::calculate_crc;

let data = [0x01, 0x02, 0x03];
let crc = calculate_crc(&data);
assert_eq!(crc, 0x06);
```

---

## 数据打包

### `Package`

数据打包工具。

```rust
pub struct Package;
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `pack_u8` | `data: u8` | `Vec<u8>` | 打包u8 |
| `pack_u16` | `data: u16` | `Vec<u8>` | 打包u16 |
| `pack_u32` | `data: u32` | `Vec<u8>` | 打包u32 |
| `pack_u64` | `data: u64` | `Vec<u8>` | 打包u64 |
| `pack_i8` | `data: i8` | `Vec<u8>` | 打包i8 |
| `pack_i16` | `data: i16` | `Vec<u8>` | 打包i16 |
| `pack_i32` | `data: i32` | `Vec<u8>` | 打包i32 |
| `pack_i64` | `data: i64` | `Vec<u8>` | 打包i64 |
| `pack_f64` | `data: f64` | `Vec<u8>` | 打包f64 |
| `pack_u8_array` | `data: &[u8]` | `Vec<u8>` | 打包u8数组 |
| `pack_u16_array` | `data: &[u16]` | `Vec<u8>` | 打包u16数组 |
| `pack_u32_array` | `data: &[u32]` | `Vec<u8>` | 打包u32数组 |
| `pack` | `format: &str, values: &[u8]` | `Result<Vec<u8>, RpcError>` | 根据格式打包 |

**示例：**

```rust
use rpc::Package;

let packed_u16 = Package::pack_u16(0x1234);
assert_eq!(packed_u16, vec![0x34, 0x12]);

let packed_arr = Package::pack_u8_array(&[1, 2, 3, 4, 5]);
assert_eq!(packed_arr.len(), 7);  // 2字节长度 + 5字节数据
```

---

## 数据解包

### `Unpackage`

数据解包工具。

```rust
pub struct Unpackage;
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `unpack_u8` | `data: &[u8]` | `Result<u8, RpcError>` | 解包u8 |
| `unpack_u16` | `data: &[u8]` | `Result<u16, RpcError>` | 解包u16 |
| `unpack_u32` | `data: &[u8]` | `Result<u32, RpcError>` | 解包u32 |
| `unpack_u64` | `data: &[u8]` | `Result<u64, RpcError>` | 解包u64 |
| `unpack_i8` | `data: &[u8]` | `Result<i8, RpcError>` | 解包i8 |
| `unpack_i16` | `data: &[u8]` | `Result<i16, RpcError>` | 解包i16 |
| `unpack_i32` | `data: &[u8]` | `Result<i32, RpcError>` | 解包i32 |
| `unpack_i64` | `data: &[u8]` | `Result<i64, RpcError>` | 解包i64 |
| `unpack_f64` | `data: &[u8]` | `Result<f64, RpcError>` | 解包f64 |
| `unpack_u8_array` | `data: &[u8]` | `Result<Vec<u8>, RpcError>` | 解包u8数组 |
| `unpack_u16_array` | `data: &[u8]` | `Result<Vec<u16>, RpcError>` | 解包u16数组 |
| `unpack_u32_array` | `data: &[u8]` | `Result<Vec<u32>, RpcError>` | 解包u32数组 |
| `unpack_with_format` | `data: &[u8], format: &str` | `Result<Vec<u8>, RpcError>` | 根据格式解包 |

**示例：**

```rust
use rpc::Unpackage;

let data = vec![0x34, 0x12];
let value = Unpackage::unpack_u16(&data).unwrap();
assert_eq!(value, 0x1234);

let arr_data = vec![5, 0, 1, 2, 3, 4, 5];
let arr = Unpackage::unpack_u8_array(&arr_data).unwrap();
assert_eq!(arr, vec![1, 2, 3, 4, 5]);
```

---

## 数据解码器

### `DataUnpacker`

通用数据解码器，支持多种数据类型的解码。

```rust
pub struct DataUnpacker { /* ... */ }
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | - | `Self` | 创建新的数据解码器 |
| `unpack` | `data: &[u8], format: &str` | `Result<UnpackValue, UnpackError>` | 根据格式解码数据 |

---

### `UnpackValue`

解码值枚举，包含所有支持的值类型。

```rust
pub enum UnpackValue {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U8Array(Vec<u8>),
    I8Array(Vec<i8>),
    U16Array(Vec<u16>),
    I16Array(Vec<i16>),
    U32Array(Vec<u32>),
    I32Array(Vec<i32>),
    U64Array(Vec<u64>),
    I64Array(Vec<i64>),
    String(String),
}
```

---

### `UnpackError`

数据解码错误类型。

```rust
pub enum UnpackError {
    InsufficientData,   // 数据不足
    InvalidHeader,      // 无效的头部
    InvalidFormat,      // 无效的格式
    UnsupportedType,    // 不支持的类型
}
```

---

### 便捷解码函数

| 函数 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `unpack` | `data: &[u8], format: &str` | `Result<UnpackValue, UnpackError>` | 通用解码函数 |
| `unpack_u8_array` | `data: &[u8]` | `Vec<u8>` | 解码u8数组 |
| `unpack_u16_array` | `data: &[u8]` | `Vec<u16>` | 解码u16数组 |
| `unpack_u32_array` | `data: &[u8]` | `Vec<u32>` | 解码u32数组 |
| `unpack_u64_array` | `data: &[u8]` | `Vec<u64>` | 解码u64数组 |
| `unpack_i8_array` | `data: &[u8]` | `Vec<i8>` | 解码i8数组 |
| `unpack_i16_array` | `data: &[u8]` | `Vec<i16>` | 解码i16数组 |
| `unpack_i32_array` | `data: &[u8]` | `Vec<i32>` | 解码i32数组 |
| `unpack_i64_array` | `data: &[u8]` | `Vec<i64>` | 解码i64数组 |
| `unpack_string` | `data: &[u8]` | `String` | 解码字符串 |

**示例：**

```rust
use rpc::{unpack, unpack_u16_array, unpack_string, UnpackValue};

let data: &[u8] = &[0x60, 0x34, 0x12];
let value = unpack(data, "<u16>").unwrap();
if let UnpackValue::U16(v) = value {
    println!("解码值: {}", v);
}

let arr_data: &[u8] = &[0x64, 0x02, 0x34, 0x12, 0x78, 0x56];
let arr = unpack_u16_array(arr_data);
println!("数组: {:?}", arr);

let str_data: &[u8] = &[0x5C, 0x05, b'H', b'e', b'l', b'l', b'o'];
let s = unpack_string(str_data);
println!("字符串: {}", s);
```

---

## 类型定义

### `TypeHeader`

类型头部结构。

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeHeader {
    pub pack_type: u8,    // 打包类型
    pub is_array: bool,   // 是否数组
    pub width: u8,        // 宽度（位数的log2值）
    pub end: bool,        // 是否结束
    pub split: bool,      // 是否分割
}
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `to_byte` | - | `u8` | 转换为字节 |
| `from_byte` | `byte: u8` | `Self` | 从字节创建 |
| `head_type` | - | `u8` | 获取头部类型 |

---

### `FormatInfo`

格式信息结构。

```rust
#[derive(Debug, Clone, Default)]
pub struct FormatInfo {
    pub headers: Vec<TypeHeader>,  // 类型头部列表
    pub data_size: usize,          // 数据大小
    pub array_num: usize,          // 数组数量
}
```

**方法：**

```rust
pub fn parse(format: &str) -> Result<Self, RpcError>
```

解析格式字符串。

**示例：**

```rust
use rpc::FormatInfo;

let info = FormatInfo::parse("<u16><u32>").unwrap();
assert_eq!(info.headers.len(), 2);
assert_eq!(info.data_size, 6);  // 2 + 4 字节
```

---

### `TypeKey`

类型键结构，用于帧头部。

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct TypeKey {
    pub pack_type: u8,    // 打包类型
    pub is_array: bool,   // 是否数组
    pub width: u8,        // 宽度
    pub secure: bool,     // 是否安全帧
    pub fin: bool,        // 是否最后一帧
}
```

---

### `FrameIndex`

帧索引结构。

```rust
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameIndex {
    pub invoke_idx: u8,   // 调用索引
    pub frame_idx: u8,    // 帧索引
}
```

---

### `PackHeader`

打包头部标志（使用bitflags）。

```rust
bitflags::bitflags! {
    pub struct PackHeader: u32 {
        const RAWDATA_EN = 1 << 0;      // 原始数据使能
        const PHY_VALUE_EN = 1 << 1;    // 物理值使能
        const GS_DATA_EN = 1 << 2;      // G传感器数据使能
        const FLAGS_EN = 1 << 3;        // 标志使能
        const ALG_DATA_EN = 1 << 4;     // 算法数据使能
        const AGC_INFO_EN = 1 << 5;     // AGC信息使能
        const TIMESTAMP_EN = 1 << 6;    // 时间戳使能
        const FRAMEID_EN = 1 << 7;      // 帧ID使能
        const FUNC_ID_EN = 1 << 8;      // 功能ID使能
        const SLOT_CFG_EN = 1 << 9;     // 时隙配置使能
    }
}
```

---

## 日志接口

### `LogCallback`

日志回调trait。

```rust
pub trait LogCallback: Send + Sync {
    fn log(&self, level: LogLevel, context: &str, message: &str);
}
```

---

### `DefaultLogger`

默认日志实现，使用`log` crate。

```rust
pub struct DefaultLogger;

impl LogCallback for DefaultLogger {
    fn log(&self, level: LogLevel, _context: &str, message: &str) {
        match level {
            LogLevel::Trace => log::trace!("{}", message),
            LogLevel::Debug => log::debug!("{}", message),
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warn => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
        }
    }
}
```

---

### `NullLogger`

空日志实现，不输出任何日志。

```rust
pub struct NullLogger;

impl LogCallback for NullLogger {
    fn log(&self, _level: LogLevel, _context: &str, _message: &str) {}
}
```

---

### `LogLevel`

日志级别枚举。

```rust
pub enum LogLevel {
    Trace,   // 跟踪
    Debug,   // 调试
    Info,    // 信息
    Warn,    // 警告
    Error,   // 错误
}
```

---

## 错误类型

### `RpcError`

RPC错误类型枚举。

```rust
pub enum RpcError {
    MemoryNotEnough,    // 内存不足
    FormatError,        // 格式错误
    KeyOverMaxSize,     // 键超过最大大小
    NotUnderInvoke,     // 不在调用上下文中
    SendFail,           // 发送失败
    SendStatus,         // 发送状态错误
    LoseFrame,          // 丢帧
    CrcMismatch,        // CRC校验失败
    InvalidHeader,      // 无效帧头
    Timeout,            // 超时
    ChannelClosed,      // 通道已关闭
    MaxRetryExceeded,   // 超过最大重试次数
    CommandNotFound,    // 命令未找到
    InvalidParameter,   // 参数错误
    UnpackageError,     // 解包错误
    ParamTooMuch,       // 参数过多
}
```

---

## 常量

| 常量名 | 值 | 描述 |
|--------|-----|------|
| `FRAME_HEADER` | `[0xAA, 0x11]` | 帧头标识 |
| `GHRPC_FRAME_SIZE` | 240 | 默认帧大小 |
| `MAX_SUPPORT_KEY_SIZE` | 32 | 最大键名长度 |
| `DEFAULT_TIMEOUT_MS` | 200 | 默认超时时间（毫秒） |
| `MAX_RETRY_COUNT` | 3 | 最大重试次数 |
| `DYNAMIC_NODE_SIZE` | 3 | 动态节点大小 |
| `COMM_RETRY_TIME` | 500 | 通信重试时间 |
| `COMM_RETRY_ROUND` | 100 | 通信重试轮数 |

---

## 类型别名

```rust
pub type DeviceAddr = u8;      // 设备地址
pub type CommandId = u8;       // 命令ID
pub type SequenceId = u8;      // 序列号
pub type Payload = Vec<u8>;    // 载荷数据
pub type SendFunction = Arc<dyn Fn(&[u8]) -> Result<(), RpcError> + Send + Sync>;
pub type RpcHandler = Arc<dyn Fn(&[u8], usize, &mut InvokeContext) + Send + Sync>;
```

---

## 完整使用示例

### 示例1：基本命令注册与调用

```rust
use rpc::{RpcCore, RpcConfig, RpcHandler, InvokeContext, SendFunction};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core = RpcCore::default();
    
    let handler: RpcHandler = Arc::new(|data: &[u8], size: usize, ctx: &mut InvokeContext| {
        println!("收到命令，数据长度: {}", size);
        ctx.set_response(vec![0x01, 0x02, 0x03]);
    });
    
    core.register("test", handler).await?;
    
    let send_fn: SendFunction = Arc::new(|data: &[u8]| {
        println!("发送: {:02X?}", data);
        Ok(())
    });
    core.set_send_function(send_fn).await;
    
    let response = core.call("test", "<u8>", &[0x42]).await?;
    println!("响应: {:02X?}", response);
    
    Ok(())
}
```

### 示例2：帧解析与构建

```rust
use rpc::{FrameParser, FrameBuilder, calculate_crc};

fn main() {
    let mut builder = FrameBuilder::new();
    let mut parser = FrameParser::new();
    
    let key = "G";
    let param = vec![0x01, 0x02, 0x03];
    let frame = builder.build_frame(key, &param, false, true, 0, 0);
    
    println!("构建帧: {:02X?}", frame);
    
    let results = parser.process(&frame);
    for result in results {
        match result {
            Ok(parse_result) => {
                println!("解析成功:");
                println!("  键: {}", parse_result.key);
                println!("  参数: {:02X?}", parse_result.param);
                println!("  安全帧: {}", parse_result.is_secure);
                println!("  最后一帧: {}", parse_result.is_fin);
            }
            Err(e) => {
                println!("解析错误: {:?}", e);
            }
        }
    }
}
```

### 示例3：数据打包与解包

```rust
use rpc::{Package, Unpackage, FormatInfo};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value: u32 = 0x12345678;
    let packed = Package::pack_u32(value);
    println!("打包u32: {:02X?}", packed);
    
    let unpacked = Unpackage::unpack_u32(&packed)?;
    println!("解包: {:#X}", unpacked);
    
    let arr: Vec<u16> = vec![0x1234, 0x5678, 0xABCD];
    let packed_arr = Package::pack_u16_array(&arr);
    println!("打包u16数组: {:02X?}", packed_arr);
    
    let unpacked_arr = Unpackage::unpack_u16_array(&packed_arr)?;
    println!("解包数组: {:04X?}", unpacked_arr);
    
    let info = FormatInfo::parse("<u8><u16><u32>")?;
    println!("格式信息: {} 个参数, 数据大小 {} 字节", 
        info.headers.len(), info.data_size);
    
    Ok(())
}
```

### 示例4：多帧数据处理

```rust
use rpc::{RpcCore, RpcConfig, FrameBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let core = RpcCore::default();
    
    let large_data: Vec<u8> = (0..=255u8).cycle().take(500).collect();
    
    let mut builder = FrameBuilder::new();
    let frames = builder.build_frames("Test", &large_data, false);
    
    println!("数据大小: {} 字节", large_data.len());
    println!("帧数量: {}", frames.len());
    
    for (i, frame) in frames.iter().enumerate() {
        println!("帧 {}: {} 字节", i, frame.len());
    }
    
    Ok(())
}
```

---

## 版本信息

- **版本**: 0.1.0
- **Rust版本**: 2021 Edition
- **依赖**:
  - `tokio` (异步运行时)
  - `log` (日志门面)
  - `bitflags` (位标志支持)
