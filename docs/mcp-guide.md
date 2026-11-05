# ComBridge MCP 使用指南

面向两类读者：**接进来的 AI 客户端**（按本文学会正确调用）和**维护者**（改工具时同步这里）。

> 服务端还提供了机器可读版本：调用 `combridge_tools_guide`（不传参返回全部工具，传 `tool` 只返回该工具）。
> 两者内容同源（`src-tauri/src/mcp/guide.rs`），**不一致时以接口返回为准**。

---

## 1. 服务是什么

ComBridge 是一个串口 / BLE / 芯片协议调试工具。MCP 服务把它已有的能力暴露给 AI：
列端口、收发串口、扫描连接 BLE、读写 GATT、驱动 GH3036 芯片协议与产测、波形缓冲、Lua 协议插件、BLE OTA。

设计取向是"**可安全地自动化**"：

- 单芯片 / 单端口是独占资源 → 用**租约**管住并发
- 危险动作两段式：先 dry-run 拿令牌，再带令牌执行
- 失败返回结构化错误（`error_code` + `hint` + `retryable`），不靠模型猜

## 2. 接入

### 2.1 开启服务

设置页 → MCP → 启用。服务默认关闭。启用后写入发现文件：

```
%LOCALAPPDATA%\combridge\mcp-endpoint.json
{
  "url": "http://127.0.0.1:47821/mcp",
  "token": "<Bearer token>",
  "tier": "standard",
  "app_version": "0.8.0"
}
```

客户端用 `Authorization: Bearer <token>` 访问 `url`。token 持久化，只有点"轮换"才作废 —— 所以配置一次即可，重启不用重贴。

启用了 `auto_wire` 时，应用启动会自动把本机服务写进本地 MCP 客户端配置。

### 2.2 档位（tier）

| 档位 | 行为 | 适用 |
|---|---|---|
| `standard`（默认） | 写设备 / 产测 / 烧录前向客户端请求用户确认；OTA `start` 直接 `TIER_BLOCKED` | 日常、共享机器 |
| `dev` | 免确认 | 自用、连续调试 |

改档位在设置页，**立即生效，不需要重启服务**（内部是共享 cell）。

### 2.3 会话开始做什么

```
1) combridge_tools_guide            # 读全局约定 + 错误码（一次就够）
2) combridge_about                  # 确认版本/档位/在线设备
3) combridge_ports / combridge_devices   # 拿真实端口名或 BLE 地址
```

不要跳过第 3 步去猜端口名。`COM3`、`AA:BB:...` 这类标识一律来自工具返回。

---

## 3. 全局约定

| 约定 | 说明 |
|---|---|
| **租约 lease** | 对设备有副作用的操作都要带 `lease_id`。`serial_open`、`ble_connect`、首次 GH3036 调用会签发。同一 `owner` 重复调用沿用旧租约。 |
| **owner** | 默认 `ai-client`，建议填客户端名（`claude-code`），方便在 GUI 租约面板认出占用者。 |
| **抢占 force** | 返回 `*_BUSY` 时先确认对方已放弃，再 `force=true`。抢占会打断对方正在跑的任务。 |
| **游标 cursor** | `serial_read` / `serial_expect` / `ble_gatt(notifications)` 都是游标语义：首次省略 `since` 读历史，之后**必须回传** `next_cursor`。 |
| **编码 encoding** | 文本 `text`；二进制 `hex`（`"AA BB 01"` 或 `"AABB01"` 均可）。固件帧、寄存器值一律用 hex。 |
| **只读工具** | `about` / `ports` / `devices` / `ble_scan` / `serial_read` / `serial_expect` / `tools_guide` 不需要租约，也不改状态。 |
| **错误模型** | `isError=true` 时看 `structuredContent.error_code` → `hint`（下一步该做什么）→ `retryable`（原样重试是否有意义）。`INVALID_ARGUMENT` 重试无意义。 |

### 3.1 典型错误码

| 码 | 含义与处置 |
|---|---|
| `PORT_BUSY` | 端口被别人占；查 `active_leases`，确认后可 `force=true` |
| `PORT_NOT_OWNED_BY_MCP` | 端口由 GUI 打开，MCP 不能关；需独占请先在 GUI 关闭 |
| `LEASE_REQUIRED` / `LEASE_EXPIRED` | 缺租约或已过期；重新 open / connect |
| `BLE_NOT_CONFIGURED` | BLE 后端未启用；去 GUI 打开 |
| `BLE_BUSY` | 设备被别的客户端连着 |
| `CHAR_NOT_FOUND` | UUID 不对或没先 `discover_services` |
| `GH3036_INIT_FAILED` | 协议栈初始化失败；重试 `action=init`（幂等） |
| `GH3036_NO_TX_CHANNEL` | 没配 TX 通道；先 `action=set_channel` |
| `GH3036_RPC_FAILED` | 通道没通、设备没响应、或参数格式不对（对照 `action=catalog`） |
| `TIER_BLOCKED` | 当前档位禁止（OTA 烧录）；改档或走 GUI |
| `OTA_CONFIRM_REQUIRED` / `OTA_CONFIRM_INVALID` | 缺令牌 / 令牌过期或与目标不匹配；重新 dry-run |
| `CONFIRM_DENIED` | 用户在弹窗里拒绝；不要立刻重试同一操作 |
| `RATE_LIMITED` | 超档位限流；降频 |

---

## 4. 工具参考

### 4.1 core

#### `combridge_tools_guide`

| | |
|---|---|
| 参数 | `tool?`（工具名，可省 `combridge_` 前缀）、`domain?`（`core\|serial\|ble\|gh3036\|waveform\|protocol\|ota`）、`include_conventions?`（默认 true） |
| 返回 | `total`、`tools[]`（`summary` / `prerequisites` / `notes` / `example` / `risk`）、`conventions`（全局规则 + 错误码） |
| 注意 | 无前置条件。第一次连上建议不带参数调一次 |

```json
{"tool": "combridge_gh3036_rpc"}
```

#### `combridge_about`

| | |
|---|---|
| 参数 | `action?`（`summary` 默认 / `logs`）、`lines?`（默认 100，上限 500） |
| 返回 | 版本、档位、已连接设备、工具域；`logs` 时返回日志路径与尾部 |
| 注意 | `action=logs` 是排障主力：任何工具报错而 `hint` 不够具体时先读它 |

```json
{"action": "logs", "lines": 200}
```

### 4.2 serial

#### `combridge_ports`

无参数。返回端口名、类型、制造商/产品/序列号、`open`（是否已打开）、`lease`（占用者）。
**串口操作前必调**：端口名只能从这里拿。

#### `combridge_serial_open`

| 参数 | 默认 | 说明 |
|---|---|---|
| `port_name` | 必填 | 来自 `combridge_ports` |
| `baud_rate` | 115200 | |
| `data_bits` / `parity` / `stop_bits` | 8 / none / 1 | |
| `flow_control` | none | `none\|software\|hardware` |
| `owner` | ai-client | |
| `ttl_s` | 3600 | 上限 86400 |
| `force` | false | 抢占他人租约 |

返回 `lease_id`、`cursor`（后续 read 的起点）、`already_open`。
`already_open=true` 表示端口已被打开（可能是 GUI）：**可读、不可 close**。

```json
{"port_name": "COM6", "baud_rate": 115200, "owner": "claude-code"}
```

#### `combridge_serial_write`

必带 `lease_id`。`encoding=text|hex`、`append_crlf`（AT 指令常用）。
写成功只代表进了系统缓冲 —— 要确认设备响应，紧跟 `combridge_serial_expect`。

#### `combridge_serial_read`

`since?`、`encoding?`。**不需要租约**。
缓冲上限 1MB，久不读会丢数据并置 `truncated=true`；`lagged=true` 表示区间内可能有缺口。

#### `combridge_serial_expect`

```json
{"port_name": "COM6", "pattern": "OK", "timeout_ms": 2000}
```

- 默认只匹配 `since` 之后的新数据；要匹配等待前已收到的内容，传 `read` 拿到的 `since`
- 超时返回 `matched=false`，但 `buffer` 里带全部已收文本 —— **直接看 buffer 判断设备到底回了什么**
- `disconnected=true`：等待期间设备掉线，别重试
- 这是与设备交互的首选方式，比 write + read 轮询稳

#### `combridge_serial_close`

必带 `lease_id`，且端口必须由 MCP 打开。用完就关，否则下一个客户端吃 `PORT_BUSY`。

### 4.3 ble

#### `combridge_devices`

无参数。返回 `ble_mode`、`ble_configured`、`ble_connections[]`（含 `subscriptions`）、`app_devices`、`active_leases`。
`ble_configured=false` 时所有 BLE 工具不可用。

#### `combridge_ble_scan`

`duration_ms?`（默认 3000）、`stop?`。返回 `address/name/rssi/is_connectable`。
**已连接的设备通常不再出现在扫描结果里** —— 用 `combridge_devices` 看。扫描独占适配器，扫完就停。

#### `combridge_ble_connect`

`action=connect|disconnect|list`，`address`、`lease_id?`、`owner?`、`ttl_s?`、`force?`。
`connect` 返回 `lease_id`；连上后**必须** `discover_services` 才能读写。

> 被动断连后设备可能不再广播：直接再 `connect` 同一地址即可，服务端会从系统"已连接设备"列表里找回，**不需要复位器件**。

#### `combridge_ble_gatt`

| action | 额外参数 | 需要 lease |
|---|---|---|
| `discover_services` | — | 否 |
| `discover_characteristics` | `service_uuid` | 否 |
| `read` | `char_uuid`、`encoding` | 否 |
| `write` | `char_uuid`、`data`、`encoding`、`without_response` | 是 |
| `subscribe` / `unsubscribe` | `char_uuid` | 是 |
| `notifications` | `char_uuid`、`since`、`encoding` | 否 |
| `mtu` | `mtu`（默认 247） | 是 |
| `rssi` | — | 否 |

标准顺序：`discover_services` → `discover_characteristics(service_uuid)` → `subscribe(char_uuid)` → `notifications` 轮询。
UUID 用返回值原样，不要自己补 128 位（服务端已做大小写与连字符规范化）。
Windows 后端 MTU 可能无法真正协商，返回值以实际为准，别按 247 假设单包长度。

### 4.4 gh3036

#### `combridge_gh3036_rpc`

| action | 用途 | 关键参数 |
|---|---|---|
| `status` | 看初始化 / 通道 / 产测状态 | — |
| `init` | 初始化协议栈（幂等） | — |
| `set_channel` | 配 TX/RX 通道 | `direction`、`channel_type`、`device_id`、`characteristic_uuid` |
| `catalog` | 列出可用 RPC 命令键 | — |
| `rpc` | 执行一条 RPC | `command_key`、`params[]` |

**顺序**：`set_channel(direction=both)` → `catalog` → `rpc`。没配 TX 通道直接发 RPC 会返回 `GH3036_NO_TX_CHANNEL`。

- **协议栈在应用启动时已自动初始化（含 CSV 偏好）**，一般不需要手动 `init`；只有报 `GH3036_INIT_FAILED` 时补一次，`init` 幂等
- `command_key` 必须来自 `catalog`，不要凭经验编；`params` 按目录顺序，十六进制可带 `0x`
- GH3036 是**单芯片会话**：别的客户端持租约时 `GH3036_BUSY`，`force=true` 抢占
- CSV 是否落盘由 **GUI 偏好**（enabled + 输出目录）决定，MCP 不改偏好。采到数据却没有文件 → 先查偏好

```json
{"action": "set_channel", "channel_type": "ble", "device_id": "AA:BB:CC:DD:EE:FF",
 "characteristic_uuid": "00000002-0000-1000-8000-00805f9b34fb", "direction": "both"}
```

```json
{"action": "rpc", "command_key": "S", "params": ["4", "0"]}
```

#### `combridge_gh3036_factory_test`

`action=run` 是主路径：内部完成 启动 → 轮询 → 等待 → 汇总，**一次调用直接返回结论**（`pass` / `failed_item` / `items[]` / `csv_path`）。不要自己拆步骤轮询状态。

- `max_wait_s` 默认 180、上限 600；超时返回 `status=running` + `job_id`，用 `action=status` 续等（job 不丢）
- 返回 `waiting_environment_switch` 时必须**人工切环境**再 `action=continue`；重新 `run` 会从头开始
- `stop` 中止 job，不释放租约
- `validate_config` 不连设备，适合测前自查卡控文件

### 4.5 gh3220

#### `combridge_gh3220`

| action | 用途 | 关键参数 |
|---|---|---|
| `status` | 连接/采集/解码统计（无需连接） | — |
| `info` | 读版本、最大包长、已开功能 | — |
| `connect` / `disconnect` | 建立/断开会话 | `address`、`kind`（ble 默认 / tcp 调试） |
| `work_mode` | 设置工作模式与功能位 | `mode`、`func_mask` |
| `start` / `stop` | 启动/停止采样 | `func_mask`、`slot_en` |
| `switch_func` | 槽位功能开关 | `slot`、`extra`、`func_mask`、`on` |
| `reg_read` / `reg_write` | 寄存器读写 | `addr`、`count` / `values[]` |
| `load_reg_list` | 批量寄存器表 | `file_path`（每行 addr,value） |
| `chip_ctrl` | 唤醒/休眠/复位 | `chip_action` |
| `capture_start` / `capture_stop` | 采集 + CSV/波形 | `func_mask`、`auto_start`、`channels_override` |
| `send_raw` | 发送原始帧 | `hex` |

**顺序**：`connect` → `info` → `work_mode` → `start`。`WORK_MODE` 是 `START` 的前置条件。
`capture_start` 默认 `auto_start=true`，会自动完成 `WORK_MODE + START` 并开启 CSV/波形。

- GH3220 是**单会话**：GUI 已连接时先断开再 `connect`。
- 真机 `STOP` 后通常需要复位才能再次 `START`；`auto_stop` 默认 `false`。
- `0x16` 事件由后端自动 ACK，无需额外操作。
- 调试 TCP 通道默认关闭，仅用于接本地 `uprotocol-sim`。

```json
{"action": "capture_start", "func_mask": 2, "channels_override": 4}
```

### 4.6 waveform

| action | 说明 |
|---|---|
| `list` | 列出缓冲 ID |
| `create` | `buffer_id`，可选 `capacity`（默认 1000）、`column_names`（默认 CH0..CH4） |
| `configure_parser` | `parser.parser_type=delimiter`（`delimiter` 默认 ","）或 `regex`（`pattern`） |
| `parse` | `data` 可含多行，每行一条样本 |
| `read` | `rows`（默认 200，上限 5000） |
| `clear` / `status` / `remove` | |

典型链路：`create` → `configure_parser` → `parse(串口整段文本)` → `read`。
`parse` 返回 `accepted` / `rejected` / `errors`（前 5 条失败原因）。
缓冲与 GUI 波形面板**共用**，写进去 GUI 立刻可见。

### 4.7 protocol

`action=list|get|load|unload|enable|disable|bind|unbind|bound`，`plugin_id`、`path`（load 用绝对路径）、`device_id`（bind 用）。

- `disable` 只停止响应事件，脚本与 hook 仍占资源；**彻底移除用 `unload`**
- 插件必须 `bind` 到设备才处理该设备的数据
- `load` 失败时 `plugin.error` 带 Lua 报错行号

### 4.8 ota

`action=protocols|validate|preflight|start|status|cancel`。

- `validate` 只校验包、**不连设备**，最便宜的预检
- `preflight` 连设备读版本，能提前发现型号不匹配
- **`start` 不带 `confirm` 永远只做 dry-run**：返回 `plan`、`warnings`、`confirm_token`，不烧录
- 令牌一次性、5 分钟过期，绑定 **目标设备 + 固件路径 + 镜像 sha256** —— 换设备或换包必须重新 dry-run
- `standard` 档下 `start` 直接 `TIER_BLOCKED`
- 升级过程中不要 disconnect，也不要让机器休眠

```json
{"action": "start", "address": "AA:BB:CC:DD:EE:FF", "firmware_path": "D:/fw/app.zip"}
```

---

## 5. 常见任务剧本

### 5.1 串口 AT 交互

```
combridge_ports
combridge_serial_open      {"port_name":"COM6","baud_rate":115200,"owner":"claude-code"}
combridge_serial_write     {"port_name":"COM6","lease_id":"...","data":"AT+RST","append_crlf":true}
combridge_serial_expect    {"port_name":"COM6","pattern":"OK","timeout_ms":3000}
combridge_serial_close     {"port_name":"COM6","lease_id":"..."}
```

### 5.2 BLE 读传感器通知

```
combridge_ble_scan                     {"duration_ms":5000}
combridge_ble_connect                  {"action":"connect","address":"AA:BB:..."}
combridge_ble_gatt                     {"action":"discover_services","address":"AA:BB:..."}
combridge_ble_gatt                     {"action":"subscribe","address":"AA:BB:...","char_uuid":"00000003-...","lease_id":"..."}
combridge_ble_gatt                     {"action":"notifications","address":"AA:BB:...","char_uuid":"00000003-..."}
```

### 5.3 GH3036 采 SpO2

```
combridge_gh3036_rpc   {"action":"set_channel","channel_type":"ble","device_id":"AA:BB:...","characteristic_uuid":"00000002-...","direction":"both"}
combridge_gh3036_rpc   {"action":"catalog"}
combridge_gh3036_rpc   {"action":"rpc","command_key":"S","params":["4","0"]}   # 4=SpO2，0=单功能
# 数据写到 GUI 偏好设定的 CSV 目录；停止采样：
combridge_gh3036_rpc   {"action":"rpc","command_key":"S","params":["255"]}
```

### 5.4 GH3220 采集

```
combridge_gh3220  {"action":"connect","address":"AA:BB:..."}
combridge_gh3220  {"action":"info"}
combridge_gh3220  {"action":"capture_start","func_mask":2}
# 数据写入 GUI 偏好目录，同时生成 gh3220:funcN 波形缓冲
combridge_gh3220  {"action":"status"}
combridge_gh3220  {"action":"capture_stop"}
```

### 5.5 产测

```
combridge_gh3036_factory_test  {"action":"validate_config"}
combridge_gh3036_factory_test  {"action":"run","max_wait_s":240}
# status=running 时
combridge_gh3036_factory_test  {"action":"status"}
```

### 5.6 OTA

```
combridge_ota  {"action":"validate","firmware_path":"D:/fw/app.zip"}
combridge_ota  {"action":"preflight","address":"AA:BB:..."}
combridge_ota  {"action":"start","address":"AA:BB:...","firmware_path":"D:/fw/app.zip"}      # dry-run
combridge_ota  {"action":"start","address":"AA:BB:...","firmware_path":"D:/fw/app.zip","confirm":"<token>"}
combridge_ota  {"action":"status"}
```

---

## 6. 排障

| 现象 | 原因与处置 |
|---|---|
| 所有工具都 401 | token 被轮换过；重新读 `mcp-endpoint.json` 或设置页复制新 token |
| 连不上 `127.0.0.1:<port>` | 应用没开、或 MCP 被关（默认关）；看设置页状态 |
| `GH3036_INIT_FAILED` | 应用状态异常；重试 `action=init`。正常情况下协议栈已在启动时初始化 |
| GH3036 采到数据但没 CSV 文件 | GUI 偏好里 CSV 被关闭或输出目录不可写；MCP 不改偏好 |
| 波形"值不变、图还在刷新" | 设备端停止发送（例如固件侧定时器关掉了采样），不是前端卡死。看 `combridge_about(action=logs)` 与 GH3036 状态 |
| BLE 断开后扫描不到设备 | 直接再 `connect` 同一地址；服务端会从系统已连接列表找回，不需要复位器件 |
| `CHAR_NOT_FOUND` | 没先 `discover_services`，或 UUID 来自别的设备 |
| `ble_configured=false` | BLE 后端没启用，去 GUI 打开 |
| 拿不到任何线索 | `combridge_tools_guide` 查该工具的 `notes`，再 `combridge_about(action=logs)` |

---

## 7. 维护者须知

- 新增工具时，**必须**在 `src-tauri/src/mcp/guide.rs` 的 `TOOL_GUIDES` 补一条，并加进该文件的 `every_registered_tool_has_a_guide` 测试清单。测试会拦住漏写。
- 工具描述的 `description` 只写"是什么"，"前置条件 / 坑 / 示例"放 `guide.rs` —— 那里是结构化数据，能被模型按需查询，也不占 `tools/list` 的常驻上下文。
- 本文件与 `guide.rs` 保持一致；改完跑 `cargo test --lib mcp::guide`。
