# gh-rpc 模块使用指南

## 目录

1. [概述](#概述)
2. [环境要求](#环境要求)
3. [快速开始](#快速开始)
4. [核心概念](#核心概念)
5. [详细使用流程](#详细使用流程)
6. [关键初始化代码示例](#关键初始化代码示例)
7. [错误处理方法](#错误处理方法)
8. [重要使用注意事项](#重要使用注意事项)
9. [高级用法](#高级用法)
10. [常见问题](#常见问题)

---

## 概述

`gh-rpc` 是 GH Protocol 的高级命令库，提供以下核心功能：

- **G协议帧解码**：解码传感器数据帧，提取IPD、原始数据、AGC信息等
- **命令执行**：封装RPC调用，提供便捷的命令执行接口
- **数据解码**：支持多种数据类型的解码，包括整数、数组、字符串等
- **异步支持**：基于Tokio异步运行时，支持高并发操作

### 架构层次

```
┌─────────────────────────────────────┐
│           应用层 (Application)        │
├─────────────────────────────────────┤
│         gh-rpc (高级封装层)           │
│  ┌──────────┐ ┌──────────┐ ┌──────┐ │
│  │Executor  │ │FrameDecoder│ │Unpacker│ │
│  └──────────┘ └──────────┘ └──────┘ │
├─────────────────────────────────────┤
│          rpc (核心协议层)             │
│  ┌──────────┐ ┌──────────┐ ┌──────┐ │
│  │ RpcCore  │ │FrameParser│ │Package│ │
│  └──────────┘ └──────────┘ └──────┘ │
├─────────────────────────────────────┤
│         传输层 (Transport)           │
└─────────────────────────────────────┘
```

---

## 环境要求

### Rust 版本

- **最低版本**: Rust 1.70.0 或更高
- **推荐版本**: Rust 1.75.0 或更高
- **Edition**: 2021

### 依赖项

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
gh-rpc = { path = "../gh-rpc" }
tokio = { version = "1", features = ["full"] }
```

### 异步运行时要求

`gh-rpc` 需要 Tokio 异步运行时支持。确保在程序入口启用运行时：

```rust
#[tokio::main]
async fn main() {
}
```

或手动创建运行时：

```rust
fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
    });
}
```

---

## 快速开始

### 最简示例

```rust
use gh_rpc::{FrameDecoder, GhFuncFrame};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let decoder = FrameDecoder::new();
    let raw_data: &[u8] = &[/* G协议帧数据 */];
    
    let frames = decoder.decode_frames(raw_data)?;
    
    for frame in frames {
        println!("帧ID: {}, 时间戳: {}ns", frame.frame_cnt, frame.timestamp);
    }
    
    Ok(())
}
```

### 完整示例

```rust
use gh_rpc::{CommandExecutor, RpcConfig, FrameCallback, GhFuncFrame, KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION};
use rpc::{SendFunction, RpcError};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig {
        timeout_ms: 500,
        retry_count: 3,
        ..Default::default()
    };
    
    let mut executor = CommandExecutor::new(config);
    
    let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
        println!("收到帧: ID={}, 功能={:?}", frame.frame_cnt, frame.id);
    });
    executor.register_frame_callback(callback);
    
    let send_fn: SendFunction = Arc::new(|data: &[u8]| -> Result<(), RpcError> {
        println!("发送数据: {} 字节", data.len());
        Ok(())
    });
    executor.set_send_function(send_fn).await;
    
    executor.register_g_handler().await?;
    
    let version = executor.call(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[0]).await?;
    println!("版本信息: {:02X?}", version);
    
    Ok(())
}
```

---

## 核心概念

### 1. G协议帧

G协议帧是传感器数据的基本传输单元，包含：

| 字段 | 类型 | 描述 |
|------|------|------|
| `frame_cnt` | `u32` | 帧计数器 |
| `timestamp` | `u64` | 时间戳（纳秒） |
| `gsensor_data` | `GhGsensorData` | 三轴加速度数据 |
| `id` | `GhFuncFixIdx` | 功能类型标识 |
| `ch_num` | `u8` | 有效通道数 |
| `data` | `Vec<GhFrameData>` | 各通道数据 |

### 2. 命令执行模式

| 模式 | 方法 | 描述 |
|------|------|------|
| 同步调用 | `call()` | 发送命令并等待响应 |
| 发送确认 | `send()` | 发送命令并等待ACK |
| 发布模式 | `publish()` | 发送命令不等待响应 |
| 安全调用 | `sall()` | 使用安全帧传输 |

### 3. 数据格式字符串

格式字符串用于描述参数类型：

```
格式语法: <类型[宽度][*]>
```

| 格式 | 示例 | 描述 |
|------|------|------|
| `<u8>` | `<u8>` | 无符号8位整数 |
| `<u16>` | `<u16>` | 无符号16位整数 |
| `<u32>` | `<u32>` | 无符号32位整数 |
| `<d32>` | `<d32>` | 有符号32位整数 |
| `<u16*>` | `<u16*>` | 无符号16位数组 |
| `<s>` | `<s>` | 字符串 |

---

## 详细使用流程

### 流程图

```
┌──────────────────┐
│   初始化配置      │
│  (RpcConfig)     │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ 创建执行器        │
│ (CommandExecutor)│
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ 设置发送函数      │
│ (SendFunction)   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│ 注册回调处理器    │
│ (FrameCallback)  │
└────────┬─────────┘
         │
         ├──────────────┐
         │              │
         ▼              ▼
┌────────────────┐ ┌────────────────┐
│ 注册G协议处理器 │ │ 执行命令调用   │
│ (register_g_   │ │ (call/send/    │
│  handler)      │ │  publish)      │
└────────┬───────┘ └────────┬───────┘
         │                  │
         └──────────┬───────┘
                    │
                    ▼
         ┌──────────────────┐
         │   处理接收数据    │
         │   (process)      │
         └──────────────────┘
```

### 步骤详解

#### 步骤1：创建配置

```rust
use rpc::RpcConfig;

let config = RpcConfig {
    timeout_ms: 500,      // 超时时间
    retry_count: 3,       // 重试次数
    retry_delay_ms: 100,  // 重试延迟
    frame_size: 240,      // 帧大小
};
```

#### 步骤2：创建执行器

```rust
use gh_rpc::CommandExecutor;

let executor = CommandExecutor::new(config);
```

#### 步骤3：设置发送函数

```rust
use rpc::SendFunction;
use std::sync::Arc;

let send_fn: SendFunction = Arc::new(|data: &[u8]| -> Result<(), RpcError> {
    // 实际发送逻辑，例如通过串口发送
    serial_port.write_all(data)?;
    Ok(())
});

executor.set_send_function(send_fn).await;
```

#### 步骤4：注册帧回调

```rust
use gh_rpc::{FrameCallback, GhFuncFrame};
use std::sync::Arc;

let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
    // 处理接收到的帧数据
    println!("帧ID: {}", frame.frame_cnt);
    for (i, ch) in frame.data.iter().enumerate() {
        println!("  通道{}: IPD={}", i, ch.ipd_pa);
    }
});

executor.register_frame_callback(callback);
```

#### 步骤5：注册G协议处理器

```rust
executor.register_g_handler().await?;
```

#### 步骤6：处理接收数据

```rust
// 在数据接收线程/回调中
let received_data: &[u8] = &[/* 从设备接收的数据 */];
let results = executor.process(received_data).await;

for result in results {
    if let Err(e) = result {
        eprintln!("处理错误: {:?}", e);
    }
}
```

---

## 关键初始化代码示例

### 示例1：串口通信场景

```rust
use gh_rpc::{CommandExecutor, RpcConfig, FrameCallback, GhFuncFrame};
use rpc::{SendFunction, RpcError};
use std::sync::Arc;
use tokio::sync::Mutex;
use serialport::{SerialPort, SerialPortType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig::default();
    let mut executor = CommandExecutor::new(config);
    
    let port = serialport::new("/dev/ttyUSB0", 115200)
        .timeout(std::time::Duration::from_millis(100))
        .open()?;
    let port = Arc::new(Mutex::new(port));
    
    {
        let port_clone = port.clone();
        let send_fn: SendFunction = Arc::new(move |data: &[u8]| {
            let port = port_clone.clone();
            let data = data.to_vec();
            
            let rt = tokio::runtime::Handle::try_current()
                .expect("需要在Tokio运行时中调用");
            
            rt.block_on(async {
                let mut port = port.lock().await;
                port.write_all(&data)
                    .map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        });
        
        executor.set_send_function(send_fn).await;
    }
    
    let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
        println!("帧: ID={}, 时间={}", frame.frame_cnt, frame.timestamp);
    });
    executor.register_frame_callback(callback);
    
    executor.register_g_handler().await?;
    
    let executor = Arc::new(Mutex::new(executor));
    let port_clone = port.clone();
    let executor_clone = executor.clone();
    
    tokio::spawn(async move {
        let mut buffer = [0u8; 1024];
        loop {
            let mut port = port_clone.lock().await;
            match port.read(&mut buffer) {
                Ok(n) if n > 0 => {
                    let executor = executor_clone.lock().await;
                    executor.process(&buffer[..n]).await;
                }
                _ => {}
            }
        }
    });
    
    tokio::signal::ctrl_c().await?;
    println!("程序退出");
    
    Ok(())
}
```

### 示例2：TCP网络通信场景

```rust
use gh_rpc::{CommandExecutor, RpcConfig, FrameCallback, GhFuncFrame};
use rpc::{SendFunction, RpcError};
use std::sync::Arc;
use tokio::net::{TcpStream, TcpListener};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig::default();
    let mut executor = CommandExecutor::new(config);
    
    let stream = TcpStream::connect("192.168.1.100:8080").await?;
    let (reader, writer) = stream.into_split();
    let reader = Arc::new(Mutex::new(reader));
    let writer = Arc::new(Mutex::new(writer));
    
    {
        let writer_clone = writer.clone();
        let send_fn: SendFunction = Arc::new(move |data: &[u8]| {
            let writer = writer_clone.clone();
            let data = data.to_vec();
            
            let rt = tokio::runtime::Handle::try_current()
                .expect("需要在Tokio运行时中调用");
            
            rt.block_on(async {
                let mut writer = writer.lock().await;
                writer.write_all(&data).await
                    .map_err(|_| RpcError::SendFail)?;
                writer.flush().await
                    .map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        });
        
        executor.set_send_function(send_fn).await;
    }
    
    let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
        println!("帧: ID={}", frame.frame_cnt);
    });
    executor.register_frame_callback(callback);
    
    executor.register_g_handler().await?;
    
    let executor = Arc::new(Mutex::new(executor));
    let reader_clone = reader.clone();
    let executor_clone = executor.clone();
    
    tokio::spawn(async move {
        let mut buffer = [0u8; 4096];
        loop {
            let mut reader = reader_clone.lock().await;
            match reader.read(&mut buffer).await {
                Ok(n) if n > 0 => {
                    let executor = executor_clone.lock().await;
                    executor.process(&buffer[..n]).await;
                }
                Ok(_) => break,
                Err(e) => {
                    eprintln!("读取错误: {}", e);
                    break;
                }
            }
        }
    });
    
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

### 示例3：自定义日志

```rust
use gh_rpc::{CommandExecutor, RpcConfig};
use rpc::{LogCallback, LogLevel, DefaultLogger};
use std::sync::Arc;

struct CustomLogger;

impl LogCallback for CustomLogger {
    fn log(&self, level: LogLevel, context: &str, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        println!("[{}][{:?}][{}] {}", timestamp, level, context, message);
    }
}

fn main() {
    let config = RpcConfig::default();
    let logger = Arc::new(CustomLogger);
    
    let executor = CommandExecutor::new(config)
        .with_logger(logger);
}
```

---

## 错误处理方法

### 错误类型

`gh-rpc` 模块主要涉及两类错误：

#### 1. `DecodeError` - 解码错误

```rust
use gh_rpc::DecodeError;

match decoder.decode_frames(&data) {
    Ok(frames) => {
        // 处理帧数据
    }
    Err(DecodeError::InsufficientData) => {
        eprintln!("数据不足，需要更多数据");
    }
    Err(DecodeError::InvalidFormat) => {
        eprintln!("数据格式无效");
    }
    Err(DecodeError::CrcMismatch) => {
        eprintln!("CRC校验失败，数据可能损坏");
    }
    Err(e) => {
        eprintln!("其他解码错误: {:?}", e);
    }
}
```

#### 2. `RpcError` - RPC错误

```rust
use rpc::RpcError;

match executor.call(key, format, params).await {
    Ok(response) => {
        // 处理响应
    }
    Err(RpcError::Timeout) => {
        eprintln!("命令执行超时");
    }
    Err(RpcError::CommandNotFound) => {
        eprintln!("命令未找到");
    }
    Err(RpcError::SendFail) => {
        eprintln!("发送失败，检查连接");
    }
    Err(RpcError::CrcMismatch) => {
        eprintln!("CRC校验失败");
    }
    Err(e) => {
        eprintln!("RPC错误: {:?}", e);
    }
}
```

### 错误处理最佳实践

```rust
use gh_rpc::{CommandExecutor, DecodeError};
use rpc::RpcError;

async fn safe_call(
    executor: &CommandExecutor,
    key: &str,
    format: &str,
    params: &[u8],
) -> Option<Vec<u8>> {
    match executor.call(key, format, params).await {
        Ok(response) => Some(response),
        Err(RpcError::Timeout) => {
            log::warn!("命令 {} 超时，可能设备未响应", key);
            None
        }
        Err(RpcError::SendFail) => {
            log::error!("发送失败，请检查连接状态");
            None
        }
        Err(e) => {
            log::error!("命令 {} 执行失败: {:?}", key, e);
            None
        }
    }
}

fn safe_decode(data: &[u8]) -> Option<Vec<gh_rpc::GhFuncFrame>> {
    use gh_rpc::FrameDecoder;
    
    let decoder = FrameDecoder::new();
    match decoder.decode_frames(data) {
        Ok(frames) => Some(frames),
        Err(DecodeError::InsufficientData) => {
            log::debug!("数据不完整，等待更多数据");
            None
        }
        Err(e) => {
            log::error!("解码失败: {:?}", e);
            None
        }
    }
}
```

---

## 重要使用注意事项

### 1. 异步运行时要求

**必须**在 Tokio 异步运行时环境中使用：

```rust
// 正确 ✓
#[tokio::main]
async fn main() {
    let executor = CommandExecutor::new(RpcConfig::default());
    executor.call("cmd", "<u8>", &[0]).await.unwrap();
}

// 错误 ✗
fn main() {
    let executor = CommandExecutor::new(RpcConfig::default());
    // executor.call() 是异步方法，无法直接调用
}
```

### 2. 资源释放

执行器和内部资源通过 `Arc` 和 `Mutex` 管理生命周期：

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let executor = Arc::new(Mutex::new(executor));

// 在多个任务间共享
let executor_clone = executor.clone();
tokio::spawn(async move {
    let exec = executor_clone.lock().await;
    exec.process(&data).await;
});

// 当所有 Arc 引用被释放后，资源自动清理
```

### 3. 线程安全

`CommandExecutor` 和 `FrameDecoder` 都是线程安全的：

```rust
use std::sync::Arc;

let decoder = Arc::new(FrameDecoder::new());

// 可以安全地在多个线程间共享
let decoder1 = decoder.clone();
let decoder2 = decoder.clone();

std::thread::spawn(move || {
    decoder1.decode_frames(&data).unwrap();
});

std::thread::spawn(move || {
    decoder2.decode_frames(&data).unwrap();
});
```

### 4. 版本兼容性

| gh-rpc 版本 | rpc 版本 | Rust 版本 | Tokio 版本 |
|-------------|----------|-----------|------------|
| 0.1.0 | 0.1.0 | 1.70+ | 1.x |

**注意**：`gh-rpc` 和 `rpc` 版本必须匹配，建议使用工作空间统一管理。

### 5. 性能考虑

- **帧解码**：`FrameDecoder` 内部使用 `Mutex` 保护状态，高并发场景下可能成为瓶颈
- **命令调用**：每次调用都会创建 `oneshot` 通道，大量短命令建议使用 `publish` 模式
- **内存管理**：大帧数据会自动分割，注意配置合适的 `frame_size`

### 6. 超时配置

根据实际设备响应时间调整超时：

```rust
let config = RpcConfig {
    timeout_ms: 1000,     // 慢速设备使用较长超时
    retry_count: 5,       // 增加重试次数
    retry_delay_ms: 200,  // 重试间隔
    ..Default::default()
};
```

### 7. 日志级别

建议在生产环境使用 `Info` 级别：

```rust
use rpc::{LogCallback, LogLevel};

impl LogCallback for ProductionLogger {
    fn log(&self, level: LogLevel, context: &str, message: &str) {
        if level >= LogLevel::Info {
            println!("[{:?}][{}] {}", level, context, message);
        }
    }
}
```

### 8. AGC 数据解码注意事项

AGC（自动增益控制）数据解码有特殊规则：

**传输格式与存储格式差异：**

| 字段 | 存储格式 (gh_agc_info_t) | 传输格式 (gh_agc_upload_t) |
|------|--------------------------|---------------------------|
| gain_code | ✓ | ✓ |
| bg_cancel_range | ✓ | ✓ |
| dc_cancel_range | ✓ | ✓ |
| dc_cancel_code | ✓ | ✓ |
| led_drv0 | ✓ | ✓ |
| led_drv1 | ✓ | ✓ |
| **bg_cancel_code** | ✓ | ✗ (不传输) |
| **tia_gain** | ✓ | ✗ (不传输) |
| **led_drv_fs** | ✗ (帧级别) | ✓ (帧级别) |

**重要说明：**

1. **AGC 数据使用绝对值编码**，不使用差分编码
2. `led_drv_fs` 是帧级别数据，存储在 `GhFuncFrame.led_drv_fs[0]`
3. `bg_cancel_code` 和 `tia_gain` 不在传输格式中，解码时设为默认值 0
4. `led_drv_fs[1]` 在传输格式中未编码，解码时保持为 0

```rust
// 解码后的 AGC 信息
let agc = &frame.data[0].agc_info;
println!("gain_code: {}", agc.gain_code);        // 正确解码
println!("bg_cancel_code: {}", agc.bg_cancel_code);  // 始终为 0

// led_drv_fs 在帧级别
println!("led_drv_fs: {}", frame.led_drv_fs[0]);  // 正确解码
```

---

## 高级用法

### 自定义命令处理器

```rust
use gh_rpc::CommandExecutor;
use rpc::{RpcHandler, InvokeContext};
use std::sync::Arc;

let handler: RpcHandler = Arc::new(|data: &[u8], size: usize, ctx: &mut InvokeContext| {
    println!("自定义处理器收到 {} 字节", size);
    
    // 解析参数
    if size >= 2 {
        let param1 = u16::from_le_bytes([data[0], data[1]]);
        println!("参数1: {}", param1);
    }
    
    // 设置响应
    ctx.set_response(vec![0x00, 0x01]);
});

executor.register("custom_cmd", handler).await?;
```

### 批量帧处理

```rust
use gh_rpc::{FrameDecoder, GhFuncFrame};

fn process_batch(data: &[u8]) -> Vec<GhFuncFrame> {
    let decoder = FrameDecoder::new();
    
    match decoder.decode_frames(data) {
        Ok(frames) => frames,
        Err(_) => Vec::new(),
    }
}

fn analyze_frames(frames: &[GhFuncFrame]) {
    let total_frames = frames.len();
    let avg_channels: f64 = frames.iter()
        .map(|f| f.ch_num as f64)
        .sum::<f64>() / total_frames as f64;
    
    println!("总帧数: {}", total_frames);
    println!("平均通道数: {:.2}", avg_channels);
}
```

### 数据统计

```rust
use gh_rpc::GhFuncFrame;
use std::collections::HashMap;

fn collect_statistics(frames: &[GhFuncFrame]) -> HashMap<u8, usize> {
    let mut stats = HashMap::new();
    
    for frame in frames {
        *stats.entry(frame.ch_num).or_insert(0) += 1;
    }
    
    stats
}
```

---

## 常见问题

### Q1: 解码失败返回 `InsufficientData`

**原因**：输入数据不完整，帧被截断。

**解决方案**：
- 确保数据接收完整后再解码
- 检查数据传输是否有丢包

```rust
// 使用缓冲区累积数据
let mut buffer = Vec::new();

fn on_receive(data: &[u8]) {
    buffer.extend_from_slice(data);
    
    // 尝试解码
    let decoder = FrameDecoder::new();
    match decoder.decode_frames(&buffer) {
        Ok(frames) => {
            // 成功解码后清空缓冲区
            buffer.clear();
            process_frames(&frames);
        }
        Err(DecodeError::InsufficientData) => {
            // 等待更多数据
        }
        Err(e) => {
            // 其他错误，清空缓冲区
            buffer.clear();
        }
    }
}
```

### Q2: 命令调用超时

**原因**：设备未响应或响应延迟过长。

**解决方案**：
- 增加超时时间
- 检查设备连接状态
- 确认发送函数正确实现

```rust
let config = RpcConfig {
    timeout_ms: 2000,  // 增加到2秒
    ..Default::default()
};
```

### Q3: 多线程访问冲突

**原因**：直接使用非线程安全的类型。

**解决方案**：使用 `Arc<Mutex<T>>` 包装：

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

let executor = Arc::new(Mutex::new(executor));

// 在异步任务中使用
let exec = executor.lock().await;
exec.call("cmd", "<u8>", &[0]).await?;
```

### Q4: 内存占用过高

**原因**：大量帧数据未及时处理。

**解决方案**：
- 及时消费帧数据
- 限制缓冲区大小
- 使用流式处理

```rust
// 限制缓冲区大小
const MAX_BUFFER_SIZE: usize = 65536;

if buffer.len() > MAX_BUFFER_SIZE {
    buffer.clear();  // 防止内存溢出
}
```

### Q5: CRC校验失败

**原因**：数据传输错误或帧格式不匹配。

**解决方案**：
- 检查传输链路质量
- 确认帧格式配置正确
- 添加重传机制

---

## 附录

### 完整示例项目结构

```
my-gh-app/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── device.rs
│   └── handlers.rs
└── tests/
    └── integration_test.rs
```

### Cargo.toml 示例

```toml
[package]
name = "my-gh-app"
version = "0.1.0"
edition = "2021"

[dependencies]
gh-rpc = { path = "../gh-rpc" }
rpc = { path = "../rpc" }
tokio = { version = "1", features = ["full"] }
log = "0.4"
env_logger = "0.11"
serialport = "4"
chrono = "0.4"
```

### 参考链接

- [Tokio 文档](https://docs.rs/tokio)
- [Rust 异步编程](https://rust-lang.github.io/async-book/)
- [GH Protocol 规范](内部文档)
