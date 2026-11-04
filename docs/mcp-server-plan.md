# ComBridge MCP Server 设计与实施计划（最终版）

| 项 | 内容 |
|---|---|
| 文档版本 | v1.0 FINAL |
| 日期 | 2026-11-04 |
| 适用代码版本 | ComBridge v0.7.2（Tauri 2.0，后端 Rust 10,521 行） |
| 状态 | 方案定稿，可进入执行 |
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
2. 生成 32 字节随机 token（每次启动轮换）
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

> 表中 18 行，其中 `about` 吸收 `catalog` 与 `logs` 的子命令后，对外实际 **16 个工具**（`catalog` / `logs` 作为 `about` 的 `action` 保留，避免模型记不住）。

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
{ "action": "start", "package_path": "D:/fw.zip", "protocol": "goodix" }
// 返回
{
  "dry_run": true,
  "plan": { "protocol": "goodix", "target": "AA:BB:CC:DD:EE:FF", "image_size": 245760, "est_seconds": 68 },
  "warnings": ["目标电量未读取"],
  "confirm_token": "ota_9f2c"   // 需在下次调用原样带回
}
// 第二次调用：带 confirm 才真正执行
{ "action": "start", "package_path": "D:/fw.zip", "protocol": "goodix", "confirm": "ota_9f2c" }
```

`confirm` 一次性有效、5 分钟过期、绑定包路径 hash。**没有正确 `confirm` 时，`start` 永远只做 dry-run**，这不是提示词层面的约定，而是代码里的硬门。

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

错误码清单（初版）：`PORT_BUSY` / `PORT_NOT_FOUND` / `PORT_NOT_OPEN` / `LEASE_EXPIRED` / `DEVICE_DISCONNECTED` / `TIMEOUT` / `INVALID_PARAM` / `CONFIRM_REQUIRED` / `TIER_BLOCKED` / `HW_ERROR` / `JOB_NOT_FOUND` / `RATE_LIMITED`。

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

**实现**：档位是服务端强制的，不依赖客户端行为。`TIER_BLOCKED` 错误码用于档位拒绝。

### 8.2 网络与认证

| 层 | 措施 |
|---|---|
| 绑定 | 仅 `127.0.0.1`，**禁止** `0.0.0.0`（`axum::serve` 前显式校验） |
| Host 校验 | 校验 `Host` 头为 `127.0.0.1` / `localhost`，防 DNS rebinding |
| 认证 | 启动生成 32 字节随机 token，`Authorization: Bearer`；每次启动轮换 |
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

### 8.4 审计

- 路径：`log/mcp-audit-YYYY-MM-DD.jsonl`
- 每条记录：`ts` / `session_id` / `tier` / `tool` / `args_digest`（参数结构 + 数据字段的 sha256 与前 64 字节 hex）/ `result_code` / `duration_ms` / `dry_run`
- 审计不可关闭；写失败时降级为 `tracing::error` 但**不阻断**业务（避免审计故障导致产线停摆）
- 保留 30 天，随现有日志轮转策略清理

### 8.5 熔断与限流

- 每会话 20 QPS（研发）/ 5 QPS（标准），全局 100 QPS
- `serial_expect` 并发上限 8，超出排队等待而非拒绝
- 同一端口连续 5 次硬件错误 → 暂停该端口 30s，返回明确原因与恢复方式

---

## 9. 执行计划

### 9.0 实施进度

> 分支 `feat/mcp-server`，随代码同步更新。

| 阶段 | 状态 | 落地位置 / 说明 |
|---|---|---|
| P0 骨架 | ✅ 已完成 | `src-tauri/src/mcp/{mod,auth,ctx,server}.rs`；`rmcp = "=3.8.0"`；默认关闭，`%LOCALAPPDATA%\combridge\mcp.json` 开启 |
| P1 串口闭环 | ✅ 已完成 | 新增 `mcp/{lease,serial_hub}.rs`；7 个工具（`about` + `ports` + 5 个 `serial_*`）；租约随 P1 一起落地（按 §9.2 建议提前） |
| P2 设备与 BLE | ✅ 已完成 | 新增 `mcp/ble_hub.rs`；工具 `combridge_devices` / `combridge_ble_scan` / `combridge_ble_connect` / `combridge_ble_gatt`；BLE 连接租约资源名 `ble:<address>` |
| P3 GH3036 与产测 | 待开始 | — |
| P4 其余业务域 | 待开始 | — |
| P5 安全与产品化 | 部分前置 | 租约已在 P1 完成；两档策略的 UI 与 elicitation 待 P5 |

P1 实现要点（与本文档契约的对应）：

- `combridge_serial_read`：游标 = hub 内单调递增的累计字节数；`since` 省略表示"从缓冲区最旧处"；缓冲上限 1MB，被裁剪时返回 `truncated=true`；broadcast 消费滞后时返回 `lagged=true`。
- `combridge_serial_expect`：`since` 省略表示"从此刻起只看新数据"；服务端阻塞等待，命中即返回；超时返回 `matched=false` 与已收到的全部文本；端口中途断开返回 `disconnected=true`。
- 租约：`combridge_serial_open` 返回 `lease_id`，`write` / `close` 强制校验；`PORT_BUSY` 回传当前占用者与过期时间，`force=true` 可抢占。GUI 打开的端口 MCP 可读可写（持租约），但不允许 `close`。

P2 实现要点：

- 工具收敛为 4 个（对应 §5.2 的 #8~#11）：`combridge_devices` 汇总设备/连接/租约，`combridge_ble_scan`（阻塞扫描或 stop），`combridge_ble_connect`（connect/disconnect/list，租约资源 `ble:<address>`），`combridge_ble_gatt`（9 个 action：discover_services / discover_characteristics / read / write / subscribe / unsubscribe / notifications / mtu / rssi）。
- 通知数据：`ble:data` 与串口同样走 msgpack 广播，`mcp/ble_hub.rs` 按 `address|char_uuid` 分桶缓存并提供游标读取；`subscribe` 时清空旧桶（游标从订阅时刻起算）。
- 租约粒度：写/订阅/取消订阅/改 MTU 需要租约（会改设备状态）；discover/read/notifications/rssi 免租约。
- 二进制建议：GATT 读写用 `encoding=hex`，text 模式为 UTF-8 lossy，仅适合 ASCII 载荷。

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
