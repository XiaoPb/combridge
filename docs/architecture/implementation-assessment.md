# ComBridge 项目实现完整性评估报告

> 评估日期：2026-04-13
> 评估范围：ComBridge 全栈项目（Rust 后端 + React 前端）
> 评估版本：基于当前 main 分支最新代码

---

## 1. 未实现功能清单

### 1.1 后端未实现功能

| # | 功能名称 | 模块位置 | 优先级 | 预估工作量 | 说明 |
|---|---------|---------|--------|-----------|------|
| 1 | WebSocket 数据发送 | [device_manager.rs](../../src-tauri/src/device/device_manager.rs#L251-L253) | P0 | M | `send_direct()` 和 `send_to_device()` 中 `DeviceType::WebSocket` 分支返回 `ComBridgeError::websocket("WebSocket 数据发送尚未实现")`，仅有序列化错误，无实际发送逻辑 |
| 2 | WebSocketClient 实际消息发送 | [client.rs](../../src-tauri/src/websocket/client.rs#L217-L224) | P0 | M | `handle_outgoing_message()` 仅检查连接状态，未实际通过 WebSocket 流发送数据；`send_message()` 将消息推入 `mpsc` 通道但消费端只调用 `handle_outgoing_message()` 不执行发送 |
| 3 | WebSocket 重连后消息流恢复 | [client.rs](../../src-tauri/src/websocket/client.rs#L177-L215) | P1 | M | `reconnect()` 成功后仅更新状态，未重新启动消息接收循环（`ws_receiver.next()`），重连后无法接收新消息 |
| 4 | EventBus 集成 | [event_bus.rs](../../src-tauri/src/service/event_bus.rs) | P1 | L | EventBus 已实现 `publish`/`subscribe`/`subscribe_channel` 完整 API，但全项目无任何调用点，未被 `ActionDispatcher`、`DeviceManager` 或其他模块使用 |
| 5 | DeviceManager WebSocket 设备注册 | [device_manager.rs](../../src-tauri/src/device/device_manager.rs) | P1 | M | `DeviceManager` 支持 Serial/Ble 设备的注册和连接，但无 WebSocket 设备的 `open_websocket`/`close_websocket` 方法 |
| 6 | GH3036 RX 通道配置 | [manager.rs](../../src-tauri/src/gh3036/manager.rs#L360-L363) | P1 | S | `configure_rx_channel()` 仅记录日志，未实际配置 RX 通道；`get_rx_channel()` 始终返回 `None` |
| 7 | GH3036 RPC 命令实际执行 | [manager.rs](../../src-tauri/src/gh3036/manager.rs#L434-L458) | P1 | L | 所有 RPC 命令（V/W/R/B/C/D/L/S/P/M/TS/TM）仅解析参数并记录日志，返回空数据或占位数据，未实际与硬件通信 |
| 8 | Dispatcher WebSocket 设备操作 | [dispatcher.rs](../../src-tauri/src/state/dispatcher.rs) | P1 | M | `handle_device_connect`/`handle_data_send` 仅处理 Serial/Ble，无 WebSocket 分支 |
| 9 | 心跳机制 | [client.rs](../../src-tauri/src/websocket/client.rs#L20) | P2 | S | `WebSocketConfig` 定义了 `heartbeat_interval_ms` 但未实现心跳发送/检测逻辑 |
| 10 | 数据路由过滤回调通知 | [device_manager.rs](../../src-tauri/src/device/device_manager.rs#L166-L186) | P2 | S | `route_data()` 路由成功后未调用 `notify_callbacks()`，数据回调仅由 `open_serial` 的接收回调触发 |

### 1.2 前端未实现功能

| # | 功能名称 | 模块位置 | 优先级 | 预估工作量 | 说明 |
|---|---------|---------|--------|-----------|------|
| 1 | waveformStore 定时刷新 | [waveformStore.ts](../../src/stores/waveformStore.ts#L162-L168) | P0 | M | `startRefresh()` 仅设置 `isRunning: true`，`stopRefresh()` 仅设置 `isRunning: false`，无 `setInterval`/`requestAnimationFrame` 定时读取数据逻辑 |
| 2 | stateApi ↔ Zustand Store 桥接 | [stateApi.ts](../../src/api/stateApi.ts) + [stores/](../../src/stores/) | P0 | L | `useAppState()` 通过 `subscribeToStateChanges` 监听后端状态，但未将状态同步到各 Zustand Store（serialStore/bleStore/gh3036Store 等）；各 Store 独立管理状态，与后端 AppState 存在数据不一致风险 |
| 3 | connectionStore 僵尸代码 | [connectionStore.ts](../../src/stores/connectionStore.ts) | P1 | S | `connectionStore` 已完整定义（含 `ConnectionInfo`/`WebSocketConnection` 类型和方法），但全项目无任何组件或 hook 直接使用该 store（仅 `useWebSocket` 内部引用） |
| 4 | BLE AT 模式 Hook 操作 | [useBle.ts](../../src/hooks/useBle.ts) | P1 | M | `useBle` 缺少 AT 模式专用方法：无 `sendAtData`、`getAtConfig`、`updateAtUuidConfig`、`getAtTabs`、`clearAtTabData`、`removeAtTab` 等 AT 相关操作封装 |
| 5 | useWebSocket 发送与后端不一致 | [useWebSocket.ts](../../src/hooks/useWebSocket.ts#L196-L238) | P1 | M | 前端 `send()` 调用 `invoke('send_websocket_message')`，但后端 `WebSocketClient.send_message()` 实际不发送数据（见后端 #2），前端乐观更新 messages 列表与实际状态不同步 |
| 6 | Dashboard 实时数据流 | [dashboardStore.ts](../../src/stores/dashboardStore.ts) | P2 | M | `isRunning` 状态和 `addDataPoint`/`addRawDataPoint` 方法已定义，但无定时轮询或事件监听逻辑将后端数据推入 store |
| 7 | 配置持久化迁移 | [configService.ts](../../src/services/configService.ts) | P2 | M | `configService` 使用 `localStorage` 存储配置（22 处调用），项目规范要求使用 Tauri 原生存储（`app_data_dir`），需迁移至后端 `preferences` 命令 |

---

## 2. 待测试组件清单

### 2.1 后端测试覆盖

| # | 组件名称 | 当前覆盖率 | 目标覆盖率 | 缺失测试类型 | 说明 |
|---|---------|-----------|-----------|-------------|------|
| 1 | SerialManager | ~30% | >70% | 单元/集成 | 仅有 `serial_config` 和 `serial_manager` 基础测试，缺少 `open_port`/`close_port`/`send_data` 的集成测试 |
| 2 | BleManager | 0% | >70% | 单元/集成 | 无任何 `#[cfg(test)]` 模块，BLE 双模式切换（Native↔AT）需集成测试 |
| 3 | DeviceManager | 0% | >70% | 单元/集成 | 无测试，设备注册/注销/路由/过滤逻辑需覆盖 |
| 4 | ActionDispatcher | 0% | >70% | 单元/集成 | 无测试，16 种 Action 的 dispatch 逻辑需逐一验证 |
| 5 | Gh3036Manager | 0% | >70% | 单元/集成 | 无测试，帧解码/RPC 命令执行/CSV 写入需覆盖 |
| 6 | Dashboard 模块 | 0% | >50% | 单元 | `parser_scripts`/`json_config`/`commands` 无测试 |
| 7 | WebSocket 模块 | ~10% | >70% | 单元/集成 | 仅 `reconnection.rs` 有测试，`client`/`connection_pool`/`message_handler` 无测试 |
| 8 | EventBus | 0% | >50% | 单元 | 已实现但未使用且无测试 |
| 9 | State 模块 | 0% | >70% | 单元 | `app_state`/`persistence`/`types` 无测试 |
| 10 | Protocol 模块 | ~40% | >70% | 单元/集成 | `plugin_manager`/`lua_engine`/`hook_executor`/`script_loader` 有基础测试，缺少协议加载/绑定流程的集成测试 |

**已有测试的模块（14 个文件）：**

| 文件 | 测试状态 |
|------|---------|
| [error.rs](../../src-tauri/src/error.rs) | ✅ 5 个测试 |
| [cache.rs](../../src-tauri/src/device/cache.rs) | ✅ 6 个测试 |
| [serial_config.rs](../../src-tauri/src/device/serial/serial_config.rs) | ✅ 有测试 |
| [serial_manager.rs](../../src-tauri/src/device/serial/serial_manager.rs) | ✅ 有测试 |
| [at_parser.rs](../../src-tauri/src/device/ble/at/at_parser.rs) | ✅ 有测试 |
| [msgpack_handler.rs](../../src-tauri/src/service/msgpack_handler.rs) | ✅ 有测试 |
| [reconnection.rs](../../src-tauri/src/websocket/reconnection.rs) | ✅ 有测试 |
| [buffer.rs](../../src-tauri/src/waveform/buffer.rs) | ✅ 4 个测试 |
| [parser.rs](../../src-tauri/src/waveform/parser.rs) | ✅ 有测试 |
| [waveform commands](../../src-tauri/src/commands/waveform.rs) | ✅ 有测试 |
| [plugin_manager.rs](../../src-tauri/src/protocol/plugin_manager.rs) | ✅ 5 个测试 |
| [lua_engine.rs](../../src-tauri/src/protocol/lua_engine.rs) | ✅ 有测试 |
| [hook_executor.rs](../../src-tauri/src/protocol/hook_executor.rs) | ✅ 有测试 |
| [script_loader.rs](../../src-tauri/src/protocol/script_loader.rs) | ✅ 有测试 |

### 2.2 前端测试覆盖

| # | 组件名称 | 当前覆盖率 | 目标覆盖率 | 缺失测试类型 | 说明 |
|---|---------|-----------|-----------|-------------|------|
| 1 | SerialPage | 0% | >70% | 渲染/交互 | 无任何测试文件 |
| 2 | BlePage | 0% | >70% | 渲染/交互 | 无任何测试文件 |
| 3 | DashboardPage | 0% | >50% | 渲染/交互 | 无任何测试文件 |
| 4 | WaveformPage | 0% | >50% | 渲染/交互 | 无任何测试文件 |
| 5 | ProtocolPage | 0% | >50% | 渲染/交互 | 无任何测试文件 |
| 6 | 全部 Store | 0% | >70% | 单元 | 10 个 Zustand Store 无任何测试 |
| 7 | 全部 Hook | 0% | >70% | 单元 | 13 个 Hook 无任何测试 |
| 8 | API 层 | 0% | >70% | 单元 | 8 个 API 模块无任何测试 |

---

## 3. 已知问题清单

### 3.1 严重问题（Critical）— 可能导致崩溃或数据丢失

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| 1 | WebSocket 消息发送功能失效 | [client.rs:217-224](../../src-tauri/src/websocket/client.rs#L217-L224) | `handle_outgoing_message()` 仅检查连接状态不发送数据，`send_message()` 推入通道的消息被丢弃，WebSocket 发送功能完全不可用 |
| 2 | WebSocket 重连后接收循环丢失 | [client.rs:177-215](../../src-tauri/src/websocket/client.rs#L177-L215) | `reconnect()` 成功后未重新 `split()` WebSocket 流并启动接收循环，重连后无法接收任何新消息 |
| 3 | 全项目 125 处 `unwrap()` 调用 | 分布于 20 个源文件 | 违反项目规范"禁止使用 `unwrap()`/`expect()`（除非在测试代码）"，其中 AT 子模块 33 处、protocol 模块 41 处，生产代码中可能导致 panic |

### 3.2 高优先级问题（High）— 导致行为不正确

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| 1 | `broadcast_state_change` 每次分发发送完整状态 | [dispatcher.rs:515-528](../../src-tauri/src/state/dispatcher.rs#L515-L528) | 每次 Action 成功后序列化并发送完整 `AppState`，高频操作（如数据接收）会导致大量 JSON 序列化和前端重渲染开销 |
| 2 | SerialManager 使用 `std::sync::RwLock` 而 BleManager 使用 `tokio::sync::RwLock` | [serial_manager.rs:28](../../src-tauri/src/device/serial/serial_manager.rs#L28) vs [ble_manager.rs:98-103](../../src-tauri/src/device/ble/ble_manager.rs#L98-L103) | 锁机制不一致：SerialManager 使用同步锁（`std::sync::RwLock`/`Mutex`），BleManager 使用异步锁（`tokio::sync::RwLock`），混合使用可能导致死锁或性能问题 |
| 3 | `ChannelBuffer` 使用 `Vec::remove(0)` | [types.rs:59](../../src-tauri/src/state/types.rs#L59) | `Vec::remove(0)` 时间复杂度 O(n)，高频数据场景下性能低下；同问题存在于 [cache.rs:88](../../src-tauri/src/device/cache.rs#L88) |
| 4 | WebSocket `disconnect()` 未实际关闭连接 | [client.rs:226-231](../../src-tauri/src/websocket/client.rs#L226-L231) | `disconnect()` 仅修改状态为 `Disconnected`，未关闭底层 WebSocket 流或终止接收任务，连接实际仍保持 |
| 5 | `stateApi` 与 Zustand Store 双重状态源 | [stateApi.ts](../../src/api/stateApi.ts) + [stores/](../../src/stores/) | 后端 `AppState` 通过 `useAppState` hook 暴露，各 Zustand Store 独立管理状态，两套状态系统无同步机制，可能导致 UI 显示不一致 |
| 6 | DeviceManager WebSocket 发送未实现 | [device_manager.rs:251-253](../../src-tauri/src/device/device_manager.rs#L251-L253) | `send_direct()` 和 `send_to_device()` 对 `DeviceType::WebSocket` 返回错误，通过 DeviceManager 路由的 WebSocket 数据无法发送 |

### 3.3 中优先级问题（Medium）— 影响代码质量或可维护性

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| 1 | 错误处理使用手动 `Error` trait 实现而非 `thiserror` | [error.rs](../../src-tauri/src/error.rs) | 项目规范要求"错误处理使用 `thiserror` + 自定义 `Result` 类型"，当前手动实现 `Display`/`Error` trait，缺少 `From` 派生和错误链支持 |
| 2 | `configService` 使用 `localStorage` 而非 Tauri 原生存储 | [configService.ts](../../src/services/configService.ts) | 22 处 `localStorage` 调用，Tauri 应用应使用 `@tauri-apps/plugin-store` 或后端 `preferences` 命令，`localStorage` 在 WebView 清理时可能丢失 |
| 3 | `connectionStore` 僵尸代码 | [connectionStore.ts](../../src/stores/connectionStore.ts) | 完整定义了 `ConnectionInfo`/`WebSocketConnection` 类型和 CRUD 方法，但无组件直接使用，增加维护负担 |
| 4 | EventBus 已实现但未使用 | [event_bus.rs](../../src-tauri/src/service/event_bus.rs) | 完整实现了 `publish`/`subscribe`/`subscribe_channel` API，但全项目无调用点，属于死代码 |
| 5 | `waveformStore.startRefresh/stopRefresh` 仅设标志 | [waveformStore.ts:162-168](../../src/stores/waveformStore.ts#L162-L168) | 定时刷新逻辑未实现，`isRunning` 标志无实际作用 |
| 6 | GH3036 RPC 命令仅返回占位数据 | [manager.rs:434-622](../../src-tauri/src/gh3036/manager.rs#L434-L622) | 所有 RPC 命令解析参数后返回空 `Vec` 或占位数据，未与硬件交互 |
| 7 | `Gh3036Manager` 使用 `unsafe impl Send/Sync` | [manager.rs:165-166](../../src-tauri/src/gh3036/manager.rs#L165-L166) | 手动实现 `Send`/`Sync` 不安全，应通过设计确保线程安全而非强制标记 |
| 8 | PluginManager 使用 `unsafe impl Send/Sync` | [plugin_manager.rs:345-346](../../src-tauri/src/protocol/plugin_manager.rs#L345-L346) | 同上，`Arc<Mutex<>>` 本身已实现 `Send/Sync`，但 `LuaEngine` 可能不满足，强制标记可能隐藏线程安全问题 |

### 3.4 低优先级问题（Low）— 次要或外观问题

| # | 问题 | 位置 | 说明 |
|---|------|------|------|
| 1 | `dashboardStore.addDataPoint` 使用 `shift()` | [dashboardStore.ts:149-156](../../src/stores/dashboardStore.ts#L149-L156) | `Array.shift()` 时间复杂度 O(n)，应使用环形缓冲区 |
| 2 | `Gh3036Manager.get_rx_channel()` 硬编码返回 `None` | [manager.rs:370](../../src-tauri/src/gh3036/manager.rs#L370) | RX 通道配置未持久化，始终返回 `None` |
| 3 | `Gh3036Manager.is_library_linked()` 硬编码返回 `true` | [manager.rs:188-190](../../src-tauri/src/gh3036/manager.rs#L188-L190) | 未实际检测库链接状态 |
| 4 | `useBle` 中 `console.error`/`console.debug` 调用 | [useBle.ts:11](../../src/hooks/useBle.ts#L11) | 生产代码中存在 `console.error`/`console.debug` 调用，规范要求仅开发环境使用 |

---

## 4. 推荐实现优先级排序

### Phase 1：紧急修复（1-2 周）

> 目标：修复导致功能不可用的严重问题

| # | 任务 | 关联问题 | 预估工作量 |
|---|------|---------|-----------|
| 1 | 修复 WebSocket 消息发送功能 | Critical #1, High #6 | M |
| 2 | 修复 WebSocket 重连后接收循环丢失 | Critical #2 | M |
| 3 | 修复 WebSocket `disconnect()` 未关闭连接 | High #4 | S |
| 4 | 消除生产代码中的 `unwrap()` 调用 | Critical #3 | L |
| 5 | 实现 `waveformStore` 定时刷新逻辑 | 未实现 #1.2.1 | M |

### Phase 2：短期优先（2-4 周）

> 目标：补全核心功能缺失

| # | 任务 | 关联问题 | 预估工作量 |
|---|------|---------|-----------|
| 1 | 实现 DeviceManager WebSocket 设备支持 | 未实现 #1.1.5, #1.1.8 | M |
| 2 | 实现 stateApi ↔ Zustand Store 桥接 | 未实现 #1.2.2, High #5 | L |
| 3 | 实现 BLE AT 模式 Hook 操作 | 未实现 #1.2.4 | M |
| 4 | 优化 `broadcast_state_change` 增量更新 | High #1 | M |
| 5 | 替换 `ChannelBuffer`/`RingBuffer` 中 `Vec::remove(0)` 为 `VecDeque` | High #3 | S |
| 6 | 统一 SerialManager/BleManager 锁机制 | High #2 | M |

### Phase 3：中期改进（1-2 月）

> 目标：架构改进与测试覆盖

| # | 任务 | 关联问题 | 预估工作量 |
|---|------|---------|-----------|
| 1 | 迁移错误处理至 `thiserror` | Medium #1 | M |
| 2 | 迁移 `configService` 至 Tauri 原生存储 | Medium #2 | M |
| 3 | 集成 EventBus 替代直接 Tauri emit | Medium #4, 未实现 #1.1.4 | L |
| 4 | 清理 `connectionStore` 僵尸代码 | Medium #3 | S |
| 5 | 实现 GH3036 RX 通道配置和 RPC 实际执行 | 未实现 #1.1.6, #1.1.7, Medium #6 | L |
| 6 | 实现 WebSocket 心跳机制 | 未实现 #1.1.9 | S |
| 7 | 补充后端核心模块单元测试（SerialManager/BleManager/DeviceManager/ActionDispatcher） | 待测试 #2.1 | L |
| 8 | 补充前端核心组件测试（SerialPage/BlePage/Store/Hook） | 待测试 #2.2 | L |

### Phase 4：长期优化（2+ 月）

> 目标：质量提升与功能完善

| # | 任务 | 关联问题 | 预估工作量 |
|---|------|---------|-----------|
| 1 | Dashboard 实时数据流集成 | 未实现 #1.2.6 | M |
| 2 | 消除 `unsafe impl Send/Sync` | Medium #7, #8 | M |
| 3 | 补充 GH3036/Dashboard/WebSocket/EventBus 模块测试 | 待测试 #2.1 | L |
| 4 | 前端 E2E 测试框架搭建 | 待测试 #2.2 | L |
| 5 | 数据路由回调通知完善 | 未实现 #1.1.10 | S |
| 6 | 性能优化：高频数据场景下的状态更新策略 | High #1 | M |

---

## 附录 A：项目代码统计

### 后端模块统计

| 模块 | 文件数 | 有测试 | `unwrap()` 数量 | 关键问题 |
|------|--------|--------|----------------|---------|
| device/serial | 4 | ✅ 2/4 | 17 | 锁机制不一致 |
| device/ble | 8 | ✅ 1/8 | 33 | AT 子模块 unwrap 密集 |
| device/cache | 1 | ✅ 1/1 | 6 | `Vec::remove(0)` |
| device/device_manager | 1 | ❌ | 0 | WebSocket 未实现 |
| websocket | 4 | ✅ 1/4 | 0 | 发送功能失效 |
| protocol | 5 | ✅ 4/5 | 41 | unsafe Send/Sync |
| gh3036 | 4 | ❌ | 3 | RPC 占位实现 |
| waveform | 3 | ✅ 2/3 | 10 | - |
| dashboard | 4 | ❌ | 0 | - |
| state | 3 | ❌ | 2 | 完整状态广播 |
| service | 3 | ✅ 1/3 | 4 | EventBus 未使用 |
| error | 1 | ✅ 1/1 | 1 | 未使用 thiserror |

### 前端模块统计

| 模块 | 文件数 | 有测试 | 关键问题 |
|------|--------|--------|---------|
| stores | 10 | ❌ | waveformStore 定时刷新未实现，connectionStore 僵尸代码 |
| hooks | 13 | ❌ | useBle 缺 AT 方法，useWebSocket 发送不可用 |
| pages | 6 目录 | ❌ | 无任何测试 |
| api | 8 | ❌ | stateApi 与 Store 未桥接 |
| services | 2 | ❌ | localStorage 应迁移 |

---

## 附录 B：`unwrap()` 调用分布

全项目共 125 处 `unwrap()` 调用，分布于 20 个源文件：

| 文件 | 数量 | 紧急程度 |
|------|------|---------|
| at_backend.rs | 16 | 🔴 高 |
| plugin_manager.rs | 18 | 🔴 高 |
| hook_executor.rs | 9 | 🟡 中 |
| lua_engine.rs | 7 | 🟡 中 |
| script_loader.rs | 7 | 🟡 中 |
| serial_port.rs | 10 | 🔴 高 |
| adapter.rs | 9 | 🟡 中 |
| at_transport.rs | 6 | 🟡 中 |
| at_parser.rs | 6 | 🟡 中 |
| at_cache.rs | 5 | 🟡 中 |
| waveform/parser.rs | 5 | 🟢 低 |
| msgpack_handler.rs | 3 | 🟢 低 |
| csv_writer.rs | 3 | 🟢 低 |
| serial_manager.rs | 5 | 🟡 中 |
| cache.rs | 6 | 🟡 中 |
| commands/waveform.rs | 4 | 🟢 低 |
| buffer.rs | 1 | 🟢 低 |
| logger.rs | 1 | 🟢 低 |
| app_state.rs | 2 | 🟢 低 |
| serial_config.rs | 2 | 🟢 低 |

> 注：🟢 低 = 测试代码中的 unwrap 可接受；🟡 中 = 非关键路径；🔴 高 = 关键路径可能 panic
