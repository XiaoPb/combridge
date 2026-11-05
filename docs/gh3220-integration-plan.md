# GH3220 UProtocol 合入方案

| 项 | 内容 |
|---|---|
| 状态 | P1–P5 已实现（分支 `feat/gh3220-uprotocol`）；P6 波形/MCP 后置 |
| 目标工程 | ComBridge（`src-tauri` + React 前端） |
| 源工程 | `l-evk-t2-gh3020/client/`（commit `2eda9f4`） |
| 日期 | 2026-11-05 |
| 版本 | 计划 v1 |

## 0. 已确认决策

| 决策项 | 结论 |
|---|---|
| 合入范围 | 全栈打通：Rust 后端 + Tauri 命令 + React 页面 |
| 首期传输 | 仅 BLE（原生模式） |
| 串口 | 首期不做，架构预留扩展点 |
| TCP 调试通道 | 保留，默认关闭，仅用于接 `uprotocol-sim` 与自动化联调 |
| 波形 | 纳入方案，P6 后置实现 |
| MCP 工具域 | 纳入方案，P6 后置实现 |
| 协议库形态 | 本地 crate `libs/gh-uprotocol/`，以 path 依赖引入 |
| 移植范围 | 仅 `client/protocol/`；`uprotocol-host/`（CLI、C 模拟器、BLE Bridge）不进入产品工程 |

## 1. 目标与非目标

### 1.1 目标

1. 在 ComBridge 内完成 GH3220 UProtocol 的完整闭环：连接、设备信息、控制命令、数据解码、事件上报、CSV 落盘、实时展示。
2. 协议编解码与原工程保持逐字节一致，移植后可回归验证。
3. 与现有 GH3036、BLE、波形、MCP 体系解耦，新增功能不改变既有链路行为。
4. 为后续波形显示与 MCP 工具域预留稳定契约，后置实现时不改协议层。

### 1.2 非目标

1. 不实现串口传输（仅保留 `Link` 枚举扩展位）。
2. 不迁移 CLI（`uprotocol-host`）、不把 C 模拟器编译进 Tauri 产物。
3. 不改动 GH3036 的 RPC 核心、CSV 策略和页面。
4. 不实现 OTA（GH3220 OTA 不在本次范围）。

## 2. 源资产盘点

### 2.1 迁入内容

| 文件 | 行数 | 职责 | 移植动作 |
|---|---:|---|---|
| `protocol/src/frame.rs` | 185 | `AA 11 CMD LEN PAYLOAD CRC8` 编解码、流式 `FrameDecoder` | 原样迁入 |
| `protocol/src/crc8.rs` | 55 | 表驱动 CRC8（poly 0x07 / init 0xFF） | 原样迁入 |
| `protocol/src/cmd.rs` | 208 | 20 条命令构造与响应解析 | 原样迁入 |
| `protocol/src/data/mod.rs` | 497 | `0x0B` 数据包、包头、普通帧、结果块、拼接 | 原样迁入 |
| `protocol/src/data/zip.rs` | 226 | 4bit 差分解码 | 原样迁入 |
| `protocol/src/event.rs` | 35 | `0x16` 事件解析与 ACK | 原样迁入 |
| `protocol/src/error.rs` | 31 | 错误类型 | 原样迁入 |
| `protocol/src/lib.rs` | 61 | 命令号、版本类型、芯片控制常量 | 原样迁入 |
| `protocol/Cargo.toml` | - | crate 定义 | 补充 workspace 无关字段 |
| `protocol/README.md` | - | 使用说明 | 更新路径与工程说明 |

合计约 1300 行，17 个内联单测；生产代码无 `unsafe`、无 `unwrap`，仅依赖 `thiserror 2`，与 ComBridge 现有依赖兼容。

### 2.2 不迁入内容

| 内容 | 原因 |
|---|---|
| `uprotocol-host/src/*` | Clap CLI，产品内由 Tauri 命令与页面替代 |
| `uprotocol-host/server/*` | SDK 原始 C 模拟器，属于联调工具，不进产物 |
| `uprotocol-host/tools/ble_bridge.py` | ComBridge 已有原生 BLE 栈，不需要 TCP 桥 |
| `uprotocol-host/tests/e2e.rs` | 依赖外部模拟器进程，改为本地可选联调脚本 |
| `target/`、`Cargo.lock` | 构建产物与库锁文件不入库 |

## 3. 现有工程接入点

| 位置 | 现状 | 本次改动 |
|---|---|---|
| `src-tauri/Cargo.toml:51-52` | 已有 `gh-rpc`、`rpc` 的 path 依赖 | 新增 `gh-uprotocol` path 依赖 |
| `src-tauri/src/lib.rs:1-13` | 模块声明 | 新增 `pub mod gh3220;` |
| `src-tauri/src/lib.rs:113-116` | GH3036 管理器初始化 | 新增 `Gh3220Manager::new(...)` 并注入 |
| `src-tauri/src/lib.rs:149` | `.manage(gh3036_manager)` | 新增 `.manage(gh3220_manager)` |
| `src-tauri/src/lib.rs:274-280` | EventBridge 前缀过滤 | 增加 `"gh3220:"` |
| `src-tauri/src/lib.rs:311-465` | Tauri 命令注册表 | 增加 `commands::gh3220::*` |
| `src-tauri/src/commands/mod.rs` | 命令子模块 | 新增 `pub mod gh3220;` |
| `src-tauri/src/service/event_bus.rs:519-551` | 事件主题常量 | 新增 `GH3220_*` 主题 |
| `src-tauri/src/state/types.rs:619-651` | `Preferences` | 新增 `gh3220` 配置节 |
| `src-tauri/src/commands/preferences.rs` | 偏好命令 | 新增 GH3220 偏好读写命令 |
| `src/App.tsx:23,278` | 懒加载与路由 | 新增 `/gh3220` |
| `src/components/Layout/Sidebar.tsx:70` | 菜单项 | 新增 GH3220 菜单 |
| `src/components/MenuVisibilityConfigModal.tsx:23,35,45` | 菜单可见性 | 新增 `gh3220` 键与子页签 |
| `src/stores/menuVisibilityStore.ts` | 菜单键类型与默认值 | 新增 `gh3220` |
| `src/stores/pageTabsStore.ts` | 页面页签状态 | 新增 `gh3220ActiveTab` |
| `src/stores/index.ts:8` | Store 导出 | 新增 `gh3220Store` |
| `src/api/events.ts:141-147` | 前端事件主题映射 | 新增 GH3220 主题 |
| `docs/architecture/backend/` | 模块文档 | 新增 `gh3220-module.md` |
| `.github/workflows/build.yml` | 只构建不跑测试 | 增加协议 crate 测试步骤 |

## 4. 总体设计

### 4.1 目录结构

```
libs/gh-uprotocol/                     # 协议编解码 crate（纯字节，无 I/O）
  Cargo.toml
  README.md
  UPSTREAM.md                          # 来源仓库、commit、移植说明
  src/{lib,frame,crc8,cmd,error,event}.rs
  src/data/{mod,zip}.rs

src-tauri/src/gh3220/
  mod.rs                               # 导出、Gh3220ManagerRef
  types.rs                             # 配置、状态、DTO、统计
  link.rs                              # BleLink / TcpLink（调试）
  session.rs                           # 命令串行化、请求-响应、超时
  manager.rs                           # 生命周期、解码任务、ACK、采集、统计
  csv_writer.rs                        # 按功能分文件、变宽列

src-tauri/src/commands/gh3220.rs       # Tauri IPC 命令

src/pages/Gh3220/
  index.tsx                            # 页面与页签
  ConnectTab.tsx
  ControlTab.tsx
  CaptureTab.tsx
  EventTab.tsx
  DebugTab.tsx                         # 仅开发构建显示
src/stores/gh3220Store.ts
src/api/gh3220.ts
src/types/gh3220.ts
src/locales/{zh-CN,en-US}/gh3220.json
```

### 4.2 分层职责

```
┌─────────────────────────────────────────────────────┐
│ 前端：页面 / Zustand / Tauri invoke / 事件订阅        │
└───────────────┬─────────────────────────────────────┘
                │ invoke / event
┌───────────────▼─────────────────────────────────────┐
│ commands/gh3220.rs：参数校验、错误映射、DTO 转换      │
└───────────────┬─────────────────────────────────────┘
                │
┌───────────────▼─────────────────────────────────────┐
│ Gh3220Manager：会话状态、解码任务、采集、CSV、统计    │
│   ├─ session.rs：命令串行队列 + oneshot + 超时        │
│   ├─ link.rs：BLE 分片写 / TCP 调试读写              │
│   └─ csv_writer.rs：按 function_id 分文件落盘         │
└───────────────┬─────────────────────────────────────┘
                │ gh-uprotocol（纯编解码）
┌───────────────▼─────────────────────────────────────┐
│ gh-uprotocol：Frame / FrameDecoder / DataDecoder     │
└─────────────────────────────────────────────────────┘
```

### 4.3 数据流

```
上行：
BLE 通知 → BleManager 发布 ble:data
  → Gh3220Manager 入口任务（过滤 address + notify_uuid）
  → FrameDecoder
      ├─ 0x0B：DataDecoder → 记录批 → gh3220:data
      │        → CSV / 波形 / 前端
      ├─ 0x16：事件解析 → 自动 ACK → gh3220:event
      └─ 其他：匹配 pending 请求 → 唤醒调用方

下行：
前端 invoke → command → session.request
  → Frame::encode
  → BleLink（按 MTU 分片、串行写）
```

## 5. 详细设计

### 5.1 协议库

- 保留 crate 名称 `gh-uprotocol` 与全部公开 API，减少后续与源工程同步成本。
- 仅做两处工程化调整：
  1. 删除源仓库的 `Cargo.lock` 与 `target/`；
  2. README 中补 ComBridge 接入说明与上游 commit。
- 公开 API 保持：`Frame`、`FrameDecoder`、`DataDecoder`、`PacketHeader`、`Record`、`cmd::Request`、`cmd::parse_*`、`event::parse/ack`。
- 单测原样保留，作为合入后的第一道回归。

### 5.2 传输层（link.rs）

首期实现两种链路：

| 链路 | 用途 | 说明 |
|---|---|---|
| `BleLink` | 正式功能 | 走 `BleManager`，写 notify 特征与订阅 |
| `TcpLink` | 调试 | 连接 `uprotocol-sim`，默认关闭 |

BLE 固定参数：

| 项 | 值 |
|---|---|
| Service UUID | `0000190e-0000-1000-8000-00805f9b34fb` |
| 写特征 | `00000004-0000-1000-8000-00805f9b34fb` |
| 通知特征 | `00000003-0000-1000-8000-00805f9b34fb` |
| 广播名过滤 | 前缀 `GH3220`（辅助识别，不作为唯一条件） |
| 默认分片 | `min(MTU-3, 244)`，MTU 未知时取 20 |

关键实现约束：

1. **连接后必须先发现**：`BleManager::connect` 不会填充特征缓存；`write_characteristic` / `subscribe_notify` 都依赖 `find_characteristic`。固定流程：
   `connect → discover_services → discover_characteristics(service) → subscribe_notify(notify_uuid)`。
2. **写特征必须分片**：`gatt_client.write_characteristic` 是整包写，不分片。UProtocol 单帧最大 243B，必须按 `mtu-3` 切片，并用异步锁保证同一条命令的多个分片不被其他写打断。
3. **避免重复订阅**：`subscribe_notify` 每次调用都会新建一个通知任务，重复订阅会导致同一份数据被投递多次。订阅前先查 `get_subscriptions(address)`，已订阅则跳过。
4. **MTU 获取**：Windows 上 `set_mtu` 为 no-op；通过 `max_write_len` 查询，失败回退 20。分片大小在连接建立后固定，不在运行中变化。
5. **仅原生模式**：首期要求 BLE 处于 Native 模式；AT 模式下 notify/写语义不同，直接返回明确错误。

TCP 调试通道：

- 仅在偏好 `debugTcp.enabled = true` 时允许创建，默认 `false`。
- 默认地址 `127.0.0.1:9000`，只允许回环地址；非回环需显式开启。
- 打开后与 BLE 链路互斥：同一时间只允许一个 `Gh3220Link`。
- 前端只在开发构建（`import.meta.env.DEV`）显示调试页签。

### 5.3 会话层（session.rs）

协议响应没有序列号，只能串行请求：

```
request(cmd, payload, timeout):
  acquire command_lock (async Mutex)
  register oneshot by cmd in pending
  link.send(frame.encode())
  await oneshot with timeout (默认 800ms)
  clear pending
  return Frame 或 Timeout/Disconnected
```

规则：

1. 同一时刻只允许一个未完成请求；超出排队等待，不并发下发。
2. `0x0B`、`0x16` 不参与请求匹配，直接进入数据/事件通道。
3. 收到未知响应时记录 debug 日志并计数，不影响后续命令。
4. 断连时立即失败所有等待者，返回 `Disconnected`，避免 UI 卡在超时。
5. 每个命令的超时可单独覆盖，默认 800ms；`LOAD_REG_LIST` 可按长度放大。

### 5.4 解码与事件

- 入口任务由 `mpsc` 驱动，`FrameDecoder` / `DataDecoder` 只在该任务内使用，避免锁竞争。
- `DataDecoder::chnl_override` 支持 UI 按功能覆盖通道数；默认按包头掩码 popcount。
- 记录时间戳取“收到该帧”的本地毫秒时间，写入 DTO。
- 每功能维护独立 `DataDecoder` 状态？——不。协议层 `DataDecoder` 内部已按 `function_id` 维护 zip/splice/frame 状态，共用一个实例即可。
- `0x16` 事件：解析 `(event_id, report_id)` 后立即编码 ACK 并下发；ACK 失败只告警，不阻塞数据流。
- 统计：`frames / crc_errors / resyncs / bad_len / packets / records / splice_fragments / lost_frames` 定期快照。
- `0x2C` 功能信息畸形短包属于 SDK 已知行为，重同步后继续；不得上报为错误。

### 5.5 会话状态机

```
Disconnected
  └─ connect() ──► Connecting
                      ├─ 失败 ──► Disconnected（返回错误）
                      └─ 成功 ──► Ready
Ready
  ├─ startCapture() ──► Capturing（解码与 CSV 同时开启）
  ├─ disconnect() ───► Disconnected
  └─ 链路断开事件 ───► Disconnected（清理 pending/解码状态）
Capturing
  ├─ stopCapture() ──► Ready（flush CSV，不自动 STOP 设备）
  └─ 链路断开 ──────► Disconnected
```

说明：

- `Ready` 状态下解码任务已经运行，但默认不写 CSV；采集开关只控制落盘与前端批量上报。
- `STOP` 与 `startCapture` 解耦：停止采集不等于向设备发 STOP，避免误触发“STOP 后无法直接 START”的固件限制。
- 设备 `MAX_CONN=1`，重连前必须先断开旧连接；UI 在连接失败时提示先执行 BLE 断开。

### 5.6 数据上报与节流

- 批量窗口：60ms 或累计 100 条，先到者触发。
- 单批 payload 建议不超过 256KB；超出则再切批。
- 前端只消费批量事件，不逐条渲染；采集页显示最近值，波形页消费环形缓冲。
- 统计事件 1s 一次，仅在页面订阅或采集进行中发布。

### 5.7 CSV

按 `function_id` 分文件，避免不同功能通道数不同导致列错位：

```
gh3220_<funcname>_<yyyyMMdd_HHmmss>.csv
```

列定义（前缀与 CLI 工具保持一致，便于对比）：

```
func_id,frame_id,raw0..rawN,agc0..agcN,gs_x,gs_y,gs_z,ts_ms,algo,
cap0..cap3,temp0..temp3,flag2,flag3,flag4,slot_cfg
```

规则：

1. 首条记录确定该功能的 `raw/agc` 宽度；后续出现更宽记录时，关闭当前文件并新建 `_partN`，不回头改表头。
2. 可选字段缺失时写空列，保持列数稳定。
3. `algo` 编码为 `key:value;key:value`，与 CLI 一致。
4. 写入使用 `BufWriter`，每批 flush；空闲超过阈值关闭句柄，下一条记录重新打开新文件。
5. 断开连接、停止采集、应用退出时强制 flush + close。

### 5.8 配置与偏好

在 `Preferences` 中新增：

```rust
pub struct Gh3220Preferences {
    pub csv: Gh3220CsvPreferences,           // enabled / output_dir / idle_close_secs
    pub capture: Gh3220CapturePreferences,   // channels_override / auto_stop
    pub debug_tcp: Gh3220DebugTcpPreferences,// enabled=false / addr=127.0.0.1:9000
}
```

- 所有字段使用 `#[serde(default = "...")]`，旧版 `preferences.json` 可无损升级。
- 默认输出目录与 GH3036 保持一致的解析规则（相对路径按可执行文件目录展开）。
- `debug_tcp.enabled` 默认 `false`，不在正式 UI 暴露；仅开发构建的调试页签可切换。

### 5.9 错误处理

新增 `Gh3220Error`（`thiserror`），统一映射到 `ComBridgeError`：

| 错误 | 触发 | 前端提示 |
|---|---|---|
| `NotConnected` | 未连接就发命令 | 请先连接设备 |
| `Busy` | 命令队列异常 | 设备忙，请重试 |
| `Timeout { cmd }` | 800ms 无响应 | 命令 0xXX 超时 |
| `Status { code }` | 设备返回 1 / 0xFF | 设备拒绝 / 不支持 |
| `Ble(String)` | 链路错误 | 透传 BLE 错误 |
| `Protocol(String)` | 解码/参数错误 | 协议错误 |
| `DebugTcpDisabled` | 未开启调试通道 | 调试通道未启用 |

## 6. 接口契约

### 6.1 Tauri 命令

| 命令 | 入参 | 返回 |
|---|---|---|
| `gh3220_connect` | `config: Gh3220LinkConfig` | `Gh3220Status` |
| `gh3220_disconnect` | - | `()` |
| `gh3220_get_status` | - | `Gh3220Status` |
| `gh3220_get_device_info` | - | `Gh3220DeviceInfo` |
| `gh3220_set_work_mode` | `mode: u8, func_mask: u32` | `u8` |
| `gh3220_start` | `func_mask: u32, slot_en: u8` | `u8` |
| `gh3220_stop` | `func_mask: u32` | `u8` |
| `gh3220_switch_slot_func` | `slot: u8, extra: u8, func_mask: u32, on: bool` | `u8` |
| `gh3220_reg_read` | `addr: u16, count: u8` | `Vec<u16>` |
| `gh3220_reg_write` | `addr: u16, values: Vec<u16>` | `()` |
| `gh3220_load_reg_list` | `file_path: String` | `u8` |
| `gh3220_chip_ctrl` | `action: String` | `u8` |
| `gh3220_set_fifo_thr` | `value: u16` | `u8` |
| `gh3220_set_report_event` | `mask: u16` | `u8` |
| `gh3220_debug_status` | `kind: String` | `Gh3220DebugStatus` |
| `gh3220_package_test` | `hex: String` | `Vec<u8>` |
| `gh3220_send_raw` | `hex: String` | `()` |
| `gh3220_capture_start` | `config: Gh3220CaptureConfig` | `()` |
| `gh3220_capture_stop` | - | `()` |
| `gh3220_get_capture_status` | - | `Gh3220CaptureStatus` |
| `gh3220_get_stats` | - | `Gh3220Stats` |
| `gh3220_clear_stats` | - | `()` |

约定：

- 设备状态码 `0/1/0xFF` 原样返回 `u8`，由前端统一翻译文案。
- 十六进制入参统一接受 `AA11...`、`AA 11 ...`、`0xAA11...` 三种写法。
- 所有命令在未连接时返回 `NotConnected`，不做隐式连接。

### 6.2 事件

| 主题 | 载荷 |
|---|---|
| `gh3220:status` | `{ state, address, name, error? }` |
| `gh3220:data` | `{ seq, records: Gh3220Record[], dropped }` |
| `gh3220:event` | `{ event_id, report_id, acked, ts }` |
| `gh3220:stats` | `Gh3220Stats` |
| `gh3220:error` | `{ code, message }` |

### 6.3 核心 Rust 类型

```rust
pub enum Gh3220LinkKind { Ble, Tcp }

pub struct Gh3220LinkConfig {
    pub kind: Gh3220LinkKind,
    pub address: String,          // BLE 地址或 host:port
    pub write_uuid: Option<String>,
    pub notify_uuid: Option<String>,
    pub mtu: Option<u16>,
}

pub enum Gh3220State { Disconnected, Connecting, Ready, Capturing }

pub struct Gh3220Record {
    pub function_id: u8,
    pub frame_id: u8,
    pub raw: Vec<u32>,
    pub agc: Vec<u32>,
    pub gs: Option<[i16; 3]>,
    pub gyro: Option<[i16; 3]>,
    pub cap: Option<[u32; 4]>,
    pub temp: Option<[u32; 4]>,
    pub flags: [u32; 5],
    pub algo: Vec<(u8, i32)>,
    pub ts_ms: u64,
}

pub struct Gh3220Stats {
    pub frames: u64,
    pub crc_errors: u64,
    pub resyncs: u64,
    pub bad_len: u64,
    pub packets: u64,
    pub records: u64,
    pub splice_fragments: u64,
    pub splice_aborts: u64,
    pub lost_frames: u64,
}
```

## 7. 文件变更清单

### 7.1 新增

| 路径 | 说明 |
|---|---|
| `libs/gh-uprotocol/**` | 协议 crate（约 1300 行 + 测试） |
| `src-tauri/src/gh3220/mod.rs` | 模块入口 |
| `src-tauri/src/gh3220/types.rs` | DTO/配置/状态/统计 |
| `src-tauri/src/gh3220/link.rs` | BLE/TCP 链路 |
| `src-tauri/src/gh3220/session.rs` | 命令串行与超时 |
| `src-tauri/src/gh3220/manager.rs` | 核心管理器 |
| `src-tauri/src/gh3220/csv_writer.rs` | CSV 落盘 |
| `src-tauri/src/commands/gh3220.rs` | Tauri 命令 |
| `src/pages/Gh3220/*` | 页面与页签组件 |
| `src/stores/gh3220Store.ts` | Zustand 状态 |
| `src/api/gh3220.ts` | 前端 API |
| `src/types/gh3220.ts` | 前端类型 |
| `src/locales/zh-CN/gh3220.json` | 中文文案 |
| `src/locales/en-US/gh3220.json` | 英文文案 |
| `docs/architecture/backend/gh3220-module.md` | 模块架构文档 |
| `docs/protocols/gh3220/uprotocol-reference.md` | 协议规范副本 |
| `libs/gh-uprotocol/UPSTREAM.md` | 上游来源与同步说明 |

### 7.2 修改

| 路径 | 改动 |
|---|---|
| `src-tauri/Cargo.toml` | 新增 path 依赖 |
| `src-tauri/src/lib.rs` | 模块声明、管理器初始化、manage、事件前缀、命令注册 |
| `src-tauri/src/commands/mod.rs` | 新增子模块 |
| `src-tauri/src/service/event_bus.rs` | 新增主题常量 |
| `src-tauri/src/state/types.rs` | `Preferences` 新增 `gh3220` 节与菜单键 |
| `src-tauri/src/commands/preferences.rs` | 新增 GH3220 偏好命令 |
| `src/App.tsx` | 懒加载 + 路由 |
| `src/pages/index.ts` | 导出（如页面走统一导出） |
| `src/components/Layout/Sidebar.tsx` | 菜单项 |
| `src/components/MenuVisibilityConfigModal.tsx` | 可见性键与子页签 |
| `src/stores/menuVisibilityStore.ts` | 菜单键类型与默认值 |
| `src/stores/pageTabsStore.ts` | 页签状态 |
| `src/stores/index.ts` | 导出 |
| `src/api/events.ts` | 事件主题映射 |
| `src/locales/*/sidebar.json` | 菜单文案 |
| `docs/architecture/README.md` | 模块索引（如需要） |
| `CLAUDE.md` | 架构模块说明补充 |
| `.github/workflows/build.yml` | 增加协议 crate 测试步骤 |
| `package.json` / `src-tauri/Cargo.toml` | 版本号 `0.9.0` |

## 8. 实施阶段

### P0 基线与契约（无功能改动）

- 固化本文档；确认类型命名、事件主题、命令签名不再变动。
- 在源工程打 tag/记录 commit，作为后续同步基线。
- 验收：文档评审通过。

### P1 协议库入库

- 复制 `protocol/` 到 `libs/gh-uprotocol/`，删除 `target/`、`Cargo.lock`。
- 新增 `UPSTREAM.md`；更新 README；`src-tauri/Cargo.toml` 增加 path 依赖。
- 验收：
  - `cargo test --manifest-path libs/gh-uprotocol/Cargo.toml` 17 项全绿；
  - `cargo check`（src-tauri）通过；
  - `cargo clippy` 无新增警告。

### P2 链路与会话

- 实现 `link.rs`：BLE 分片写、连接后服务/特征发现、防重复订阅；TCP 调试链路。
- 实现 `session.rs`：串行队列、oneshot、超时、断连失败。
- 实现 `manager.rs` 骨架与入口任务。
- 验收（单元测试 + 模拟器）：
  - 分片边界（20/244 字节）；
  - 半帧/粘包/噪声重同步；
  - 超时与断连；
  - TCP 接 `uprotocol-sim` 可完成 `GET_VER` / `GET_MAX_LEN`。

### P3 协议能力与采集

- 20 条命令封装；状态码解析。
- `0x16` 自动 ACK；`0x0B` 解码；统计。
- CSV 写入、空闲关闭、断开 flush。
- 验收：
  - 模拟器连续采集 60s，记录数、通道宽度、CRC 统计正确；
  - 真机完成 `WORK_MODE → START → 采集 → STOP`；
  - 事件只上报一次（ACK 生效）。

### P4 后端接线

- `commands/gh3220.rs` 全量命令；错误映射。
- `lib.rs` 初始化、manage、EventBridge 前缀、命令注册。
- 偏好接入；启动期不自动连接设备。
- 验收：命令可从前端调用；断开后状态、pending、CSV 均正确清理。

### P5 前端

- 页面五个页签：连接、控制、采集、事件、调试（仅开发）。
- Store、API、类型、i18n、菜单、路由、页签状态。
- 验收：
  - 扫描/连接/断开；
  - 设备信息展示；
  - 控制命令与状态码提示；
  - 实时记录与事件日志；
  - 采集开关与 CSV 路径展示；
  - 中英文文案完整。

### P6 后置能力

- 波形：每 `function_id` 创建独立 buffer，列为 `raw0..rawN`，批量 append；页面复用现有波形组件。
- MCP：新增 GH3220 工具组（连接状态、设备信息、启停、采集、统计），沿用现有确认门与审计。
- 验收：波形可实时滚动；MCP 工具在关闭状态返回明确错误，不越权自动连接。

### P7 收尾

- 文档、CI、版本号、发布说明。
- 真机验收清单全部通过。
- 验收：三平台构建通过；既有 GH3036/BLE/OTA 功能回归通过。

## 9. 测试计划

### 9.1 单元测试

| 范围 | 用例 |
|---|---|
| 协议库 | 原有 17 项全部保留 |
| 分片 | 20/244 边界、空 payload、超长 payload |
| 会话 | 串行、超时、断连唤醒、未知响应 |
| 解码 | 半帧、粘包、噪声、CRC 错、拼接包、压缩帧 |
| 事件 | ACK 字节、重复事件去重计数 |
| CSV | 变宽、缺列、algo 编码、空闲重开 |

### 9.2 集成测试

- Mock Link：用固定字节序列驱动 manager，断言命令字节、超时、统计。
- TCP + C 模拟器（可选，本地执行）：
  - 构建 `uprotocol-host/server`；
  - `uprotocol-sim --listen 127.0.0.1:9000 --channels 2 --rate 25`；
  - ComBridge 开启调试 TCP，执行 info/capture，与 `--truth` 对比前 8 列。
- 该测试不进入 CI，避免把 C 工具链引入构建。

### 9.3 真机验收

1. 扫描并连接 `GH3220` 设备。
2. 读取 FW/协议/芯片/驱动版本、最大包长、已开功能、链路状态。
3. `WORK_MODE(hr)` → `START(hr)`，确认数据流。
4. 采集 30s，检查 CSV 记录数、通道宽度、数值范围。
5. 触发事件，确认日志只有一条且设备不再重发。
6. `STOP` 后检查状态提示；复位后重新连接成功。
7. 断开重连，确认无重复订阅、无 pending 卡死。

### 9.4 回归

- `cargo test`（现有后端测试）
- `npm test`（Vitest）
- BLE 页面、GH3036 页面、OTA、波形手工冒烟

## 10. 风险与对策

| 风险 | 影响 | 对策 |
|---|---|---|
| BLE 不分片 | 控制命令失败，`LOAD_REG_LIST` 必失败 | link 层按 MTU 切片 + 写锁 |
| 未发现特征就订阅 | 连接成功但无数据 | 固定连接流程，失败即回滚断开 |
| 重复订阅 | 同一数据被投递多次 | 订阅前查 `get_subscriptions` |
| 协议无序列号 | 并发命令错配 | 强制串行队列 |
| 事件未 ACK | 设备 500ms 重传 | 自动 ACK，失败仅告警 |
| 前端高频渲染 | 卡顿、丢帧 | 60ms/100 条批量上报 |
| 长采集内存增长 | CSV 缓冲膨胀 | 按功能分文件，流式写入 |
| 固件 STOP 限制 | 停止后无法直接启动 | UI 明确提示，不复位不自动恢复 |
| `0x2C` 畸形包 | 误判为故障 | 文档标注 + 统计只计数 |
| 通知回调被覆盖 | 与 BLE 页面互相干扰 | 单次订阅 + 事件总线解耦 + 文档说明 |
| 许可证 | 合规风险 | 沿用上游 `MIT OR Apache-2.0` 声明，合入前由负责人确认 |

## 11. 发布与回滚

- 版本：`0.9.0`，提交信息 `feat(gh3220): 合入 GH3220 UProtocol 全栈支持`。
- 回滚策略：功能为纯增量。
  - 前端：移除路由与菜单即可隐藏；
  - 后端：不初始化 `Gh3220Manager`、不注册命令；
  - 协议 crate 与现有模块无耦合，可独立移除。
- 数据兼容：`Preferences` 新节全部带默认值，旧配置可直接加载；回滚后旧版本会忽略 `gh3220` 字段。

## 12. 实施记录（2026-11-05）

### 12.1 验证结果

- 协议 crate：17 项单测通过。
- 新模块：9 项单测通过；TCP + SDK 模拟器端到端通过。
- 真机 BLE（GH3220_Nordic）：
  - 版本读取：`GH(M)3X2X_DEMO_v1.6` / `Health_Uprotocol_v0.4` / `v12_ev04` / `v4.3.0.0`
  - `max_len=238`、`opened_func=0x2`、链路正常
  - 8s 采集：212 条记录，56 帧，CRC/重同步/坏包/丢帧均为 0
  - CSV：func1 为 4 通道 + 4 AGC，func0 为单通道，列宽与数值正常
- 前端：`tsc`、`vite build`、206 项 Vitest 通过。

### 12.2 实施中发现并修正的问题

1. **订阅就绪竞态**：`BleManager::subscribe_notify` 返回时 CCCD 可能未生效，立即下发命令会丢失第一条响应，表现为 `GET_VER` 超时。已在 link 层等待首条通知（最长 1.5s）。
2. **命令超时偏短**：真机持续上报时 800ms 偶发超时，默认调整为 2s。
3. **BLE 分片**：真机写入上限实测 239B，按 `max_write_len` 分片；CSV 中 4 通道数据完整。
4. **构建环境**：本机无 root，Tauri 依赖通过 `apt-get download` + `dpkg -x` 解压到用户目录 sysroot 后完成编译。

### 12.3 未完成

- P6 波形 / MCP 仍为后置。
- 页面 UI 未在真机桌面环境完整手工走查（后端与命令层已通过真机测试）。

## 13. 附录

### 13.1 协议行为清单（实现时不得改动）

1. 无参命令补 1 字节（设备拒绝 `LEN==0`）。
2. `START_CTRL`：`0=启动`，`1=停止`。
3. `SWITCH_SLOT_FUNC`：`0=开`，`1=关`。
4. `WORK_MODE` 是 `START` 的隐式前置条件。
5. 通道数按掩码 popcount。
6. `package_type.bit1` 表示本包首帧为绝对帧，跨包保留。
7. 每功能首帧可能出现一个畸形 `0x2C` 包，重同步丢弃。
8. `0x16` 必须 ACK。
9. 真机 STOP 后不能直接 START；`wakeup` 会清空 WORK_MODE。
10. `GET_OPENED_FUNC` 不能用于判断当前采样状态。

### 13.2 命令号

见 `libs/gh-uprotocol/src/lib.rs` 的 `command`、`ver_type`、`chip_ctrl` 三个模块，合入后不在上层重复定义。

### 13.3 待办

- 合入前确认上游代码许可证归属与署名方式。
- 确认产品侧是否需要把 GH3220 加入首页快捷入口（当前计划只加侧边栏）。
