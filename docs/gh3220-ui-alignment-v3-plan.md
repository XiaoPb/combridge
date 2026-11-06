# GH3220 交互对齐优化计划（V3：全下拉 / 语义值驱动）

> 状态：**已实施并完成验证**（2026-11-06，P0 / P1 / P2 分阶段提交；`npx tsc --noEmit` 通过、全量 vitest 442/442 通过、Vite 模块编译 200、CDP 真机走查 9 项通过）
> 分支：`feat/gh3220-ui-alignment`
> 前置：`docs/gh3220-ui-alignment-v2-plan.md`（P0/P1/P2 已实施并完成真机验证）、`f4eceaa`（配置页补齐通道配置与命令收发表格）
> 需求来源：用户 2026-11-06 追加 ——「功能也要是下拉选择，自行按值填充。要完全对齐 GH3036 的实现方式，详细审查 GH3036 和 GH3220 的差异；能支持选择的都直接选择，而不是由用户自己去想，启动功能的组合只需要选功能即可，细节的内容代码层实现处理。」

---

## 0. 摘要

GH3036 配置页的做法是：**所有可枚举参数都是下拉，功能组合是多选，用户在 UI 上只选「功能名」，由前端 `bitMap` 归约出 `funcBits` 再下发**（`Gh3036RpcList.tsx:227-254`）。GH3220 当前虽然已有命令表格和语义常量层，但功能位仍是**手输文本**（`hr,spo2` / `0x6`），事件掩码、FIFO 阈值、槽位、调试状态类型等也都是手输数字，参数解析散落在各组件的 `run()` / `handleXxx()` 里。

本计划的目标是把 GH3220 全部命令参数改造成**「元数据驱动 + 语义值」**：

1. UI 上只出现「选择」：下拉、多选、开关、文件/目录选择器；不再要求用户记忆或输入任何枚举值、掩码值、命令号。
2. 「启动功能的组合」= 勾选若干功能名 → 代码层算出 `funcMask` → 代码层补全 WORK_MODE、slotEn、通道数、CSV 等细节 → 一次点击完成 `WORK_MODE + START`（或 `captureStart`）。
3. 参数字典、选项表、编解码、默认值、校验、联动全部下沉到 `src/constants/gh3220CommandSchema.ts` + `src/constants/gh3220Functions.ts`，组件只渲染、只存语义值。
4. 交互骨架完全对齐 GH3036 的「两个 Collapse 区 + 行式命令表格 + 行内 90px 按钮 + 集中 `rpcConfig` 状态」。

**不改 Rust 后端**：GH3220 的 24 个 `gh3220_*` 命令已经是类型化 invoke，参数约束清楚；本轮是前端交互与编码层改造（见 §6.6 的可选 P3）。

---

## 1. 需求拆解与验收判据

### 1.1 四条硬约束

| # | 约束 | 可验收判据 |
|---|---|---|
| C1 | 功能也要是下拉选择 | 配置页、调试页「功能位」控件均为 `Select mode="multiple"`，选项为 15 项语义功能名；页面上不存在 `funcMask` 文本输入框 |
| C2 | 自行按值填充 | ① 选项表由常量层按「值 → 名称」生成，不手写；② 选择后联动填充其他参数（mask 预览、通道数上限、UUID/MTU、版本类型）；③ 连接/刷新真机后由 `deviceInfo.openedFunc`、`preferences`、`status` 反向回填已选项 |
| C3 | 完全对齐 GH3036 实现方式 | 行式命令表格（每条命令一行、参数内联、行内发送）；集中 `commandConfig` 状态；「功能选择 + 启动/停止」单行开关；文件选择 + 预览；扩展原始命令入口；`start/stop` 单按钮按 `isRunning` 切换 |
| C4 | 能选的都选，细节代码层处理 | UI 不出现任何「要求用户自己想出数值」的枚举输入；所有编码（多选→mask、位→数值、名称→后端 key）、默认值、范围校验、下发顺序都在 schema/编码层完成 |

### 1.2 「可枚举」与「自由输入」的判定规则

**必须改选择（可枚举 / 有明确取值域）**

- 功能位组合（15 项，位掩码）
- WORK_MODE（8 项，0..7）
- 事件上报掩码（14 项，位掩码，u16）
- 调试状态类型 `debugStatus.kind`（6 个命名值 + 自定义数值）
- 芯片控制动作（5 项）
- 布尔参数（autoStart / autoStop / csvEnabled / slotOn / on）
- 槽位号 `slot`、槽位使能 `slotEn`、附加参数 `extra`（u8 有限域，按常用值下拉 + 自定义）
- 通道覆盖 `channelsOverride`（由所选功能 `maxChannels` 动态生成 1..N + 自动）
- MTU（自动 / 20 / 23 / 64 / 128 / 244 / 517 + 自定义）
- GATT 写/通知 UUID（默认 3 个 + 设备发现出的特征 + 自定义）
- TCP 调试地址（回环预设 + 自定义）
- CSV 空闲关闭秒数（预设档位 + 自定义）
- 版本类型 `ver_type`（9 项，仅版本页扩展用）
- 波形显示时长 / 采样率（常用档位 + 自定义）
- 寄存器表文件路径、CSV 输出目录（文件/目录选择器，不手打路径）

**保留自由输入（确无枚举表，或本质是字节流）**

| 项 | 理由 | 改进 |
|---|---|---|
| `packageTest.hex` / `sendRaw.hex` / 0x0B 帧头 `headerHex` | 任意字节流，不可能枚举 | 保留 Input，增加 HEX 格式校验与常用帧模板下拉（选模板自动填充输入框） |
| `regRead/regWrite.addr`、`regWrite.values` | 寄存器地址/值是厂商寄存器表，仓库内无枚举定义；SDK 寄存器表文件可通过「导入」加载 | 输入框加前缀/范围校验；提供「从寄存器表选择」的可选增强（P3） |
| 扩展原始命令字符串 | GH3036 也保留（`extendedCommand` 行） | 默认折叠在「高级」区，避免干扰主流程 |

> 判据一句话：**凡是代码里能列出 `value → label` 映射的，就不允许出现 `Input`。**

### 1.3 明确不做的事

- 不改变任何下发到设备的原始数值语义（`funcMask`、`mode`、`slotEn`、CMD 号、缓冲 ID `gh3220:funcN`、CSV 文件名）。
- 不新增/删除 Rust 命令，不改 `libs/gh-uprotocol` 协议布局。
- 不擅自下发 STOP 或断开当前真机连接（验证阶段只读命令优先）。
- 不把 GH3036 的 `executeRpc(commandKey, params: string[])` 字符串 RPC 生搬到 GH3220；GH3220 保留类型化 `gh3220Api.*`（理由见 §4 差异矩阵 D1）。

---

## 2. GH3036 实现审查（对齐基准）

### 2.1 配置页骨架

`src/pages/Protocol/Gh3036Panel.tsx:56-97`：两个 `Collapse` 区 —— ① 通道配置 `Gh3036ChannelConfig`；② RPC 命令 `Gh3036RpcList`。GH3220 `ConfigTab.tsx:16-37` 已经是同款两区（V2/f4eceaa 已对齐），**骨架无需再改**。

### 2.2 命令行表格范式（`Gh3036RpcList.tsx`）

| 位置 | 做法 |
|---|---|
| `319-343` | `rowStyle` = flex 行 + 底部分隔线；`labelStyle` 固定 80px；`controlStyle` flex:1；`buttonStyle` 固定 90px |
| `358-603` | 每个 `<Col span={24}>` 一条命令；10 行（模式/版本/控制命令/写寄存器/读寄存器/配置加载/功能选择/工厂模式/工厂结果/扩展命令） |
| `363-378` | 模式选择行：`Select`（3 项 `workModeOptions`）+ 行内「重启」按钮，点击时 `handleRestart` 把下拉值转成 `['0']` 下发 |
| `382-402` | 版本行：`Select`（`versionTypeOptions`，**来自后端** `gh3036Api.getVersionTypes()`，L21-36）+ 结果文本 + 行内按钮 |
| `405-425` | 控制命令行：`Select`（4 项）+ 行内按钮；`handleSendCommand` 用 `ctrlTypeMap` 把 `idle/reset/sleep/wakeup` 映射成 `0/194/196/195`（L115-123）—— **用户选名称，代码层填数值** |
| `427-490` | 寄存器行：地址/值 Input（自由数值，非枚举）+ 读回值只读展示 |
| `493-513` | 配置文件行：只读路径 Input + 按钮触发 `open()` 文件对话框；`Modal` 表格预览寄存器后确认下载（L145-225） |
| `516-541` | **功能选择行（本需求的核心基准）**：`Select mode="multiple"`（9 项 `functionOptions`）+ 一个按钮，按钮按 `rpcConfig.isRunning` 在「启动/停止」之间切换 |
| `543-579` | 工厂模式行：`Select`（6 项）+ 设置/读取按钮；`handleFactorySetMode/GetMode` 用 `bitMap` 把名称转成 `0x01..0x20`（L269-317） |
| `582-602` | 扩展命令行：自由文本输入框（唯一保留的手输入口） |

### 2.3 参数编码范式（C4 的直接参照）

```ts
// Gh3036RpcList.tsx:51-61 —— 选项只列「语义名」
const functionOptions = [
  { value: 'adt', label: t('gh3036.functions.adt') }, /* ... 9 项 */
];

// L72-82 —— 值表在代码层
const bitMap: Record<string, number> = { adt: 0x01, hr: 0x02, /* ... slot: 0x100 */ };

// L227-240 —— 用户只选功能，数值由前端归约
const funcBits = rpcConfig.selectedFunctions.reduce(
  (acc, func) => acc | (bitMap[func] || 0), 0);
await handleExecuteRpc('S', [funcBits.toString(), '0']);   // '0' = 启动
setRpcConfig({ isRunning: true });
```

要点：**UI 状态存的是 `selectedFunctions: string[]`，不是 `funcBits`**；`bitMap` 是组件内常量；启动/停止只靠第 2 个参数 `0/1` 区分，按钮状态用 `isRunning` 切换。GH3220 的 V3 方案沿用这个范式，但把 `bitMap` 换成语义层全表、把归约/校验收进 schema（比 GH3036 更严格）。

### 2.4 状态与持久化

- `src/stores/gh3036Store.ts:136-150`：集中 `rpcConfig`（workMode / command / 寄存器 4 项 / configPath / **selectedFunctions** / **isRunning** / factoryMode / factoryResult / version / versionType）。
- `:344-358`：默认值集中定义；`:570-572`：`setRpcConfig(partial)` 单一入口。
- `:250-1187`：`persist` 中间件只持久化图表设置（`partialize`），`rpcConfig` 每次会话重置；通道配置走 `preferencesApi.updateGh3036Channel()`（`:650-668`）落盘到后端偏好文件。
- `:753-767`：`executeRpc` 统一 loading / 错误格式化 / 抛错，不吞异常。
- `src/api/gh3036.ts:87-89`：`executeRpc(commandKey, params: string[])` 是唯一命令出口。

### 2.5 通道配置页范式（`Gh3036ChannelConfig.tsx`）

- `192-312`：16:8 两栏，左「连接类型 → 设备 → TX/RX UUID → 保存」，右「CSV 开关 / 目录 / 空闲关闭 / 保存」。
- `197-206`：连接类型 `Select`（serial / ble）。
- `45-51` + `213-237`：设备下拉来自 `useConnectedDevices()`（**已连接设备**，不是扫描结果），按类型过滤。
- `53-116`：`preferCharacteristic()` 自动从 BLE 特征列表挑 `00000004`（写）/ `00000003`（通知）并回填；仅当用户手动编辑过（`txCharManuallyEdited`）才停止自动填充。
- `241-264`：UUID 是 `Input`，但值基本由自动发现填充 —— 属于「可枚举但来自设备」的情形，V3 计划改成 `Select`（默认项 + 发现项 + 自定义）。
- `274-311`：右栏 CSV 用 `Switch` + 目录选择按钮 + `InputNumber`（空闲秒数）。**开关类保留 Switch，不强行改成 Select**（与 GH3036 一致）。

### 2.6 动态选项与后端目录

`src-tauri/src/gh3036/types.rs:252 get_rpc_commands()`、`:503 get_version_types()`，前端 `gh3036Api.getRpcCommands()/getVersionTypes()` 拉取。GH3036 的「命令目录」和「版本类型」是**后端数据驱动**；GH3220 没有对应命令，V3 用前端元数据目录实现同等效果（见 §6.6）。

### 2.7 归纳：GH3036 范式 8 条

| # | 范式 | V3 落地 |
|---|---|---|
| G1 | 每条命令一行，参数内联，行内 90px 按钮 | 保留 `Gh3220CommandConsole` 表格（已实现），参数控件改由 schema 生成 |
| G2 | 可枚举参数一律 `Select` / `mode="multiple"` | §5.2 / §7.2 参数字典 |
| G3 | UI 状态存语义值，数值由代码层编码 | `commandConfig` 存 `selectedFunctions: string[]`，`encodeFunctionMask()` 生成 u32 |
| G4 | 功能选择 + 单按钮启停，按 `isRunning` 切换 | §5.4「一键功能组合启动」 |
| G5 | 集中 `rpcConfig` + `setRpcConfig` 单一入口 | 新增 `gh3220CommandConfig`（§6.3） |
| G6 | 文件/目录用 `open()` 选择器 + 预览确认 | 寄存器表、CSV 目录、HEX 模板（§5.6/§5.7） |
| G7 | 选项来源可动态（后端目录、设备列表、已连接设备） | 设备下拉合并 `useConnectedDevices()` + 扫描结果；通道数按所选功能动态 |
| G8 | 保留一个「扩展原始命令」高级入口 | 高级区默认折叠（§5.3） |

---

## 3. GH3220 现状审查

### 3.1 相关文件清单

| 文件 | 职责 |
|---|---|
| `src/pages/Gh3220/ConfigTab.tsx` | 两个 Collapse 区：通道配置 / UProtocol 命令收发 |
| `src/pages/Gh3220/Gh3220ChannelConfig.tsx` | BLE/TCP 链路、UUID、MTU、连接动作、CSV 与采集偏好 |
| `src/pages/Gh3220/Gh3220CommandConsole.tsx` | 22 条命令目录表格、参数、响应、TX/RX 日志 |
| `src/pages/Gh3220/DebugTab/CaptureSection.tsx` | 采集启停、CSV、统计、最新记录 |
| `src/pages/Gh3220/DebugTab/ControlSection.tsx` | WORK_MODE / START / STOP / 槽位 / 芯片 / FIFO / 事件掩码 / 寄存器 |
| `src/pages/Gh3220/DebugTab/EventSection.tsx` | 0x16 事件表（已有语义列） |
| `src/pages/Gh3220/DebugTab/ChannelSection.tsx` | TCP 调试通道、原始帧、回显、帧头解析 |
| `src/pages/Gh3220/MonitorTab.tsx` / `VersionTab.tsx` | 实时波形 / 版本表 |
| `src/constants/gh3220Functions.ts` | 语义层（V2 P0） |
| `src/types/gh3220.ts` / `src/api/gh3220.ts` / `src/stores/gh3220Store.ts` | 类型 / 24 个 tauri invoke / 全局状态 |
| `src-tauri/src/commands/gh3220.rs` + `src-tauri/src/gh3220/manager.rs` + `libs/gh-uprotocol/src/{lib.rs,cmd.rs}` | 命令签名、参数编码、协议常量 |

### 3.2 命令控制台现状（逐项）

`Gh3220CommandConsole.tsx` 已有：`CommandDef{key, cmd, fields, danger, run}`（`:46-54`）、22 条 `COMMANDS`（`:111-263`）、`renderField()`（`:375-406`）、`commandColumns`（`:408-458`）、`logColumns`、快捷命令、响应面板、收发日志。

**问题清单：**

| # | 问题 | 位置 |
|---|---|---|
| P1 | `CommandField.kind` 只有 `text \| select`；22 条命令里 **13 类参数字段、共 18 处是手输文本框** | `:39-44`、`:111-263` |
| P2 | `maskField()` 返回 `kind:'text', defaultValue:'hr'`：功能位要求用户自己写 `hr,spo2` 或 `0x6` | `:104-108` |
| P3 | 参数解析散落在每条 `run()`：`toNumber` / `parseFunctionMaskText` / `toBool` / `toValueList`（`:67-83`、`:127-181`、`:198-245`），无统一校验/默认值/联动 | 全文件 |
| P4 | 每行参数值是组件本地 `rowValues` state（`:311`），不是集中 store；调试页、采集页各自再维护一份 `funcMaskText`，三处不同步 | `:311` vs `ControlSection.tsx:31` / `CaptureSection.tsx:57` |
| P5 | 日志 `request` 直接 `JSON.stringify(rowValues)`（`:342`），显示的是用户输入的文本，不是**实际下发的数值**（无法核对 mask） | `:337-349` |
| P6 | 提示文案仍教用户手输：`commands.hint = "功能位支持 hr,spo2 或 0x6"` | `src/locales/zh-CN/gh3220.json:89` |
| P7 | CMD 号标注不全/有误（详见 §3.6） | `:111-263` |

当前 22 条命令的参数控件分类：

| 控件 | 字段（出现次数） |
|---|---|
| `Select` | `mode`(2)、`on`、`autoStart`、`autoStop`、`csvEnabled`、`action` = 7 处 |
| `Input` 文本 | `funcMask`(5 处：setWorkMode / start / stop / switchSlotFunc / captureStart)、`slotEn`(2)、`hex`(2)、`addr`(2)，以及 `slot`、`extra`、`count`、`values`、`filePath`、`value`(fifo)、`mask`(event)、`kind`(debug)、`channelsOverride` 各 1 处 = **18 处** |

> 即：**约 2/3 的参数仍要求用户自己想值**，与 C1/C4 冲突。

### 3.3 通道配置页现状

| 控件 | 现状 | 目标 |
|---|---|---|
| 连接类型 | `Select`（BLE / TCP） ✓ | 保留 |
| BLE 设备 | `Select` + 扫描按钮，选项**只来自本次扫描结果**（`:122-150`、`:270-291`）；已连接时禁止扫描 | 合并 `useConnectedDevices()` + 扫描结果 + 当前地址 |
| 服务 UUID | 只读 `Input`（`SERVICE_UUID`） ✓ | 保留为只读文本（非输入） |
| 写/通知 UUID | `Input` 手输（`:293-314`） | `Select`：默认值 + 设备发现特征 + 自定义 |
| MTU | `InputNumber` 23..517（`:315-326`） | `Select`：自动 + 常用档位 + 自定义 |
| TCP 地址 | `Input` 手输（`:333-338`） | `Select`：回环预设 + 自定义 |
| TCP 调试开关 | `Switch` ✓ | 保留（对齐 GH3036 的 Switch） |
| CSV 开关/目录/空闲秒/通道覆盖/autoStop | Switch ×2 + 目录按钮 + `InputNumber` ×2 | 空闲秒与通道覆盖改 `Select`，通道数按所选功能动态 |
| 保存/连接/断开/刷新/恢复默认 | 5 个按钮 ✓ | 保留 |

### 3.4 调试页 / 监控页 / 版本页现状

| 位置 | 手输项 | 已有下拉 |
|---|---|---|
| `ControlSection.tsx` | `funcMaskText` Input（`:171-179`）、`slotEn`/`slot`/`extra` InputNumber（`:180-183`、`:200-209`）、`fifoThr`/`eventMask` InputNumber（`:240-259`）、`regAddr`/`regValues` Input（`:266-296`） | WORK_MODE `Select`（`:160-169`）、chipAction `Select`（label 为英文 `wakeup` 等，`:222-233`） |
| `CaptureSection.tsx` | `funcMaskText` Input（`:171-179`）、`slotEn`/`channelsOverride` InputNumber（`:167-189`） | WORK_MODE `Select`（`:156-165`）、`autoStart` Switch |
| `ChannelSection.tsx` | TCP 地址 Input、`rawHex`/`testHex`/`headerHex` Input | 无（HEX 属自由输入，保留） |
| `MonitorTab.tsx` | 显示时长 / 采样率 `InputNumber`（`:152-176`） | 数据模式、功能选择 `Select`（`:177-194`，已是下拉且带语义名） |
| `VersionTab.tsx` | 无输入；4 行固定版本类型 + 「全部刷新」（`:25-54`） | 无版本类型下拉（GH3036 有） |

### 3.5 语义层与后端现状

- `gh3220Functions.ts` 已有：15 项功能（`GH3220_FUNCTION_LIST`，含 `mask` 与 `maxChannels`）、8 项 WORK_MODE、14 项事件位、`GH3220_DATA_TYPE_BITS` / `GH3220_PACKAGE_TYPE_BITS`、`parseFrameHeader()`、`parseFunctionMaskText()`（文本→mask）、`formatFunctionMaskNames()`（mask→名称列表）。
- **缺口**：没有「多选 keys → mask」的编码器、没有「mask → 多选 keys」的反填、没有参数 schema / 选项工厂 / 统一校验、没有枚举常量（debug kind、chip action、ver_type、slot、MTU 档位）。
- `src/api/gh3220.ts`：24 个方法，均为 `invoke('gh3220_*', { 类型化参数 })`；命令控制台覆盖除 `connect/disconnect` 外的 22 个。
- 后端 `commands/gh3220.rs:44-176` 参数类型：`mode u8`、`func_mask u32`、`slot_en u8`、`slot u8`、`extra u8`、`on bool`、`addr u16`、`count u8`、`values Vec<u16>`、`value u16`(fifo)、`mask u16`(event)、`kind String`、`hex String`、`file_path String`。
- `manager.rs:507-521` 定义了 `debugStatus.kind` 的命名值：`int=0x00 / rst=0x01 / spcs=0x02 / spdo=0x03 / conn=0x04 / status=0x10`，其他按数值字符串解析 —— **当前 UI 默认 `'0'` 且是手输框，完全没用上这张表**。
- `manager.rs:487-497` 定义了芯片动作 5 项；`libs/gh-uprotocol/src/lib.rs:42-52` 定义 9 个版本类型；`lib.rs:18-39` 定义全部 CMD 号。

### 3.6 审查中发现的数据错误（顺带修正）

| 命令 | 当前表格标注 | 协议真实 CMD | 依据 |
|---|---|---|---|
| `switchSlotFunc` | `0x0F` ❌ | **`0x1C`** `SWITCH_SLOT_FUNC` | `libs/gh-uprotocol/src/lib.rs:31` |
| `setFifoThr` | `--` | `0x12` `FIFO_THR_CONFIG` | 同上 `:24` |
| `setReportEvent` | `--` | `0x13` `REPORT_EVENT_SET` | 同上 `:25` |
| `loadRegList` | `--` | `0xA1` `LOAD_REG_LIST` | 同上 `:37` |
| `chipCtrl` | `0x17` ✓ | `0x17` | 同上 `:28` |
| `getDeviceInfo` | `0x19` ✓ | `0x19`（内含 0xA0/0x25） | `manager.rs:537-559` |
| `captureStart/Stop` | `--` | 复合：`0x10 WORK_MODE` + `0x0C START_CTRL` + CSV | `manager.rs` 采集流程 |
| `debugStatus` | `0xA2` ✓ / `packageTest` `0x05` ✓ / `regRead/Write` `0x03` ✓ / `setWorkMode` `0x10` ✓ / `start/stop` `0x0C` ✓ | — | `lib.rs:19-38` |

另外 `src/locales/zh-CN/gh3220.json:89` 的提示文案、`control.funcMaskHint`（`:206`）需改写为「勾选功能即可」。

---

## 4. 差异矩阵（GH3036 vs GH3220）

| # | 维度 | GH3036 | GH3220 现状 | V3 目标 |
|---|---|---|---|---|
| D1 | 命令出口 | 单一 `executeRpc(key, string[] params)`，字符串数组 | 22 个类型化 `gh3220Api.*` | **保留类型化**（不改），用 schema 的 `encode()` 生成参数 |
| D2 | 命令目录来源 | 后端 `get_rpc_commands()` 动态返回 | 前端 `COMMANDS` 常量 | 前端 `GH3220_COMMAND_CATALOG` 元数据（不改后端，理由 §6.6） |
| D3 | 参数定义 | 各 `handleXxx` 内联构造，无 schema | `CommandField{kind, defaultValue, options}` + `run()` 内联 parse | 统一 `ParamDef{kind, options, encode, validate}` |
| D4 | 功能组合 | `Select multiple` 9 项 → `bitMap.reduce` → `funcBits` | `Input` 手输 `hr,spo2` | `Select multiple` 15 项 → `encodeFunctionMask()` → u32 |
| D5 | 启动/停止 | 单行「功能选择 + 单按钮按 isRunning 切换」 | 表格里 2 行独立 START/STOP + 调试页 3 处重复按钮 | 配置页顶部「功能组合启动」行（同 G4），表格保留独立命令行 |
| D6 | 参数值状态 | 集中 `rpcConfig` + `setRpcConfig` | `rowValues` 局部 state；调试页/采集页各自 `funcMaskText` | 集中 `commandConfig`（跨页面共享） |
| D7 | 选项来源 | i18n 常量 + 后端版本类型 + `useConnectedDevices()` | 部分常量，设备选项仅扫描结果 | 常量层选项工厂 + 设备状态/语义层动态生成 |
| D8 | 值回填 | 自动发现 UUID 回填、读回值写回 `rpcConfig` | 从 `preferences` 回填部分表单，无 `openedFunc` 反填 | 连接/刷新后按 `openedFunc`、`status`、`preferences` 反填全部选择 |
| D9 | 文件输入 | `open()` 选择 + Modal 表格预览 | 路径手输（命令表 `filePath`）；调试页用 `open()`（已对齐） | 全部改选择器 + 预览（寄存器表） |
| D10 | 通道配置 | 已连接设备下拉；UUID Input（自动发现填充） | 扫描结果下拉；UUID/MTU/TCP 手输 | 设备/UUID/MTU/TCP/秒数/通道数全部下拉化 |
| D11 | 日志 | 无 TX/RX 日志表 | 有日志表，但 request 显示用户文本 | 日志同时显示「语义值」与「实际下发数值」 |
| D12 | 危险操作 | STOP 无二次确认 | STOP 有 `Modal.confirm` ✓ | 保留并推广到 `captureStop`/`chipCtrl(reset)` |
| D13 | 语义层 | 组件内 `bitMap`（9 项） | 独立 `gh3220Functions.ts`（15 功能/8 模式/14 事件/位解析） | 扩展为「语义层 + 参数 schema 层」双层 |
| D14 | 布局 | 10 行 flex 行式 | 22 行 `Table`（上一轮按用户要求改） | 保持 Table 行式（视觉等价 G1），列结构不变 |
| D15 | i18n | `protocol.json` 的 `gh3036.*` 选项名 | `gh3220.json` 有 `commands.fields/list`，语义名在常量层双语种 | 新增选项/提示键，语义名仍由常量层提供，避免双份维护 |

---

## 5. 目标交互设计

### 5.1 页面结构（不变，只换控件）

```
配置页 Collapse
├─ ① 通道配置  ← 16:8 两栏，全部改「选择 + 自动填充」
└─ ② UProtocol 命令收发
   ├─ 功能组合启动行（新增，对齐 GH3036 functionSelect 行）  ★ C4 核心
   ├─ 快捷命令区（保留）
   ├─ 命令目录表格 22 行（每行：命令名 + CMD Tag + 参数控件 + 90px 发送）
   ├─ 高级：扩展原始命令（折叠，保留自由输入）
   ├─ 最近一次响应（语义化展示）
   └─ 收发日志（语义值 + 实际数值双栏）
```

### 5.2 参数控件分类（schema 的 `kind`）

| kind | 控件 | 典型字段 | 代码层行为 |
|---|---|---|---|
| `functionMask` | `Select mode="multiple"` | `funcMask` | `encodeFunctionMask(keys)` → u32；旁边 `Tag` 实时显示 `0x0002 · 心率 HR` |
| `bitmask` | `Select mode="multiple"` | `eventMask` | `encodeEventMask(keys)` → u16（14 项事件位） |
| `enum` | `Select` | `mode` / `action` / `kind` / `versionType` / `mtu` / `uuid` / `tcpAddress` | value 为后端识别的字符串或数值，label 为语义名 |
| `bool` | `Select`（表格内）或 `Switch`（表单内） | `on` / `autoStart` / `autoStop` / `csvEnabled` / `slotOn` | `'true'/'false'` → boolean（沿用 GH3036 的 Select 风格用于表格） |
| `boundedNumber` | `Select` + 「自定义…」 | `slot` / `slotEn` / `extra` / `channelsOverride` / `fifoThr` / `idleCloseSecs` / `sampleRate` / `displayDurationSeconds` | 常用值下拉；选「自定义」才展开 `InputNumber`，带 min/max 校验 |
| `file` | 只读 Input + `open()` 按钮 | `filePath` / `outputDir` | 选文件→回填路径；寄存器表可加 Modal 预览 |
| `hex` | Input（+ 模板下拉） | `hex` / `rawHex` / `headerHex` | 校验 `parse_hex` 规则；模板选择自动填充 |
| `numberList` | Input（+ 校验） | `values` / `addr` / `count` | 自由数值，仅做格式与范围校验 |

### 5.3 功能组合多选 → mask（核心设计）

**数据流（严格单向）**

```
用户勾选 ["hr", "spo2"]
   ↓  commandConfig.selectedFunctions: string[]   ← UI 只存语义值，对齐 G3
   ↓  encodeFunctionMask(["hr","spo2"])           ← 代码层，全表 mask OR
   ↓  0x0042                                      ← 预览 Tag / 日志「实际下发值」
   ↓  gh3220Api.setWorkMode(mode, 0x42) → gh3220Api.start(0x42, slotEn)
```

**编码器约定（写进 `gh3220Functions.ts`，可单测）**

```ts
/** 多选语义键 → u32 掩码；兼容历史别名与 0x???? / 十进制字面量 */
export function encodeFunctionMask(keys: string[]): number;

/** 设备回读 mask（deviceInfo.openedFunc）→ 多选键；未知位生成 '0x…' 占位项 */
export function decodeFunctionMask(mask: number): string[];

/** '0x0042 · 心率 HR / 血氧 SPO2'，未知位兜底原始值 */
export function formatFunctionMaskPreview(mask: number, lang: Gh3220Lang): string;

export function functionOptions(lang: Gh3220Lang): FieldOption[];  // 15 项，value = key
export function eventMaskOptions(lang: Gh3220Lang): FieldOption[];  // 14 项，value = bit 字符串
export function encodeEventMask(keys: string[]): number;            // → u16
```

- 与 GH3036 的差别：`bitMap` 从组件常量升级为语义层全表（15 项 + 别名），并额外提供 `decodeFunctionMask` 支持**从真机值反填**。
- 未识别的位保留 `0x…` 显示，不丢数据（沿用 V2 的「未知值兜底原始值」约定）。
- `parseFunctionMaskText()` / `parseFuncMask()` **保留兼容**（命令行工具、旧调用、扩展原始命令仍可用），但 UI 不再暴露文本入口。

### 5.4 一键功能组合启动（「细节代码层处理」的落点）

对齐 `Gh3036RpcList.tsx:516-541`：配置页命令区顶部增加一行 ——

```
[功能组合]  <Select multiple 15 项>   [0x0042 · 心率 HR / 血氧 SPO2]   [ 启动 / 停止 ] 90px
[工作模式]  <Select 8 项>            ← 可选，默认「2 · MCU 在线模式」
[高级 ▸]    slotEn / channelsOverride / csvEnabled（全部有代码层默认值，默认折叠）
```

点击「启动」时代码层按固定顺序完成（用户不填任何数值）：

1. 校验 `selectedFunctions.length > 0`，否则提示（对齐 `functionSelectPlaceholder`）。
2. `mask = encodeFunctionMask(selectedFunctions)`。
3. `mode = commandConfig.workMode ?? DEFAULT_WORK_MODE (2)`；`slotEn = 0`；`channelsOverride = null`（自动）；`csvEnabled = preferences.csv.enabled`。
4. 若需要采集：`gh3220Api.captureStart({ autoStart: true, autoStop: false, mode, slotEn: 0, funcMask: mask, channelsOverride: null, csvEnabled })`（后端内部按 `WORK_MODE → START` 顺序下发）；否则 `setWorkMode(mode, mask)` → `start(mask, 0)`。
5. 成功后 `commandConfig.isRunning = true`，按钮切换为「停止」（红色）；停止走 `captureStop()` 或 `stop(mask)`，**STOP 必须 `Modal.confirm` 二次确认**（沿用现有 V2 保护）。注意 `captureStop()` 在后端 `autoStop=true` 时也会连带下发 STOP（`manager.rs:649-656`），因此按钮文案需明确「将停止采样并可能下发 STOP」，不能让用户误以为是纯本地停止。
6. 按钮状态与 `useGh3220Store.status.capturing` / `capture.active` 同步，避免 UI 与真机不一致。

> 真机注意：STOP 后通常需复位才能再次 START，因此默认路径**不下发 STOP**；按钮仅在确实需要时由用户确认。

### 5.5 「自行按值填充」——联动与反填规则表

| 触发 | 自动填充 | 数据来源 / 规则 |
|---|---|---|
| 勾选 / 取消功能 | mask 预览 Tag；`channelsOverride` 选项上限 | `max(selectedFunctions.map(maxChannels))`，无 maxChannels 时默认 32 |
| 选 WORK_MODE | `slotEn` 默认值、`captureStart.autoStart` | 模式→默认值小表（EVK/APP/MCU 在线/离线/透传等） |
| 勾选事件位 | 事件掩码 Tag `0x…` + 中文位名列表 | `GH3220_EVENT_LIST` |
| 选 BLE 设备 | 写/通知 UUID 自动选中设备发现的特征 | 对齐 `Gh3036ChannelConfig.preferCharacteristic`（`00000004`/`00000003` 优先） |
| 选连接类型 | BLE → 填默认 UUID/MTU；TCP → 填 `127.0.0.1:9000` + 打开调试开关 | `SERVICE_UUID`/`WRITE_UUID`/`NOTIFY_UUID`、`preferences.debugTcp` |
| 连接成功 / 刷新信息 | 功能多选项按 `deviceInfo.openedFunc` 反填 | `decodeFunctionMask()`；未知位生成 `0x…` 项 |
| `loadPreferences` | CSV 开关/目录/空闲秒、通道覆盖、autoStop、TCP 开关 | `gh3220Api.getPreferences()` |
| 选版本类型 | 版本表对应行刷新 | `ver_type` 9 项（P2，见 §5.8） |
| 选 HEX 模板 | 自动填入 `hex` / `rawHex` / `headerHex` | 内置模板：`AA 11 19 01 01`、`01 03 00 00 00 0F 80 10`、`0102` |

### 5.6 通道配置页下拉化明细

| 控件 | 选项（value → label） | 备注 |
|---|---|---|
| 连接类型 | `ble` / `tcp`（已有） | 不变 |
| BLE 设备 | `useConnectedDevices()` 的 ble 项 ∪ 扫描结果 ∪ `status.address` | 去重按 address；显示「名称 (地址) RSSI」；扫描仅作「补充选项」，不再要求先扫描才能连已连接设备 |
| 写 UUID TX | 默认 `00000004-…`、设备发现的可写特征、`custom` | 选 `custom` 才展开 Input；保留 `txCharManuallyEdited` 语义（对齐 GH3036） |
| 通知 UUID RX | 默认 `00000003-…`、设备发现的可通知特征、`custom` | 同上 |
| MTU | `auto`(undefined) / 20 / 23 / 64 / 128 / 244 / 517 / `custom` | 真机当前 244 B；`auto` 由后端按 `max_write_len` 决定 |
| TCP 地址 | `127.0.0.1:9000` / `localhost:9000` / `[::1]:9000` / `custom` | 后端仅允许回环（见 `channel.tcpHint`），预设即合法值 |
| TCP 开关 | `Switch`（保留） | 对齐 GH3036 的 `enableCsv` Switch 风格 |
| CSV 空闲关闭 | 0 / 5 / 10 / 30 / 60 / 300 / 3600 / `custom` | 后端上限 3600s |
| 通道覆盖 | `auto`(null) / 1..min(32, maxChannels) | 选项随 §5.5 功能选择动态生成 |
| CSV 目录 | `open({directory:true})` 按钮（已有） | 不变 |
| autoStop / csvEnabled | `Switch`（已有） | 二值开关，不强制改写 |

### 5.7 调试页四分区下拉化明细

| 分区 | 改造 |
|---|---|
| CaptureSection | `funcMaskText` → 共享 `commandConfig.selectedFunctions` 多选；`slotEn`、`channelsOverride` → 下拉；`mode` 已是下拉；`autoStart` Switch 保留 |
| ControlSection | `funcMaskText` → 多选；`slot`/`extra`/`slotEn` → 下拉；`fifoThr` → 下拉（0/1/2/4/8/16/32/64/128/256/自定义）；`eventMask` → **事件位多选 + 掩码预览**（替代 InputNumber）；`chipAction` 下拉改显示中文语义名；`regAddr`/`values` 保留 Input + 校验；寄存器表用 `open()`（已有） |
| EventSection | 不变（已有事件语义列）；新增「按勾选事件过滤」下拉（P3 可选） |
| ChannelSection | TCP 地址改下拉（与配置页共用选项工厂）；HEX 三类保留 Input，附加模板下拉与格式校验 |
| 一致性 | 四处「功能选择」共用同一 `commandConfig.selectedFunctions`，任一处修改立即同步到其余三处与 mask 预览 |

### 5.8 监控页 / 版本页

- `MonitorTab`：显示时长改 `Select`（1/2/5/10/20/30/60 s + 自定义），采样率改 `Select`（5/25/50/100/200 Hz + 自定义）；功能选择 `Select` 已是语义名，保留。
- `VersionTab`：新增「版本类型」下拉（9 项 `ver_type`，来自 `libs/gh-uprotocol/src/lib.rs:42-52`），选中后刷新对应行；保留「全部刷新」与现有 4 行表格。前端当前 `getDeviceInfo()` 一次读 4 个类型，单项刷新需后端支持或复用完整读取（现状 `:77-91` 已注明），因此**版本类型下拉在 P2 只做「选择 + 高亮/定位对应行」，不改后端读取逻辑**，避免制造不一致。

### 5.9 日志与响应显示

`LogEntry.request` 由「用户输入文本」升级为双列：

| 列 | 内容示例 |
|---|---|
| 语义值 | `funcMask=[心率 HR, 血氧 SPO2] · mode=2 · slotEn=0` |
| 实际下发 | `gh3220_set_work_mode { mode:2, funcMask:0x42 }` / `CMD 0x10` |

响应面板保持 `formatResponse()`（V2 已实现数值数组 HEX 展示），并追加一行「语义解读」（如 `openedFunc=0x2 → 心率 HR`）。

---

## 6. 代码层设计

### 6.1 新增 `src/constants/gh3220CommandSchema.ts`（核心）

```ts
export type ParamKind =
  | 'functionMask' | 'bitmask' | 'enum' | 'bool'
  | 'boundedNumber' | 'file' | 'hex' | 'numberList';

export interface FieldOption { value: string; label: string; note?: string }

export interface ParamDef {
  key: string;                       // 同时是 i18n key: commands.fields.<key>
  kind: ParamKind;
  defaultValue: string | string[];  // 语义值，不是数值文本
  /** 静态选项或按语言/上下文动态生成（上下文 = 设备信息/偏好/已选功能） */
  options?: FieldOption[] | ((ctx: OptionContext) => FieldOption[]);
  min?: number; max?: number;       // boundedNumber / numberList 校验
  required?: boolean;               // 多选至少 1 项
  /** 语义值 → 该命令所需的实际参数（数值/布尔/字符串/数组） */
  encode: (raw: string | string[], ctx: OptionContext) => unknown;
  validate?: (raw: string | string[], ctx: OptionContext) => string | null;
}

export interface CommandDef {
  key: string;                       // i18n: commands.list.<key>，也是日志 key
  cmd: string;                       // CMD 号（按 §3.6 修正），'--' 表示复合/非协议命令
  group: 'status' | 'sampling' | 'register' | 'debug' | 'config';
  params: ParamDef[];
  danger?: boolean;                  // 需二次确认
  /** 由 schema 编码后的参数调用 gh3220Api，返回响应 */
  invoke: (encoded: Record<string, unknown>) => Promise<unknown>;
}

export const GH3220_COMMAND_CATALOG: CommandDef[] = /* 22 条，逐条见 §7.1 */;
export const DEFAULT_COMMAND_VALUES: Record<string, Record<string, ...>>;
export function validateCommand(def, values, ctx): string | null;
export function encodeCommand(def, values, ctx): Record<string, unknown>;
export function describeCommand(def, values, ctx): { semantic: string; wire: string };
```

设计要点：

- **`encode` 是唯一数值出口**：`functionMask` 调 `encodeFunctionMask`；`bitmask` 调 `encodeEventMask`；`bool` 转 boolean；`enum`/`boundedNumber` 转 Number；`hex` 去空格校验；`file` 透传路径。组件里不再出现 `toNumber` / `parseFuncMaskText` / `toBool` / `toValueList`。
- **`options` 支持上下文**：`channelsOverride` 的 1..N 依赖已选功能 `maxChannels`；UUID 依赖设备发现列表；版本类型依赖常量表。
- **CMD 号集中**：一处定义，表格 Tag、日志、文档三处共用，杜绝 §3.6 的错误复发。
- `GH3220_COMMAND_CATALOG` 与 `api/gh3220.ts` 一一对应，新增命令时先加目录再加 API（可加一条单测断言覆盖）。

### 6.2 扩展 `src/constants/gh3220Functions.ts`

| 新增 | 说明 |
|---|---|
| `encodeFunctionMask(keys)` / `decodeFunctionMask(mask)` / `formatFunctionMaskPreview(mask, lang)` | §5.3 |
| `functionOptions(lang)` / `eventMaskOptions(lang)` / `encodeEventMask(keys)` / `formatEventMaskPreview` | 多选选项工厂 |
| `GH3220_DEBUG_STATUS_KINDS` | 6 项：`int/rst/spcs/spdo/conn/status` + 对应数值，来自 `manager.rs:507-521` |
| `GH3220_CHIP_ACTIONS` | 5 项，含中文/英文名与后端字符串 key |
| `GH3220_VERSION_TYPES` | 9 项 `ver_type`，来自 `lib.rs:42-52` |
| `GH3220_BOUNDED_PRESETS` | slot(0..7)、slotEn(0/1)、extra(0/1/2)、fifoThr、idleCloseSecs、sampleRate、displayDuration、MTU、TCP 地址 |
| `GH3220_HEX_TEMPLATES` | 3 个常用帧模板 |
| `GH3220_COMMAND_CODES` | 全部 CMD 号常量（从 `libs/gh-uprotocol` 抄录并注明出处） |

保留既有 `parseFunctionMaskText()`、`formatFunctionMaskNames()`、`parseFrameHeader()` 等 V2 能力不动（向后兼容 + 单测已覆盖）。

### 6.3 新增 `src/stores/gh3220CommandConfig.ts`（对齐 `rpcConfig`）

```ts
interface Gh3220CommandConfig {
  selectedFunctions: string[];   // ★ 语义值，跨页面共享（G3/G5）
  workMode: string;              // '0'..'7'
  slotEn: string; slot: string; extra: string; on: string;
  eventMaskKeys: string[];
  debugKind: string; chipAction: string; versionType: string;
  mtu: string; tcpAddress: string; writeUuid: string; notifyUuid: string;
  channelsOverride: string; idleCloseSecs: string; csvEnabled: string;
  autoStart: string; autoStop: string; slotOn: string;
  isRunning: boolean;            // ★ 对齐 GH3036 的启停按钮切换（G4）
  rowValues: Record<string, Record<string, string | string[]>>; // 22 行参数
  lastUpdatedFromDevice: number | null;
  setRowValue(commandKey, fieldKey, value): void;
  setSelectedFunctions(keys): void;
  syncFromDevice(info, prefs, status): void;   // ★ C2「自行按值填充」
  reset(): void;
}
```

- 用 zustand（与 `gh3036Store` 一致的范式）；是否 `persist` 由 §12 待确认项决定，建议**只持久化非危险偏好**（selectedFunctions / workMode / mtu / uuid / csv），`isRunning` 不持久化。
- `syncFromDevice()` 把 `deviceInfo.openedFunc` 反填为功能多选项、把 `preferences` 反填 CSV/通道/TCP、把 `status.address/chunkSize` 反填连接信息；**仅在用户未手动改过时覆盖**（对齐 GH3036 的 `txCharManuallyEdited` 思路）。

### 6.4 组件改造清单

| 文件 | 改动 |
|---|---|
| `Gh3220CommandConsole.tsx` | 删除 `CommandField`/`COMMANDS`/`toNumber` 等本地定义；改为消费 `GH3220_COMMAND_CATALOG` + `commandConfig`；新增「功能组合启动」行与「高级」折叠区；`renderField` 按 `ParamKind` 分派（多选/下拉/文件/HEX 模板）；日志显示 `semantic` + `wire`；CMD Tag 取目录 |
| `Gh3220ChannelConfig.tsx` | 设备/UUID/MTU/TCP/秒数/通道数下拉化；`useConnectedDevices()` 合并选项；选 BLE 设备自动回填 UUID；连接成功触发 `syncFromDevice` |
| `DebugTab/CaptureSection.tsx` | 功能位改共享多选、`slotEn`/`channelsOverride` 下拉；删除 `parseFuncMask` 依赖 |
| `DebugTab/ControlSection.tsx` | 功能位/事件掩码改多选；slot/extra/fifoThr 下拉；chipAction 语义名；删除 `readMask()`/`parseNumber()` 中的手输解析（仅保留寄存器数值解析） |
| `DebugTab/ChannelSection.tsx` | TCP 地址下拉 + HEX 模板下拉与校验 |
| `MonitorTab.tsx` | 显示时长/采样率下拉 |
| `VersionTab.tsx` | 版本类型下拉（P2，仅定位/高亮，不改读取逻辑） |
| `ConfigTab.tsx` | 不变（骨架已对齐） |

### 6.5 i18n 键规划（`src/locales/{zh-CN,en-US}/gh3220.json`）

- 新增 `commands.options.*`：功能名（可复用常量层，不重复）、工作模式、事件位、调试状态类型、芯片动作、版本类型、MTU、TCP 地址、秒数、通道数、bool、模板名。
- 新增 `commands.preview.*`：`maskLabel`（`功能位掩码`）、`selectedCount`、`custom`（`自定义…`）、`auto`（`自动`）、`fromDevice`（`已从设备回填`）、`functionRequired`（`请至少选择一个功能`）。
- 改写 `commands.hint`、`control.funcMaskHint`、`capture.maskRequired`：从「支持 hr,spo2 或 0x6」改为「勾选功能，掩码由代码自动计算」。
- 新增 `commands.advanced.*`、`commands.startCombo.*`（组合启动行标题/启动/停止/确认文案）。
- 语义名（功能/模式/事件/位定义）**继续由 `gh3220Functions.ts` 双语种提供**，i18n 只放 UI 文案，避免 V2 已经建立的单一数据源被破坏。

### 6.6 后端是否改动：结论「不改」，附可选 P3

| 方案 | 说明 | 取舍 |
|---|---|---|
| A（本计划采纳） | 命令目录/选项/编码全在前端常量层 | 不动 Rust、类型安全、可 vitest 单测、与现有 24 个 invoke 完全对应；GH3036 的「后端目录」能力以元数据目录等价实现 |
| B（可选 P3） | 新增 `gh3220_get_command_catalog` / `gh3220_get_version_types` Rust 命令，前端动态拉取 | 与 GH3036 架构完全一致、MCP 可复用；需改 `lib.rs` 命令注册 + 类型导出 + 前端 api，收益主要是跨端一致性 |

建议 V3 走 A；若后续要把 GH3220 命令目录暴露给 MCP/自动化，再单独立项做 B。

---

## 7. 参数字典（逐命令逐参数）

### 7.1 命令目录 22 行（`GH3220_COMMAND_CATALOG`）

> `CMD` 列已按 §3.6 修正；`--` = 非单一 UProtocol 命令（状态/配置/复合采集）。

| # | key | CMD | 参数（改造后控件） | 按钮 / 保护 |
|---|---|---|---|---|
| 1 | `getStatus` | `--` | 无 | 发送 |
| 2 | `getDeviceInfo` | `0x19`(含 `0xA0`/`0x25`) | 无 | 发送；成功后触发 `syncFromDevice` |
| 3 | `getStats` | `--` | 无 | 发送 |
| 4 | `getCaptureStatus` | `--` | 无 | 发送 |
| 5 | `getPreferences` | `--` | 无 | 发送；成功后回填表单 |
| 6 | `clearStats` | `--` | 无 | 发送 |
| 7 | `setWorkMode` | `0x10` | `mode`(enum 8 项)、`funcMask`(functionMask 15 项多选) | 发送 |
| 8 | `start` | `0x0C` | `funcMask`(多选)、`slotEn`(boundedNumber) | 发送 |
| 9 | `stop` | `0x0C` | `funcMask`(多选) | **二次确认**（STOP 副作用） |
| 10 | `switchSlotFunc` | **`0x1C`** | `slot`、`extra`(boundedNumber)、`funcMask`(多选)、`on`(bool) | 发送 |
| 11 | `captureStart` | `0x10`+`0x0C` | `autoStart`/`autoStop`/`csvEnabled`(bool)、`mode`(enum)、`slotEn`/`channelsOverride`(boundedNumber)、`funcMask`(多选) | 发送；高级参数默认折叠且有默认值 |
| 12 | `captureStop` | `--` | 无 | **二次确认**：`manager.rs:649-656` 显示当 `autoStop=true && funcMask≠0` 时后端会真正下发 STOP，等价于危险命令 |
| 13 | `regRead` | `0x03` | `addr`(numberList 自由数值)、`count`(boundedNumber 1..64) | 发送；结果写回只读区 |
| 14 | `regWrite` | `0x03` | `addr`、`values`(numberList 自由数值，u16 列表) | 发送 + 非空校验 |
| 15 | `loadRegList` | `0xA1` | `filePath`(file 选择器 + 预览) | 发送 |
| 16 | `chipCtrl` | `0x17` | `action`(enum 5 项，中文语义名) | 发送；reset 类动作二次确认 |
| 17 | `setFifoThr` | **`0x12`** | `value`(boundedNumber 0..65535 + 预设档) | 发送 |
| 18 | `setReportEvent` | **`0x13`** | `mask`(bitmask 14 项事件位多选 + 掩码预览) | 发送 |
| 19 | `debugStatus` | `0xA2` | `kind`(enum 6 命名值 + 自定义数值) | 发送 |
| 20 | `packageTest` | `0x05` | `hex`(hex + 模板) | 发送 |
| 21 | `sendRaw` | 依帧内 CMD | `hex`(hex + 模板) | 发送（仅 DEV 构建可见，沿用现有约束） |
| 22 | `updatePreferences` | `--` | 无（取当前 `commandConfig` + `preferences`） | 发送 |

### 7.2 参数字典（控件 → 编码 → 校验）

| key | kind | 选项来源 | 默认值 | encode | 校验 |
|---|---|---|---|---|---|
| `funcMask` | functionMask | `functionOptions(lang)`，15 项，`GH3220_FUNCTION_LIST` | `['hr']`（或由 `openedFunc` 反填） | `encodeFunctionMask(keys)` → u32 | 至少 1 项；未知别名报错；mask ≠ 0 |
| `mode` | enum | `GH3220_WORK_MODE_LIST`（8 项） | `'2'`（MCU 在线） | `Number` | 0..7 |
| `slotEn` | boundedNumber | `0/1/2/4/8` + 自定义 | `'0'` | `Number` | 0..255 (u8) |
| `slot` | boundedNumber | `0..7` + 自定义 | `'0'` | `Number` | 0..255 (u8) |
| `extra` | boundedNumber | `0/1/2` + 自定义 | `'0'` | `Number` | 0..255 (u8) |
| `on` / `autoStart` / `autoStop` / `csvEnabled` / `slotOn` | bool | 是/否 | 见各行 | `=== 'true'` | 必选 |
| `channelsOverride` | boundedNumber | `auto` + `1..min(32, maxChannels(已选功能))` | `'auto'` | `null` 或 `Number` | 1..32 |
| `mask`(event) | bitmask | `eventMaskOptions(lang)`，14 项 | `[]` | `encodeEventMask(keys)` → u16 | 0..65535；未知位兜底 |
| `value`(fifo) | boundedNumber | 0/1/2/4/8/16/32/64/128/256/512/自定义 | `'0'` | `Number` | 0..65535 (u16) |
| `kind`(debug) | enum | `int/rst/spcs/spdo/conn/status` + 自定义数值 | `'status'` | 字符串直传（后端解析） | 命名值或 0..255 数值 |
| `action`(chip) | enum | `wakeup/sleep/soft-reset/hard-reset/rx-reset` | `'wakeup'` | 字符串直传 | 后端枚举 |
| `addr` | numberList | 自由数值（HEX/十进制） | `'0x0000'` | `Number` | 0..65535 (u16) |
| `count` | boundedNumber | 1/2/4/8/16/32/64 | `'1'` | `Number` | 1..64 |
| `values` | numberList | 自由数值列表 | `'0x0000'` | `number[]` | 每项 0..65535，非空 |
| `filePath` | file | `open()` | `''` | 路径字符串 | 非空 + 扩展名 csv/txt |
| `hex` | hex | 模板下拉 + 自由输入 | `'0102'` | 去空格字符串 | 后端 `parse_hex` 等价规则（偶数位十六进制） |
| `versionType` | enum | `GH3220_VERSION_TYPES` 9 项 | `'0x01'` FW | 字符串/数值 | 枚举 |
| `mtu` | enum | auto/20/23/64/128/244/517/自定义 | `'auto'` | `undefined` 或 `Number` | 23..517 |
| `writeUuid` / `notifyUuid` | enum | 默认 UUID + 设备发现特征 + 自定义 | 默认 UUID | 字符串 | UUID 格式 |
| `tcpAddress` | enum | 3 个回环预设 + 自定义 | `127.0.0.1:9000` | 字符串 | 回环校验 |
| `idleCloseSecs` | boundedNumber | 0/5/10/30/60/300/3600 | `'10'` | `Number` | 0..3600 |
| `sampleRate` / `displayDurationSeconds` | boundedNumber | 常用档位 + 自定义 | 25 Hz / 10 s | `Number` | 1..1000 / 1..60 |

### 7.3 枚举选项全量表（代码层单一数据源）

| 表 | 项数 | 出处 | 备注 |
|---|---|---|---|
| 功能位 | 15 | `gh3220Functions.ts:47-63` / SDK `gh_drv.h L364-410` | id 0,1,2,3,4,5,6,7,8,9,10,11,12,15,19；含 `maxChannels` |
| WORK_MODE | 8 | `:104-113` | 0 EVK / 1 APP / 2 MCU 在线 / 3 MCU 离线 / 4 测试调优 / 5 透传 / 6 事件 ACK / 7 MPT |
| 事件位 | 14 | `:131-154` | `0x0001..0x4000`，`0x0008` 标注固件私有扩展 |
| 帧头位 | 7 + 6 | `:172-190` | data_type / package_type（仅解析展示，V2 已实现） |
| 调试状态类型 | 6 | `manager.rs:507-521` | int/rst/spcs/spdo/conn/status |
| 芯片动作 | 5 | `manager.rs:487-497` | wakeup/sleep/soft-reset/hard-reset/rx-reset |
| 版本类型 | 9 | `lib.rs:42-52` | FW/DEMO/VIRTUAL_REG/BOOTLOADER/PROTOCOL/FUNC_SUPPORT/DRV/CHIP/ALGO |
| CMD 号 | 20 | `lib.rs:18-38` | 含 §3.6 的修正 |
| 布尔 / 槽位 / MTU / TCP / 秒数 / 通道 | — | 本计划 §7.2 | 前端预设，注明为 UI 档位而非协议限制 |

---

## 8. 分阶段实施计划

### P0｜语义层与参数 schema（无 UI 行为变化）

- **文件**：新增 `src/constants/gh3220CommandSchema.ts`、扩展 `src/constants/gh3220Functions.ts`、新增 `src/constants/gh3220CommandSchema.test.ts`。
- **内容**：§6.1/§6.2 全部类型、22 条目录、编码器、选项工厂、校验、CMD 常量；修正 §3.6 的 CMD 号。
- **验收**：`encodeFunctionMask(['hr','spo2']) === 0x42`；`decodeFunctionMask(0x2)` → `['hr']`；未知位兜底；别名 `bp/ir/lead_det` 兼容；事件位 OR 正确；22 条目录与 `api/gh3220.ts` 方法一一对应；`tsc` 通过；单测 ≥ 20 用例全绿。
- **提交**：`feat(gh3220): 建立命令参数 schema 与语义值编解码层`

### P1｜命令控制台全下拉化（配置页）

- **文件**：新增 `src/stores/gh3220CommandConfig.ts`、重写 `Gh3220CommandConsole.tsx`、修改 `src/locales/{zh-CN,en-US}/gh3220.json`。
- **内容**：§5.2 控件分派、§5.3 功能多选 + mask 预览、§5.4 功能组合启动行、§5.5 联动/反填、§5.9 日志双列、高级原始命令折叠区；删除所有本地 `toNumber`/`parseFunctionMaskText` 调用。
- **验收**：22 行表格仍是一行一命令 + 行内 90px 按钮；表格内 **0 个手输枚举框**（仅 `hex`/`addr`/`values`/模板自定义允许输入）；勾选 2 个功能后预览显示正确 mask；STOP 二次确认；日志同时显示语义值与实际数值；`tsc` + 全量 `vitest` 通过。
- **提交**：`refactor(gh3220): 命令参数改为下拉/多选并集中编码`

### P2｜通道配置、调试页、监控/版本页下拉化

- **文件**：`Gh3220ChannelConfig.tsx`、`DebugTab/{CaptureSection,ControlSection,ChannelSection}.tsx`、`MonitorTab.tsx`、`VersionTab.tsx`、i18n。
- **内容**：§5.6/§5.7/§5.8；四处功能选择共享 `commandConfig.selectedFunctions`；`syncFromDevice` 在连接/刷新后回填。
- **验收**：配置页与调试页对同一功能选择的修改互相同步；设备 UUID 选设备后自动填充；`channelsOverride` 选项随功能变化；真机 `openedFunc=0x2` 时功能多选自动勾中「心率 HR」；不改缓冲 ID / CSV 命名。
- **提交**：`refactor(gh3220): 通道与调试页参数全面下拉化并共享选择状态`

### P3｜验证、文档与可选后端目录

- **文件**：`docs/gh3220-ui-alignment-v3-plan.md`（状态改为「已实施并完成验证」+ 结论）、工作区记忆日志；可选 §6.6 方案 B。
- **内容**：§9 的单测、类型检查、Vite 冒烟、CDP 真机走查、截图留证。
- **提交**：`docs(gh3220): 记录 V3 交互对齐验证结论`

> 每阶段独立可编译、可验证；不把 UI 改造与语义层改造混在一个 commit。无关的 `src-tauri/Cargo.toml`、`.release/`、`.worktree/`、`release-notes.md` 等改动始终不纳入提交。

---

## 9. 测试与验证

### 9.1 单元测试（新增 `gh3220CommandSchema.test.ts`）

| 组 | 用例 |
|---|---|
| 功能位 | 单选/多选 OR、15 项全选、别名兼容（`bp`/`ir`/`lead_det`/`sadt_g`）、空数组报错、未知文本报错、未知 mask 位兜底、`decode(encode(x)) === x` |
| 事件位 | 14 项 OR、组合、空掩码、u16 溢出拒绝 |
| 选项工厂 | 功能 15 项 / 模式 8 项 / 事件 14 项 / 调试 6 项 / 芯片 5 项 / 版本 9 项中英双语 label 非空；`channelsOverride` 随所选 `maxChannels` 变化 |
| 编码 | bool→boolean、enum→Number、boundedNumber→Number/`null`、hex 校验、file 非空、numberList u16 列表 |
| 目录一致性 | 22 条 key 唯一、CMD 号与 `GH3220_COMMAND_CODES` 一致、`invoke` 均可调用、`defaultValue` 覆盖全部 params、危险命令仅 `stop`/reset 类 |
| 联动 | `syncFromDevice({openedFunc:0x2})` → `['hr']`；用户手动改过后不被覆盖 |
| 回归 | 既有 `gh3220Functions.test.ts`（8 用例）与 `menuVisibilityStore.test.ts`（4 用例）继续通过 |

### 9.2 工程检查

- `npx tsc --noEmit`；`npx vitest run`（当前基线 415/415）；Vite dev server 请求首页与新增模块（注意环境代理会让 `curl` 误报 502，需 `--noproxy '*'`）；i18n 两个 JSON `JSON.parse` 校验。

### 9.3 CDP 真机走查（GH3220_Nordic 在场）

沿用 V2 的临时 CDP 驱动（按 `title=ComBridge` 精确选页；`scripts/cdp.py` 的 `find_page()` 会误选 DevTools 页，本轮可顺手修复，见 §12）：

| # | 检查项 | 期望 |
|---|---|---|
| 1 | 配置页命令目录 | 22 行、行内按钮数 22、无 `funcMask` 文本输入框 |
| 2 | 功能多选 | 勾选「心率 HR + 血氧 SPO2」→ 预览 `0x0042` 且语义名正确 |
| 3 | 组合启动行 | 默认模式「2 · MCU 在线」；一键逻辑准备就绪（**验证阶段不真正 START/STOP，避免打断真机采集**） |
| 4 | 只读命令实测 | `getDeviceInfo/getStatus/getStats/getCaptureStatus/getPreferences/packageTest` 均返回真机数据，日志双列正确 |
| 5 | 值回填 | 刷新信息后功能多选自动勾中 `openedFunc` 对应项；CSV/TCP/UUID 从偏好回填 |
| 6 | 通道配置 | 设备下拉含已连接真机；UUID/MTU/TCP 均为选择控件；扫描作为补充 |
| 7 | 调试页四区 | 功能选择与配置页同步；事件掩码多选显示位名；HEX 模板可填充 |
| 8 | 监控/版本/波形 | 语义名、缓冲 ID `gh3220:funcN` 不变；显示时长/采样率下拉；版本类型下拉 |
| 9 | 截图留证 | 配置页表格、功能多选展开、调试页四区、日志双列 |

**红线**：全程不下发 STOP、不断开当前真机连接、不切换 TCP（单会话会打断正在进行的采集）；新增的写类命令仅在用户明确要求时逐条验证。特别注意 `captureStop` 不是纯本地停止 —— `manager.rs:649-656` 在 `autoStop=true` 时会连带下发 STOP，因此真机走查时**不得点击「停止采集」/ `captureStop`**，组合启动行的「停止」按钮也只在用户确认后使用。

---

## 10. 风险、兼容与约束

| 风险 | 缓解 |
|---|---|
| STOP/START 顺序误操作打断真机采集 | 组合启动默认只准备参数；STOP 二次确认；验证阶段只读；`isRunning` 与后端 `status.capturing`/`capture.active` 同步 |
| schema 与后端签名漂移 | 命令目录与 `api/gh3220.ts` 一一对应单测；CMD 常量集中且注明 `libs/gh-uprotocol` 出处；后端不改 |
| 多选 → mask 与文本解析不一致 | `encodeFunctionMask` 与 `parseFunctionMaskText` 共用同一 `GH3220_FUNCTION_BIT_ALIASES`；单测双向一致性 |
| 未知功能位 / 未知事件位丢数据 | 未知位生成 `0x…` 占位项并在预览显示，沿用 V2「未知值兜底原始值」 |
| 持久化旧值不兼容 | `commandConfig` 加 `version` + `merge` 归一化：旧的 `funcMaskText: 'hr'` 转成 `selectedFunctions: ['hr']`；旧通道快照 key `gh3220:channelConfig` 保留读取 |
| i18n 双语遗漏 | 选项名集中在常量层（已有 nameZh/nameEn），i18n 只加 UI 文案；两语种 JSON 结构一致性校验 |
| 「完全对齐 GH3036」被误解为照搬字符串 RPC | §4 D1、§6.6 明确：只对齐**交互范式与状态组织**，保留 GH3220 类型化 invoke 与更强的前端校验 |
| 表格行过宽 | 多选控件 `maxTagCount={2}` + `scroll.x`；高级参数折叠；沿用上一轮已验证的表格布局 |

---

## 11. 文件清单

| 类型 | 文件 |
|---|---|
| 新增 | `src/constants/gh3220CommandSchema.ts`、`src/constants/gh3220CommandSchema.test.ts`、`src/stores/gh3220CommandConfig.ts`、`docs/gh3220-ui-alignment-v3-plan.md`（本文档） |
| 修改 | `src/constants/gh3220Functions.ts`、`src/pages/Gh3220/Gh3220CommandConsole.tsx`、`src/pages/Gh3220/Gh3220ChannelConfig.tsx`、`src/pages/Gh3220/DebugTab/{CaptureSection,ControlSection,ChannelSection}.tsx`、`src/pages/Gh3220/MonitorTab.tsx`、`src/pages/Gh3220/VersionTab.tsx`、`src/locales/{zh-CN,en-US}/gh3220.json` |
| 不改 | `src/pages/Gh3220/ConfigTab.tsx`（骨架已对齐）、`src/api/gh3220.ts`、`src/types/gh3220.ts`、`src/stores/gh3220Store.ts`、`src-tauri/**`、`libs/gh-uprotocol/**` |
| 可选 | `scripts/cdp.py`（修复 `find_page()` 误选 DevTools 页）、§6.6 方案 B 的 Rust catalog 命令 |

---

## 12. 决策记录（按文档建议默认值执行）

用户以「执行，并验证」授权按建议默认推进，5 项决策如下：

| # | 决策 | 落地 |
|---|---|---|
| 1 | 一键「启动」走 `captureStart({autoStart:true, autoStop:false, ...})`（含 CSV/波形），用户语义即「启动功能」；WORK_MODE / slotEn / 通道覆盖 / CSV 开关由代码层补完 | `Gh3220CommandConsole.handleComboStart()`；**验证阶段未真正点击启动/停止** |
| 2 | 选择类语义值跨会话持久化（localStorage `gh3220:commandConfig`），`isRunning` 与 `rowValues` 不持久化；旧 `funcMaskText` 归一化兼容 | `src/stores/gh3220CommandConfig.ts` + 单测 |
| 3 | 版本类型下拉只做「选择 + 高亮定位对应行」，不新增后端单项 GET_VER | `VersionTab` 的 `onRow` 背景高亮 + `locateHint` 说明 |
| 4 | 修复 `scripts/cdp.py` 的 `find_page()`（优先 `title=ComBridge` 且 `http` 页面，回退排除 `devtools://`） | 已纳入 P2 提交，并用 `cdp.find_page()` 实测返回 `ComBridge http://localhost:1420/gh3220` |
| 5 | §6.6 方案 B（Rust 命令目录）不提前，V3 走方案 A 前端元数据目录 | `GH3220_COMMAND_CATALOG` 与 22 个 `gh3220Api.*` 一一对应单测覆盖 |

---

## 附录 A：术语与参考路径

| 术语 | 含义 |
|---|---|
| 语义值 | UI 状态里保存的 `key`（`'hr'`、`'int'`、`'wakeup'`），与后端数值解耦 |
| 实际下发值 | `funcMask u32` / `mode u8` / `mask u16` 等 tauri invoke 参数 |
| 功能位掩码 | `1 << function_id`，`function_id` 也是 0x0B 帧头首字节与 CSV/缓冲 ID 的编号 |
| WORK_MODE | `0x10` 命令的 payload[0]；START 的前置条件 |
| 事件上报掩码 | `0x13` 的 u16 掩码，位定义见 `GH3220_EVENT_LIST` |
| GH3036 范式 | 本文档 §2.7 的 8 条 |

**关键参考**：`src/pages/Protocol/Gh3036RpcList.tsx`、`src/pages/Protocol/Gh3036ChannelConfig.tsx`、`src/stores/gh3036Store.ts`、`src/pages/Protocol/Gh3036Panel.tsx`、`src-tauri/src/gh3036/types.rs`、`src/constants/gh3220Functions.ts`、`src-tauri/src/gh3220/manager.rs`、`libs/gh-uprotocol/src/{lib.rs,cmd.rs}`、`docs/gh3220-ui-alignment-v2-plan.md`。

---

## 13. 实施与验证结论（2026-11-06）

### 13.1 分阶段提交

| 阶段 | 提交主题 | 主要文件 |
|---|---|---|
| P0 | `feat(gh3220): 建立命令参数 schema 与语义值编解码层` | 扩展 `gh3220Functions.ts`；新增 `gh3220CommandSchema.ts`（22 条目录、8 类 ParamKind、encode/validate/describe、CMD 常量）与 22 个单测用例 |
| P1 | `refactor(gh3220): 命令参数改为下拉/多选并集中编码` | 新增 `gh3220CommandConfig.ts`（集中语义状态 + `syncFromDevice` + 旧值归一化）与 5 个单测；重写 `Gh3220CommandConsole.tsx`（功能组合启动行、全下拉参数、日志双列）；中英 i18n |
| P2 | `refactor(gh3220): 通道与调试页参数全面下拉化并共享选择状态` | `Gh3220ChannelConfig`、`DebugTab/{Capture,Control,Channel}Section`、`MonitorTab`、`VersionTab`、`scripts/cdp.py`；目录表发送按钮 fixed right + 布局收紧 |
| P3 | `docs(gh3220): 记录 V3 交互对齐验证结论` | 本文档状态、§12 决策、§13 结论；工作区记忆日志 |

> 无关的 `src-tauri/Cargo.toml`、`.release/`、`.worktree/`、`release-notes.md` 等改动始终未纳入提交。

### 13.2 实现要点与计划差异

- **完全按 §6.1 落地**：`GH3220_COMMAND_CATALOG` 22 条与 `gh3220Api`（除 `connect/disconnect`）一一对应；`encodeCommand()` 是唯一数值出口，组件不再出现 `toNumber` / `parseFuncMask` / `toBool`。
- **多选统一编码约定**：选项 `custom` 在 UI 状态里存成 `custom:<value>`，`encodeBoundedNumber()` / `defaultEncoder('enum')` 统一解析，避免「选了自定义却下发空值」。
- **CMD 号修正**：`switchSlotFunc` 改为 `0x1C`，`setFifoThr=0x12`、`setReportEvent=0x13`、`loadRegList=0xA1` 已标注，`captureStart=0x10+0x0C` 复合标注。
- **危险操作保护扩展**：`stop`、`captureStop` 固定二次确认；`chipCtrl` 的 reset 类动作按编码后的值动态确认；调试页「停止采样」「停止采集」也都加确认（`captureStop` 在 `autoStop=true` 时会连带下发 STOP）。
- **组合启动行**：`running` 与真机 `status.capturing` / `capture.active` 同步，真机采集中显示「停止」；验证阶段未点击。
- **表格可用性微调（计划外小改）**：命令目录「操作」列 `fixed: 'right'`，最外层改 flex 列布局并 `minWidth:0; overflow:hidden`，使 22 个行内发送按钮在默认视口内全部可见；日志「结果」列同样固定右侧。

### 13.3 工程检查

| 项 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过（P0/P1/P2 及最终均通过） |
| `npx vitest run` | **442/442 通过**（42 个测试文件；含新增 22 个 schema 用例、5 个 store 用例；V2 的 8 + 4 用例保持通过） |
| Vite 模块编译 | 首页 + 新增/改动的 10 个模块均返回 200，无 `Transform failed` / `Failed to resolve`（`curl` 需 `--noproxy '*'`，环境代理会误报 502） |
| i18n | `zh-CN` / `en-US` 两个 `gh3220.json` `JSON.parse` 通过，新增 `preview/combo/advanced/confirmTitles/confirmContents/semanticColumn/wireColumn` 等键双语齐备 |
| 工具链 | `scripts/cdp.py find_page()` 修复后实测返回 `ComBridge http://localhost:1420/gh3220`（此前会选中 DevTools 页） |

### 13.4 CDP 真机走查（GH3220_Nordic 在场，全程未下发 STOP / START、未断连、未切 TCP）

| # | 检查项 | 实测结果 |
|---|---|---|
| 1 | 命令目录 | 22 行、22 个行内发送按钮 + 1 个文件选择按钮；表格内 6 个多选（5 × funcMask + 1 × 事件掩码）；仅 6 个自由输入框（HEX×2 / 寄存器地址×2 / 写入值 / 只读路径），**无 funcMask 文本输入框** |
| 2 | 功能多选 | 追加勾选「血氧 SPO2」后预览 `0x0042 · 心率 HR / 血氧 SPO2`，目录里 5 条 funcMask 行同步显示，调试页也同步（共享状态生效） |
| 3 | 组合启动行 | 默认模式「2 · MCU 在线模式」；真机采集中按钮为「停止」并有二次确认；验证阶段未点击 |
| 4 | 只读命令实测 | `getDeviceInfo / getStatus / getStats / getCaptureStatus / packageTest` 全部返回真机数据：版本串 `GH(M)3X2X_DEMO_v1.6_AC...`、`state=capturing` / 244 B、`frames=239689`、`records=1192110`、CSV 双文件、CRC/坏包/丢帧/重同步全 0；回显 `[2,1] HEX: 02 01` |
| 5 | 日志双列 | 表头「语义值 / 实际下发」，实测行 `hex=0102` 与 `packageTest(hex:"0102") · CMD 0x05` 同时显示 |
| 6 | 值回填 | 点「刷新信息」后功能多选自动勾中 `openedFunc=0x2` →「心率 HR」，页面提示「已根据真机回读的功能位自动回填」；手动改过后不被覆盖（单测覆盖） |
| 7 | 通道配置 | 设备下拉含 `GH3220_Nordic (BluetoothLE#…) · 当前链路`；写/通知 UUID 为下拉（默认 + 自定义），MTU 下拉展开显示「自动 / 20 / 23 / 64 / 128 / 244 / 517 / 自定义…」，当前值由真机 244 B 回填；CSV 空闲秒与通道覆盖均为下拉 |
| 8 | 调试页四区 | 四区展开正常；共享功能选择含「血氧 SPO2」且 `0x0042` 标签出现；事件掩码多选 + `0x0000 · 无事件` 预览；芯片动作显示中文「唤醒芯片」；3 个 HEX 模板下拉可用；帧头解析 `01 03 00 00 00 0F 80 10` → 心率 HR / 4 CH / GS+算法结果 / 新数据 |
| 9 | 监控 / 版本 / 波形 | 监控页显示时长「10 s」、采样率「25 Hz」下拉，「当前开启功能 心率 HR」；版本页 9 项类型下拉 + 全部刷新读到 4 个真机版本串、maxLen 238；波形缓冲 `gh3220:func0 · 佩戴检测 ADT` / `gh3220:func1 · 心率 HR`，**缓冲 ID 未变** |

**截图留证**：`%TEMP%/gh3220_v3_catalog_fixed.png`（22 行表格 + 固定发送按钮）、`gh3220_v3_combo.png`（功能多选展开 + 掩码 0x0042）、`gh3220_v3_log.png`（日志双列）、`gh3220_v3_debug.png`（调试页四区）。

### 13.5 未执行 / 保留项（如实）

- 验证阶段**未真正下发 START / STOP / captureStart / captureStop**：真机正在持续采集，停止会打断链路且 STOP 后通常需复位才能再次 START；组合启动逻辑由代码层 + 单测覆盖，未做真机写命令验证。
- `scripts/gh3220_sim.py` 模拟器链路未单独跑：GH3220 是单会话，切 TCP 会打断真机采集。
- 真机本次未产生 `0x16` 事件（事件表 0 条），事件位语义名由语义层单测覆盖。
- §6.6 方案 B（Rust 命令目录 / 版本类型目录）未实施，按 §12 决策 5 延后。
- 版本类型下拉只做定位/高亮，未改后端读取逻辑（前端仍用一次完整 `GET_VER`）。

### 13.6 走查后的现场状态

CDP 走查曾把功能多选改为 `['hr','spo2']` 用于验证 `0x0042`；验证完成后已把 localStorage 的 `selectedFunctions` 恢复为 `['hr']` 并清除手动编辑标记，reload 后页面显示「心率 HR / 0x0002 · 心率 HR」，与真机 `openedFunc=0x2` 一致。真机采集全程未中断。
