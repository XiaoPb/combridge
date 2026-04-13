# ComBridge 架构优化设计方案

> 版本：v1.0
> 日期：2026-04-13
> 涉及任务：Task 27 ~ Task 31

---

## 目录

- [Task 27：统一状态管理设计](#task-27统一状态管理设计)
- [Task 28：错误处理优化](#task-28错误处理优化)
- [Task 29：性能优化](#task-29性能优化)
- [Task 30：EventBus 集成决策](#task-30eventbus-集成决策)
- [Task 31：前端架构优化](#task-31前端架构优化)

---

## Task 27：统一状态管理设计

### 27.1 现状分析

当前后端存在两套并行的设备状态管理：

| 组件 | 文件 | 职责 | 设备存储 |
|------|------|------|----------|
| `DeviceManager` | `device/device_manager.rs` | 底层设备操作（open/close/read/write） | `devices: Arc<RwLock<HashMap<String, DeviceInfo>>>` |
| `AppState` | `state/app_state.rs` | UI 状态（设备列表、活跃设备、Tab） | `devices: HashMap<String, Device>` |

**核心问题：状态双源（Dual Source of Truth）**

1. `DeviceManager.devices` 存储 `DeviceInfo`（id, name, device_type, is_connected, bytes_received, bytes_sent, metadata）
2. `AppState.devices` 存储 `Device`（Serial/ Ble 的枚举，包含 channels、config 等详细信息）
3. 两者均维护连接状态，但字段名不同（`DeviceInfo.is_connected` vs `Device.connected`）
4. `ActionDispatcher` 在处理连接/断开时需要同时更新两处状态

前端同样存在状态重叠：

| Store | 文件 | 职责 |
|-------|------|------|
| `connectionStore` | `stores/connectionStore.ts` | 通用连接管理（connections, activeConnectionId） |
| `serialStore` | `stores/serialStore.ts` | 串口专属状态（tabs, receivedData, sentData, preferences） |
| `bleStore` | `stores/bleStore.ts` | BLE 专属状态（devices, connections, services, notifications） |

`connectionStore` 的 `ConnectionInfo` 与 `serialStore`/`bleStore` 中的连接信息高度重叠。

### 27.2 设计方案

#### 27.2.1 DeviceManager 与 AppState 职责分离

**原则：DeviceManager 管硬件，AppState 管界面**

```
┌─────────────────────────────────────────────────────┐
│                   ActionDispatcher                    │
│              （协调层，唯一修改入口）                    │
├──────────────────┬──────────────────────────────────┤
│   DeviceManager  │           AppState               │
│   (硬件操作层)    │         (UI 状态层)               │
├──────────────────┼──────────────────────────────────┤
│ - open_serial()  │ - devices: HashMap<String, Device>│
│ - close_serial() │ - active_device_id                │
│ - connect_ble()  │ - settings                        │
│ - disconnect_ble │ - window_state                    │
│ - send_data()    │ - channels + buffers              │
│ - read_data()    │ - tabs                            │
│                  │                                    │
│ ❌ 移除:          │ ❌ 移除:                           │
│   devices map    │                                    │
│   routes         │                                    │
│   callbacks      │                                    │
└──────────────────┴──────────────────────────────────┘
```

**具体变更：**

1. **DeviceManager 移除 `devices` 字段**：不再维护 `HashMap<String, DeviceInfo>`，设备注册/注销操作转交 AppState
2. **DeviceManager 移除 `routes` 和 `callbacks`**：数据路由和回调机制由 AppState + ActionDispatcher 统一管理
3. **DeviceManager 精简为纯 I/O 层**：仅保留 `open_serial`, `close_serial`, `connect_ble`, `disconnect_ble`, `send_data` 等方法
4. **AppState 成为唯一设备状态源**：所有设备信息（包括连接状态、统计数据）均由 AppState 管理

重构后的 DeviceManager 接口：

```rust
pub struct DeviceManager {
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
}

impl DeviceManager {
    pub async fn open_serial(&self, config: SerialPortConfig, on_data: SerialDataCallback) -> Result<()>;
    pub async fn close_serial(&self, port_name: &str) -> Result<()>;
    pub async fn connect_ble(&self, address: &str) -> Result<BleConnection>;
    pub async fn disconnect_ble(&self, address: &str) -> Result<()>;
    pub async fn send_serial_data(&self, port_name: &str, data: &[u8]) -> Result<usize>;
    pub async fn write_ble_characteristic(&self, address: &str, uuid: &str, data: &[u8]) -> Result<()>;
    pub async fn configure_ble_at(&self, config: AtConfig) -> Result<()>;
    pub async fn configure_ble_native(&self) -> Result<()>;
}
```

#### 27.2.2 事件同步机制

消除状态双源后，需要建立从 DeviceManager 到 AppState 的事件同步通道：

```
DeviceManager (I/O 事件)
    │
    ▼
EventBus / Tauri Event
    │
    ▼
ActionDispatcher (统一处理)
    │
    ▼
AppState (更新状态) → broadcast_state_change → 前端
```

**具体方案：**

1. DeviceManager 的数据回调通过 `EventBus` 发布事件（见 Task 30 集成方案）
2. ActionDispatcher 订阅 EventBus 事件，统一更新 AppState
3. 移除 ActionDispatcher 中直接调用 DeviceManager 的回调注册逻辑

```rust
// ActionDispatcher 订阅设备数据事件
event_bus.subscribe("device.data.received", |topic, payload| {
    let data_event: DeviceDataEvent = serde_json::from_str(payload).unwrap_or_default();
    // 通过 tokio channel 发送到 async 上下文处理
    action_dispatcher_tx.send(Action::DataReceive {
        device_id: data_event.device_id,
        channel_id: data_event.channel_id,
        data: data_event.data,
    }).await;
}).await;
```

#### 27.2.3 connectionStore 迁移方案

**决策：移除 connectionStore，将功能整合到 serialStore 和 bleStore**

理由：
- `connectionStore` 的 `ConnectionInfo` 与 `serialStore`/`bleStore` 中的连接信息高度重叠
- 当前没有任何组件直接使用 `connectionStore` 管理实际的串口/BLE 连接
- 其辅助函数（`formatBytes`, `getConnectionStatusColor` 等）应移至 `utils/` 工具模块

**迁移步骤：**

| 步骤 | 操作 | 说明 |
|------|------|------|
| 1 | 将 `formatBytes` 移至 `utils/format.ts` | 通用格式化工具 |
| 2 | 将 `getConnectionStatusColor` / `getConnectionStatusText` 移至 `utils/status.ts` | 状态显示工具 |
| 3 | 将 `generateConnectionId` 移至 `utils/id.ts` | ID 生成工具 |
| 4 | 检查所有 `useConnectionStore` 引用并替换 | 搜索 `connectionStore` 导入 |
| 5 | 删除 `stores/connectionStore.ts` | 清理文件 |

**serialStore 需补充的字段：**

```typescript
interface SerialState {
  // ... 现有字段
  isConnecting: boolean;  // 新增：从 connectionStore 迁移
}
```

**bleStore 需补充的字段：**

```typescript
interface BleState {
  // ... 现有字段（已包含 isConnecting, isScanning）
  // 无需额外迁移
}
```

### 27.3 实施优先级

| 优先级 | 任务 | 预估工时 | 风险 |
|--------|------|----------|------|
| P0 | DeviceManager 移除 devices/routes/callbacks | 4h | 中（需确保所有调用点更新） |
| P0 | connectionStore 迁移 | 2h | 低 |
| P1 | EventBus 事件同步机制 | 6h | 中（依赖 Task 30 决策） |
| P2 | ActionDispatcher 重构 | 4h | 中 |

---

## Task 28：错误处理优化

### 28.1 现状分析

当前 `ComBridgeError` 存在以下问题：

1. **手动实现 `Display` 和 `Error` trait**：未使用 `thiserror`，代码冗余
2. **所有变体仅包装 `String`**：无法携带结构化错误上下文
3. **缺少 `DeviceError` 变体**：设备管理层错误复用 `ConfigError`
4. **`persistence.rs` 返回 `Result<_, String>`**：绕过统一错误处理
5. **`commands.rs` 返回 `Result<_, String>`**：Tauri 命令层丢失错误码信息

```rust
// 当前问题示例：
// persistence.rs - 使用 String 而非 ComBridgeError
pub async fn save(&self, state: &AppState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state)
        .map_err(|e| format!("序列化状态失败: {}", e))?;
    // ...
}

// device_manager.rs - 设备不存在错误复用 ConfigError
.ok_or_else(|| ComBridgeError::config(format!("设备不存在: {}", device_id)))?;
```

### 28.2 设计方案

#### 28.2.1 使用 thiserror 重写 ComBridgeError

```rust
use thiserror::Error;

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

#[derive(Debug, Error, Clone, serde::Serialize)]
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

    #[error("[E8000] {0}")]
    DeviceError(String),
}
```

**thiserror 带来的改进：**
- 自动实现 `Display` trait，`#[error]` 属性定义格式
- 自动实现 `Error` trait
- 支持 `#[from]` 自动转换
- 代码量减少约 40%

#### 28.2.2 新增 DeviceError 变体（错误码 8000-8999）

DeviceError 专门用于设备管理层错误，与 ConfigError 解耦：

```rust
impl ComBridgeError {
    pub fn device_not_found(device_id: &str) -> Self {
        ComBridgeError::DeviceError(format!("设备不存在: {}", device_id))
    }

    pub fn device_already_connected(device_id: &str) -> Self {
        ComBridgeError::DeviceError(format!("设备已连接: {}", device_id))
    }

    pub fn device_not_connected(device_id: &str) -> Self {
        ComBridgeError::DeviceError(format!("设备未连接: {}", device_id))
    }

    pub fn device_operation_failed(device_id: &str, operation: &str, reason: &str) -> Self {
        ComBridgeError::DeviceError(format!(
            "设备 {} 操作 {} 失败: {}", device_id, operation, reason
        ))
    }

    pub fn channel_not_found(device_id: &str, channel_id: &str) -> Self {
        ComBridgeError::DeviceError(format!(
            "通道不存在: {}/{}", device_id, channel_id
        ))
    }

    pub fn route_not_found(source: &str, target: &str) -> Self {
        ComBridgeError::DeviceError(format!(
            "路由不存在: {} -> {}", source, target
        ))
    }
}
```

**错误码范围规划：**

| 错误码范围 | 分类 | 说明 |
|-----------|------|------|
| 8000-8099 | 设备通用错误 | 设备不存在、操作失败 |
| 8100-8199 | 设备连接错误 | 已连接、未连接、连接超时 |
| 8200-8299 | 通道错误 | 通道不存在、订阅失败 |
| 8300-8399 | 路由错误 | 路由不存在、路由冲突 |
| 8400-8999 | 预留 | 未来扩展 |

#### 28.2.3 统一 persistence.rs 错误类型

将 `Result<_, String>` 替换为 `Result<_, ComBridgeError>`：

```rust
use crate::error::{ComBridgeError, Result};

impl StatePersistence {
    pub async fn save(&self, state: &AppState) -> Result<()> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| ComBridgeError::config(format!("序列化状态失败: {}", e)))?;

        if let Some(parent) = self.state_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ComBridgeError::io(format!("创建目录失败: {}", e)))?;
        }

        tokio::fs::write(&self.state_path, json)
            .await
            .map_err(|e| ComBridgeError::io(format!("写入状态文件失败: {}", e)))?;

        Ok(())
    }

    pub async fn load(&self) -> Result<AppState> {
        if !self.state_path.exists() {
            return Ok(AppState::default());
        }

        let content = tokio::fs::read_to_string(&self.state_path)
            .await
            .map_err(|e| ComBridgeError::io(format!("读取状态文件失败: {}", e)))?;

        let state: AppState = serde_json::from_str(&content)
            .map_err(|e| ComBridgeError::parse(format!("解析状态文件失败: {}", e)))?;

        Ok(state)
    }

    pub async fn save_preferences(&self, prefs: &Preferences) -> Result<()> {
        let yaml = serde_yaml::to_string(prefs)
            .map_err(|e| ComBridgeError::config(format!("序列化偏好设置失败: {}", e)))?;

        if let Some(parent) = self.preferences_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ComBridgeError::io(format!("创建目录失败: {}", e)))?;
        }

        tokio::fs::write(&self.preferences_path, yaml)
            .await
            .map_err(|e| ComBridgeError::io(format!("写入偏好设置文件失败: {}", e)))?;

        Ok(())
    }

    pub async fn load_preferences(&self) -> Result<Preferences> {
        if !self.preferences_path.exists() {
            let default_prefs = Preferences::default();
            self.save_preferences(&default_prefs).await?;
            return Ok(default_prefs);
        }

        let content = tokio::fs::read_to_string(&self.preferences_path)
            .await
            .map_err(|e| ComBridgeError::io(format!("读取偏好设置文件失败: {}", e)))?;

        let prefs: Preferences = serde_yaml::from_str(&content)
            .map_err(|e| ComBridgeError::parse(format!("解析偏好设置文件失败: {}", e)))?;

        Ok(prefs)
    }
}
```

#### 28.2.4 Tauri 命令层错误转换

为 Tauri 命令添加统一的错误转换，保留错误码信息传递到前端：

```rust
impl From<ComBridgeError> for String {
    fn from(err: ComBridgeError) -> String {
        let response = err.to_error_response();
        serde_json::to_string(&response).unwrap_or_else(|_| err.message().to_string())
    }
}

// 命令示例 - 自动通过 From trait 转换
#[tauri::command]
pub async fn save_preferences(
    persistence: State<'_, StatePersistenceRef>,
    prefs: Preferences,
) -> Result<(), String> {
    let p = persistence.read().await;
    p.save_preferences(&prefs).await?;
    Ok(())
}
```

### 28.3 实施优先级

| 优先级 | 任务 | 预估工时 | 风险 |
|--------|------|----------|------|
| P0 | thiserror 重写 ComBridgeError | 2h | 低 |
| P0 | 新增 DeviceError 变体 | 1h | 低 |
| P0 | persistence.rs 错误类型统一 | 2h | 低 |
| P1 | Tauri 命令层错误转换 | 3h | 中（需前端配合） |
| P2 | device_manager.rs 错误迁移 | 2h | 低 |

---

## Task 29：性能优化

### 29.1 现状分析

| 问题 | 位置 | 影响 |
|------|------|------|
| `ChannelBuffer` 使用 `Vec` + `remove(0)` | `state/types.rs:59` | 每次 O(n) 移动，高频数据场景下性能差 |
| `dispatch` 每次调用都 `save_state` | `state/dispatcher.rs:95` | 高频操作（如数据接收）触发大量磁盘 I/O |
| `broadcast_state_change` 广播全量状态 | `state/dispatcher.rs:515-528` | 序列化+传输完整 AppState，网络开销大 |
| 前端 `.slice(-N)` 模式 | `serialStore.ts:175`, `bleStore.ts:229,316,321` | 每次创建新数组，O(n) 复制 |

#### 29.1.1 ChannelBuffer 性能瓶颈分析

```rust
// 当前实现 - Vec::remove(0) 是 O(n) 操作
impl ChannelBuffer {
    pub fn add_entry(&mut self, data: &[u8], max_size: usize) {
        self.entries.push(BufferEntry { timestamp, data: data.to_vec() });
        self.total_bytes += data.len();
        while self.total_bytes > max_size {
            if let Some(removed) = self.entries.first() {
                self.total_bytes -= removed.data.len();
                self.entries.remove(0);  // ← O(n) 每次移动所有后续元素
            }
        }
    }
}
```

在高速串口（921600 baud）场景下，每秒可能产生数百次 `add_entry` 调用，每次 `remove(0)` 需要移动整个数组。

#### 29.1.2 前端 .slice(-N) 性能瓶颈分析

```typescript
// serialStore.ts - 每次添加数据都创建新数组
addReceivedData: (portName, entry) =>
  set((state) => ({
    tabs: state.tabs.map((t) =>
      t.portName === portName && t.tabType === 'port'
        ? { ...t, receivedData: [...t.receivedData, entry].slice(-1000) }
        : t
    ),
  })),
```

每次数据到达：`[...t.receivedData, entry]` 复制整个数组（O(n)），然后 `.slice(-1000)` 再次复制。

### 29.2 设计方案

#### 29.2.1 ChannelBuffer 使用 VecDeque 替换 Vec

```rust
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelBuffer {
    pub entries: VecDeque<BufferEntry>,
    pub total_bytes: usize,
}

impl Default for ChannelBuffer {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            total_bytes: 0,
        }
    }
}

impl ChannelBuffer {
    pub fn add_entry(&mut self, data: &[u8], max_size: usize) {
        let timestamp = current_timestamp();
        self.entries.push_back(BufferEntry {
            timestamp,
            data: data.to_vec(),
        });
        self.total_bytes += data.len();

        while self.total_bytes > max_size {
            if let Some(removed) = self.entries.pop_front() {  // ← O(1)
                self.total_bytes -= removed.data.len();
            } else {
                break;
            }
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}
```

**性能对比：**

| 操作 | Vec | VecDeque | 提升 |
|------|-----|----------|------|
| `push_back` | O(1) 均摊 | O(1) 均摊 | 相同 |
| `pop_front` / `remove(0)` | O(n) | O(1) | **显著** |
| 随机访问 | O(1) | O(1) | 相同 |
| 内存布局 | 连续 | 分段连续 | Vec 略优 |

**注意事项：**
- `VecDeque` 的 `serde` 序列化默认使用数组格式，与当前 `Vec` 序列化结果兼容
- 需验证 `VecDeque` 的 `Serialize` 输出与前端 `Array` 解析兼容

#### 29.2.2 ActionDispatcher.dispatch 添加 save_state 防抖

```rust
use tokio::sync::mpsc;
use std::time::{Duration, Instant};

const SAVE_DEBOUNCE_MS: u64 = 500;

pub struct ActionDispatcher {
    state: AppStateRef,
    persistence: StatePersistenceRef,
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
    save_tx: mpsc::UnboundedSender<()>,
}

impl ActionDispatcher {
    pub fn new(
        state: AppStateRef,
        persistence: StatePersistenceRef,
        serial_manager: SerialManagerRef,
        ble_manager: BleManagerRef,
    ) -> Self {
        let (save_tx, mut save_rx) = mpsc::unbounded_channel::<()>();

        let state_clone = state.clone();
        let persistence_clone = persistence.clone();
        tokio::spawn(async move {
            let mut last_save = Instant::now();
            while save_rx.recv().await.is_some() {
                let elapsed = last_save.elapsed();
                if elapsed < Duration::from_millis(SAVE_DEBOUNCE_MS) {
                    tokio::time::sleep(
                        Duration::from_millis(SAVE_DEBOUNCE_MS) - elapsed
                    ).await;
                }
                let state = state_clone.read().await;
                let persistence = persistence_clone.read().await;
                if let Err(e) = persistence.save(&state).await {
                    tracing::error!("保存状态失败: {}", e);
                }
                last_save = Instant::now();

                // 排空积压的保存请求
                while save_rx.try_recv().is_ok() {}
            }
        });

        Self { state, persistence, serial_manager, ble_manager, save_tx }
    }

    pub async fn dispatch(&self, action: Action, app: &AppHandle) -> ActionResult {
        // ... 处理 action ...

        if result.success {
            self.broadcast_state_change(app).await;
            let _ = self.save_tx.send(());  // 非阻塞发送保存请求
        }

        result
    }
}
```

**防抖策略说明：**

- 首次 dispatch 触发保存请求
- 500ms 内的后续 dispatch 不触发新的保存，而是重置计时器
- 500ms 无新 dispatch 后执行保存
- 高频数据接收场景下，磁盘 I/O 从每秒数百次降至每 500ms 最多 1 次

#### 29.2.3 broadcast_state_change 增量更新

当前实现：每次广播完整 `AppState` JSON。

优化方案：根据 Action 类型发送增量更新事件。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "type")]
pub enum StateChange {
    DeviceAdded { device_id: String, device: Device },
    DeviceRemoved { device_id: String },
    DeviceConnected { device_id: String },
    DeviceDisconnected { device_id: String },
    DataReceived { device_id: String, channel_id: String, entry: BufferEntry },
    DataSent { device_id: String, channel_id: String, entry: BufferEntry },
    BufferCleared { device_id: String, channel_id: String },
    ActiveDeviceChanged { device_id: Option<String> },
    TabChanged { window_state: WindowState },
    SettingsChanged { settings: AppSettings },
}
```

```rust
impl ActionDispatcher {
    async fn broadcast_incremental(&self, app: &AppHandle, change: StateChange) {
        if let Err(e) = app.emit("state-change", &change) {
            tracing::error!("广播增量状态变更失败: {}", e);
        }
    }

    async fn dispatch(&self, action: Action, app: &AppHandle) -> ActionResult {
        let (result, change) = match action {
            Action::DeviceAddSerial { id, name, baud_rate } => {
                let r = self.handle_device_add_serial(&id, &name, baud_rate).await;
                let c = if r.success {
                    let state = self.state.read().await;
                    state.get_device(&id).map(|d| StateChange::DeviceAdded {
                        device_id: id.clone(),
                        device: d.clone(),
                    })
                } else { None };
                (r, c)
            }
            Action::DataReceive { device_id, channel_id, data } => {
                let r = self.handle_data_receive(&device_id, &channel_id, &data).await;
                let c = if r.success {
                    let state = self.state.read().await;
                    state.get_channel(&device_id, &channel_id)
                        .and_then(|ch| ch.buffer.entries.back().cloned())
                        .map(|entry| StateChange::DataReceived {
                            device_id: device_id.clone(),
                            channel_id: channel_id.clone(),
                            entry,
                        })
                } else { None };
                (r, c)
            }
            // ... 其他 Action 类型
            _ => (self.handle_action(&action, app).await, None),
        };

        if result.success {
            if let Some(change) = change {
                self.broadcast_incremental(app, change).await;
            } else {
                self.broadcast_state_change(app).await;
            }
            let _ = self.save_tx.send(());
        }

        result
    }
}
```

**前端适配：**

```typescript
// 前端监听增量更新
listen<StateChange>('state-change', (event) => {
  const change = event.payload;
  switch (change.type) {
    case 'dataReceived':
      // 仅更新对应通道的数据，无需替换整个状态
      updateChannelBuffer(change.deviceId, change.channelId, change.entry);
      break;
    case 'deviceConnected':
      updateDeviceConnection(change.deviceId, true);
      break;
    // ... 其他类型
    default:
      // 未知类型回退到全量刷新
      refreshFullState();
  }
});
```

#### 29.2.4 前端环形缓冲区替换 .slice(-N)

```typescript
class RingBuffer<T> {
  private buffer: (T | undefined)[];
  private head: number = 0;
  private tail: number = 0;
  private size: number = 0;
  private readonly capacity: number;

  constructor(capacity: number) {
    this.capacity = capacity;
    this.buffer = new Array(capacity);
  }

  push(item: T): void {
    this.buffer[this.tail] = item;
    this.tail = (this.tail + 1) % this.capacity;
    if (this.size === this.capacity) {
      this.head = (this.head + 1) % this.capacity;
    } else {
      this.size++;
    }
  }

  toArray(): T[] {
    const result: T[] = [];
    for (let i = 0; i < this.size; i++) {
      const idx = (this.head + i) % this.capacity;
      const item = this.buffer[idx];
      if (item !== undefined) result.push(item);
    }
    return result;
  }

  clear(): void {
    this.buffer = new Array(this.capacity);
    this.head = 0;
    this.tail = 0;
    this.size = 0;
  }

  get length(): number {
    return this.size;
  }
}
```

**Store 集成方式：**

```typescript
interface SerialTab {
  // ... 现有字段
  receivedData: RingBuffer<DataEntry>;  // 替换 DataEntry[]
  sentData: RingBuffer<DataEntry>;      // 替换 DataEntry[]
}

// 使用方式
addReceivedData: (portName, entry) =>
  set((state) => ({
    tabs: state.tabs.map((t) => {
      if (t.portName === portName && t.tabType === 'port') {
        t.receivedData.push(entry);  // O(1)，无需创建新数组
        return { ...t };
      }
      return t;
    }),
  })),
```

**性能对比（1000 条数据场景）：**

| 操作 | .slice(-N) | RingBuffer | 提升 |
|------|-----------|------------|------|
| 添加一条数据 | O(n) 复制 | O(1) | **1000x** |
| 内存占用 | 持续分配新数组 | 固定大小 | 稳定 |
| 渲染读取 | O(1) 直接访问 | O(n) toArray | 略慢 |

**注意事项：**
- `RingBuffer.toArray()` 仅在渲染时调用，频率远低于数据添加
- 需确保 `RingBuffer` 正确实现序列化（Zustand persist 中间件兼容）
- 可考虑使用 `immer` 中间件简化不可变更新

### 29.3 实施优先级

| 优先级 | 任务 | 预估工时 | 风险 |
|--------|------|----------|------|
| P0 | ChannelBuffer Vec → VecDeque | 2h | 低（serde 兼容需验证） |
| P0 | dispatch save_state 防抖 | 3h | 中（需测试异步保存正确性） |
| P1 | 增量状态广播 | 6h | 高（需前后端协同改造） |
| P2 | 前端 RingBuffer | 4h | 中（需验证 Zustand 兼容性） |

---

## Task 30：EventBus 集成决策

### 30.1 现状分析

项目中存在两套事件机制：

| 机制 | 文件 | 用途 | 特点 |
|------|------|------|------|
| `EventBus` | `service/event_bus.rs` | 后端内部事件发布/订阅 | broadcast channel + callback map |
| Tauri Event | `state/dispatcher.rs` | 后端→前端事件推送 | `app.emit()` 跨进程通信 |

**EventBus 当前状态：**
- 已实现 `publish` / `subscribe` / `subscribe_channel` 功能
- 支持 topic 过滤
- 使用 `broadcast::Sender<Event>` + `HashMap<String, Vec<EventCallback>>`
- **但实际使用率极低**——搜索代码发现 `EventBus` 几乎未被任何模块引用

**Tauri Event 当前使用：**
- `ActionDispatcher.broadcast_state_change` 直接调用 `app.emit("state-change", state_json)`
- 串口数据回调中直接调用 `app.emit(STATE_CHANGE_EVENT, ())`
- 前端通过 `listen()` 监听事件

### 30.2 功能重叠分析

```
┌──────────────────────────────────────────────────────────────┐
│                        EventBus                              │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │  publish()   │───▶│  subscribe() │ (Rust callback)       │
│  └──────────────┘    └──────────────┘                       │
│  ┌──────────────┐    ┌───────────────────┐                  │
│  │  publish()   │───▶│ subscribe_channel │ (broadcast::Recv) │
│  └──────────────┘    └───────────────────┘                  │
│                                                              │
│  限制：仅在 Rust 进程内部通信，无法到达前端                      │
└──────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────┐
│                      Tauri Event                             │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │  app.emit()  │───▶│  listen()    │ (前端 JS 回调)         │
│  └──────────────┘    └──────────────┘                       │
│  ┌──────────────┐    ┌──────────────┐                       │
│  │  app.emit()  │───▶│  Rust listen │ (后端监听)             │
│  └──────────────┘    └──────────────┘                       │
│                                                              │
│  优势：跨进程通信，前后端均可监听                                │
└──────────────────────────────────────────────────────────────┘
```

**关键差异：**

| 维度 | EventBus | Tauri Event |
|------|----------|-------------|
| 通信范围 | Rust 进程内部 | 前后端跨进程 |
| 性能 | 更高（无 IPC 开销） | 有 IPC 序列化开销 |
| 类型安全 | 弱（String payload） | 弱（JSON payload） |
| 前端可达 | ❌ | ✅ |
| 事件历史 | broadcast channel 缓存 | 无 |

### 30.3 决策：集成 EventBus 作为后端内部事件总线

**理由：**
1. Tauri Event 适合后端→前端通信，但后端内部模块间通信不应依赖 IPC
2. EventBus 的 `broadcast::Receiver` 支持事件历史回放，适合异步消费者
3. 集成后可解耦 DeviceManager 与 ActionDispatcher 的直接依赖

**不是废弃 EventBus 的理由：**
- 后端模块间需要解耦通信（如 DeviceManager → ActionDispatcher）
- Tauri Event 的 `app.emit()` 需要 `AppHandle`，在底层模块中传递不便
- EventBus 可作为 Tauri Event 的上游，统一事件流

### 30.4 集成方案

```
DeviceManager (数据回调)
    │
    ▼ EventBus.publish("serial.data.received", payload)
    │
EventBus (后端内部)
    │
    ├──▶ ActionDispatcher (subscribe) → 更新 AppState
    │
    └──▶ TauriEventBridge (subscribe_channel) → app.emit() → 前端
```

#### 30.4.1 TauriEventBridge：连接 EventBus 与 Tauri Event

```rust
pub struct TauriEventBridge {
    event_bus: EventBusRef,
}

impl TauriEventBridge {
    pub fn new(event_bus: EventBusRef) -> Self {
        Self { event_bus }
    }

    pub async fn start(&self, app: AppHandle) {
        let mut rx = self.event_bus.subscribe_channel();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let _ = app.emit(&event.topic, &event.payload);
            }
        });
    }
}
```

#### 30.4.2 DeviceManager 使用 EventBus 发布事件

```rust
pub struct DeviceManager {
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
    event_bus: EventBusRef,
}

impl DeviceManager {
    pub async fn open_serial(&self, config: SerialPortConfig) -> Result<()> {
        let event_bus = self.event_bus.clone();
        let device_id = format!("serial-{}", config.port_name);

        self.serial_manager.open_port(config, move |_name, data| {
            let payload = serde_json::json!({
                "deviceId": device_id,
                "data": data,
            }).to_string();

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let bus = event_bus.clone();
                handle.spawn(async move {
                    bus.publish("serial.data.received", payload).await;
                });
            }
        })?;

        Ok(())
    }
}
```

#### 30.4.3 ActionDispatcher 订阅 EventBus

```rust
impl ActionDispatcher {
    pub async fn subscribe_events(&self, event_bus: &EventBusRef) {
        let state = self.state.clone();
        let save_tx = self.save_tx.clone();

        event_bus.subscribe("serial.data.received", move |topic, payload| {
            let event: serde_json::Value = serde_json::from_str(payload)
                .unwrap_or_default();
            let device_id = event["deviceId"].as_str().unwrap_or("");
            let data = event["data"].as_array()
                .map(|arr| arr.iter().filter_map(|v| v.as_u64().map(|b| b as u8)).collect::<Vec<_>>())
                .unwrap_or_default();

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let state = state.clone();
                let save_tx = save_tx.clone();
                handle.spawn(async move {
                    let mut state = state.write().await;
                    state.add_serial_rx_data(device_id, &data);
                    drop(state);
                    let _ = save_tx.send(());
                });
            }
        }).await;
    }
}
```

### 30.5 实施优先级

| 优先级 | 任务 | 预估工时 | 风险 |
|--------|------|----------|------|
| P0 | 实现 TauriEventBridge | 3h | 低 |
| P1 | DeviceManager 集成 EventBus | 4h | 中 |
| P1 | ActionDispatcher 订阅 EventBus | 3h | 中 |
| P2 | 移除 Dispatcher 中直接 app.emit 调用 | 2h | 低 |

---

## Task 31：前端架构优化

### 31.1 现状分析

#### 31.1.1 configService 问题

`configService.ts` 使用手动实现的观察者模式 + localStorage：

```typescript
class ConfigService {
  private config: AppConfig;
  private listeners: Map<string, Set<(config: AppConfig) => void>> = new Map();

  private loadConfig(): AppConfig { /* localStorage.getItem */ }
  private saveConfig(): void { /* localStorage.setItem + notifyListeners */ }
  subscribe(listener: (config: AppConfig) => void): () => void { /* 手动管理 */ }
}
```

**问题：**
- 与项目 Zustand 状态管理方案不一致
- 手动管理 listeners，易导致内存泄漏
- 无法使用 Zustand 的 devtools / persist 中间件
- `configService` 是单例，无法在 React 组件外方便使用

#### 31.1.2 eventListeners 问题

`eventListeners.ts` 直接调用 Ant Design `message` 组件：

```typescript
serialListeners.error = await onSerialError((event) => {
  const store = useSerialStore.getState();
  store.setError(event.error);
  useLogStore.getState().addLog('error', 'SerialManager', `串口错误: ${event.error}`);
  message.error(`串口错误: ${event.error}`);  // ← 直接耦合 UI 组件
});
```

**问题：**
- 事件处理层与 UI 层（Ant Design message）紧耦合
- 无法在非浏览器环境（如测试）中使用
- 通知策略（是否显示 message）硬编码在事件监听中
- 无法自定义通知行为（如静默模式、通知去重）

#### 31.1.3 API 冗余别名问题

`tauri.ts` 中存在大量 snake_case 到 camelCase 的冗余别名：

```typescript
export const serialApi = {
  async listPorts(): Promise<SerialPortInfo[]> { ... },
  scanPorts(): Promise<SerialPortInfo[]> { return this.listPorts(); },  // 冗余

  async open(portName: string, config: SerialConfig): Promise<void> { ... },
  openPort(portName: string, config: SerialConfig): Promise<void> { return this.open(...); },  // 冗余

  async write(portName: string, data: number[]): Promise<void> { ... },
  sendData(portName: string, data: number[]): Promise<void> { return this.write(...); },  // 冗余
};

export const bleApi = {
  async configure(params: BleConfigureParams): Promise<void> { ... },
  configureBle(mode: 'native' | 'at', serialPort?: string): Promise<void> { ... },  // 冗余

  async scan(options?: BleScanOptions): Promise<BleDeviceInfo[]> { ... },
  scanBleDevices(options?: BleScanOptions): Promise<BleDeviceInfo[]> { ... },  // 冗余
  // ... 更多冗余
};
```

### 31.2 设计方案

#### 31.2.1 迁移 configService 到 Zustand Store + persist 中间件

```typescript
import { create } from 'zustand';
import { persist } from 'zustand/middleware';

interface AppConfig {
  theme: 'system' | 'light' | 'dark';
  language: string;
  autoReconnect: boolean;
  autoReconnectInterval: number;
  maxLogLines: number;
  soundEnabled: boolean;
  soundOnConnect: boolean;
  soundOnDisconnect: boolean;
  soundOnData: boolean;
}

interface SerialConfig {
  baudRate: number;
  dataBits: number;
  parity: string;
  stopBits: number;
  flowControl: string;
}

interface BleModeConfig {
  mode: 'native' | 'at';
  atPort?: string;
  atBaudRate?: number;
}

interface RecentConnection {
  type: string;
  identifier: string;
  name?: string;
  lastConnected: number;
}

interface ConfigState {
  config: AppConfig;
  serialConfig: SerialConfig;
  bleConfig: BleModeConfig;
  recentConnections: RecentConnection[];

  updateConfig: (updates: Partial<AppConfig>) => void;
  resetConfig: () => void;
  saveSerialConfig: (config: SerialConfig) => void;
  saveBleConfig: (config: BleModeConfig) => void;
  addRecentConnection: (connection: Omit<RecentConnection, 'lastConnected'>) => void;
  removeRecentConnection: (identifier: string) => void;
  clearRecentConnections: () => void;
}

const DEFAULT_CONFIG: AppConfig = {
  theme: 'system',
  language: 'zh-CN',
  autoReconnect: false,
  autoReconnectInterval: 3000,
  maxLogLines: 1000,
  soundEnabled: true,
  soundOnConnect: true,
  soundOnDisconnect: true,
  soundOnData: false,
};

export const useConfigStore = create<ConfigState>()(
  persist(
    (set, get) => ({
      config: DEFAULT_CONFIG,
      serialConfig: { baudRate: 115200, dataBits: 8, parity: 'none', stopBits: 1, flowControl: 'none' },
      bleConfig: { mode: 'native' },
      recentConnections: [],

      updateConfig: (updates) =>
        set((state) => ({ config: { ...state.config, ...updates } })),

      resetConfig: () => set({ config: DEFAULT_CONFIG }),

      saveSerialConfig: (serialConfig) => set({ serialConfig }),

      saveBleConfig: (bleConfig) => set({ bleConfig }),

      addRecentConnection: (connection) =>
        set((state) => {
          const filtered = state.recentConnections.filter(
            (c) => c.identifier !== connection.identifier
          );
          return {
            recentConnections: [
              { ...connection, lastConnected: Date.now() },
              ...filtered,
            ].slice(0, 10),
          };
        }),

      removeRecentConnection: (identifier) =>
        set((state) => ({
          recentConnections: state.recentConnections.filter(
            (c) => c.identifier !== identifier
          ),
        })),

      clearRecentConnections: () => set({ recentConnections: [] }),
    }),
    {
      name: 'combridge-config',
      partialize: (state) => ({
        config: state.config,
        serialConfig: state.serialConfig,
        bleConfig: state.bleConfig,
        recentConnections: state.recentConnections,
      }),
    }
  )
);
```

**迁移映射表：**

| configService 方法 | useConfigStore 方法 |
|--------------------|---------------------|
| `getConfig()` | `useConfigStore.getState().config` |
| `updateConfig(updates)` | `useConfigStore.getState().updateConfig(updates)` |
| `resetConfig()` | `useConfigStore.getState().resetConfig()` |
| `subscribe(listener)` | `useConfigStore.subscribe(listener)` |
| `getSerialConfig()` | `useConfigStore.getState().serialConfig` |
| `saveSerialConfig(config)` | `useConfigStore.getState().saveSerialConfig(config)` |
| `getBleConfig()` | `useConfigStore.getState().bleConfig` |
| `saveBleConfig(config)` | `useConfigStore.getState().saveBleConfig(config)` |
| `getRecentConnections()` | `useConfigStore.getState().recentConnections` |
| `addRecentConnection(conn)` | `useConfigStore.getState().addRecentConnection(conn)` |

#### 31.2.2 提取通用错误处理高阶函数

```typescript
type AsyncAction<TArgs extends unknown[], TResult> = (
  ...args: TArgs
) => Promise<TResult>;

interface ErrorHandlerOptions {
  store?: { setError: (error: string | null) => void };
  logStore?: { addLog: (level: string, source: string, message: string) => void };
  logSource?: string;
  errorMessage?: string;
}

function withErrorHandler<TArgs extends unknown[], TResult>(
  action: AsyncAction<TArgs, TResult>,
  options: ErrorHandlerOptions = {}
): AsyncAction<TArgs, TResult> {
  return async (...args: TArgs): Promise<TResult> => {
    try {
      return await action(...args);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      const displayMessage = options.errorMessage ?? message;

      options.store?.setError(displayMessage);
      options.logStore?.addLog(
        'error',
        options.logSource ?? 'Unknown',
        displayMessage
      );

      throw err;
    }
  };
}
```

**使用示例：**

```typescript
const connectSerial = withErrorHandler(
  async (portName: string, config: SerialConfig) => {
    await serialApi.open(portName, config);
  },
  {
    store: useSerialStore.getState(),
    logStore: useLogStore.getState(),
    logSource: 'SerialManager',
    errorMessage: '连接串口失败',
  }
);
```

#### 31.2.3 解耦 eventListeners 与 UI 组件

**核心思路：事件监听层只更新 Store，由 Store 变更驱动 UI 通知**

```typescript
import { useSerialStore, generateId } from '../stores/serialStore';
import { useBleStore, generateBleId } from '../stores/bleStore';
import { useLogStore } from '../stores/logStore';
import { useNotificationStore } from '../stores/notificationStore';

// 新增 notificationStore（轻量级通知状态管理）
interface Notification {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
  timestamp: number;
}

interface NotificationState {
  notifications: Notification[];
  addNotification: (type: Notification['type'], message: string) => void;
  removeNotification: (id: string) => void;
}

export const useNotificationStore = create<NotificationState>((set) => ({
  notifications: [],
  addNotification: (type, message) =>
    set((state) => ({
      notifications: [
        ...state.notifications,
        { id: generateId(), type, message, timestamp: Date.now() },
      ],
    })),
  removeNotification: (id) =>
    set((state) => ({
      notifications: state.notifications.filter((n) => n.id !== id),
    })),
}));
```

**重构后的 eventListeners：**

```typescript
export async function initSerialEventListeners(): Promise<void> {
  if (serialInitialized) return;
  if (serialInitPromise) return serialInitPromise;

  serialInitPromise = (async () => {
    serialListeners.data = await onSerialData((event) => {
      useSerialStore.getState().addReceivedData(event.port_name, {
        id: generateId(),
        timestamp: event.timestamp ?? Date.now(),
        data: event.data,
        direction: 'receive',
        format: 'hex',
      });
    });

    serialListeners.error = await onSerialError((event) => {
      const store = useSerialStore.getState();
      store.setError(event.error);
      useLogStore.getState().addLog('error', 'SerialManager', `串口错误: ${event.error}`);
      useNotificationStore.getState().addNotification('error', `串口错误: ${event.error}`);
    });

    serialListeners.connected = await onSerialConnected((portName) => {
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已连接`);
      useNotificationStore.getState().addNotification('success', `串口 ${portName} 已连接`);
    });

    serialListeners.disconnected = await onSerialDisconnected((portName) => {
      const store = useSerialStore.getState();
      const tab = store.tabs.find((t) => t.portName === portName && t.tabType === 'port');
      if (tab) {
        store.updateTab(tab.key, { isConnected: false });
      }
      useLogStore.getState().addLog('info', 'SerialManager', `串口 ${portName} 已断开`);
      useNotificationStore.getState().addNotification('info', `串口 ${portName} 已断开`);
    });

    serialInitialized = true;
    serialInitPromise = null;
  })();

  return serialInitPromise;
}
```

**UI 层消费通知：**

```tsx
function NotificationConsumer() {
  const notifications = useNotificationStore((s) => s.notifications);
  const removeNotification = useNotificationStore((s) => s.removeNotification);

  useEffect(() => {
    for (const n of notifications) {
      const hide = message[n.type](n.message, 3, () => {
        removeNotification(n.id);
      });
    }
  }, [notifications]);

  return null;
}
```

#### 31.2.4 清理 API 冗余别名

**原则：保留 camelCase 命名，移除 snake_case 别名**

当前 API 命名规范（根据项目规则）：
- 后端 Rust：`snake_case`（如 `scan_serial_ports`）
- 前端调用：`camelCase`（如 `listPorts`）

**需要移除的冗余别名清单：**

| API 模块 | 保留方法（camelCase） | 移除别名（snake_case 风格） |
|----------|----------------------|---------------------------|
| serialApi | `listPorts` | `scanPorts` |
| serialApi | `open` | `openPort` |
| serialApi | `close` | `closePort` |
| serialApi | `write` | `sendData` |
| bleApi | `configure` | `configureBle` |
| bleApi | `scan` | `scanBleDevices` |
| bleApi | `stopScan` | `stopBleScan` |
| bleApi | `connect` | `connectBle` |
| bleApi | `disconnect` | `disconnectBle` |
| bleApi | `discoverServices` | `discoverBleServices` |
| bleApi | `discoverCharacteristics` | `discoverBleCharacteristics` |
| bleApi | `read` | `readBleCharacteristic` |
| bleApi | `write` | `writeBleCharacteristic` |
| bleApi | `writeWithoutResponse` | `writeBleWithoutResponse` |
| bleApi | `subscribe` | `subscribeBleNotify` |
| bleApi | `unsubscribe` | `unsubscribeBleNotify` |
| bleApi | `setMtu` | `setBleMtu` |
| protocolApi | `load` | `loadProtocol` |
| protocolApi | `unload` | `unloadProtocol` |
| protocolApi | `enable` | `enableProtocol` |
| protocolApi | `disable` | `disableProtocol` |
| protocolApi | `bind` | `bindProtocol` |
| protocolApi | `unbind` | `unbindProtocol` |
| protocolApi | `list` | `listProtocols` |
| protocolApi | `get` | `getProtocol` |
| protocolApi | `getBound` | `getBoundProtocols` |

**迁移步骤：**

1. 全局搜索所有冗余别名的调用点
2. 替换为保留的 camelCase 方法名
3. 删除 API 对象中的冗余别名定义
4. 运行 TypeScript 编译检查确保无遗漏

```typescript
// 重构后的 serialApi - 简洁无冗余
export const serialApi = {
  async listPorts(): Promise<SerialPortInfo[]> {
    return invoke<SerialPortInfo[]>('scan_serial_ports');
  },
  async open(portName: string, config: SerialConfig): Promise<void> {
    await invoke<void>('open_serial_port', { config: { portName, ...config } });
  },
  async close(portName: string): Promise<void> {
    await invoke<void>('close_serial_port', { portName });
  },
  async write(portName: string, data: number[]): Promise<void> {
    await invoke<void>('send_serial_data', { portName, data });
  },
  async getOpenPorts(): Promise<string[]> {
    return invoke<string[]>('get_open_ports');
  },
  async isConnected(portName: string): Promise<boolean> {
    return invoke<boolean>('is_port_open', { portName });
  },
  async exportData(portName: string, allData: Array<{timestamp: number; data: number[]; direction: string}>, rxData: number[]): Promise<{logPath: string; datPath: string}> {
    return invoke<{logPath: string; datPath: string}>('export_serial_data', { portName, allData, rxData });
  },
  async getCache(portName: string): Promise<CacheData> {
    return invoke<CacheData>('get_serial_cache', { portName });
  },
};
```

### 31.3 实施优先级

| 优先级 | 任务 | 预估工时 | 风险 |
|--------|------|----------|------|
| P0 | 清理 API 冗余别名 | 3h | 低（需全局搜索替换） |
| P0 | 迁移 configService → useConfigStore | 4h | 中（需更新所有引用） |
| P1 | 解耦 eventListeners 与 UI | 4h | 中（需新增 notificationStore） |
| P2 | 提取通用错误处理高阶函数 | 3h | 低 |

---

## 附录：实施路线图

### 阶段一（基础优化，1-2 周）

1. **Task 28**：thiserror 重写 ComBridgeError + DeviceError 变体
2. **Task 29**：ChannelBuffer Vec → VecDeque
3. **Task 31**：清理 API 冗余别名

### 阶段二（架构优化，2-3 周）

4. **Task 27**：DeviceManager 精简 + connectionStore 移除
5. **Task 28**：persistence.rs 错误类型统一
6. **Task 29**：dispatch save_state 防抖
7. **Task 31**：configService → Zustand Store

### 阶段三（深度优化，2-3 周）

8. **Task 30**：EventBus 集成 + TauriEventBridge
9. **Task 27**：EventBus 事件同步机制
10. **Task 29**：增量状态广播
11. **Task 31**：eventListeners 解耦 + 错误处理高阶函数

### 阶段四（前端性能，1-2 周）

12. **Task 29**：前端 RingBuffer
13. 全局回归测试 + 性能基准测试
