# ComBridge 设备管理框架

## 一、概述

ComBridge 设备管理框架采用分层架构设计，实现了串口通信和蓝牙低功耗（BLE）设备的统一管理。框架核心特性：

- **统一设备管理**：DeviceManager 统一管理串口和 BLE 设备
- **双模式 BLE**：支持原生蓝牙和 AT 指令两种 BLE 实现方式
- **环形缓冲区**：每个通道独立收发缓冲区，支持历史数据追溯
- **事件总线**：基于 EventBus 的发布/订阅模式，支持跨模块通信

---

## 二、架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端 (React + TypeScript)                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │
│  │ useDeviceStore│ │useDashboardStore│ │ useSerial / useBle     │  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────┬───────────┘  │
│         │                │                       │               │
│  ┌──────▼────────────────▼───────────────────────▼───────────┐  │
│  │                    API Layer (deviceApi.ts)                │  │
│  └─────────────────────────────┬─────────────────────────────┘  │
└────────────────────────────────┼────────────────────────────────┘
                                 │ Tauri IPC
┌────────────────────────────────┼────────────────────────────────┐
│                        Rust 后端                                 │
│  ┌─────────────────────────────▼─────────────────────────────┐  │
│  │                 DeviceManager (device_manager.rs)          │  │
│  │  ┌──────────────────────┐  ┌────────────────────────────┐  │  │
│  │  │   SerialManager      │  │       BleManager           │  │  │
│  │  │  ┌────────────────┐  │  │  ┌──────────┐ ┌─────────┐  │  │  │
│  │  │  │ SerialPort[]   │  │  │  │NativeMode│ │ AT Mode │  │  │  │
│  │  │  └────────────────┘  │  │  └──────────┘ └─────────┘  │  │  │
│  │  └──────────────────────┘  └────────────────────────────┘  │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                    EventBus (event_bus.rs)                 │  │
│  │              发布/订阅模式，跨模块通信                        │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────▼───────────────────────────────┐  │
│  │                    Ring Buffer (cache.rs)                  │  │
│  │              每通道独立 TX/RX 缓冲区 (4MB)                   │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 三、核心模块详解

### 3.1 设备管理模块 (`device/`)

#### 3.1.1 模块结构

| 文件 | 职责 |
|------|------|
| `mod.rs` | 模块入口，统一导出 |
| `device_manager.rs` | 设备管理器，统一管理串口和 BLE 设备 |
| `cache.rs` | 环形缓冲区实现，支持数据缓存和历史追溯 |
| `serial/mod.rs` | 串口模块入口 |
| `serial/serial_manager.rs` | 串口管理器，管理多个串口连接 |
| `serial/serial_port.rs` | 单个串口封装，读写循环 |
| `serial/serial_config.rs` | 配置定义：波特率、数据位、校验位等 |
| `ble/mod.rs` | BLE 模块入口 |
| `ble/ble_manager.rs` | BLE 管理器，统一接口层 |
| `ble/ble_traits.rs` | BleBackend trait 定义 |
| `ble/native/` | 原生 BLE 后端实现 |
| `ble/at/` | AT 指令 BLE 后端实现 |

#### 3.1.2 核心数据结构

```rust
pub enum DeviceType {
    Serial,
    Ble,
}

pub struct DeviceManager {
    pub serial_manager: SerialManagerRef,
    pub ble_manager: BleManagerRef,
}

impl DeviceManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self;
    pub async fn send_direct(&self, device_type: DeviceType, device_name: &str, char_uuid: Option<&str>, data: &[u8]) -> Result<()>;
    pub async fn open_serial(&self, config: SerialPortConfig) -> Result<()>;
    pub async fn close_serial(&self, port_name: &str) -> Result<()>;
    pub async fn configure_ble_at(&self, config: AtConfig) -> Result<()>;
    pub async fn configure_ble_native(&self) -> Result<()>;
    pub async fn connect_ble(&self, address: &str) -> Result<BleConnection>;
    pub async fn disconnect_ble(&self, address: &str) -> Result<()>;
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
│  │ ports: RwLock   │  │callbacks:RwLock │  │caches:RwLock│ │
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
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              SerialPortCache                         │   │
│  │  ┌──────────────┐  ┌──────────────┐                 │   │
│  │  │ tx_buffer    │  │ rx_buffer    │                 │   │
│  │  │ (RingBuffer) │  │ (RingBuffer) │                 │   │
│  │  └──────────────┘  └──────────────┘                 │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

#### 3.2.3 核心方法

```rust
impl SerialManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self;
    pub fn scan_ports(&self) -> Result<Vec<PortInfo>>;
    pub fn open_port<F>(&self, config: SerialPortConfig, callback: F) -> Result<()>
    where F: Fn(&str, &[u8]) + Send + Sync + 'static;
    pub fn close_port(&self, port_name: &str) -> Result<()>;
    pub fn close_all_ports(&self) -> Result<()>;
    pub fn send_data(&self, port_name: &str, data: &[u8]) -> Result<usize>;
    pub fn is_port_open(&self, port_name: &str) -> Result<bool>;
    pub fn get_open_ports(&self) -> Result<Vec<String>>;
    pub fn register_callback<F>(&self, port_name: &str, callback: F) -> Result<()>;
    pub fn unregister_callback(&self, port_name: &str) -> Result<()>;
    pub fn clear_callbacks(&self) -> Result<()>;
    pub fn get_port_config(&self, port_name: &str) -> Result<SerialPortConfig>;
    pub fn get_cache(&self, port_name: &str) -> Result<ChannelCache>;
    pub fn clear_cache(&self, port_name: &str) -> Result<bool>;
    pub fn get_cache_size(&self, port_name: &str) -> Result<Option<(usize, usize)>>;
}
```

#### 3.2.4 数据流

```
发送路径:
前端 → Tauri IPC → SerialManager.send_data() → SerialPort.write() → 硬件
                      │
                      └──> tx_buffer.write(data)  // 同时写入发送缓存

接收路径:
硬件 → SerialPort.read_loop() → rx_buffer.write(data) → callback() 
      │
      └──> EventBus.publish(SerialDataEvent) → 前端
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
│   ├── adapter.rs
│   ├── gatt_client.rs
│   └── native_backend.rs
└── at/                 # AT 指令 BLE 后端
    ├── mod.rs
    ├── at_backend.rs
    ├── at_cache.rs
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
pub type NotifyCallback = Arc<dyn Fn(&str, &str, &[u8]) + Send + Sync>;

#[async_trait]
pub trait BleBackend: Send + Sync {
    async fn configure(&mut self) -> Result<()>;
    async fn scan(&self, duration_ms: u64) -> Result<Vec<BleDevice>>;
    async fn stop_scan(&self) -> Result<Vec<BleDevice>>;
    async fn connect(&self, address: &str) -> Result<BleConnection>;
    async fn disconnect(&self, address: &str) -> Result<()>;
    async fn get_connections(&self) -> Result<Vec<BleConnection>>;
    async fn discover_services(&self, address: &str) -> Result<Vec<BleService>>;
    async fn discover_characteristics(&self, address: &str, service_uuid: &str) -> Result<Vec<BleCharacteristic>>;
    async fn read_characteristic(&self, address: &str, char_uuid: &str) -> Result<Vec<u8>>;
    async fn write_characteristic(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()>;
    async fn write_without_response(&self, address: &str, char_uuid: &str, data: &[u8]) -> Result<()>;
    async fn subscribe_notify(&self, address: &str, char_uuid: &str, callback: NotifyCallback) -> Result<()>;
    async fn unsubscribe_notify(&self, address: &str, char_uuid: &str) -> Result<()>;
    async fn get_rssi(&self, address: &str) -> Result<i16>;
    async fn set_mtu(&self, address: &str, mtu: u16) -> Result<u16>;
}
```

#### 3.3.4 核心数据结构

```rust
pub enum BleMode {
    Native,
    At,
}

pub struct AtConfig {
    pub port_name: String,
    pub baud_rate: u32,
    pub timeout_ms: u64,
    pub tx_uuid: Option<String>,
    pub rx_uuid: Option<String>,
    pub srv_uuid: Option<String>,
}

pub struct BleDevice {
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
    pub is_connectable: bool,
}

pub struct BleConnection {
    pub address: String,
    pub name: Option<String>,
    pub is_connected: bool,
    pub services: Vec<BleService>,
}

pub struct BleService {
    pub uuid: String,
    pub primary: bool,
    pub characteristics: Vec<BleCharacteristic>,
}

pub struct BleCharacteristic {
    pub uuid: String,
    pub service_uuid: String,
    pub properties: BleCharacteristicProperties,
    pub subscribed: bool,
}

pub struct BleCharacteristicProperties {
    pub read: bool,
    pub write: bool,
    pub write_without_response: bool,
    pub notify: bool,
    pub indicate: bool,
}

pub struct AtConnectionTab {
    pub id: String,
    pub address: String,
    pub name: Option<String>,
    pub tx_uuid: String,
    pub rx_uuid: String,
    pub connected_at: u64,
    pub received_data: Vec<DataEntry>,
    pub sent_data: Vec<DataEntry>,
}

pub struct DataEntry {
    pub id: String,
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub direction: String,
}
```

#### 3.3.5 模式切换

```rust
// 配置原生模式
ble_manager.configure_native().await?;

// 配置 AT 模式
ble_manager.configure_at(AtConfig {
    port_name: "COM3".to_string(),
    baud_rate: 115200,
    timeout_ms: 1000,
    tx_uuid: Some("...".to_string()),
    rx_uuid: Some("...".to_string()),
    srv_uuid: Some("...".to_string()),
}).await?;

// 获取当前模式
let mode = ble_manager.mode().await;

// 切换模式
ble_manager.set_mode(BleMode::At).await?;
```

---

### 3.4 环形缓冲区 (`device/cache.rs`)

#### 3.4.1 设计目标

- 固定容量（默认 4MB），自动覆盖旧数据
- 线程安全，支持并发读写
- 记录时间戳，支持历史查询

#### 3.4.2 数据结构

```rust
pub struct CacheEntry {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

pub struct CacheData {
    pub entries: Vec<CacheEntry>,
    pub total_bytes: usize,
    pub entry_count: usize,
}

pub struct RingBuffer {
    buffer: Vec<u8>,
    capacity: usize,
    head: usize,
    tail: usize,
    entries: VecDeque<CacheEntry>,
}

pub struct ChannelCache {
    pub tx_cache: CacheData,
    pub rx_cache: CacheData,
}

pub struct ThreadSafeRingBuffer {
    inner: Mutex<RingBuffer>,
}

pub type RingBufferRef = Arc<ThreadSafeRingBuffer>;
```

#### 3.4.3 核心操作

```rust
impl RingBuffer {
    pub fn new() -> Self;
    pub fn with_capacity(capacity: usize) -> Self;
    pub fn write(&mut self, data: &[u8]);
    pub fn read_all(&self) -> Vec<u8>;
    pub fn get_entries(&self) -> Vec<&CacheEntry>;
    pub fn get_cache_data(&self) -> CacheData;
    pub fn clear(&mut self);
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;
    pub fn capacity(&self) -> usize;
}

impl ThreadSafeRingBuffer {
    pub fn new() -> Self;
    pub fn with_capacity(capacity: usize) -> Self;
    pub fn write(&self, data: &[u8]) -> Result<()>;
    pub fn read_all(&self) -> Result<Vec<u8>>;
    pub fn get_cache_data(&self) -> Result<CacheData>;
    pub fn clear(&self) -> Result<()>;
    pub fn len(&self) -> Result<usize>;
    pub fn is_empty(&self) -> Result<bool>;
}

pub fn create_ring_buffer() -> RingBufferRef;
pub fn create_ring_buffer_with_capacity(capacity: usize) -> RingBufferRef;
```

---

### 3.5 事件总线 (`service/event_bus.rs`)

#### 3.5.1 设计目标

- 发布/订阅模式，支持跨模块通信
- 支持 msgpack 序列化，高效传输
- 类型安全的事件定义

#### 3.5.2 事件类型

```rust
pub struct SerialDataEvent {
    pub device_id: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

pub struct SerialConnectedEvent {
    pub port_name: String,
}

pub struct SerialDisconnectedEvent {
    pub port_name: String,
}

pub struct BleDataEvent {
    pub device_id: String,
    pub address: String,
    pub characteristic_uuid: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

pub struct BleConnectionEvent {
    pub address: String,
    pub name: Option<String>,
}
```

#### 3.5.3 事件主题

```rust
pub mod topics {
    pub const SERIAL_DATA: &str = "serial-data";
    pub const SERIAL_CONNECTED: &str = "serial-connected";
    pub const SERIAL_DISCONNECTED: &str = "serial-disconnected";
    pub const BLE_DATA: &str = "ble-data";
    pub const BLE_CONNECTED: &str = "ble-connected";
    pub const BLE_DISCONNECTED: &str = "ble-disconnected";
}
```

---

## 四、前端集成

### 4.1 状态管理

前端使用 Zustand 进行状态管理，主要 Store 包括：

- `useDeviceStore` - 设备状态管理
- `useDashboardStore` - Dashboard 状态管理
- `useLogStore` - 日志状态管理

### 4.2 API 调用

```typescript
import { deviceApi } from '../../api/device';

// 扫描串口
const ports = await deviceApi.scanPorts();

// 打开串口
await deviceApi.openSerialPort({
  portName: 'COM3',
  baudRate: 115200,
  dataBits: 8,
  parity: 'none',
  stopBits: 1,
  flowControl: 'none',
});

// 发送数据
await deviceApi.sendSerialData('COM3', [0x01, 0x02, 0x03]);

// 关闭串口
await deviceApi.closeSerialPort('COM3');
```

### 4.3 事件监听

```typescript
import { onSerialData, onBleData } from '../../api/events';

// 监听串口数据
const unlisten = await onSerialData((event) => {
  console.log('Received:', event.data);
});

// 取消监听
unlisten();
```

---

## 五、最佳实践

### 5.1 设备连接

```typescript
// 推荐：通过 API 层连接
const { openSerialPort, sendSerialData, closeSerialPort } = useDeviceActions();

// 1. 打开串口
await openSerialPort(config);

// 2. 发送数据
await sendSerialData(portName, data);

// 3. 关闭串口
await closeSerialPort(portName);
```

### 5.2 数据发送

```typescript
// 推荐：使用 Uint8Array
const data = new Uint8Array([0x01, 0x02, 0x03]);
await sendSerialData(portName, Array.from(data));

// 或使用文本转换
const text = "Hello";
const encoder = new TextEncoder();
await sendSerialData(portName, Array.from(encoder.encode(text)));
```

### 5.3 错误处理

```typescript
try {
  await openSerialPort(config);
  console.log('连接成功');
} catch (error) {
  console.error('连接失败:', error);
  // 处理错误
}
```

---

## 六、扩展指南

### 6.1 添加新的设备类型

1. 在 `device_manager.rs` 中添加新的 `DeviceType` 变体
2. 创建新的设备管理器（参考 `SerialManager`）
3. 在 `DeviceManager` 中添加对应的管理器引用
4. 实现必要的 Tauri 命令

### 6.2 自定义缓冲区大小

```rust
// 创建自定义容量的缓冲区
let buffer = create_ring_buffer_with_capacity(8 * 1024 * 1024); // 8MB
```

### 6.3 添加新的事件类型

1. 在 `service/event_bus.rs` 中定义新的事件结构
2. 在 `topics` 模块中添加事件主题常量
3. 在对应的 Manager 中发布事件
4. 在前端添加事件监听器
