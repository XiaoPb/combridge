# elicitation UI 侧验收 —— 新会话交接单

> 读这一份就够，不用再翻日志、聊天记录或代码。写于 2026-11-04 23:30（v0.7.2）。
> 起因：MCP 连接在会话启动时建立，运行中的会话不热加载新 server；而「点确认」这个动作本身
> 就是被测对象，所以必须在新会话里由人点一次。服务端侧的 4 条用例已全部通过，见 §6。

## 0. 一句话目标

在新会话里调用一个 L3 工具（产测 `stop`），确认 **WorkBuddy 会弹出 elicitation 确认框、点确认后工具继续执行**。
这是 `docs/mcp-server-plan.md` §9.0 里唯一还没验证的一行。

## 1. 前置检查（3 条，全部通过才继续）

```bash
# ① ComBridge 在跑吗？（必须，且必须在本会话启动之前就在跑）
tasklist | grep -i combridge            # 期望有 combridge-rust.exe

# ② 端点与服务端状态
python -c "import json,io,os;d=json.load(io.open(os.path.expanduser('~/AppData/Local/combridge/mcp-endpoint.json'),encoding='utf-8'));print(d['url'],d['tier'],d['pid'])"
# 期望 http://127.0.0.1:47821/mcp  standard  <pid>

# ③ 本会话到底有没有 combridge 工具（决定性检查）
#    看工具清单里是否有 combridge_about / combridge_gh3036_factory_test；
#    没有就别往下试了 —— 见 §5.1，只能重开会话，不能在会话内补。
```

若 ① 没在跑：`cd E:/Code/CPP/combridge-rust && npm run tauri dev`，
起来后 endpoint 文件 mtime 会刷新，然后**再开一个新会话**（当前会话拿到的是空清单，救不回来）。

### 1.1 实测修正（2026-11-04 23:35，未完成 UI 用例）

> ⚠️ 本节的「必须完全重启应用」结论已被 **§1.2** 修正 —— 重启确实生效了，但真正的原因是启动顺序。读 §1.2 即可，本节只留作过程记录。

按 §1 逐条实测，三条前置 **全部通过**，但 §1 第③条仍为「无工具」，且原因不是「会话启动早于 ComBridge」：

| 检查 | 实测 |
|---|---|
| ① 进程 | ✅ `combridge-rust.exe` pid 75508 |
| ② 端点 | ✅ `http://127.0.0.1:47821/mcp` / `standard` / 75508 |
| 服务端直连 | ✅ 自己 `initialize` + `tools/list`：v0.7.2、16 个工具、协议 `2025-06-18`；`mcp.json` 里的 token 有效 |
| ③ 本会话工具清单 | ❌ 无任何 `combridge_*`，`combridge_about` 也搜不到 |

**真正的阻塞点在应用级，不在会话级：**

- 会话 `be912d7d`（pid 49092，23:31:38 创建，**晚于** ComBridge 启动）在 `[MCP] reconcile` 里拿到的是
  8 个 server，全部为 `custom-mcp:{ardot,netdrive,agent-mail,genie-baas,space-library,miora,weixinpay,sheetagent}`，
  日志里**没有任何** combridge 的连接尝试或失败记录 —— 不是「连不上被剔除」，是**根本没进期望清单**。
- `~/.workbuddy/mcp-approvals.json` 的信任键与 `sha256("http://127.0.0.1:47821")::combridge` 完全匹配，
  `~/.workbuddy/mcp.json` 的条目结构也与官方 http 示例一致（`type: http` + `url` + `headers.Authorization`），配置本身没问题。
- 决定性证据：桌面端主进程 pid 77856 **启动于 2026-11-03 15:58**，而 `mcp.json` 写入 combridge 是今天 23:16。
  应用在启动时定型了 MCP registry，之后没有重新读取用户级配置文件；新会话只是新建 CLI worker，
  继承的还是那份旧 registry。

**结论**：§5.1 的「开新会话」不够，必须**完全退出 WorkBuddy 再重启**（不是新开对话），
让用户级 `mcp.json` 被重新读取。重启后先复查 §1 第③条，再执行 §3 的单次调用。

复盘线索（重启后可用同样方式确认是否已修复）：项目日志
`~/.workbuddy/logs/<日期>/combridge-rust__*.log` 中 `[MCP] reconcile` 的 server 数应从 8 变 9，
并出现 `server=combridge` 的工具声明行。

### 1.2 根因定位（2026-11-04 23:45，UI 用例仍未执行）

23:35:27 WorkBuddy 冷启动（`AppStartup.log` 里 `uptimeSec=0`），用户级 `mcp.json` 被重新读取 —— 也就是说
§1.1 的「重启整个应用」这一步**已经生效**：`combridge` 这次确实进了注册表，`reconcile` 的 7 个 server 里就有它。

失败记录（新会话 `ceb58b29`，worker pid 80912，23:36:17）：

```
[MCP][cacheDbg] doConnect START: server=combridge type=http
[MCP][cacheDbg] status setter: server=combridge connecting -> disconnected
[MCP][cacheDbg] doConnect FAIL: server=combridge error=Streamable HTTP error: Error POSTing to endpoint: {"error":"internal_error"}
```

而 ComBridge 进程是 23:36:49 才起来的 —— **会话（23:36:16）又比 ComBridge 早 33 秒**。所以这次既不是注册表、
也不是配置或信任问题，纯粹是启动顺序：MCP 连接只在会话 worker 启动那一刻做一次，上游不在就当场置
`disconnected`，本会话内不再重试。

另一个此前没有记录的事实：客户端并不直连 47821，而是走本地连接器代理。

```json
"combridge": { "url": "http://127.0.0.1:4201/<hash>/mcp",
               "_workbuddyMcpOrigin": "custom",
               "_workbuddyUpstreamOrigin": "http://127.0.0.1:47821" }
```

上游没起来时代理返回 `{"error":"internal_error"}`（不是 `ECONNREFUSED`），所以日志里的报错看着像服务端 500，
实际是「上游不可达」。ComBridge 起来后，同一条代理路径 `initialize` 返回 200 并给出 `serverInfo=combridge 0.7.2`，
链路本身没问题 —— 不需要改 `mcp.json`，也不需要动信任记录。

**修正后的结论：**

1. 应用只需重启一次（已完成），之后不必再重启；`combridge` 会稳定出现在 `reconcile` 清单里。
2. 唯一要守的规矩：**先起 ComBridge → 确认端口在听 → 再开新会话**。顺序反了必然 no tools。
3. §1③ 的复查办法不变，另加一条更快的判据：会话日志里出现 `server=combridge status=connected` 才算连上；
   只有 `connecting -> disconnected` 就是顺序反了，直接重开会话，别在会话里折腾。

### 1.3 执行结果（2026-11-04 23:40，§3 已调用完毕）

**结论：无弹窗，`CONFIRM_REQUIRED`「无法向用户确认：客户端未声明 elicitation 能力」——§5 第 4 行（客户端能力缺口）。**

前置三条全部通过：`combridge-rust.exe` pid 17164；端点 `http://127.0.0.1:47821/mcp` / `standard` / 17164；
本会话工具清单含 16 个 `combridge_*`，`combridge_about` 返回 `0.7.2` / `standard`。
客户端日志 `doConnect OK server=combridge elapsed=170ms` → `status=connected`，链路无问题。

调用 `combridge_gh3036_factory_test {"action":"stop"}` 的实测：

| 观察点 | 实测 |
|---|---|
| 弹窗 | ❌ 没有出现（调用 25ms 结束，没有等待 120s 的机会） |
| 返回值 | `{"code":"CONFIRM_REQUIRED","message":"无法向用户确认：客户端未声明 elicitation 能力","retryable":false}` |
| 审计 | `mcp-audit-2026-11-04.jsonl` 末条 `result_code=CONFIRM_REQUIRED` / `duration_ms=0` |
| `elicitation/create` | ❌ 服务端日志里没有这条；即请求根本没发出 |
| 业务逻辑 | ❌ 未执行（不是「产测未在运行」，是连确认环节都没进） |

根因（服务端日志 `combridge_system_2026-11-04-23-36-49.log`，23:40:17.656）：

```
client_info: Implementation { name: "WorkBuddy Connector", version: "5.9.3" }
protocol_version: "2025-11-25"
capabilities: ClientCapabilities { experimental: None, extensions: None, roots: None, sampling: None, elicitation: None }
```

`elicitation: None` —— 客户端在 `initialize` 时没有声明 elicitation 能力（`extensions` 也是 None）。
客户端进程里有 `CODEBUDDY_HOST_CAPABILITIES=elicitation.form`，说明宿主本身具备表单式确认能力，
但 MCP 客户端层没有把它转发进 `initialize.capabilities`。`~/.workbuddy` 配置里也没有可开启的开关。

**归属**：客户端侧缺口，服务端行为正确（fail-closed：问不到就拒绝，符合设计）。
按 §5，此路到此为止，重开会话、重启应用都不会变；需要客户端修正能力声明后才能验证「弹窗 + 点击」。
结论已同步写入 `docs/mcp-server-plan.md` §9.0。

### 1.4 处置：改由本机窗口弹确认（2026-11-04 23:5x 落地）

客户端能力缺口短期不会修，但 ComBridge 自己就是本机 GUI 进程 —— 确认框不必经过 MCP 协议。
服务端已改为**两条通道**（`docs/mcp-server-plan.md` §8.3.1）：

1. 客户端声明了 `elicitation` → 用客户端弹窗（原逻辑，不变）；
2. 没声明 → 服务端 `emit("mcp-confirm-request")`，由 ComBridge 前端弹 `McpConfirmModal`，
   用户点完经 `resolve_mcp_confirm` 回传，等待方是一对 oneshot。

顺带解决了"每次都弹"的体验问题：弹窗里有「本次会话内允许对 `<目标>` 执行 `<操作>`」，
授权按 `(tool, scope)` 记账、30 分钟过期、服务重启清空，OTA 不参与。设置页可查看与一键撤销。

于是本文档 §3 的验收目标不再依赖客户端能力，改成对着 ComBridge 窗口看：

| 原判定行 | 现在的含义 |
|---|---|
| §5 第 1 行（弹窗出现） | 改为看 **ComBridge 窗口**是否弹出确认框 |
| §5 第 2 行（点确认后执行） | 不变，只是点击位置从客户端弹窗换成 ComBridge 弹窗 |
| §5 第 3 行（取消后中止） | 同上 |
| §5 第 4 行（客户端能力缺口） | 已由本机通道兜住，不再是死路 |

`elicit_probe.py` 那套脚本客户端用例依然有效（它声明了能力，走的是通道 1）。

## 2. 环境快照（2026-11-04 23:30，仅作对照，请以 §1 实测为准）

| 项 | 值 |
|---|---|
| 服务端 | `combridge` v0.7.2，`http://127.0.0.1:47821/mcp`，tier = `standard`，pid 75508（23:26:36 启动） |
| 客户端配置 | `~/.workbuddy/mcp.json` 里 `combridge`（type=http，Bearer 直接复用 endpoint 里的 token） |
| 信任预置 | `~/.workbuddy/mcp-approvals.json` → `83894b45c5e57569c4d2ab9cb093b0359eeb64f21d847fb33df7f2d8bdcdb612::combridge`（= `sha256("http://127.0.0.1:47821")::combridge`，免点 Trust） |
| 工具数 | 16 个（`combridge_about`、`combridge_serial_*`、`combridge_ble_*`、`combridge_gh3036_factory_test`、`combridge_ota` …） |

token 本地就有，**不要手抄、不要贴进对话**。

### 2.1 交接时的实际状态（2026-11-04 23:45）

| 项 | 值 |
|---|---|
| ComBridge | ✅ 运行中，pid 17164，`npm run tauri dev` 起于 23:36:49 |
| 端点 | `http://127.0.0.1:47821/mcp`，tier `standard`，endpoint 文件 mtime 已刷新 |
| 服务端自检 | ✅ `initialize` → `combridge 0.7.2` / 协议 `2025-06-18`；`tools/list` → 16 个工具（含 `combridge_gh3036_factory_test`） |
| 代理链路 | ✅ `127.0.0.1:4201/<hash>/mcp` → 上游 47821，`initialize` 200 |
| 本会话工具 | ❌ 无（会话早于 ComBridge 启动，combridge `connecting -> disconnected`） |

也就是说：**除了「开一个晚于 ComBridge 的新会话」之外，其余都已就绪**。

## 3. 唯一要做的调用

```
工具：combridge_gh3036_factory_test
参数：{"action": "stop"}
```

为什么是它：

- `run` / `continue` / `stop` 三个动作在标准档都会逐次触发确认（`server.rs:2171`）。
- `stop` 在产测没运行时**不驱动硬件**，最坏只返回「产测流程未在运行」，是目前唯一零副作用的 L3 探针。
- **不要**用 `action=run`（真的会跑产测、要真机）、**不要**动 `combridge_ota`、**不要**为了省事把档位切到 `dev`（dev 档按设计跳过确认，测了等于没测）。

调用后 120s 内必须完成点击（`elicit.rs` 的 `CONFIRM_TIMEOUT`），超时按未确认处理。

## 4. 弹窗应该长什么样

- 文案（原始 message，两行）：`【GH3036 产测】即将执行产测 action=stop（最长等待 180s）` / `确认执行？`
- 表单：`type=object`，唯一字段 `confirm`（`boolean`，required），title `ConfirmForm`
- 语义：勾选 = 确认；不勾选提交、点拒绝、关弹窗、超时 —— 一律视为拒绝
- 客户端不一定渲染成「勾选框 + 提交」，可能是一个确认/取消按钮组，等价即可

## 5. 结果判定

| 你看到的 | 说明 | 下一步 |
|---|---|---|
| 弹窗出现 → 点确认 → 返回 `FACTORY_TEST_STATE`「产测流程未在运行」 | ✅ **这就是通过**。工具越过了确认、走到了业务逻辑 | 记入 §9.0，收工 |
| 弹窗出现 → 点拒绝/取消 → `CONFIRM_REQUIRED`「用户未确认：用户拒绝 / 用户取消」 | ✅ 负向路径在 UI 上同样成立 | 记入 §9.0 |
| 弹窗出现 → 提交了但 `CONFIRM_REQUIRED`「用户未确认：用户未勾选确认项」 | ⚠️ UI 把 `confirm` 渲染成了默认未勾选，且提交路径没有回填 | 记为 UI 缺陷，附客户端版本 |
| 无弹窗 → `CONFIRM_REQUIRED`「无法向用户确认：客户端未声明 elicitation 能力」 | ❌ **客户端侧能力缺口**，服务端没毛病 | 这是本次要暴露的结论，写清客户端名称/版本 |
| 无弹窗 → `CONFIRM_REQUIRED`「无法向用户确认：…」（非上面两条） | 大概率是 120s 超时兜底（超时未在 `Decision` 里单列，走 `Unsupported(e.to_string())`） | 重试一次，注意别在等的时候切走 |
| 调用直接报「找不到工具 `combridge_gh3036_factory_test`」 | 会话启动时 ComBridge 没在跑，combridge 被静默剔除 | 见 §5.1 |

### 5.1 「工具不见了」的唯一解法

MCP server 清单在会话/worker 启动那一刻定型，运行中不重连。若会话启动时 ComBridge 没跑，
它在 reconcile 阶段就被剔除（日志会看到 `pending=8` 但解析出来只有 7 个 server），**本会话无法补救**。
处理顺序：确认 ComBridge 在跑 → 关掉本会话 → 开新会话 → 复查 §1 第 ③ 条。

## 6. 已经验过的部分（不用重跑，除非你想确认环境没坏）

2026-11-04 23:27，v0.7.2 / tier=standard，用 `scripts/mcp/elicit_probe.py`（声明 `elicitation` 能力的脚本客户端）：

| 客户端回应 | 实际返回 |
|---|---|
| `accept` + `confirm:true` | `FACTORY_TEST_STATE`「产测流程未在运行」——工具继续执行 |
| `decline` | `CONFIRM_REQUIRED`「用户未确认：用户拒绝」 |
| `cancel` | `CONFIRM_REQUIRED`「用户未确认：用户取消」 |
| `accept` + `confirm:false` | `CONFIRM_REQUIRED`「用户未确认：用户未勾选确认项」 |

四条各收到 1 次 `elicitation/create`，全部 `retryable:false`；审计同步落 4 条，`result_code` 与返回值一致。
四条用例合计约 2 秒，重跑不会污染现场（但会与 UI 侧的手工点击混在审计里，按时间戳区分即可）。

## 7. 取证材料（写结论时附上）

- 审计：`E:\Tools\ComBridge\log\mcp-audit-2026-11-04.jsonl`
  最后一条应是 `combridge_gh3036_factory_test` + `FACTORY_TEST_STATE`（或 `CONFIRM_REQUIRED`）。
- 服务端日志：`E:\Tools\ComBridge\log\combridge_system_<启动时间>.log`
- 客户端日志：`~/.workbuddy/logs/<日期>/*.log`，关键字 `elicitation/create`、`elicitationSettled`、`server=combridge`。

## 8. 收尾：把结果补进文档

`docs/mcp-server-plan.md` §9.0 的「仍未验证」段落（约 924 行）需要替换成结论，建议格式：

```
UI 客户端 elicitation（<客户端名> <版本>，2026-11-0X）：
| 弹窗渲染 | ✅/❌ <描述> |
| 点确认 → 工具继续执行 | ✅ FACTORY_TEST_STATE |
| 点拒绝 → CONFIRM_REQUIRED「用户未确认：用户拒绝」 | ✅ |
```

## 9. 别做的事

- 别在**当前**这个会话里试图调用 combridge 工具（它没有），也别为此重启 WorkBuddy 后继续用旧会话。
- 别把档位切 `dev` 来「先跑通」——dev 档不弹窗，测出来的绿灯是假的。
- 别用 `action=run` 做探针。
- 别把 token 贴到对话或文档里。
