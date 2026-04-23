# gh-rpc 模块 API 文档

## 概述

`gh-rpc` 是 GH Protocol 的高级命令库，提供 GH 协议命令的封装和 G 协议解析功能。该模块基于 `rpc` 核心库构建，提供更高级别的抽象和便捷的命令执行接口。

## 模块结构

| 模块 | 描述 |
|------|------|
| [`types`] | G协议数据类型定义 |
| [`commands`] | 命令定义（命令键常量、格式字符串、Command枚举等） |
| [`frame_decoder`] | G协议帧解码器 |
| [`executor`] | 命令执行器 |

> **注意**：数据解码器（`DataUnpacker`、`UnpackValue`、`UnpackError`及相关便捷函数）由 `rpc` 模块提供，`gh-rpc` 重新导出这些类型以便于使用。详见 [rpc模块API文档](rpc-api.md#数据解码器)。

---

## 核心类型

### 数据类型

#### `GhFuncFrame`

功能帧结构，包含帧计数、时间戳、G传感器数据和通道数据。

```rust
pub struct GhFuncFrame {
    pub frame_cnt: u32,           // 帧计数
    pub timestamp: u64,           // 时间戳
    pub gsensor_data: GhGsensorData,  // G传感器数据
    pub id: GhFuncFixIdx,         // 功能ID
    pub ch_num: u8,               // 通道数
    pub ch_max: u8,               // 最大通道数
    pub gsensor_en: u8,           // G传感器使能
    pub fifo_end_flag: u8,        // FIFO结束标志
    pub led_drv_fs: [u8; 2],      // LED驱动配置
    pub data: Vec<GhFrameData>,   // 帧数据列表
}
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `from_bytes` | `data: &[u8]` | `Result<Self, DecodeError>` | 从字节数组解码功能帧 |
| `to_bytes` | - | `Vec<u8>` | 将功能帧编码为字节数组 |

**示例：**

```rust
use gh_rpc::GhFuncFrame;

let raw_data: &[u8] = &[/* 帧数据 */];
let frame = GhFuncFrame::from_bytes(raw_data)?;
println!("帧计数: {}, 时间戳: {}", frame.frame_cnt, frame.timestamp);
```

---

#### `GhFrameData`

帧数据结构，包含IPD、原始数据和AGC信息。

```rust
pub struct GhFrameData {
    pub ipd_pa: i32,              // IPD物理值
    pub rawdata: i32,             // 原始数据
    pub flag: GhFrameDataFlag,    // 数据标志
    pub agc_info: GhAgcInfo,      // AGC信息
}
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `from_bytes` | `data: &[u8]` | `Result<Self, DecodeError>` | 从字节数组解码帧数据 |
| `to_bytes` | `led_drv_fs: u8` | `Vec<u8>` | 将帧数据编码为字节数组 |

---

#### `GhAgcInfo`

AGC（自动增益控制）信息结构。

```rust
pub struct GhAgcInfo {
    pub gain_code: u8,            // 增益代码
    pub bg_cancel_range: u8,      // 背景消除范围
    pub dc_cancel_range: u8,      // DC消除范围
    pub dc_cancel_code: u8,       // DC消除代码
    pub led_drv0: u8,             // LED驱动0
    pub led_drv1: u8,             // LED驱动1
    pub bg_cancel_code: u8,       // 背景消除代码（解码时为0）
    pub tia_gain: u8,             // TIA增益（解码时为0）
}
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `from_bytes` | `data: &[u8]` | `Result<(Self, u8), DecodeError>` | 从字节数组解码，返回 (GhAgcInfo, led_drv_fs) |
| `to_bytes` | `led_drv_fs: u8` | `[u8; 8]` | 编码为字节数组 |

> **注意**：
> - `led_drv_fs` 是帧级别数据，存储在 `GhFuncFrame.led_drv_fs[0]` 中
> - `bg_cancel_code` 和 `tia_gain` 不在传输格式中，解码时设为默认值 0
> - AGC 数据使用绝对值编码，不使用差分编码

---

#### `GhGsensorData`

G传感器数据结构。

```rust
pub struct GhGsensorData {
    pub acc: [i16; 3],  // 三轴加速度数据 [x, y, z]
}
```

---

#### `GhFuncFixIdx`

功能固定索引枚举，定义不同的功能类型。

```rust
pub enum GhFuncFixIdx {
    Adt = 0,        // ADT功能
    Hr = 1,         // 心率
    Spo2 = 2,       // 血氧
    Hrv = 3,        // 心率变异性
    Gnadt = 4,      // GNADT
    Irnadt = 5,     // IRNADT
    AlgoMax = 6,    // 算法最大值
    Test2 = 7,      // 测试模式2
    PpgCfg0 = 8,    // PPG配置0-7
    PpgCfg1 = 9,
    PpgCfg2 = 10,
    PpgCfg3 = 11,
    PpgCfg4 = 12,
    PpgCfg5 = 13,
    PpgCfg6 = 14,
    PpgCfg7 = 15,
    CapCfg = 16,    // 电容配置
    Max = 17,       // 最大值
}
```

---

#### `GhFrameDataFlag`

帧数据标志结构。

```rust
pub struct GhFrameDataFlag {
    pub led_adj_flag: bool,       // LED调整标志
    pub sa_flag: bool,            // SA标志
    pub param_change_flag: bool,  // 参数变化标志
    pub dre_update: bool,         // DRE更新标志
    pub skip_ok_flag: bool,       // 跳过OK标志
}
```

---

### 错误类型

#### `DecodeError`

解码错误类型枚举。

```rust
pub enum DecodeError {
    InsufficientData,     // 数据不足
    InvalidFormat,        // 格式无效
    InvalidChannelCount,  // 通道数无效
    CrcMismatch,          // CRC校验失败
}
```

---

## 命令类型

### `Command`

命令枚举，包含所有支持的命令类型。

```rust
pub enum Command {
    Event(EventParams),
    F(FParams),
    Fw(FwParams),
    FGetMode(FGetModeParams),
    FSetMode(FSetModeParams),
    G(GParams),
    Gh3xChipCtrl(Gh3xChipCtrlParams),
    Gh3xGetVersion(Gh3xGetVersionParams),
    Gh3xRegBitFieldWriteCmd(Gh3xRegBitFieldWriteCmdParams),
    Gh3xRegsBitFieldWriteCmd(Gh3xRegsBitFieldWriteCmdParams),
    Gh3xRegsListWriteCmd(Gh3xRegsListWriteCmdParams),
    Gh3xRegsReadCmd(Gh3xRegsReadCmdParams),
    Gh3xRegsWriteCmd(Gh3xRegsWriteCmdParams),
    Gh3xSwFunctionCmd(Gh3xSwFunctionCmdParams),
    GhSetWorkModeCmd(GhSetWorkModeCmdParams),
    DownloadConfig(DownloadConfigParams),
    GetChipLinkStatus(GetChipLinkStatusParams),
    GhLowPowerCmd(GhLowPowerCmdParams),
    GhTimeSet(GhTimeSetParams),
    GhTimestampSet(GhTimestampSetParams),
}
```

**方法：**

| 方法 | 返回值 | 描述 |
|------|--------|------|
| `key(&self) -> &'static str` | 命令键字符串 | 获取命令对应的键名 |
| `format(&self) -> &'static str` | 格式字符串 | 获取命令的参数格式 |

---

### 命令参数结构体

#### `Gh3xGetVersionParams`

获取版本命令参数。

```rust
pub struct Gh3xGetVersionParams {
    pub ver_type: u8,  // 版本类型
}
```

#### `Gh3xRegsReadCmdParams`

寄存器读取命令参数。

```rust
pub struct Gh3xRegsReadCmdParams {
    pub reg_addr: u16,   // 寄存器地址
    pub read_len: i32,   // 读取长度
}
```

#### `Gh3xRegsWriteCmdParams`

寄存器写入命令参数。

```rust
pub struct Gh3xRegsWriteCmdParams {
    pub regs: Vec<u16>,  // 寄存器数据列表
}
```

#### `FParams`

F命令参数。

```rust
pub struct FParams {
    pub buf: Vec<u8>,    // 数据缓冲区
    pub fifo_id: u32,    // FIFO ID
}
```

---

### `Response`

响应枚举，包含不同命令的响应类型。

```rust
pub enum Response {
    Gh3xGetVersion(Vec<u8>),       // 版本信息
    Gh3xRegsReadCmd(Vec<u16>),     // 寄存器读取结果
    Fw(Vec<u8>),                   // 固件数据
    GetChipLinkStatus(Vec<i8>),    // 芯片连接状态
    FGetMode(Vec<u16>),            // 模式信息
    Empty,                          // 空响应
}
```

---

## 命令键常量

| 常量名 | 值 | 描述 |
|--------|-----|------|
| `KEY_EVENT` | `"Event"` | 事件命令 |
| `KEY_F` | `"F"` | F命令 |
| `KEY_FW` | `"FW"` | 固件命令 |
| `KEY_F_GET_MODE` | `"F_GetMode"` | 获取模式命令 |
| `KEY_F_SET_MODE` | `"F_SetMode"` | 设置模式命令 |
| `KEY_G` | `"G"` | G协议命令 |
| `KEY_GH3X_CHIP_CTRL` | `"GH3X_ChipCtrl"` | 芯片控制命令 |
| `KEY_GH3X_GET_VERSION` | `"GH3X_GetVersion"` | 获取版本命令 |
| `KEY_GH3X_REGS_READ_CMD` | `"GH3X_RegsReadCmd"` | 寄存器读取命令 |
| `KEY_GH3X_REGS_WRITE_CMD` | `"GH3X_RegsWriteCmd"` | 寄存器写入命令 |
| `KEY_GH_SET_WORK_MODE_CMD` | `"GHSetWorkModeCmd"` | 设置工作模式命令 |
| `KEY_DOWNLOAD_CONFIG` | `"download_config"` | 下载配置命令 |
| `KEY_GET_CHIP_LINK_STATUS` | `"get_chip_link_status"` | 获取芯片连接状态命令 |
| `KEY_GH_LOW_POWER_CMD` | `"gh_low_power_cmd"` | 低功耗命令 |
| `KEY_GH_TIME_SET` | `"gh_time_set"` | 设置时间命令 |
| `KEY_GH_TIMESTAMP_SET` | `"gh_timestamp_set"` | 设置时间戳命令 |

---

## 格式字符串常量

格式字符串用于指定参数的编码格式：

| 格式 | 描述 |
|------|------|
| `<u8>` | 无符号8位整数 |
| `<u16>` | 无符号16位整数 |
| `<u32>` | 无符号32位整数 |
| `<u64>` | 无符号64位整数 |
| `<i8>` / `<d8>` | 有符号8位整数 |
| `<i16>` / `<d16>` | 有符号16位整数 |
| `<i32>` / `<d32>` | 有符号32位整数 |
| `<i64>` / `<d64>` | 有符号64位整数 |
| `<u8*>` | 无符号8位数组 |
| `<u16*>` | 无符号16位数组 |
| `<u32*>` | 无符号32位数组 |

---

## 帧解码器

### `FrameDecoder`

G协议帧解码器，用于解码G协议帧数据。

```rust
pub struct FrameDecoder { /* ... */ }
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | - | `Self` | 创建新的帧解码器 |
| `with_logger` | `logger: Arc<dyn LogCallback>` | `Self` | 设置日志回调 |
| `decode_frames` | `data: &[u8]` | `Result<Vec<GhFuncFrame>, DecodeError>` | 解码帧数据 |

**示例：**

```rust
use gh_rpc::FrameDecoder;

let decoder = FrameDecoder::new();
let raw_data: &[u8] = &[/* G协议数据 */];
let frames = decoder.decode_frames(raw_data)?;

for frame in frames {
    println!("帧ID: {}, 通道数: {}", frame.frame_cnt, frame.ch_num);
}
```

---

## 命令执行器

### `CommandExecutor`

命令执行器，封装 `RpcCore`，提供高级命令调用接口。

```rust
pub struct CommandExecutor { /* ... */ }
```

**方法：**

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `new` | `config: RpcConfig` | `Self` | 使用配置创建执行器 |
| `from_core` | `core: RpcCore` | `Self` | 从现有RpcCore创建执行器 |
| `with_logger` | `logger: Arc<dyn LogCallback>` | `Self` | 设置日志回调 |
| `get_core` | - | `&RpcCore` | 获取内部RpcCore引用 |
| `set_send_function` | `func: SendFunction` | `()` | 设置发送函数 |
| `register_frame_callback` | `callback: FrameCallback` | `()` | 注册帧数据回调 |
| `handle_frame_data` | `data: &[u8]` | `Result<Vec<GhFuncFrame>, DecodeError>` | 处理帧数据 |
| `register_g_handler` | - | `Result<(), RpcError>` | 注册G协议处理器 |
| `process` | `data: &[u8]` | `Vec<Result<ParseResult, RpcError>>` | 处理接收数据 |
| `call` | `key: &str, format: &str, params: &[u8]` | `Result<Vec<u8>, RpcError>` | 同步调用命令 |
| `send` | `key: &str, format: &str, params: &[u8]` | `Result<(), RpcError>` | 发送命令（等待ACK） |
| `publish` | `key: &str, format: &str, params: &[u8]` | `Result<(), RpcError>` | 发布命令（不等待响应） |
| `sall` | `key: &str, format: &str, params: &[u8]` | `Result<Vec<u8>, RpcError>` | 安全调用命令 |
| `register` | `key: &str, handler: RpcHandler` | `Result<(), RpcError>` | 注册命令处理器 |

---

### `FrameCallback`

帧数据回调类型定义。

```rust
pub type FrameCallback = Arc<dyn Fn(&GhFuncFrame) + Send + Sync>;
```

**示例：**

```rust
use std::sync::Arc;
use gh_rpc::{CommandExecutor, FrameCallback, GhFuncFrame, RpcConfig};

let config = RpcConfig::default();
let mut executor = CommandExecutor::new(config);

let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
    println!("收到帧: ID={}, 时间戳={}", frame.frame_cnt, frame.timestamp);
});

executor.register_frame_callback(callback);
```

---

## 数据解码器（来自rpc模块）

数据解码器功能由 `rpc` 模块提供，`gh-rpc` 重新导出以下类型和函数：

- `DataUnpacker` - 通用数据解码器
- `UnpackValue` - 解码值枚举
- `UnpackError` - 解码错误类型
- `unpack` - 通用解码函数
- `unpack_u8_array`, `unpack_u16_array`, `unpack_u32_array`, `unpack_u64_array` - 数组解码函数
- `unpack_i8_array`, `unpack_i16_array`, `unpack_i32_array`, `unpack_i64_array` - 有符号数组解码函数
- `unpack_string` - 字符串解码函数

详细API文档请参考 [rpc模块API文档 - 数据解码器](rpc-api.md#数据解码器)。

**示例：**

```rust
use gh_rpc::{unpack, unpack_u16_array, unpack_string, UnpackValue};

let data: &[u8] = &[0x60, 0x34, 0x12];
let value = unpack(data, "<u16>")?;
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

## Varint 解码

### `varint_decode`

Varint 解码函数，用于解码变长整数。

```rust
pub fn varint_decode(buffer: &[u8], pos: &mut usize) -> Result<u32, DecodeError>
```

**参数：**
- `buffer`: 输入数据缓冲区
- `pos`: 当前位置指针（会被更新）

**返回值：**
- `Ok(u32)`: 解码后的值
- `Err(DecodeError)`: 解码错误

---

### `zigzag_decode`

Zigzag 解码函数，用于将有符号整数从无符号表示还原。

```rust
pub fn zigzag_decode(x: u32) -> i32
```

**示例：**

```rust
use gh_rpc::{varint_decode, zigzag_decode};

let data = [0x5D];
let mut pos = 0;
let value = varint_decode(&data, &mut pos)?;
let signed_value = zigzag_decode(value);
println!("解码值: {}", signed_value);
```

---

## 完整使用示例

### 示例1：解码G协议帧

```rust
use gh_rpc::{FrameDecoder, GhFuncFrame};

fn decode_g_protocol(data: &[u8]) -> Result<Vec<GhFuncFrame>, gh_rpc::DecodeError> {
    let decoder = FrameDecoder::new();
    decoder.decode_frames(data)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_data: &[u8] = &[/* G协议帧数据 */];
    let frames = decode_g_protocol(raw_data)?;
    
    for frame in frames {
        println!("帧ID: {}, 时间戳: {}ns", frame.frame_cnt, frame.timestamp);
        println!("功能类型: {:?}", frame.id);
        println!("通道数: {}", frame.ch_num);
        
        for (i, ch_data) in frame.data.iter().enumerate() {
            println!("  通道{}: IPD={}, 原始数据={}", 
                i, ch_data.ipd_pa, ch_data.rawdata);
        }
    }
    
    Ok(())
}
```

### 示例2：执行命令

```rust
use gh_rpc::{CommandExecutor, RpcConfig, KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig::default();
    let executor = CommandExecutor::new(config);
    
    let send_fn = Arc::new(|data: &[u8]| -> Result<(), rpc::RpcError> {
        println!("发送数据: {:02X?}", data);
        Ok(())
    });
    executor.set_send_function(send_fn).await;
    
    let ver_type: u8 = 0;
    let result = executor.call(
        KEY_GH3X_GET_VERSION,
        FMT_GH3X_GET_VERSION,
        &[ver_type]
    ).await?;
    
    println!("版本信息: {:02X?}", result);
    
    Ok(())
}
```

### 示例3：注册G协议处理器

```rust
use gh_rpc::{CommandExecutor, RpcConfig, FrameCallback, GhFuncFrame};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = RpcConfig::default();
    let mut executor = CommandExecutor::new(config);
    
    let callback: FrameCallback = Arc::new(|frame: &GhFuncFrame| {
        println!("收到G协议帧: ID={}", frame.frame_cnt);
    });
    executor.register_frame_callback(callback);
    
    executor.register_g_handler().await?;
    
    Ok(())
}
```

---

## 版本信息

- **版本**: 0.1.0
- **Rust版本**: 2021 Edition
- **依赖**:
  - `rpc` (本地路径依赖)
  - `tokio` (异步运行时)
  - `log` (日志门面)
  - `serde` (序列化支持)
