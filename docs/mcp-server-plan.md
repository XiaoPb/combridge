# ComBridge MCP Server 设计与实施计划（最终版）

| 项 | 内容 |
|---|---|
| 文档版本 | v1.0 FINAL |
| 日期 | 2026-11-04（§6.4 / §8.3~§8.6 / §9.0 随 P4、P5a、P5b 落地同步更新） |
| 适用代码版本 | ComBridge v0.7.2（Tauri 2.0，后端 Rust 10,521 行） |
| 状态 | 方案定稿；P0~P5 已实现（含设置页与 i18n），P3 真机验收完成，P6 待排期 |
| 决策人 | 产品 + 研发 |

---

## 0. 决策摘要

三条产品决策已确认，并已固化进本方案：

| # | 决策 | 对方案的直接影响 |
|---|---|---|
| D1 | **内部研发为主，终端客户也可用，提供开关** | 开关默认关闭；首次开启时选「研发档 / 标准档」，两档的安全策略不同（§8） |
| D2 | **需要 AI 驱动产测，但只需要出结果** | 产测工具从三段式（start/status/result）改为**同步出结果**的 `run` 语义，内部轮询、一次返回结论（§6.4） |
| D3 | **headless 二进制不随主包发布，改为可选下载** | 主安装包体积不变；headless 作为独立可选组件从 release 资产下载（§9.3） |

**核心结论：**

1. **可行**。后端 65% 的代码与 UI 无关，仅 4 个文件依赖 Tauri，改造成本低于同规模 Tauri 项目的常规水平。
2. **必须做在后端 Rust 侧**。前端方案不是"不推荐"，是**技术上不成立**（§3.1）。
3. **两阶段交付**：Phase 1 内嵌 HTTP MCP（约 1.5~2 周出可用版）→ Phase 2 独立 headless 二进制（可选下载）。
4. **16 个域级工具**，不做 132 个命令的 1:1 映射。
5. **真正的风险不在技术，在安全与并发**：LLM 直接操作硬件、多客户端抢占串口、误刷固件。

---

## 1. 背景与目标

### 1.1 要解决的问题

ComBridge 目前的所有能力（串口、BLE、GH3036 RPC、产测、波形、OTA、Lua 协议插件）只能通过 GUI 操作。希望把这些能力以 MCP（Model Context Protocol）形式释放，让 AI Agent 能够：

- 自主完成串口联调（打开端口 → 发指令 → 等待回包 → 判断结果）
- 自主完成 BLE 设备探索与数据采集
- 驱动 GH3036 产测并拿到结论
- 读取波形数据并做分析
- 在受控条件下执行 OTA

### 1.2 非目标

以下明确**不做**，避免范围蔓延：

- 不做"AI 自动改 UI 配置"（preferences / 窗口 / 主题）
- 不做进程与系统级操作（`open_url`、`show_in_folder`、`devtools`、`download_update`）
- 不做多机远程调度（只绑 loopback）
- Phase 2 之前不做 daemon 化（GUI 与 headless 不并存共享设备）

### 1.3 成功标准

| 编号 | 标准 | 验证方式 |
|---|---|---|
| S1 | 模型可只凭工具描述完成「打开串口 → 发 AT → 收到 OK」 | 剧本 A，零人工干预 |
| S2 | 模型可完成「BLE 扫描 → 连接 → 订阅 → 采 20 帧」 | 剧本 B |
| S3 | 模型可完成「GH3036 初始化 → 读版本 → 跑产测 → 给出 pass/fail」 | 剧本 C |
| S4 | OTA 在未确认时**不可能**执行 | 剧本 D，且审计日志可证 |
| S5 | 两个 AI 客户端同时操作同一端口时，第二个得到明确 `PORT_BUSY` 而非数据错乱 | 并发测试 |
| S6 | 单次工具调用的 schema + 描述总占用 < 10k tokens | `tools/list` 响应体量测量 |

---

## 2. 源码勘察结论

### 2.1 现有分层

```
src-tauri/src/
├── device/      SerialManager / BleManager(native + AT)    ← 硬件抽象
├── service/     EventBus(tokio broadcast) / EventBridge / MsgPack / Logger
├── state/       AppState + ActionDispatcher + Persistence
├── protocol/    mlua Lua 5.4 插件引擎（Arc<Mutex<Lua>>）
├── gh3036/      RPC、产测、阈值、金标监听（C 库 FFI）
├── waveform/    环形缓冲 + 可配置解析器
├── ota/         SMP / Goodix 双协议
├── commands/    Tauri IPC 层（DTO ↔ Manager 胶水）
└── dashboard/   解析脚本 + JSON 配置
```

代码量：后端 10,521 行，其中 `commands/` + `dashboard/` 合计 3,730 行（约 35%），**其余 65% 与 UI 无关**。

### 2.2 Tauri 耦合点：仅 4 处

| 文件 | 耦合方式 | 改造难度 |
|---|---|---|
| `service/event_bridge.rs` | `AppHandle` + `Emitter`，把 EventBus 事件转发给 webview | 无需改造（MCP 直接订阅 EventBus，不走它） |
| `state/dispatcher.rs` | `dispatch(&self, action, app: &AppHandle)`，`AppHandle` 仅用于 `emit("state-change")` | 低：抽 `trait StateEmitter`，headless 用 `NoopEmitter` |
| `ota/manager.rs`（约 120 行） | `tauri::async_runtime::spawn` | 极低：换 `tokio::spawn` |
| `dashboard/commands.rs` | `#[tauri::command]` 标记 | 极低：与 `commands/` 同性质 |

**除这 4 处，整个业务内核零 Tauri 依赖。**

### 2.3 三个有利条件

1. **所有 Manager 显式构造 + 构造注入**（`lib.rs:80-131`），没有隐藏的全局 IoC。任何进程都能自己装一套完整运行时：
   ```rust
   let event_bus = Arc::new(EventBus::new(1024));
   let device_manager = Arc::new(DeviceManager::new(event_bus.clone()));
   let gh3036 = Arc::new(Gh3036Manager::new(device_manager.clone(), event_bus.clone()));
   ```
2. **`EventBus` 是裸 tokio broadcast**，MCP 层可 `subscribe_channel()` 直接拿数据流，不经前端。
3. **lib 已是 `rlib`**，且 `Gh3036Manager` 已实现 `unsafe impl Send + Sync`，可直接被第二个 bin 消费。

### 2.4 三个硬约束

| 约束 | 证据 | 架构影响 |
|---|---|---|
| **硬件独占** | `SerialManager.ports: RwLock<HashMap<String, Arc<Mutex<SerialPort>>>>`（进程内私有），OS 层 COM 口本身独占 | 独立进程的 MCP **无法**与运行中的 GUI 共用同一端口 |
| **单进程单运行时** | gh3036 的 `CALLBACK_CONTEXT` 全局、`HR_REF_MONITOR` 静态、`LOGGER: OnceLock` | 一个进程只能有一套运行时，不能"GUI 里再嵌一套" |
| **状态落盘共享** | `%LOCALAPPDATA%\combridge`（`lib.rs:51-56`） | 两进程同时运行会写坏 preferences / 状态文件 |

### 2.5 勘察中发现的两个既有缺陷

1. **`get_serial_cache` / `get_ble_cache` 后端不存在**。`src/api/tauri.ts:88` 与 `:198` 在调用，但 `lib.rs` 的 `generate_handler!` 未注册，后端也无同名函数 —— 前端死调用。MCP 层实现 `serial_read` 时需要补上真正的读取路径（§7.1）。
2. **串口/蓝牙数据是 msgpack 发布的**（`device/serial/serial_manager.rs:97` → `topics::SERIAL_DATA`），不是 JSON。MCP 侧订阅必须用 `rmp-serde` 解码，按字符串处理会直接失败。

---

## 3. 架构选型论证

### 3.1 为什么前端方案不成立

"前端做 MCP"指起一个 Node 进程跑 MCP Server，内部用 `@tauri-apps/api` 的 `invoke` 调后端。这条路断在第一步：

- `invoke` 的实现依赖 `window.__TAURI_INTERNALS__.invoke`，**只存在于 webview 上下文**。外部 Node 进程没有任何合法途径获得该 IPC 通道。
- Tauri 2 的 IPC 是 webview ↔ Rust core 的私有协议（自定义 protocol + postMessage），**不对外暴露 TCP / Unix socket**。
- 唯一绕法是 CDP / WebDriver 注入 JS：
  - 要求窗口存在且 webview 已加载完成（不能 headless、不能在托盘态下无 webview）
  - 依赖前端组件已挂载、Zustand store 已初始化
  - 每次调用退化为"注入脚本 + 轮询结果"，超时与错误语义全部丢失
  - 生产包默认不开 devtools 端口，等于要求用户开调试模式

**这是能力缺失，不是工程取舍。**

附加理由：132 个 `invoke` 调用点全在 TS 里，若 MCP 走前端，要么在 TS 重写全部参数校验与类型转换（与 Rust 侧重复且会漂移），要么把命令名当字符串反射（无类型、无补全、后端改名即静默失效）。

### 3.2 硬件独占决定"进程归属"

MCP Server 必须与持有串口 / 蓝牙适配器的进程是同一个：

```
GUI 进程 open("COM3") ──► 成功，占用
MCP 进程 open("COM3") ──► OS 返回 ERROR_ACCESS_DENIED
```

若要让两个进程共享设备，必须引入守护进程做设备代理 —— 那是更远期的形态（§9.6），不是起步方案。

### 3.3 三种形态对比

| 维度 | A. 内嵌 HTTP（Tauri 进程内） | B. 独立 headless 二进制 | C. 前端 Node 桥接 |
|---|---|---|---|
| 复用现有代码 | 100%，零改动 | ~65% 平移（抽 core crate） | 0%（全靠 IPC 反射） |
| 硬件访问 | 直接 | 直接（但与 GUI 互斥） | 仅透过 GUI |
| 是否需 GUI 运行 | 是 | 否 | 是 |
| AI 客户端接入 | HTTP URL | stdio / HTTP | stdio（依赖 webview） |
| 流式数据 | EventBus 直连，零额外拷贝 | 同 A | 需前端 store 中转，丢帧 |
| 可否 CI / 服务器运行 | 否 | 是 | 否 |
| 人力成本 | 低（1.5~2 周） | 中（+2~3 周） | 高且脆 |
| 采用 | **Phase 1** | **Phase 2（可选下载）** | **不采用** |

### 3.4 结论

```
Phase 1（先做）                          Phase 2（验证后，可选下载）
┌────────────────────────┐              ┌──────────────────────────────┐
│ ComBridge.exe          │              │ combridge-mcp.exe (headless) │
│  ├ Tauri / WebView     │              │  ├ rmcp (stdio + HTTP)       │
│  ├ 业务内核（现有）     │  ──────────► │  ├ 业务内核（同一份 core）    │
│  └ rmcp :47821         │              │  └ 独占设备                   │
└────────────────────────┘              └──────────────────────────────┘
        ↑ AI 客户端                              ↑ AI 客户端（无需 GUI）
```

Phase 1 有一个被低估的优势：**人在环路**。AI 操作硬件时，用户能同时看到界面上的真实状态，异常时能立即按下停止。Phase 2 反而需要额外机制才能达到同等可观测性。

---

## 4. 目标架构

### 4.1 Phase 1：内嵌 Streamable HTTP MCP

**新增依赖**（`src-tauri/Cargo.toml`）：

```toml
rmcp = { version = "3.4", features = ["server", "transport-streamable-http-server"] }
axum = "0.8"
tower-http = { version = "0.6", features = ["trace"] }
schemars = "1"
uuid = { version = "1", features = ["v4"] }   # 已有，用于 lease_id / job_id
```

> **版本安全**：`rmcp` 2.0.0 之前的 Streamable HTTP 服务端存在未认证会话泄漏导致的远程 DoS（CVE-2026-63128 / GHSA-9pj6-vhgr-3mwh），**必须 pin ≥ 3.x**。锁定精确小版本，rmcp 3.x 存在破坏性变更。

**启动流程**（在 `lib.rs` 的 `setup` 内，拿到 `AppHandle` 之后）：

```
1. 读设置：MCP 是否启用（默认 false）→ 未启用则直接返回
2. 取持久化 token：配置文件里有就复用，没有才生成 32 字节随机值并立刻落盘
3. 构造 McpContext：从 AppHandle 取各 Manager 的 Arc 句柄
4. 绑定 127.0.0.1:47821（失败则回退 0 端口自动分配）
5. 挂载 StreamableHttpService 于 /mcp，加 auth + Host 校验中间件
6. 写 endpoint 发现文件 %LOCALAPPDATA%\combridge\mcp-endpoint.json
7. 启动审计写入器（JSONL）
```

**服务发现文件**：

```json
{
  "url": "http://127.0.0.1:47821/mcp",
  "token": "<32 bytes hex>",
  "pid": 18844,
  "app_version": "0.7.2",
  "protocol_version": "2026-07-28",
  "tier": "dev",
  "started_at": 1730793600
}
```

**关键实现约束**：工具实现**直接调用 Manager**，不复用 `#[tauri::command]` 函数。原因：

- command 签名含 `State<'_, T>`，MCP handler 无法构造
- command 的 DTO 是 camelCase（面向前端），MCP 输入按 snake_case 习惯设计
- command 层未来会随 UI 演进，MCP 工具契约需要独立稳定

### 4.2 Phase 2：拆分 core crate

```
combridge/
├── Cargo.toml                       # workspace
├── crates/
│   └── combridge-core/              # 无 tauri 依赖
│       ├── device/ service/ state/ protocol/ gh3036/ waveform/ ota/ dashboard/
│       └── runtime.rs               # ComBridgeRuntime::new() -> 全套 Arc 装配
├── src-tauri/                       # 只留 commands/ + lib.rs + main.rs
│   ├── src/main.rs
│   └── src/commands/
└── mcp-server/                      # bin: combridge-mcp（可选下载）
    ├── src/main.rs
    └── src/tools/
```

需要改动的仅三件事：

1. `state/dispatcher.rs`：`dispatch(&self, action, app: &AppHandle)` → `dispatch(&self, action, emitter: &dyn StateEmitter)`；Tauri 侧实现 `TauriEmitter`，headless 用 `NoopEmitter`。
2. `ota/manager.rs`：`tauri::async_runtime::spawn` → `tokio::spawn`。
3. `service/logger.rs`：`OnceLock<LoggerService>` 保持不变（一个进程一个 logger），但在 `runtime.rs` 中支持传入不同日志目录，避免与 GUI 抢同一文件。

**headless 二进制的运行形态**：

```bash
combridge-mcp                                    # stdio，供 Claude Code / Codex CLI
combridge-mcp --http --port 47821                # HTTP，供多客户端 / 远程
combridge-mcp --port COM3 --baud 921600          # 预连接，省掉模型先 open 的往返
combridge-mcp --print-config                     # 输出客户端 JSON 配置
```

### 4.3 单实例保护

- 复用/新增单实例锁：`%LOCALAPPDATA%\combridge\.instance.lock`（文件锁 + pid 探活）
- 第二个实例检测到端口占用 → 不启动 MCP，界面提示"已有实例在提供服务"，并给出该实例的 endpoint
- 补充机制：MCP 侧维护**设备租约**（§7.3），防止多个 AI 会话互踩同一端口

---

## 5. 工具面总体设计

### 5.1 设计原则

1. **不做 1:1 映射**。132 个 command 中大量是 UI 专用（`open_devtools`、`show_main_window`）或过度细碎（偏好设置拆成 7 个）。工具过多会显著降低模型选择准确率。
2. **按域聚合，域内用 `action` 枚举**。16 个工具在准确率与 schema 体积上均优于 132 个。
3. **统一 `combridge_` 前缀**，避免多 MCP 并存时命名冲突。
4. **`description` 是模型唯一的文档**，必须写清"何时用 / 何时不要用 / 返回什么形状"。
5. **不暴露 shell 面**：`open_url`、`show_in_folder`、`download_update`、`open_devtools`、窗口控制全部排除。
6. **危险操作默认 dry-run**，需显式 `confirm` 才执行（§8）。

### 5.2 工具总表（16 个）

| # | 工具 | 覆盖的现有能力 | 风险级别 |
|---|---|---|---|
| 1 | `combridge_about` | 版本、能力清单、已连接设备快照、当前档位、**headless 是否已安装** | L0 |
| 2 | `combridge_ports` | `scan_serial_ports` + `get_open_ports` + `is_port_open` | L0 |
| 3 | `combridge_serial_open` | `open_serial_port`（含配置校验） | L1 |
| 4 | `combridge_serial_close` | `close_serial_port` + 释放租约 | L1 |
| 5 | `combridge_serial_write` | `send_serial_data` | L2 |
| 6 | `combridge_serial_read` | **新增**（读环形缓冲 + 游标） | L0 |
| 7 | `combridge_serial_expect` | **新增**（服务端阻塞等待匹配） | L0 |
| 8 | `combridge_ble_scan` | `scan_ble_devices` / `stop_ble_scan` | L0 |
| 9 | `combridge_ble_connect` | `connect_ble` / `disconnect_ble` / `get_ble_connections` | L1 |
| 10 | `combridge_ble_gatt` | discover services/chars、read、write、subscribe、mtu、rssi | L1 / L2（write） |
| 11 | `combridge_devices` | `get_state` 的设备/通道视图 + 活跃租约 | L0 |
| 12 | `combridge_gh3036_rpc` | `gh3036_execute_rpc` | L2 |
| 13 | `combridge_gh3036_catalog` | `gh3036_get_rpc_commands` + `get_version_types` | L0 |
| 14 | `combridge_gh3036_factory_test` | 产测，**同步出结果**（D2） | L3 |
| 15 | `combridge_waveform` | list / create / configure_parser / read / clear / status | L1 |
| 16 | `combridge_protocol` | list / load / enable / disable / bind / unbind | L1 |
| 17 | `combridge_ota` | list_protocols / validate / preflight / start / status / cancel | **L3** |
| 18 | `combridge_logs` | **新增**（读日志尾部） | L0 |

> 表中 18 行，落地后对外实际 **16 个工具**：`catalog` 并入 `combridge_gh3036_rpc(action="catalog")`，
> `logs` 并入 `combridge_about(action="logs")`（都是"模型记不住第 17 个工具"的取舍）。
> 已实现清单（`tools/list` 实测 16 个）：`about` / `ports` / `serial_open` / `serial_close` / `serial_write` /
> `serial_read` / `serial_expect` / `devices` / `ble_scan` / `ble_connect` / `ble_gatt` / `gh3036_rpc` /
> `gh3036_factory_test` / `waveform` / `protocol` / `ota`。

**阈值配置**合并进 `combridge_gh3036_factory_test` 的 `action="threshold_*"`，不单独开工具 —— 它与产测共用同一份 YAML 与校验逻辑。

### 5.3 被刻意排除的接口

| 排除项 | 理由 |
|---|---|
| `preferences::*`（7 个） | UI 偏好，模型不该改 |
| `dispatch_action` | 内部状态机入口，绕过全部参数校验与业务规则 |
| `system::open_url` / `show_in_folder` | shell 面，安全风险 |
| `system::open_devtools` / `close_devtools` | 调试用，暴露等于开放调试通道 |
| `update::*` | 与设备调试无关 |
| 窗口控制（`show_main_window` / `get_window_status`） | 与任务无关 |

### 5.4 Resources（只读、可订阅，不占上下文）

| URI | 内容 |
|---|---|
| `combridge://devices` | 设备 / 通道 / 连接状态快照 |
| `combridge://serial/{port}/buffer` | 环形缓冲（可订阅，变更时推送摘要） |
| `combridge://gh3036/rpc-catalog` | RPC 命令表 + 参数说明（比工具返回更省调用） |
| `combridge://protocol/{plugin_id}/hooks` | Lua 插件的 hook 定义与示例 |
| `combridge://logs/tail` | 最近 N 行日志 |

### 5.5 Prompts（把使用剧本沉淀，减少模型试错）

| Prompt | 用途 |
|---|---|
| `serial-debug` | 串口联调剧本：scan → open → expect banner → 发指令 → 判读 |
| `gh3036-factory-test` | 产测完整流程，含前置自检 |
| `ble-gatt-explore` | GATT 探索顺序（service → char → 订阅 → 采帧） |
| `ota-safe-flash` | OTA 前置检查清单与确认话术 |

---

## 6. 关键工具契约

### 6.1 `combridge_serial_read`

读环形缓冲，游标式，永不一次吐全量。

```jsonc
// 入参
{
  "port": "COM3",
  "cursor": 10240,        // 可选，缺省=从最新往前取
  "max_bytes": 4096,      // 上限 16384
  "encoding": "text"      // text | hex | base64
}
// 出参
{
  "data": "...",
  "bytes": 1830,
  "next_cursor": 12070,
  "dropped": false,       // 环形缓冲已覆盖 → 模型需知道"漏了数据"
  "port_open": true
}
```

### 6.2 `combridge_serial_expect`（最高频工具）

在接收流上阻塞等待匹配，命中即返回。**这是串口调试的核心原语** —— 比"模型反复 read 轮询"省几十次往返。

```jsonc
// 入参
{
  "port": "COM3",
  "pattern": "OK\\r\\n",   // 正则；或 {"hex":"0D0A"} 形式做二进制匹配
  "timeout_ms": 3000,      // 上限 30000
  "since_cursor": 12070,   // 可选，只匹配此游标之后的数据
  "context_bytes": 128     // 命中前保留的上下文
}
// 出参（命中）
{ "matched": true, "at_cursor": 12148, "text": "AT+VERSION\r\nOK\r\n", "elapsed_ms": 84 }
// 出参（超时）
{ "matched": false, "elapsed_ms": 3000, "tail": "...最后 128 字节..." }
```

**实现要点**：在 MCP 侧维护每端口一个 `tokio::sync::broadcast` 转发缓冲 + `Notify`，命中立即唤醒等待者；不要在 handler 里 sleep 轮询。

### 6.3 `combridge_gh3036_factory_test`（D2：同步出结果）

```jsonc
// 主路径：一次调用拿到结论
{ "action": "run", "config_dir": "D:/cfg", "max_wait_s": 180 }
// 返回
{
  "job_id": "ft_9f2c",
  "status": "completed",
  "pass": false,
  "duration_s": 42,
  "failed_item": "SPO2_ACC",
  "items": [
    { "name": "HR_ACC",   "value": 0.8, "threshold": "<2",  "pass": true  },
    { "name": "SPO2_ACC", "value": 3.1, "threshold": "<2",  "pass": false }
  ],
  "csv_path": "D:/out/GH3036_2026-11-04_16-31-02.csv"
}
```

**语义**：内部执行 `start → 轮询 status → 等待完成 → 汇总`，模型只看到最终结论，不需要理解中间进度。

**降级**：超过 `max_wait_s` 时返回 `{ "status": "running", "job_id": "ft_9f2c" }`，模型可用 `action="status"` 续等。

**其他 action**：`status` / `stop` / `validate_config` / `threshold_get` / `threshold_validate` / `threshold_save`。

产测结果同时落 CSV（复用现有 `csv_writer`），返回体带路径，方便模型或用户回溯。

### 6.4 `combridge_ota`（L3，默认 dry-run）

```jsonc
// 第一次调用：只做前置检查，不烧录
{ "action": "start", "firmware_path": "D:/fw.zip", "address": "AA:BB:CC:DD:EE:FF", "protocol": "goodix" }
// 返回
{
  "dry_run": true,
  "plan": {
    "protocol": "goodix",
    "target": "AA:BB:CC:DD:EE:FF",
    "image_name": "app.bin",
    "image_size": 245760,
    "image_sha256": "…",
    "est_seconds": 68,            // 8s 固定开销 + 镜像 / 12KB/s，仅作量级参考
    "device": { "version": "1.0.7", … }   // 预检成功才有
  },
  "package": { … },
  "warnings": ["设备预检未通过：…"],       // 非阻断，如实回传
  "confirm_token": "ota_9f2c1b04",        // 需在下次调用原样带回
  "confirm_expires_in": 300
}
// 第二次调用：带 confirm 才真正执行
{ "action": "start", "firmware_path": "D:/fw.zip", "address": "AA:BB:CC:DD:EE:FF", "confirm": "ota_9f2c1b04" }
```

`confirm` **一次性有效、5 分钟过期、绑定「目标设备 + 固件路径 + 镜像 sha256 + 协议」**。
**没有正确 `confirm` 时，`start` 永远只做 dry-run** —— 这不是提示词层面的约定，而是代码里的硬门
（`mcp/ota_confirm.rs`）。

实现要点（P4）：

- **判定顺序固定**：先校验固件包（本地只读）→ 再预检设备（非破坏性）→ 最后才看令牌。
  包已经损坏时，即使令牌有效也走不到烧录。
- **漏带 vs 带错**：不带 `confirm` 是正常 dry-run（返回 200 + plan）；
  带了但无效则返回 `isError` 的 `CONFIRM_INVALID` / `CONFIRM_EXPIRED` / `CONFIRM_MISMATCH`，
  **并在错误负载里直接附上新的 `confirm_token`** —— 省掉"重跑一遍拿令牌"的往返，模型照着换一个即可。
- **令牌不被误消费**：参数不一致（换设备 / 换包）时令牌保持有效，只有校验通过才核销。
- **档位硬门**：标准档（`standard`）在真正执行前被 `TIER_BLOCKED` 拦下。
  计划里标准档是「dry-run + elicitation 双确认」，但 elicitation 属 P5；
  在它落地之前宁可拒绝执行，也不留一条无确认的烧录路径。
- `est_seconds` 是量级参考，不承诺精度；真实进度以 `action=status` 的 `stage/percent` 为准。

### 6.5 `combridge_ble_gatt`

按 `action` 分发：`services` / `characteristics` / `read` / `write` / `subscribe` / `unsubscribe` / `set_mtu` / `rssi`。

**体积治理**：`services` 默认只返回 UUID + 名称摘要；`include_characteristics=true` 才展开。`read` 返回长度 + 按 encoding 编码的数据，超 4KB 截断并给 `truncated_bytes`。

### 6.6 统一错误结构

```json
{
  "code": "PORT_BUSY",
  "message": "COM3 已被会话 lease_a1b2 占用（最后活跃 12s 前）",
  "hint": "调用 combridge_serial_close(port=\"COM3\") 释放，或复用该 lease",
  "retryable": false
}
```

模型的错误恢复能力完全取决于 `hint` 质量。错误码必须稳定可枚举，**不允许**直接把 `Serial(Os { code: 5, ... })` 透出。

错误码清单（初版）：`PORT_BUSY` / `PORT_NOT_FOUND` / `PORT_NOT_OPEN` / `PORT_NOT_OWNED` / `LEASE_EXPIRED` / `DEVICE_DISCONNECTED` / `TIMEOUT` / `INVALID_ARGUMENT` / `CONFIRM_REQUIRED` / `TIER_BLOCKED` / `HW_ERROR` / `JOB_NOT_FOUND` / `RATE_LIMITED`；
GH3036 域另有 `GH3036_BUSY` / `GH3036_INIT_FAILED` / `GH3036_RPC_FAILED` / `GH3036_CHANNEL_FAILED` / `CHANNEL_NOT_CONFIGURED` / `CONFIG_INVALID` / `CONFIG_NOT_LOADED` / `FACTORY_TEST_FAILED` / `FACTORY_TEST_STATE`。

P4 新增（已落地）：`CONFIRM_INVALID` / `CONFIRM_EXPIRED` / `CONFIRM_MISMATCH` / `PACKAGE_INVALID` /
`OTA_ERROR` / `OTA_NOT_RUNNING` / `BUFFER_NOT_FOUND` / `BUFFER_EXISTS` / `PARSER_INVALID` /
`PROTOCOL_NOT_FOUND` / `PROTOCOL_ERROR` / `LOG_NOT_FOUND` / `LOG_READ_FAILED`。
`CONFIRM_REQUIRED` 在 P5a 已启用：标准档下用户拒绝、超时、或客户端没有 elicitation 能力时返回它。
P5a 另增 `PORT_SUSPENDED`（端口熔断）与 `RATE_LIMITED`（QPS / 并发上限）。

**实现约束（踩过）**：错误必须是 `isError: true` 的工具结果，不能抛 `Err(ErrorData)` —— 后者是 JSON-RPC 协议错误，
客户端会渲染成 "internal error"，模型读不到 `message`，`hint` 等于白写。

---

## 7. 关键工程问题

### 7.1 数据流：msgpack + 游标 + expect

串口/BLE 是高速流，直接把字节塞进工具返回值会在几十毫秒内打爆上下文。三层设计：

1. **游标读取**：`combridge_serial_read(port, cursor, max_bytes)` 返回 `{data, next_cursor, dropped}`。`cursor` 是单调递增字节序号，在 `RingBuffer` 上加 `total_written: AtomicU64` 即可。
2. **expect 等待**：服务端阻塞匹配，命中即返回（§6.2）。
3. **编码策略**：默认 text（lossy UTF-8）+ 长度；二进制用 hex/base64；超长截断并明确告知。

**订阅实现（关键，易踩坑）**：`serial:data` 是 **msgpack**（`serial_manager.rs:97`），必须：

```rust
let mut rx = event_bus.subscribe_channel();
while let Ok(ev) = rx.recv().await {
    if ev.topic == topics::SERIAL_DATA && ev.encoding == EventEncoding::MsgPack {
        let payload: SerialDataEvent = rmp_serde::from_slice(&ev.payload)?;
        hub.push(&payload.device_id, &payload.data);   // 推入 MCP 侧游标缓冲
    }
}
```

**不要**复用 `EventBridge` —— 它面向 webview，会做 base64 包装并丢失类型信息。

`RingBuffer` 最小改动（`device/cache.rs`）：

```rust
pub struct RingBuffer {
    entries: VecDeque<CacheEntry>,
    total_written: u64,          // 新增：单调游标基准
    // ...
}
impl RingBuffer {
    pub fn total_written(&self) -> u64 { self.total_written }
    pub fn read_since(&self, cursor: u64, max: usize) -> (Vec<u8>, u64, bool) { /* ... */ }
}
```

### 7.2 长任务：job 模型

OTA、产测可能跑数分钟，多数 MCP 客户端有 60s 超时。

- D2 已把产测收敛为"同步出结果"（内部等待 + 超时降级）
- OTA 仍是 `start(dry-run) → start(confirm) → status(job_id)` 三段
- job 状态来源：`topics::OTA_PROGRESS`（OTA 已有）、产测的 `factory_test_status` 轮询
- MCP 侧维护 `job_id → 最近进度快照`，`status` 读取快照即刻返回，不做长阻塞
- 若客户端支持 progress notification，额外推 `notifications/progress`；不支持时纯轮询也要能跑通

### 7.3 并发：设备租约（必做）

**最易被忽略、上线后最容易出事的一点**：用户同时开着 Claude Code 和 Cursor，两个 AI 都想操作 COM3。

- `combridge_serial_open` 返回 `{ lease_id }`，同一端口同一时刻仅允许一个 lease
- 第二个客户端 open 同一端口 → `PORT_BUSY` + 当前持有者 + 最后活跃时间
- lease TTL 默认 5 分钟无调用自动释放（防崩溃后端口永久锁死）
- `combridge_devices` 展示全部 lease，用户可手动踢出
- BLE 同理，按 address 维度
- 同一客户端的连续调用凭 `lease_id` 复用，不需要每次重开

### 7.4 上下文体积治理

| 风险 | 对策 |
|---|---|
| 波形 10 万点 | `waveform read` 的 `rows` 上限 200，默认返回统计摘要（min/max/mean/len）而非原始数组 |
| 日志数千行 | `lines` 上限 200，支持 `level` 过滤 |
| GATT 服务树庞大 | 默认摘要，按需展开 |
| 工具 schema 过大 | 16 个工具 + 紧凑 schema，实测估算 < 8k tokens（验收项 S6） |
| 二进制数据 | 强制 encoding 参数，超限截断并标注 `truncated_bytes` |

### 7.5 版本与兼容

- MCP 协议版本：跟随 rmcp 声明的稳定版本（当前 `2026-07-28`），`initialize` 时协商
- App 版本握手：`combridge_about` 返回 `app_version`；工具契约随 minor 版本演进，major 变更时保留旧工具名一个版本周期
- headless 与 GUI 的版本匹配：headless 启动时校验 `app_version` 同 minor，否则拒绝启动并提示重新下载

---

## 8. 安全模型

### 8.1 两档策略（D1）

一个总开关 + 两个档位，首次开启时强制选择：

| 能力 | 研发档（内部包默认） | 标准档（正式包默认） |
|---|---|---|
| 总开关默认值 | 开（内部构建） | 关 |
| L0 只读 | 放行 | 放行 |
| L1 状态变更 | 放行 + 审计 | 放行 + 审计 |
| L2 设备写入 | 放行 + 审计 | **每次 elicitation 确认** |
| L3 高危（OTA / 产测） | dry-run 一次 + 审计 | **dry-run + elicitation 双确认** |
| 审计日志 | 开（不可关） | 开（不可关） |
| 限流 | 20 QPS/会话 | 5 QPS/会话 |

**实现**：档位是服务端强制的，不依赖客户端行为。L2/L3 的拦截靠 elicitation 确认（见 §8.3），
`TIER_BLOCKED` 保留给"档位本身不具备该能力"的场景（当前未使用；P4 曾用它临时拒绝标准档的 L3）。

### 8.2 网络与认证

| 层 | 措施 |
|---|---|
| 绑定 | 仅 `127.0.0.1`，**禁止** `0.0.0.0`（`axum::serve` 前显式校验） |
| Host 校验 | 校验 `Host` 头为 `127.0.0.1` / `localhost`，防 DNS rebinding |
| 认证 | 32 字节随机 token，`Authorization: Bearer`；**持久化**（首次生成后固定，`rotate` 才作废） |
| 文件权限 | endpoint 文件仅当前用户可读（Windows ACL / Unix 0600） |
| 会话 | Streamable HTTP 会话绑定 token，会话闲置 30 分钟回收 |

### 8.3 确认机制（elicitation）

L2 / L3 通过 MCP 的 **elicitation** 让客户端弹窗询问用户，而不是依赖模型自律：

```
服务端 → 客户端: elicitation "即将向 COM3 写入 128 字节：41 54 2B ...，确认？"
客户端 → 用户: 弹窗
用户 → 客户端 → 服务端: accept / decline
```

标准档下 decline 直接返回 `CONFIRM_REQUIRED`，不重试。同时前端（GUI）也弹一次确认框作双保险（L3）。

**实现（P5a，`mcp/elicit.rs`）**：

- 只问一句、只收一个布尔字段（`ConfirmForm { confirm }`），弹窗文案必须写清"改什么、改哪个口、改多少"，
  例如 `即将向 COM3 写入 6 字节：AT+RST\r\n`。二进制载荷转 hex 展示，超 32 字节截断并标总长。
- 超时 120s；拒绝 / 取消 / 超时 / 客户端不支持 elicitation / 协议版本禁止服务端发起请求 —— **全部按拒绝处理**，
  统一返回 `CONFIRM_REQUIRED`（`retryable: false`）。"问不到"绝不能等于"默认同意"。
- 研发档不弹窗（内部联调每次点弹窗会逼疯人），但审计照记 —— 跳过的是交互，不是留痕。
- **校验顺序**：参数解析 → 租约校验 → 确认 → 执行。不会为一个本身非法的请求先弹窗打扰用户。
- 覆盖范围：`serial_write`、`ble_gatt`（write / subscribe / unsubscribe / mtu）、`gh3036_rpc`（rpc / init / set_channel）、
  `gh3036_factory_test`（run / continue / stop）、`ota`（真实烧录那一次）。
  L3 因此变成**双确认**：先拿 dry-run 令牌（机器可校验），再由用户点确认（人可判断）。

### 8.4 审计

- 路径：`log/mcp-audit-YYYY-MM-DD.jsonl`
- 每条记录：`ts` / `session_id` / `tier` / `tool` / `args_digest`（参数结构 + 数据字段的 sha256 与前 64 字节 hex）/ `result_code` / `duration_ms` / `dry_run`
- 审计不可关闭；写失败时降级为 `tracing::error` 但**不阻断**业务（避免审计故障导致产线停摆）
- 保留 30 天，随现有日志轮转策略清理

**实现（P5a，`mcp/audit.rs`）**：

- 真实字段：`{ts, tier, tool, args:{keys[], sha256, preview_hex, bytes}, result_code, duration_ms, dry_run}`。
  `session_id` 拿不到（见 §8.5 说明），暂以 `tier` + `tool` + 参数摘要定位调用。
- 收口在 `ServerHandler::call_tool`（自己实现，`#[tool_handler]` 检测到已存在就不再生成）：
  **限流拒绝也记审计**，否则"被压制的调用"在日志里不可见。
- `result_code` 成功为 `OK`，失败取工具错误结构里的 `code`（拼不出就是 `ERROR`），
  协议层错误记 `PROTOCOL_ERROR` —— 审计能区分"业务失败"与"协议故障"。
- 目录与当前日志同目录（尊重用户改过的日志路径），否则 `<exe>/log`；进程内首次写入时清理 30 天前的文件。
- `preview_hex` 只取 `data` / `payload` 字段前 64 字节，其余参数只留 key 列表与整体 sha256：
  审计要能复盘"发了什么类别的东西"，但不能把固件或密钥原文落盘。

### 8.5 熔断与限流

- 每会话 20 QPS（研发）/ 5 QPS（标准），全局 100 QPS
- `serial_expect` 并发上限 8，超出排队等待而非拒绝
- 同一端口连续 5 次硬件错误 → 暂停该端口 30s，返回明确原因与恢复方式

**实现（P5a，`mcp/ratelimit.rs`）**：

- **按全局而非按会话**：rmcp 的 `RequestContext` 不暴露 session id（`StreamableHttpService` 的 factory
  在无状态模式下是**每请求**调用，拿不到稳定的会话标识）。计划里的"每会话 20 / 全局 100"在本机单用户场景下
  等价于一个全局预算，取更保守的每会话值（研发 20 QPS / 标准 5 QPS）。令牌桶按秒补满，允许一秒突刺。
- `serial_expect` 并发上限 8，超出返回 `RATE_LIMITED`（带 `Busy { limit }` 说明）而不是无限排队 ——
  排队会让模型以为"卡住了"，报错反而能促使它改用一次等待。
- 端口熔断只统计 `SERIAL_ERROR`（真实硬件错误），**不统计 `TIMEOUT`** —— `expect` 超时是正常业务结果，
  把它算失败会让熔断在正常调试中频繁误触发。熔断期间返回 `PORT_SUSPENDED` + 剩余秒数。

### 8.6 设置页「集成」（P5b）

系统页新增第 4 个 tab「集成」（`pages/System/McpIntegration.tsx`，默认可见），三块内容：

| 区块 | 内容 |
|---|---|
| MCP 服务 | 启用开关、监听端口、安全档位、保存并应用；状态标签（运行中 / 已停止 / 启动失败）、实际地址、工具数量、启动时间、配置文件路径 |
| 客户端接入 | **自动接入开关**与状态（已写入 / 尚未写入 / 配置过期 / 未检测到客户端 / 写入失败）+「重新写入」；打码显示的 token（可显示/复制/轮换）、客户端选择（Claude Code CLI / Claude Desktop / Cursor / 通用 JSON）、可直接粘贴的配置片段 |
| 安全与审计 | 两档语义说明、当日审计路径与条数、打开审计目录 |

**生效语义（关键设计）**：

| 改动 | 生效方式 | 代价 |
|---|---|---|
| 启用 / 停用 | 起停服务（`mcp::start` / `mcp::stop`） | 无：token 已持久化，客户端配置自动跟着改 |
| 端口 | 重启服务 | 客户端 URL 与信任记录自动重写（origin 变了），界面会提前警告 |
| 轮换 token | 重启服务 | 客户端 header 自动重写；信任记录不变（origin 未变） |
| 档位 | **热切换**（`mcp::switch_tier`） | 无：不重启、不换 token，仅更新共享 tier cell 与限流速率 |
| 自动接入开关 | 立即写一次 / 停止维护 | 无：关掉不删已有条目（见 §8.7） |

档位做成热切换是刻意的：研发 ↔ 标准是这一页最常被点的开关，若走重启，每切一次都要把所有客户端配置重贴一遍。
实现上 `McpContext.tier` 是 `Arc<RwLock<String>>` 而非启动时的字符串快照，`RateLimiter.per_sec` 为原子量，
所以切换是"改一个值 + 改一个速率"，正在跑的会话不受影响。

**运维细节**：

- 停服 / 未启用时会删除 `mcp-endpoint.json` —— 留着等于对外发一个已失效的 token。
- 启动失败只影响 MCP 本身：配置已落盘，界面用 `enabled=true & running=false + last_error` 表达"开关开着但没起来"，
  不会假装成功。
- 端口被占用时自动回退随机端口，界面用 `portFallback` 明确提示实际端口（否则用户按配置端口配客户端必然连不上）。

**验证**：面板的 5 条行为（打码、开关即时应用、启动失败呈现、片段可复制、打开审计目录）由
`pages/System/McpIntegration.test.tsx` 覆盖；配置片段的字段与 `Bearer` 前缀由 `utils/mcpSnippet.test.ts` 覆盖；
中英文案 key 一致性由 `i18n/mcpKeys.test.ts` 覆盖。**未验证**：真机点击路径（自动化测试用 mock 替换了 IPC）。

### 8.7 客户端接入（token 持久化 + 信任预置）

**token 生命周期改为持久化**。早期每次 `start` 现生成，导致外部客户端把 Bearer 写进配置后，应用一重启就 401 ——
等于要求用户每重启一次重贴一次配置，集成做不下去。现在 `McpSettings.token` 落盘在
`%LOCALAPPDATA%\combridge\mcp.json`，首次生成后复用；作废旧凭据的唯一入口是设置页「轮换 token」
（置空后重启服务重新生成）。安全边界从"重启即失效"变为"能读到该用户数据目录即可用"，
与 endpoint 文件同权限等级；风险与取舍记录在此，便于后续评审。

**客户端侧只有两处需要写文件，且已改为服务端自动完成**（`mcp/client_wire.rs`，服务启动/停用时调用）：

| 文件 | 内容 | 为什么 |
|---|---|---|
| `<configDir>/mcp.json` | `{"mcpServers":{"combridge":{"type":"http","url":…,"headers":{"Authorization":"Bearer …"}}}}` | 客户端按此加载 server |
| `<configDir>/mcp-approvals.json` | `{"<sha256(origin)>::<serverName>": <ts>}` | 省掉首次连接的「Trust」点击 |

`<configDir>` 默认 `~/.workbuddy`，可用 `WORKBUDDY_CONFIG_DIR` / `CODEBUDDY_CONFIG_DIR` 覆盖；
目录不存在则整条链路跳过（不在用户家目录里凭空造配置）。所以**用户侧只剩"开个新会话"这一步**。

信任键的构造与客户端一致：`sha256(origin)` 而非完整 URL（token 在 header 里，不影响哈希），
所以**换 token 不必重新信任**，但**改端口会变 origin → 需重新写入（自动进行）**。

几条实现上的克制，都是为了避免"帮倒忙"：

- **幂等**：条目内容不变就不写盘，客户端配置文件不会被反复改写。
- **不覆盖别人的配置**：只 upsert 同名 key，顶层其它字段与其它 server 原样保留；
  `mcp.json` 解析失败时**直接放弃**，绝不把它当空对象覆盖。
- **临时文件 + rename**：避免客户端正好读到写了一半的 JSON。
- **停服即撤条目**：留一条指向无人监听端口的记录，会让客户端每次开会话先失败一次；
  信任记录保留，重新启用后无需再点 Trust。
- **失败不影响服务**：服务只依赖 endpoint 文件；接入失败只记 `warn`，设置页显示原因。
- **可关**：`autoWire`（默认开）关掉后程序不再碰这些文件，已有条目保留（那是用户正在用的东西）。

`scripts/mcp/wire_client.py` 保留作手动兜底（别的机器、或有客户端不在默认目录时）。

**绕不开的一步**：MCP 连接在会话启动时建立，运行中的会话不会热加载新 server。
所以无论配置怎么自动写，都还必须开一个新会话 —— 这是客户端行为，服务端无法代劳。

**验证**（用写出的配置原样连接，而非另起一套参数）：`initialize` 返回 `serverInfo.name=combridge`、
`tools/list` 16 个工具；错误 token 返回 `401`。token 持久化经一次应用重启核对：`started_at` 变化、token 不变。
自动接入经"手工删掉条目与信任记录 → 重启应用 → 两条都被重新写回"核对。

---

## 9. 执行计划

### 9.0 实施进度

> 分支 `feat/mcp-server`，随代码同步更新。

| 阶段 | 状态 | 落地位置 / 说明 |
|---|---|---|
| P0 骨架 | ✅ 已完成 | `src-tauri/src/mcp/{mod,auth,ctx,server}.rs`；`rmcp = "=3.8.0"`；默认关闭，`%LOCALAPPDATA%\combridge\mcp.json` 开启 |
| P1 串口闭环 | ✅ 已完成 | 新增 `mcp/{lease,serial_hub}.rs`；7 个工具（`about` + `ports` + 5 个 `serial_*`）；租约随 P1 一起落地（按 §9.2 建议提前） |
| P2 设备与 BLE | ✅ 已完成 | 新增 `mcp/ble_hub.rs`；工具 `combridge_devices` / `combridge_ble_scan` / `combridge_ble_connect` / `combridge_ble_gatt`；BLE 连接租约资源名 `ble:<address>` |
| P3 GH3036 与产测 | ✅ 已完成（真机已验收，见下） | 新增 `mcp/gh3036_job.rs`；工具 `combridge_gh3036_rpc` / `combridge_gh3036_factory_test`；`FactoryTestManager` 增加 CSV 路径与终态消息访问器 |
| P4 其余业务域 | ✅ 已完成（OTA 真机烧录待验） | 新增 `mcp/{ota_confirm,logs}.rs`；工具 `combridge_waveform` / `combridge_protocol` / `combridge_ota`；`combridge_about` 增加 `action=logs`。**对外 16 个工具** |
| P5 安全与产品化 | ✅ 已完成 | P5a 后端：`mcp/{audit,ratelimit,elicit}.rs`，审计 / 限流 / 熔断 / elicitation 全部收口在 `call_tool`。P5b 前端：系统页新增「集成」tab（`pages/System/McpIntegration.tsx` + `commands/mcp.rs`），开关/端口/档位当场生效、一键复制客户端配置、审计入口，中英文案齐备 |

P1 实现要点（与本文档契约的对应）：

- `combridge_serial_read`：游标 = hub 内单调递增的累计字节数；`since` 省略表示"从缓冲区最旧处"；缓冲上限 1MB，被裁剪时返回 `truncated=true`；broadcast 消费滞后时返回 `lagged=true`。
- `combridge_serial_expect`：`since` 省略表示"从此刻起只看新数据"；服务端阻塞等待，命中即返回；超时返回 `matched=false` 与已收到的全部文本；端口中途断开返回 `disconnected=true`。
- 租约：`combridge_serial_open` 返回 `lease_id`，`write` / `close` 强制校验；`PORT_BUSY` 回传当前占用者与过期时间，`force=true` 可抢占。GUI 打开的端口 MCP 可读可写（持租约），但不允许 `close`。

P2 实现要点：

- 工具收敛为 4 个（对应 §5.2 的 #8~#11）：`combridge_devices` 汇总设备/连接/租约，`combridge_ble_scan`（阻塞扫描或 stop），`combridge_ble_connect`（connect/disconnect/list，租约资源 `ble:<address>`），`combridge_ble_gatt`（9 个 action：discover_services / discover_characteristics / read / write / subscribe / unsubscribe / notifications / mtu / rssi）。
- 通知数据：`ble:data` 与串口同样走 msgpack 广播，`mcp/ble_hub.rs` 按 `address|char_uuid` 分桶缓存并提供游标读取；`subscribe` 时清空旧桶（游标从订阅时刻起算）。
- 租约粒度：写/订阅/取消订阅/改 MTU 需要租约（会改设备状态）；discover/read/notifications/rssi 免租约。
- 二进制建议：GATT 读写用 `encoding=hex`，text 模式为 UTF-8 lossy，仅适合 ASCII 载荷。

P1/P2 真机冒烟结果（2026-11-04，`npm run tauri dev` + `127.0.0.1:47821`）：

| 验证项 | 结果 |
|---|---|
| `initialize` 握手 / `mcp-session-id` | ✅ 返回 serverInfo + instructions，SSE 流正常 |
| `tools/list` | ✅ 11 个工具，schema 与必填项正确 |
| 无 / 伪造 Bearer | ✅ 401 |
| 监听范围 | ✅ 仅 loopback |
| `serial_open` → 租约 → `write` → `close` | ✅ COM1 真实打开，`bytes_written=4` |
| 跨客户端抢占 | ✅ 第二客户端 `PORT_BUSY`；`force=true` 抢占成功；原持有者 `write`/`close` 均被拒 |
| `serial_expect` 无应答 | ✅ 1.2s 超时降级返回 `matched=false`，不阻塞、不报错 |
| 重复 `close` | ✅ `LEASE_INVALID`（租约已释放） |

**错误语义（重要，冒烟中发现并修正）**：初版把业务错误抛成 JSON-RPC 协议错误（`-32603`）。
rmcp 文档明确：协议错误在客户端被渲染为不透明提示（如 "Tool result missing due to internal error"），
**模型读不到 message**，等于错误信息全丢。现已统一改为 `isError: true` 的工具结果，按 §6.6 输出
`{code, message, hint, retryable}`。已落地错误码：`PORT_BUSY` / `BLE_BUSY` / `LEASE_INVALID` /
`PORT_NOT_OWNED` / `SERIAL_ERROR` / `INVALID_ARGUMENT` / `ERROR`。

P3 实现要点：

- 工具收敛为 2 个（对应 §5.2 的 #12、#14）：`combridge_gh3036_rpc`（status / init / set_channel / catalog / rpc）
  与 `combridge_gh3036_factory_test`（run / status / continue / stop / validate_config / threshold_get）。
  `catalog` 保留为独立 action 而非并入 `about` —— 14 条命令 + 14 个版本类型的参数说明体积不小，
  只在模型确实要发 RPC 时才拉取。
- **租约采用「自动获取 + 同 owner 复用」**：GH3036 没有「打开设备」这一步，若照搬串口的显式 attach，
  `factory_test(action=run)` 会变成两次往返。规则是：不带 `lease_id` 时自动获取；若已被**同一 owner** 持有则直接复用；
  不同 owner 返回 `GH3036_BUSY`，`force=true` 抢占。这是 D2「一次调用出结果」的前提。
- **`run` 的三个出口**：`completed` / `failed` 返回结论；`waiting_environment_switch` **立即返回**并给 hint
  （等下去没有意义，必须人工换环境），不做无谓超时；超过 `max_wait_s` 返回 `running` + `job_id` 供 `action=status` 续等。
- **前置校验前移**：`run` 之前先校验卡控配置与 TX/RX 通道，缺什么直接给 `CONFIG_INVALID` /
  `CHANNEL_NOT_CONFIGURED` + 修复 hint，而不是等线程跑起来在中间步骤失败。
- `FactoryTestManager` 新增 `csv_path_for_project` / `result_csv_path`，`Gh3036Manager` 暴露
  `factory_test_csv_path`，路径规则不再在 MCP 侧重写一遍。
- 未实现：`threshold_validate` / `threshold_save`（属 GUI 侧编辑场景，MCP 目前只需 `threshold_get`）。

P3 真机验收（2026-11-04，GH3038_Nordic / `BluetoothLE#...f9:ac:fb:db:6e:72`，服务 `0000190e`，
写入 `00000004` / 通知 `00000003`，配置目录 `E:\Code\Goodix\l-evk-t2-gh3038-q\config\factory_config`）：

| 验证项 | 结果 |
|---|---|
| `ble_scan` → `ble_connect` → `discover_services` → `subscribe` | ✅ 4 个服务，GH3036 私有服务特征与默认 UUID 一致 |
| `gh3036_rpc init` / `set_channel(tx,rx)` | ✅ BLE 通道配置成功，`status` 回读一致 |
| `gh3036_rpc rpc` 成功路径 | ✅ `R 0x0000 8` 返回 16 字节（含芯片标识 `JKDDH32H`），`V` 返回单字节 |
| 配置校验 | ✅ 真实卡控 YAML（无 `channels`）通过 |
| 产测全链路 | ✅ prepare → chip_init → uuid → base_noise → ppg_noise → lpctr → environment_switch → lplctr → cleanup 全部执行，CSV 落盘 |
| 实测值 | base_noise `[46,23]` uV、ppg_noise `[52,51,53,48,51,46,51,42]` uV、lpctr/lplctr `[0,1,2,5,0,0,0,0]` |
| 最终结论 | `status=completed`、`pass=true`、`error_code` 为空，CSV 落盘 |

真机验收中发现并修复的 7 个缺陷（均已补单测）：

1. **卡控校验过严**：`TestItemConfig::validate` 要求 `channels > 0`，而真实产测配置普遍省略该字段
   （通道数由设备侧配置决定）。改为「省略 = 按设备返回的采样数评估，显式写 0 才报错」。
2. **MCU 模式采集窗口丢失（回归）**：`run_hardware_test` 启动 TEST1 后立即 `S 0x40 1` 停止，
   芯片来不及产出结果 → `FG` 返回空数组。改为启动后先等采集窗口（`compute.timeout_ms` 或默认 3s）、
   停止后再留 1s 给芯片整理。
3. **空数据被判合格（最危险）**：`channels=0` 时 `measurements.len() == channels` 恒真，
   加上 `evaluate_channels` 对空数组 `channel_count=0` 不进入循环 → 「0 通道全部通过」的假 PASS。
   现在空测量一律判失败（`No measurements`），MCU 侧同时要求实测非空。
4. **同 owner 重复 `ble_connect` 被自己的租约挡住**：`BLE_BUSY` 指向自己，只能 force 抢占自己。
   按 GH3036 的做法补上同 owner 复用。
5. **重复 `subscribe` 叠加回调**：同一条通知被喂多份，RPC 层出现重复响应帧与
   `Ignoring stale response` 告警。MCP 侧订阅前先查已有订阅，幂等跳过。
6. **UUID 判定误杀合格片**：固件 `factory_chip_uid_get` 只 `memset` 自身长度（默认 `0xAA`），
   后半段保持 0 属正常；原逻辑要求前后两半都非零，且不足 32 字节的读数被直接丢弃。
   现统一为「长度非 0 且值不全为 0 即有效」，`0x2002` 不再产生（常量保留以兼容历史 CSV）。
7. **LPCTR 量纲未标定导致必然失败**：新增 `ThresholdOperator::Present`（YAML 写
   `operator: "present"`），只校验是否读到数值、不做上下限卡控；GUI 算子下拉同步支持并给出提示。
   当前 EVK 配置已切到 `present`，量纲标定完成后改回 `range [100, 3000]`。

另修：`combridge_gh3036_factory_test` 的失败结论此前只有 `status=failed` + 空 `items`，
`uuid` / `prepare` 这类不在卡控表里的步骤失败时模型无法解释原因。现在
`FactoryTestManager.last_message()` 保留终态消息，失败时回填到 `hint`，`failed_item` 回退为步骤名。

已确认的判定口径（设备侧语义，非代码缺陷）：

- **UUID**：设备桩实现默认上报 `0xAA`，`AA×16 + 00×16` 视为正常，不要求两半都非零；
  只要求「长度非 0 + 值不全为 0」，全 0 或空才判失败。
- **LPCTR**：测试环境受限，读到数值即可判 OK，不做数值卡控（见缺陷 7 的 `present` 算子）。

P4 实现要点：

- 工具收敛为 3 个（对应 §5.2 的 #15~#17）：`combridge_waveform`（list / create / remove /
  configure_parser / parse / read / clear / status）、`combridge_protocol`（list / get / load / unload /
  enable / disable / bind / unbind / bound）、`combridge_ota`（protocols / validate / preflight / start /
  status / cancel）。§5.2 里的 `combridge_logs` 按计划并入 `combridge_about(action="logs")`，
  `combridge_gh3036_catalog` 已在 P3 并入 `gh3036_rpc(action="catalog")` —— 至此 **16 个工具全部落地**。
- **波形与 GUI 共用同一份 `WaveformManager`**：MCP `parse` 写入的行，GUI 波形面板立刻可见，反过来也一样。
  `parse` 按行处理并返回 `accepted/rejected` 与最多 5 条失败原因 —— 一段串口数据里混进一行坏数据不该让整段失败。
- **协议插件按 9 个 action 直连 `PluginManager`**，不经过 `#[tauri::command]`：command 层返回 `PluginInfo`
  含 `PathBuf`，且失败用 `ErrorResponse` 包装。MCP 侧转换出扁平的 `PluginView`（path 转字符串、
  state 转稳定小写枚举），错误统一 `PROTOCOL_ERROR` + 恢复提示。
- **OTA 的 dry-run / confirm 硬门**见 §6.4。`mcp/ota_confirm.rs` 只做一件事：
  签发、校验、核销令牌（一次性 / 5 分钟 / 绑定设备+路径+sha256），单测覆盖 7 个边界。
- **日志尾读**先问 `LoggerService::log_path()`（尊重用户在设置里改过的日志路径），
  找不到再回退到 `<exe>/log/` 里最新的 `.log`；只读尾部 256KB 窗口，避免整文件读入。
  这条在冒烟中踩到了：dev 构建的 `log_config.json` 指向 `E:\Tools\ComBridge\log\...`，
  最初按目录扫描拿到的是几个月前的旧日志，改为先问 LoggerService 后才对上。
- **L3 档位硬门提前落地**：`require_dev_tier` 已挂在 `gh3036_factory_test` 的 run/continue 与
  `ota` 的真实执行路径上。标准档返回 `TIER_BLOCKED`（不是静默降级），
  待 P5 的 elicitation 到位后改为「双确认」而非「拒绝」。

P4 冒烟结果（2026-11-04，`npm run tauri dev` + `127.0.0.1:47821`，tier=dev）：

| 验证项 | 结果 |
|---|---|
| `tools/list` | ✅ **16 个**，与 §5.2 收敛后的清单一致 |
| `about(summary)` | ✅ `tool_domains` 含 waveform/protocol/ota，`planned` 已空 |
| `about(logs)` | ✅ 定位到 LoggerService 当前文件，返回真实运行日志（含本次 MCP 会话） |
| `waveform` 全链路 | ✅ create → configure_parser(delimiter) → parse(3 行成功 / 1 行失败并给行号) → read(3×3 数值) → status → clear(保留列名) → list → remove |
| `waveform` 缺参 | ✅ `INVALID_ARGUMENT` + "action=read 缺少必填参数 buffer_id" |
| `protocol` 生命周期 | ✅ 真实 Lua 插件 load（识别 3 个 hook）→ list → enable → bind → bound → unbind → disable → unload → list 为空 |
| `protocol` 异常 | ✅ 不存在插件 `PROTOCOL_NOT_FOUND`；脚本路径不存在 `INVALID_ARGUMENT` |
| `ota(protocols)` | ✅ 返回 smp / goodix 两个协议 |
| `ota(validate/start)` 包不存在 | ✅ `PACKAGE_INVALID`，不进入 dry-run 之外的任何分支 |
| `ota(status)` / `cancel` 未运行 | ✅ `idle` / `OTA_NOT_RUNNING` |
| `ota(start)` 缺 address | ✅ `INVALID_ARGUMENT` |

**未验证**：真实固件包下的 dry-run → confirm → 烧录链路（仓库内无固件样本）；
标准档运行时拦截（`tier_allows_l3` 有单测，但未在 standard 档下实跑）。

P5a 实现要点（后端安全，`docs` 里 §8.3~§8.5 已同步为实际实现）：

- **统一收口**：自己实现 `ServerHandler::call_tool`（`#[tool_handler]` 检测到已存在就不再生成），
  顺序是 **限流 → 端口熔断 → 执行 → 审计**。放在这一层是因为它是唯一不可能被某个工具"忘记"的位置；
  如果按工具逐个加，第 17 个工具一定会漏。
- `peer: Peer<RoleServer>` 作为工具参数注入（rmcp 的 `FromContextPart` 支持），
  因此确认弹窗不需要把 `RequestContext` 手工穿过每一层。
- L3 的语义从 P4 的"标准档直接拒绝"升级为计划的**双确认**：
  机器可校验的 dry-run 令牌（防误触发）+ 用户点确认（防模型自作主张）。
- 限流按全局而非按会话：无状态 HTTP 下 factory 是每请求调用，拿不到稳定会话标识，细节见 §8.5。
- 端口熔断只认 `SERIAL_ERROR`，不认 `TIMEOUT` —— 否则正常调试里的"等不到响应"会反复触发熔断。

P5a 冒烟结果（2026-11-04）：

| 场景 | 档位 | 结果 |
|---|---|---|
| 45 次快速 `about` | dev（20 QPS） | ✅ 0.44s 内 27 通过 / 18 `RATE_LIMITED`（桶容量 20 + 0.44s 补 9，与实现吻合） |
| 冷却 1.2s 后再调 | dev | ✅ 恢复正常 |
| 审计落盘 | dev | ✅ `E:\Tools\ComBridge\log\mcp-audit-2026-11-04.jsonl`，47 行，含 `RATE_LIMITED` 记录 |
| 审计字段 | dev | ✅ `tool/result_code/duration_ms/dry_run/tier` + `args.keys` + `args.sha256`（前 12 位可读） |
| 只读工具 | standard | ✅ `combridge_ports` 正常，不受确认影响 |
| L3 产测 `run` | standard | ✅ `CONFIRM_REQUIRED`「无法向用户确认：客户端未声明 elicitation 能力」——问不到即拒绝 |
| 非法租约写入 | standard | ✅ `LEASE_INVALID`（确认在租约校验之后，不打扰用户） |
| 18 次快速 `about` | standard（5 QPS） | ✅ 2 通过 / 16 `RATE_LIMITED`（前序 3 次调用已耗掉 3 个令牌） |

elicitation 正向路径（2026-11-04，`scripts/mcp/elicit_probe.py`，声明 `elicitation` 能力的脚本客户端）：

| 客户端回应 | 结果 |
|---|---|
| `accept` + `confirm:true` | ✅ 工具**继续执行**（返回 `FACTORY_TEST_STATE`「产测未在运行」，而非 `CONFIRM_REQUIRED`） |
| `decline` | ✅ `CONFIRM_REQUIRED`「用户未确认：用户拒绝」，工具未执行 |
| `cancel`（关弹窗） | ✅ `CONFIRM_REQUIRED`「用户未确认：用户取消」 |
| `accept` + `confirm:false` | ✅ `CONFIRM_REQUIRED`「用户未确认：用户未勾选确认项」 |
| 未声明能力（原 P5a 冒烟） | ✅ `CONFIRM_REQUIRED`「无法向用户确认：客户端未声明 elicitation 能力」 |

服务端发出的请求本身也一并核对：`message` 带上「【GH3036 产测】即将执行产测 action=stop（最长等待 180s）」，
`requestedSchema` 是 `type=object` + 单个 `confirm: boolean`（required），符合规范对 elicitation schema 的要求。
注意脚本必须**边读 SSE 边回应**——用 `urlopen().read()` 一次性读完会在等待客户端回应时死锁。

**仍未验证**：WorkBuddy / Claude Desktop 这类带 UI 的客户端上，弹窗的渲染与点击路径
（客户端侧实现已确认存在；token 已持久化、信任记录可预置，剩下唯一的阻塞是「需新会话建立连接」，
见 §8.7。**这一步本质上需要一次人工点击** —— 点击正是被测对象）。


### 9.1 阶段划分

| 阶段 | 内容 | 人力 | 交付物 | 验收 |
|---|---|---|---|---|
| **P0 骨架** | 依赖引入；`mcp/` 模块；token + 认证 + 只绑 loopback；endpoint 发现文件；`combridge_about` 单工具打通 | 2~3 天 | 客户端 `tools/list` 可见 1 个工具并可调用 | 握手成功；`about` 返回版本与档位 |
| **P1 串口闭环** | `ports` / `serial_open` / `close` / `write` / `read` / `expect`；`RingBuffer.total_written` + `read_since`；msgpack 订阅；租约机制 | 4~5 天 | 6 个工具 | 剧本 A 全流程零人工 |
| **P2 设备与 BLE** | `devices` / `ble_scan` / `ble_connect` / `ble_gatt` | 3~4 天 | 4 个工具 | 剧本 B：扫到真机 → 连接 → 订阅 → 采 20 帧 |
| **P3 GH3036 与产测** | `gh3036_rpc`；`gh3036_factory_test`（含同步出结果 D2）；阈值子命令 | 4~5 天 | 2 个工具 | 剧本 C：初始化 → 读版本 → 产测 → pass/fail |
| **P4 其余业务域** | `waveform` / `protocol` / `ota`（含 dry-run + confirm）；`about` 内 `catalog` / `logs` 子命令 | 4~5 天 | 4 个工具 | 剧本 D：OTA 未确认不可执行 |
| **P5 安全与产品化** | 两档策略；elicitation；审计；限流熔断；设置页「集成」；客户端配置一键复制；i18n 中英 | 4~5 天 | 设置页 UI + 审计日志 + 文档 | S4/S5/S6 通过 |
| **P6 拆分 core + headless** | workspace 化；`combridge-core`；`StateEmitter` 抽象；`combridge-mcp` bin（stdio + http + print-config）；**可选下载打包**（D3） | 8~10 天 | 独立二进制 + 下载/校验/安装链路 | 无 GUI 环境跑通剧本 A~C |
| **P7（远期，可选）** | daemon 化：GUI 变 client，设备唯一 owner 是守护进程 | 15+ 天 | — | 仅在需要多客户端真正共享设备时启动 |

**累计**：P0~P5 约 **21~27 工作日**；P6 再 **8~10 天**。

### 9.2 关键路径与并行建议

```
P0 ──► P1 ──┬─► P2 ──┐
            ├─► P3 ──┼─► P5 ──► P6
            └─► P4 ──┘
```

- P2 / P3 / P4 三块相互独立，可由两人并行（P3 依赖 P1 的串口闭环，P2 只依赖 P0）
- P5 的安全机制（elicitation / 审计 / 限流）可以在 P1 完成后就开始搭，与 P2~P4 并行，不必等最后
- 建议把 P5 的「审计 + 租约」提前到 P1 —— 这两个是数据正确性问题，晚做要返工

### 9.3 headless 可选下载（D3）

不进主安装包，作为独立可选组件：

**发布侧**
- release 增加资产 `combridge-mcp-<version>-<target>.exe` + `.sha256`
- 与主包同版本号构建，CI 从同一个 tag 产出

**应用侧（设置 → 集成）**
1. 展示"headless 服务：未安装 / 已安装 v0.7.2"
2. 点击下载 → 从 release 资产拉取，显示进度
3. 校验 sha256 + 校验版本 minor 与当前 App 一致
4. 落到 `%LOCALAPPDATA%\combridge\mcp-server\combridge-mcp.exe`（不与主 exe 同目录，避免被安装器覆盖/卸载残留）
5. 安装后提供「复制客户端配置」按钮（stdio 形态，含完整路径）
6. 支持卸载（删除目录 + 清理配置）

**互不影响**：未下载时 GUI 内嵌的 HTTP MCP 照常可用；已下载时两者可选其一运行，通过租约与单实例锁避免同时抢占设备。

### 9.4 目录与文件清单（Phase 1 落地）

```
src-tauri/src/
├── mcp/
│   ├── mod.rs            # start_mcp / McpConfig / McpContext 装配
│   ├── auth.rs           # bearer token 中间件 + Host 校验
│   ├── ctx.rs            # McpContext（各 Manager Arc 句柄）
│   ├── server.rs         # ServerHandler + tool_router
│   ├── hub.rs            # msgpack 订阅 → 每端口游标缓冲 + Notify
│   ├── lease.rs          # 端口/设备租约 + TTL 回收
│   ├── jobs.rs           # 长任务进度快照
│   ├── audit.rs          # JSONL 审计
│   ├── tier.rs           # 档位策略与拦截
│   └── tools/
│       ├── mod.rs
│       ├── about.rs
│       ├── serial.rs
│       ├── ble.rs
│       ├── devices.rs
│       ├── gh3036.rs
│       ├── waveform.rs
│       ├── protocol.rs
│       ├── ota.rs
│       └── logs.rs
└── §2.2 的 4 处耦合点改造（Phase 2 才需要动）
```

前端新增（P5）：

```
src/pages/Settings/McpIntegration.tsx    # 开关、档位、端口、配置复制、headless 下载
src/api/mcp.ts                           # 读取 endpoint / 状态 / 生成配置
src/locales/{zh-CN,en-US}/mcp.json
```

### 9.5 里程碑与验证剧本

每个阶段收尾都跑同一组剧本，观察模型能否**只靠工具描述**完成：

| 剧本 | 步骤 | 通过标准 | 前置阶段 |
|---|---|---|---|
| A 串口 | 列端口 → open COM3@115200 → 发 `AT\r\n` → expect `OK` | 零人工，≤4 次工具调用 | P1 |
| B BLE | 扫描 → 连接 → 列 service → 订阅 notify → 读 20 帧 | 拿到 20 帧且无丢帧 | P2 |
| C 产测 | 初始化 → 读版本 → `factory_test(run)` → 报 pass/fail | 单次调用返回结论 + CSV 路径 | P3 |
| D OTA | 列协议 → validate → preflight → **用户拒绝** → 确认后 start | 拒绝时无任何写入 | P4/P5 |
| E 并发 | 两个客户端同时 open COM3 | 第二个得 `PORT_BUSY` 且提示 lease | P1 |
| F 体积 | `tools/list` 全量响应 | < 10k tokens | P5 |

剧本 A 通过 = 工具粒度设计成立；C/D 通过 = 长任务与安全模型成立；E 通过 = 并发模型成立。

### 9.6 远期形态（仅在明确需要时启动）

若将来要"多个 AI 客户端真正共享同一套硬件"，才进入 daemon 化：

- 独立守护进程为设备唯一 owner，暴露本地 IPC（gRPC / Unix socket / named pipe）
- GUI 与 headless MCP 都变成 client
- 成本约为 P0~P5 的两倍，且引入 IPC 序列化、守护进程生命周期、崩溃恢复、版本协商等问题
- **不建议在没有真实需求前启动**

---

## 10. 风险登记册

| # | 风险 | 影响 | 概率 | 对策 | 责任阶段 |
|---|---|---|---|---|---|
| R1 | 模型误刷固件 / 误改阈值 | 设备变砖、产线停摆 | 中 | L3 强制 dry-run + confirm token + elicitation + GUI 弹窗 | P4/P5 |
| R2 | 两个 AI 抢同一端口 | 数据错乱、互相报错 | **高** | 设备租约 + `PORT_BUSY` 明确提示 + TTL | P1 |
| R3 | 客户端 60s 超时打断长任务 | 半包状态、设备异常 | 中 | job 模型；产测改同步后内部等待 + 超时降级 | P3 |
| R4 | 工具过多导致模型选错 | 体验差 | 中 | 16 个域级工具 + prompt 模板 | P5 |
| R5 | 单次返回数据过大 | 上下文爆掉 | **高** | 游标 + 上限 + 截断标注 + 统计摘要 | P1 |
| R6 | Tauri 升级导致 command 层变动 | MCP 工具失效 | 低 | 工具直接依赖 Manager，不依赖 command | 设计已规避 |
| R7 | 端口 47821 被占用 | 启动失败 | 中 | 回退自动分配 + 更新 endpoint 文件 | P0 |
| R8 | rmcp 版本破坏性变更 | 编译失败 | 中 | 精确 pin；隔离在 `mcp/` 内 | P0 |
| R9 | headless 与 GUI 版本错配 | 行为不一致 | 中 | 启动校验 minor 版本，不匹配拒绝启动 | P6 |
| R10 | 审计写盘失败 | 无法追溯 | 低 | 降级告警但不阻断业务 | P5 |
| R11 | 用户把 MCP 端口暴露到公网 | 设备被远程操控 | 低 | 只绑 loopback + Host 校验 + token | P0 |
| R12 | C 库（gh3036 RPC）在多线程下的竞态 | 崩溃 | 中 | 沿用现有 `unsafe impl Send/Sync` + 串行化入口 | P3 |

---

## 11. 环境前置条件（执行前必须解决）

**当前阻塞项**：`cargo` 无法拉取新依赖。

项目 `.cargo/config.toml` 将 crates.io 全局替换为 `rsproxy.cn`，而该源当前不可达：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'
[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"
```

实测连通性：

| 源 | 结果 |
|---|---|
| `rsproxy.cn` | ✗ SSL 握手失败 / 连接超时 |
| `index.crates.io` | ✓ 200 |
| `mirrors.ustc.edu.cn` | ✓ 200 |
| `mirrors.tuna.tsinghua.edu.cn` | ✓ 200 |

`rmcp` 与 `axum` 本地 registry 缓存中均不存在，因此 P0 开工前必须先把源切到可用地址（建议 USTC 或清华镜像）。该项修改会影响所有协作者的构建，需由仓库维护者确认后再提交。

**其他前置**

| 项 | 要求 |
|---|---|
| Rust | ≥ 1.90（当前 1.90.0，满足） |
| 构建缓存 | `src-tauri/target` 已有 21G，增量构建正常 |
| 单实例机制 | P0 前确认是否已有；没有则 P0 内补 |
| 测试硬件 | P1 需要至少一个真实串口设备；P2 需要 BLE 适配器 + 目标设备；P3 需要 GH3036 板 |
| MCP 客户端 | Claude Code / Cursor / Codex CLI 任一，用于联调 |

---

## 12. 附录

### 12.1 客户端接入配置（Phase 1，HTTP）

```json
{
  "mcpServers": {
    "combridge": {
      "type": "http",
      "url": "http://127.0.0.1:47821/mcp",
      "headers": { "Authorization": "Bearer <token from mcp-endpoint.json>" }
    }
  }
}
```

### 12.2 客户端接入配置（Phase 2，stdio）

```json
{
  "mcpServers": {
    "combridge": {
      "command": "C:\\Users\\<user>\\AppData\\Local\\combridge\\mcp-server\\combridge-mcp.exe",
      "args": ["--port", "COM3", "--baud", "921600"]
    }
  }
}
```

### 12.3 工具命名与描述规范

- 工具名：`combridge_<域>_<动作>`，全小写蛇形，前缀固定
- `description` 必须包含三部分：**做什么** / **什么时候用** / **返回什么形状**
- 参数名蛇形；布尔参数避免否定式（用 `include_x` 而非 `no_x`）
- 所有返回对象必须可 JSON 序列化且字段稳定；新增字段只增不改名

### 12.4 与现有文档的关系

| 文档 | 关系 |
|---|---|
| `docs/api.md` | Tauri command 面，MCP 工具**不直接复用**，仅作能力索引 |
| `docs/architecture/` | 分层与数据流，本方案的 §2 是其浓缩 |
| `docs/event-bus-events.yaml` | MCP 订阅数据流的依据（topic + 编码） |

---

## 13. 结论

ComBridge 的架构天然适合释放 MCP：业务内核与 UI 解耦良好，事件总线已经是现成的数据通道，Manager 全部是显式构造的 `Arc`。**唯一必须正视的两件事是并发租约与高危操作的硬门禁** —— 这两点做好了，剩下都是工程实现。

建议按 P0 → P1 先把串口闭环打通（约 1.5 周），用剧本 A 验证工具粒度是否合理，再决定是否全量铺开。**先证明模型能可靠地开一个串口，再谈接管产线和 OTA。**
