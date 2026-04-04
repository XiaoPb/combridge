# ComBridge 设备管理框架

## 一、概述

ComBridge 设备管理框架采用分层架构设计，实现了串口通信和蓝牙低功耗（BLE）设备的统一管理。框架核心特性：

- **统一状态管理**：后端唯一数据源，支持页面刷新状态恢复
- **双模式 BLE**：支持原生蓝牙和 AT 指令两种 BLE 实现方式
- **环形缓冲区**：每个通道独立收发缓冲区，支持历史数据追溯
- **状态持久化**：自动保存设备连接状态和窗口布局

---

## 二、架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端 (React + TypeScript)                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ useAppState │  │useAppDispatch│  │ useSerial / useBle     │  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┬───────────┘  │
│         │                │                       │               │
│  ┌──────▼────────────────▼───────────────────────▼───────────┐  │
│  │                    API Layer (stateApi.ts)                 │  │
│  └─────────────────────────────┬─────────────────────────────┘  │
└────────────────────────────────┼────────────────────────────────┘
                                 │ Tauri IPC
┌────────────────────────────────┼────────────────────────────────┐
│                        Rust 后端                                 │
│  ┌─────────────────────────────▼─────────────────────────────┐  │
│  │                 State Management (state/)                  │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────────┐  │  │
│  │  │AppState  │ │ Action   │ │Dispatcher│ │Persistence   │  │  │
│  │  └────┬─────┘ └────┬─────┘ └────┬─────┘ └──────────────┘  │  │
│  └───────┼────────────┼────────────┼──────────────────────────┘  │
│          │            │            │                             │
│  ┌───────▼────────────▼────────────▼──────────────────────────┐  │
│  │                  Device Management (device/)               │  │
│  │  ┌─────────────────────┐  ┌─────────────────────────────┐  │  │
│  │  │   SerialManager     │  │       BleManager            │  │  │
│  │  │  ┌───────────────┐  │  │  ┌───────────┐ ┌──────────┐ │  │  │
│  │  │  │ SerialPort[]  │  │  │  │NativeBackend│AtBackend │ │  │  │
│  │  │  └───────────────┘  │  │  └───────────┘ └──────────┘ │  │  │
│  │  └─────────────────────┘  └─────────────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                    Ring Buffer (cache.rs)                  │  │
│  │              每通道独立 TX/RX 缓冲区 (4MB)                   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 三、核心模块详解

### 3.1 状态管理模块 (`state/`)

#### 3.1.1 模块结构

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块入口，统一导出 |
| `types.rs` | 核心类型定义：ChannelType、DeviceChannel、AppState 等 |
| `action.rs` | Action 枚举定义，支持所有状态变更操作 |
| `app_state.rs` | 全局状态结构，包含通道管理、缓冲区操作方法 |
| `dispatcher.rs` | Action 处理器，路由到具体设备管理器 |
| `persistence.rs` | 状态持久化服务，JSON 格式存储到磁盘 |

#### 3.1.2 核心数据结构

```rust
// 全局状态
pub struct AppState {
    pub channels: Vec<DeviceChannel>,      // 所有设备通道
    pub active_channel_id: Option<String>, // 当前活动通道
    pub settings: AppSettings,             // 应用设置
    pub window_state: WindowState,         // 窗口状态（TAB 信息）
}

// 设备通道
pub struct DeviceChannel {
    pub id: String,                        // 唯一标识
    pub name: String,                      // 显示名称
    pub channel_type: ChannelType,         // Serial / BluetoothCharacteristic
    pub connected: bool,                   // 连接状态
    pub tx_buffer: ChannelBuffer,          // 发送缓冲区
    pub rx_buffer: ChannelBuffer,          // 接收缓冲区
    pub config: Option<ChannelConfig>,     // 配置信息
    pub bytes_sent: u64,                   // 已发送字节数
    pub bytes_received: u64,               // 已接收字节数
}

// 缓冲区条目
pub struct BufferEntry {
    pub timestamp: u64,                    // 时间戳
    pub data: Vec<u8>,                     // 数据
    pub direction: String,                 // "send" / "receive"
}
```

#### 3.1.3 Action 类型

```rust
pub enum Action {
    // 通道管理
    ChannelAdd { name, channel_type, config },
    ChannelRemove { id },
    ChannelConnect { id, config },
    ChannelDisconnect { id },
    ChannelSwitch { channel_id },
    
    // 数据操作
    DataSend { channel_id, data },
    BufferClear { channel_id, direction },
    
    // TAB 管理
    TabAdd { channel_id, label },
    TabRemove { tab_key },
    TabSwitch { tab_key },
    
    // 设置
    SettingsUpdate { settings },
    StateRestore { window_state },
}
```

---

### 3.2 串口管理模块 (`device/serial/`)

#### 3.2.1 模块结构

| 文件 | 职责 |
|------|------|
| `serial_manager.rs` | 串口管理器，管理多个串口连接 |
| `serial_port.rs` | 单个串口封装，读写循环 |
| `serial_config.rs` | 配置定义：波特率、数据位、校验位等 |

#### 3.2.2 SerialManager 架构

```
┌─────────────────────────────────────────────────────────────┐
│                      SerialManager                          │
│  ┌─────────────────┐  ┌─────────────────┐  ┌────────────┐  │
│  │ ports: HashMap  │  │callbacks:HashMap│  │caches:HashMap│ │
│  │  <String, Port> │  │ <String, CB>    │  │ <String,Cache>│ │
│  └────────┬────────┘  └────────┬────────┘  └─────┬──────┘  │
│           │                    │                  │         │
│           ▼                    ▼                  ▼         │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    SerialPort                        │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌───────────┐  │   │
│  │  │ Config       │  │ Read Loop    │  │ Write API │  │   │
│  │  │ - baud_rate  │  │ (async)      │  │           │  │   │
│  │  │ - data_bits  │  │              │  │           │  │   │
│  │  │ - parity     │  │              │  │           │  │   │
│  │  └──────────────┘  └──────────────┘  └───────────┘  │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.3 核心方法

```rust
impl SerialManager {
    // 扫描可用串口
    pub fn scan_ports(&self) -> Result<Vec<PortInfo>>;
    
    // 打开串口（带数据回调）
    pub fn open_port<F>(&self, config: SerialPortConfig, callback: F) -> Result<()>
    where F: Fn(&str, &[u8]) + Send + Sync + 'static;
    
    // 关闭串口
    pub fn close_port(&self, port_name: &str) -> Result<()>;
    
    // 发送数据
    pub fn send_data(&self, port_name: &str, data: &[u8]) -> Result<usize>;
    
    // 获取缓冲区数据
    pub fn get_cache(&self, port_name: &str) -> Option<ChannelCache>;
    
    // 清除缓冲区
    pub fn clear_cache(&self, port_name: &str) -> bool;
}
```

#### 3.2.4 数据流

```
发送路径:
前端 → dispatch_action(DATA_SEND) → ActionDispatcher → SerialManager.send_data() → SerialPort.write() → 硬件

接收路径:
硬件 → SerialPort.read_loop() → RingBuffer.write() → callback() → Tauri emit → 前端
```

---

### 3.3 BLE 管理模块 (`device/ble/`)

#### 3.3.1 模块结构

```
device/ble/
├── mod.rs              # 模块导出
├── ble_manager.rs      # BLE 管理器，统一接口
├── ble_traits.rs       # BleBackend trait 定义
├── native/             # 原生 BLE 后端
│   ├── mod.rs
│   └── native_backend.rs
└── at/                 # AT 指令 BLE 后端
    ├── mod.rs
    ├── at_backend.rs
    ├── at_commands.rs
    ├── at_parser.rs
    └── at_transport.rs
```

#### 3.3.2 双模式架构

```
                    ┌─────────────────┐
                    │   BleManager    │
                    │  (统一接口层)    │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ BleBackend│  │ BleBackend│  │ (可扩展) │
        │  Trait   │  │  Trait   │  │          │
        └────┬─────┘  └────┬─────┘  └──────────┘
             │              │
     ┌───────▼──────┐ ┌────▼────────┐
     │NativeBackend │ │ AtBackend   │
     │              │ │             │
     │ - bluest     │ │ - AT指令    │
     │ - 系统蓝牙API│ │ - 串口传输  │
     └───────┬──────┘ └────┬────────┘
             │              │
             ▼              ▼
     ┌──────────────┐ ┌──────────────┐
     │ OS Bluetooth │ │ Serial Port  │
     │    Stack     │ │ + BLE Module │
     └──────────────┘ └──────────────┘
```

#### 3.3.3 BleBackend Trait

```rust
#[async_trait]
pub trait BleBackend: Send + Sync {
    // 配置
    async fn configure(&mut self) -> Result<()>;
    
    // 扫描
    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>>;
    async fn stop_scan(&self) -> Result<Vec<BleDevice>>;
    
    // 连接
    async fn connect(&self, address: &str) -> Result<BleConnection>;
    async fn disconnect(&self, address: &str) -> Result<()>;
    async fn get_connections(&self) -> Result<Vec<BleConnection>>;
    
    // GATT
    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>>;
    async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>>;
    async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>>;
    async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()>;
    async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()>;
    
    // 通知
    async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()>;
    async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()>;
    
    // 其他
    async fn get_rssi(&self, address: &str) -> Result<i16>;
    async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16>;
}
```

#### 3.3.4 模式切换

```rust
// 配置原生模式
ble_manager.configure_native().await?;

// 配置 AT 模式
ble_manager.configure_at(AtConfig {
    port_name: "COM3".to_string(),
    baud_rate: 115200,
    timeout_ms: 1000,
}).await?;
```

---

### 3.4 环形缓冲区 (`device/cache.rs`)

#### 3.4.1 设计目标

- 固定容量（默认 4MB），自动覆盖旧数据
- 线程安全，支持并发读写
- 记录时间戳，支持历史查询

#### 3.4.2 数据结构

```rust
pub struct RingBuffer {
    buffer: Vec<u8>,           // 底层存储
    capacity: usize,           // 容量
    head: usize,               // 读指针
    tail: usize,               // 写指针
    entries: Vec<CacheEntry>,  // 按次记录的条目
}

pub struct CacheEntry {
    pub timestamp: u64,        // 时间戳
    pub data: Vec<u8>,         // 数据
}

// 线程安全封装
pub struct ThreadSafeRingBuffer {
    inner: Mutex<RingBuffer>,
}
```

#### 3.4.3 核心操作

```rust
impl RingBuffer {
    // 写入数据（自动覆盖旧数据）
    pub fn write(&mut self, data: &[u8]);
    
    // 读取所有数据
    pub fn read_all(&self) -> Vec<u8>;
    
    // 获取条目列表
    pub fn get_entries(&self) -> &[CacheEntry];
    
    // 获取缓存数据
    pub fn get_cache_data(&self) -> CacheData;
    
    // 清空
    pub fn clear(&mut self);
    
    // 当前长度
    pub fn len(&self) -> usize;
}
```

---

## 四、前端集成

### 4.1 状态订阅

```typescript
// 获取全局状态
const state = useAppState();

// 获取活动通道
const activeChannel = useActiveChannel();

// 获取指定通道
const channel = useChannel(channelId);

// 获取已连接通道
const connectedChannels = useConnectedChannels();
```

### 4.2 Action 发送

```typescript
const { addChannel, connectChannel, disconnectChannel, sendData, clearBuffer } = useChannelActions();

// 添加串口通道
await addChannel('COM3', 'serial', { baudRate: 115200 });

// 连接通道
await connectChannel('serial-COM3', { baudRate: 115200 });

// 发送数据
await sendData('serial-COM3', [0x01, 0x02, 0x03]);

// 清空缓冲区
await clearBuffer('serial-COM3', 'rx');
```

### 4.3 状态同步流程

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Frontend   │     │   Tauri IPC  │     │   Backend    │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       │  dispatch_action   │                    │
       │───────────────────>│                    │
       │                    │  ActionDispatcher  │
       │                    │───────────────────>│
       │                    │                    │
       │                    │    state-change    │
       │<───────────────────│<───────────────────│
       │                    │                    │
       │  React re-render   │                    │
       │                    │                    │
```

---

## 五、状态持久化

### 5.1 存储位置

- Windows: `%LOCALAPPDATA%/combridge/app_state.json`
- macOS: `~/Library/Application Support/combridge/app_state.json`
- Linux: `~/.local/share/combridge/app_state.json`

### 5.2 持久化内容

```json
{
  "channels": [
    {
      "id": "serial-COM3",
      "name": "COM3",
      "type": "serial",
      "connected": false,
      "config": {
        "type": "serial",
        "baudRate": 115200,
        "dataBits": 8,
        "parity": "none",
        "stopBits": 1,
        "flowControl": "none"
      }
    }
  ],
  "activeChannelId": "serial-COM3",
  "settings": {
    "theme": "dark",
    "language": "zh-CN",
    "autoReconnect": true,
    "maxBufferSize": 4194304
  },
  "windowState": {
    "tabs": [
      {
        "key": "tab-serial-COM3-1234567890",
        "channelId": "serial-COM3",
        "label": "COM3",
        "isActive": true
      }
    ],
    "activeTabKey": "tab-serial-COM3-1234567890"
  }
}
```

### 5.3 恢复流程

```
应用启动
    │
    ▼
检查持久化文件是否存在
    │
    ├── 存在 ──> 读取 JSON ──> 解析 AppState ──> 恢复窗口 TAB
    │                                      │
    │                                      └──> 恢复设备配置（不自动连接）
    │
    └── 不存在 ──> 使用默认状态
```

---

## 六、最佳实践

### 6.1 设备连接

```typescript
// 推荐：通过 Action 系统连接
const { addChannel, connectChannel } = useChannelActions();

// 1. 先添加通道
const result = await addChannel('COM3', 'serial', config);
const channelId = result.data?.channelId;

// 2. 再连接
await connectChannel(channelId, config);
```

### 6.2 数据发送

```typescript
// 推荐：使用 Uint8Array
const data = new Uint8Array([0x01, 0x02, 0x03]);
await sendData(channelId, Array.from(data));

// 或使用文本转换
const text = "Hello";
const encoder = new TextEncoder();
await sendData(channelId, Array.from(encoder.encode(text)));
```

### 6.3 错误处理

```typescript
const result = await connectChannel(channelId, config);

if (!result.success) {
  console.error('连接失败:', result.message);
  // 处理错误
  return;
}

// 连接成功
console.log('连接成功');
```

---

## 七、扩展指南

### 7.1 添加新的设备类型

1. 在 `types.rs` 中添加新的 `ChannelType` 变体
2. 在 `Action` 枚举中添加对应的处理逻辑
3. 在 `dispatcher.rs` 中实现处理函数
4. 创建新的设备管理器（参考 `SerialManager`）

### 7.2 自定义缓冲区大小

```rust
// 在 AppSettings 中修改
pub struct AppSettings {
    pub max_buffer_size: usize,  // 默认 4MB
}
```

### 7.3 添加新的 Action

1. 在 `action.rs` 中添加 Action 变体
2. 在 `dispatcher.rs` 的 `dispatch` 方法中添加处理分支
3. 在前端 `types/state.ts` 中添加对应类型
4. 在 `useAppDispatch.ts` 中添加便捷方法
