# GH3220 页面对齐优化计划（V2）

> 状态：已实施并完成真机验证（2026-11-06，P0 / P1 / P2 三个提交 + 配置页补齐提交；验证结论见 §7，模拟器脚本未单独运行）
> 日期：2026-11-06
> 前置文档：`docs/gh3220-frontend-alignment-plan.md`（V1 对齐方案，已实施）、`docs/chip-ui-framework-design.md`（多芯片框架设计）
> 协议参考：`E:\Code\Goodix\gh3038q_v3.3.0\modules\lierda\gh3220_sdk\demo_kernel_code`

---

## 1. 背景与目标

GH3220 前端已完成 V1 对齐（GH3036 骨架 + 7 个页签），并通过模拟器与真机（GH3220_Nordic）全链路走查。本轮 V2 优化目标：

1. **界面与 GH3036 保持一致**：页面处理方式、页签命名、内容结构对齐；GH3036 主界面只有 5 个页签，GH3220 不应暴露 7 个。
2. **调试功能保留但收纳**：采集 / 控制 / 事件 / 调试通道等调试性质页面合并进**单独一个"调试"页签**。
3. **功能语义化**：把 SDK 中的功能位（func id）、工作模式、事件码、帧头字段解析出来，UI 全程显示可读名称（如"心率 HR (4 CH)"），不再显示裸的 `func1`、`0x0008`。

---

## 2. 调研结论

### 2.1 GH3036 页面结构（对齐目标）

页签定义：`src/components/TitleBar/Gh3036TitleTabs.tsx`

| key | 名称 | 图标 | 内容 |
|---|---|---|---|
| config | 配置 | SettingOutlined | 连接、通道、CSV 配置 |
| monitor | 监控 | LineChartOutlined | 生命体征卡（VitalSignCard：心率/血氧）+ HRV 卡 + 状态组合卡（StatusCombinedCard）+ 实时波形卡（时长/采样率控制） |
| version | 版本 | InfoCircleOutlined | 版本信息表 |
| factory | 出厂测试 | ExperimentOutlined | FactoryTestTab |
| threshold | 阈值配置 | FileProtectOutlined | ThresholdConfigTab |

要点：**没有**独立的采集/控制/事件页签；数据监控与波形集中在 monitor；控制随 monitor/config 走。

### 2.2 GH3220 现状

页签定义：`src/components/TitleBar/Gh3220TitleTabs.tsx`，`pageTabsStore.gh3220ActiveTab` 为 7 值 union：

| key | 内容 | V2 去向 |
|---|---|---|
| config | ConnectionSection + 采集偏好 + TCP 调试卡 | 保留（配置），TCP 调试卡迁往调试页 |
| monitor | 状态卡 + 实时波形（MultiLineChart） | 保留（监控），功能名语义化 |
| version | 版本表 | 保留（版本） |
| capture | 采集控制 + CSV 文件 + 最新记录 + 波形跳转 | **合并 → 调试页** |
| control | WORK_MODE / START / STOP / 槽位 / 寄存器 / FIFO / 事件掩码 | **合并 → 调试页** |
| events | 0x16 事件列表 + ACK | **合并 → 调试页** |
| debug | 原始帧 / 回显测试 / TCP 调试通道 | **合并 → 调试页** |

### 2.3 SDK 协议与功能映射（调研所得）

核心文件（均在 `gh3220_sdk\demo_kernel_code\` 下）：

- `module\gh_protocol\gh_uprotocol.h` / `.c`：UProtocol 帧格式与 CMD 定义
- `kernel\gh_demo_protocol.c`：收发 FIFO、事件帧组装与 ACK 重发
- `kernel\gh_demo.c`：demo 主流程、采样启停
- `driver\inc\gh_drv.h`：功能位 / 事件位 / 通道映射宏定义

#### 2.3.1 功能位映射（gh_drv.h L364-410）

`0x0B` 数据帧包头第 1 字节 `function_id` = 功能位掩码的 bit 序号（gh_uprotocol.c L1491-1498 转换逻辑）：

| func id | 宏 | 缩写 | 含义 | 真机观察 |
|---|---|---|---|---|
| 0 | GH3X2X_FUNCTION_ADT (0x01) | ADT | 佩戴检测 | func0，1 通道 ✓ |
| 1 | GH3X2X_FUNCTION_HR (0x02) | HR | 心率 | func1，4 通道 ✓ |
| 2 | GH3X2X_FUNCTION_HRV (0x04) | HRV | 心率变异性 | — |
| 3 | GH3X2X_FUNCTION_HSM (0x08) | HSM | 心脏健康监测 | — |
| 4 | GH3X2X_FUNCTION_FPBP (0x10) | FPBP | 指尖 PPG 血压 | — |
| 5 | GH3X2X_FUNCTION_PWA (0x20) | PWA | 脉搏波分析 | — |
| 6 | GH3X2X_FUNCTION_SPO2 (0x40) | SPO2 | 血氧 | — |
| 7 | GH3X2X_FUNCTION_ECG (0x80) | ECG | 心电（强制无 AGC） | — |
| 8 | GH3X2X_FUNCTION_PWTT (0x100) | PWTT | 脉搏波传导时间 | — |
| 9 | GH3X2X_FUNCTION_SOFT_ADT_GREEN (0x200) | SADT-G | 软件佩戴检测(绿光) | — |
| 10 | GH3X2X_FUNCTION_BT (0x400) | BT | 血压趋势 (BPTT) | — |
| 11 | GH3X2X_FUNCTION_RESP (0x800) | RESP | 呼吸 | — |
| 12 | GH3X2X_FUNCTION_AF (0x1000) | AF | 房颤检测 | — |
| 15 | GH3X2X_FUNCTION_SOFT_ADT_IR | SADT-IR | 软件佩戴检测(红外) | — |
| 19 | — | LEAD_DET | 心电导联检测 | — |

各功能通道数上限见 `kernel\gh_demo_config.h` L168-181（ADT=8、HR=32、SPO2=32、ECG=8 …，实际以设备 reg 配置为准）。

#### 2.3.2 WORK_MODE 模式值（gh_uprotocol.c L977-1001 + gh_drv.h L90-103/L442-449）

mode = payload[0] + 2（UPROTOCOL_CMD_EVK_MODE=2），UI 下拉应显示语义名：

| mode 值 | 枚举 | 含义 |
|---|---|---|
| 0 | EVK_MODE | EVK 上位机调试模式 |
| 1 | APP_MODE | APP 蓝牙直连模式 |
| 2 | MCU_ONLINE_MODE | MCU 在线模式（算法在主控） |
| 3 | MCU_OFFLINE_MODE | MCU 离线模式（芯片内算） |
| 4 | TEST_TUNE_MODE | 测试/调优模式 |
| 5 | PASS_THROUGH_MODE | 透传模式 |
| 6 | IRQ_STATUS_REPORT_ACK | 事件上报 ACK |
| 7 | MPT_MODE | MPT 多芯片并行模式 |

注意：START_CTRL 校验目标功能位 ⊆ WORK_MODE 设置的功能位（gh_uprotocol.c L927-961）。

#### 2.3.3 事件码（gh_drv.h L334-348）

| 位值 | 宏 | 含义 |
|---|---|---|
| 0x0001 | CHIP_WAKEUP | 芯片唤醒 |
| 0x0002 / 0x0004 | LEAD_ON / LEAD_OFF | 心电导联接上/断开 |
| 0x0008 | FAST_RECOVERY | 快速恢复 |
| 0x0010 | ADC_DONE | ADC 采样完成 |
| 0x0020 | FIFO_WATERMARK | FIFO 阈值中断 |
| 0x0040 | FIFO_FULL | FIFO 满 |
| 0x0100 / 0x0200 | TUNNING_FAIL / DONE | 调谐失败/完成 |
| 0x0400 / 0x0800 | WEAR_ON / WEAR_OFF | 佩戴上/摘下 |
| 0x1000 | SLOT_TIMEOUT | 槽位超时 |
| 0x2000 | SAMPLE_RATE_ERR | 采样率错误 |
| 0x4000 | CHIP_RESET | 芯片复位 |

设备实际只上报其中 6 类（gh_demo_protocol.c L623 掩码过滤：导联、佩戴、复位、FIFO 满）。真机观察到的 `0x0008`（FAST_RECOVERY）不在 SDK 上报掩码内，可能来自固件私有扩展——映射表按全量录入并标注。

#### 2.3.4 0x0B 帧头位定义（gh_uprotocol.h L194-213）

- 8 字节包头：`[function_id][data_type][channel_mask 大端 4B][package_type][data_len]`
- `data_type` 位：bit0=GS 加速度、bit1=算法结果、bit2=AGC、bit3=环境光、bit4=陀螺仪、bit5=CAP、bit6=TEMP
- `package_type` 位：bit0=压缩包、bit1=奇偶帧标志、bit2+=FIFO 包模式、bit3=拼包计数、bit5=拼包结束、bit7=新数据/算法结果
- 帧体首字节为 FRAME_ID，随后 GS 6B + RAWDATA chnlNum×4B + AGC chnlNum×4B + 算法结果 5B×n

#### 2.3.5 可选增强：0x2C FUNCTION_INFO_UPDATE

首帧时设备上报 `[functionID, STGh3x2xFunctionInfo, chnlMap[chnlNum]]`（gh_uprotocol.c L1373-1401），可用于动态获取通道映射。注意 CMD 0x15 FUNC_MAP 在本 SDK 中被注释未实现。

### 2.4 真机对照验证

| 观察值 | SDK 解读 | 结论 |
|---|---|---|
| func0（1 通道） | ADT 佩戴检测 | 一致 |
| func1（4 通道） | HR 心率 | 一致 |
| 事件 0x0008 | FAST_RECOVERY（SDK 掩码外） | 固件私有扩展，按全量表映射 |
| AGC 值 3486981/4605445 | AGC chnlNum×4B | 一致 |

---

## 3. 实施计划

### P0 功能语义层（新增 `src/constants/gh3220Functions.ts`）

| 项 | 内容 |
|---|---|
| F1 | `GH3220_FUNCTION_MAP`: funcOffset → `{ key, abbr, nameZh, nameEn, mask }`（2.3.1 全表 + SDK 出处注释） |
| F2 | `GH3220_WORK_MODE_MAP`: mode 值 → 名称（2.3.2 表） |
| F3 | `GH3220_EVENT_MAP`: 事件位 → 语义名（2.3.3 表，含 0x0008 特注） |
| F4 | `parseDataType(bits)` / `parsePackageType(bits)`: 帧头位解析工具（返回启用项数组） |
| F5 | `funcMaskToText(text)`: `'hr'` 等语义文本 → mask 的解析扩展（支持 hr/hrv/spo2/ecg/adt… 多名） |

### P1 页签结构对齐（7 → 4）

| 项 | 内容 |
|---|---|
| S1 | `pageTabsStore.gh3220ActiveTab` 改为 4 值 union：`'config' \| 'monitor' \| 'version' \| 'debug'` |
| S2 | `Gh3220TitleTabs` 页签表同步（图标：Setting/LineChart/InfoCircle/BugOutlined） |
| S3 | 新 `DebugTab`（合并页）：Collapse 四区——① 采集与 CSV（原 CaptureTab 主体）② 采样命令控制（原 ControlTab）③ 芯片事件（原 EventTab）④ TCP 调试通道 + 原始帧/回显（原 DebugTab） |
| S4 | `ConfigTab` 移除 TCP 调试卡（迁往调试区④）；保留连接 + 采集偏好 |
| S5 | 删除 `CaptureTab.tsx` / `ControlTab.tsx` / `EventTab.tsx` 独立页（内容迁入 DebugTab 分区，可保留为分区组件文件 `DebugTab/` 子目录） |
| S6 | `menuVisibilityStore.home.gh3220.tabs` 收敛为 4 项；持久化配置兼容旧值（旧 key 落到 debug） |
| S7 | i18n：`gh3220.json` tabs 段 + debug 页文案；`home.json` 同步 |

### P2 功能语义贯穿 UI

| 项 | 位置 | 改动 |
|---|---|---|
| U1 | MonitorTab 功能选择器 | "func1 (4 CH)" → "心率 HR (4 CH)"，未知 func 显示 "功能 funcN" 兜底 |
| U2 | MonitorTab 状态卡 | 采集/记录卡增加当前开启功能（WORK_MODE mask → 名称列表） |
| U3 | DebugTab 采集区 | "波形 funcN" 按钮、"最新记录"表功能列语义化 |
| U4 | DebugTab 控制区 | WORK_MODE 下拉语义化（保留原始数值入参）；功能位文本占位提示支持语义名 |
| U5 | DebugTab 事件区 | 事件列表新增"事件含义"列（GH3220_EVENT_MAP），未知码显示原始值 |
| U6 | 波形页联动 | `/waveform` 缓冲标注：`gh3220:func1` 旁显示语义名（保持缓冲 ID 稳定，仅展示层） |
| U7 | DebugTab TCP/原始帧区 | 帧详情解析 data_type/package_type 位（F4 工具落地处） |

### P3 可选增强（默认不排期，按需启用）

| 项 | 内容 |
|---|---|
| E1 | 解析 0x2C FUNCTION_INFO_UPDATE 首帧上报，动态建立 funcId → 通道映射（静态表兜底） |
| E2 | 事件码→业务动作联动（如佩戴摘下时波形卡提示） |
| E3 | 多芯片框架衔接：GH3220_FUNCTIONS 纳入 `ChipModuleDescriptor.capabilities`（见 chip-ui-framework-design.md 阶段 B） |

---

## 4. 涉及文件清单

**新增**：
- `src/constants/gh3220Functions.ts`（P0 全部映射与工具）
- `src/pages/Gh3220/DebugTab/index.tsx` + `CaptureSection.tsx` / `ControlSection.tsx` / `EventSection.tsx` / `ChannelSection.tsx`（S3-S5 合并页）

**修改**：
- `src/components/TitleBar/Gh3220TitleTabs.tsx`（S2）
- `src/stores/pageTabsStore.ts`（S1）、`src/stores/menuVisibilityStore.ts`（S6）
- `src/pages/Gh3220/index.tsx`（switch 收敛 + debug 分支）
- `src/pages/Gh3220/ConfigTab.tsx`（S4）
- `src/pages/Gh3220/MonitorTab.tsx`（U1/U2）
- `src/locales/zh-CN/gh3220.json`、`src/locales/en-US/gh3220.json`、`src/locales/{zh-CN,en-US}/home.json`（S7、语义名）
- `src/pages/Waveform/BufferConfigPanel` 相关展示（U6，仅标注）

**删除**：
- `src/pages/Gh3220/CaptureTab.tsx`、`ControlTab.tsx`、`EventTab.tsx`、`DebugChannelCard.tsx`（或迁为 DebugTab 子组件）

---

## 5. 验证方案

1. **每阶段**：`npx tsc --noEmit` 通过；dev server HMR 无报错。
2. **模拟器走查**（`scripts/gh3220_sim.py` + `scripts/cdp.py`）：
   - 页签渲染 4 个、切换正常、旧持久化 tab 值兼容；
   - 调试页四区功能等价回归（采集启停/CSV、WORK_MODE→START、事件 ACK、原始帧）；
   - MonitorTab 显示"心率 HR (4 CH)"/"佩戴检测 ADT (1 CH)"；
   - 事件页显示语义名；波形页缓冲标注语义名。
3. **真机复验**（GH3220_Nordic 在场）：完整 6 项走查对照（连接/版本/WORK_MODE→START/采集+CSV/波形/事件 ACK），注意不下发 STOP 以免需要复位。
4. **提交**：按 P0、P1、P2 三个 commit 提交到 `feat/gh3220-ui-alignment` 分支。

## 6. 风险与备注

- 事件 `0x0008`（FAST_RECOVERY）不在 SDK 上报掩码内，为固件私有扩展——映射表按全量录入并注释说明，未知码 UI 兜底显示原始值。
- 功能位语义文本（`'hr'`）非 SDK 定义，来自固件/对接文档——F5 保持向后兼容（未知文本按数字 mask 解析）。
- 缓冲区 ID `gh3220:funcN` 保持不变（避免破坏 /waveform 联动与 CSV 命名），语义名仅做展示层。
- 菜单可见性持久化中的旧页签 key（capture/control/events）需映射到 debug，避免配置残留导致页签消失。

---

## 7. 追加需求与验证记录（2026-11-06）

### 7.1 现场追加需求

1. 配置页要与 GH3036 一样支持「通道配置」与「各个命令的收发」；
2. 命令目录与参数要以**表格逐行列出**，样式对齐 GH3036 的 RPC 命令区（每行：命令名 + 参数控件 + 发送按钮）。

实现：

- `src/pages/Gh3220/Gh3220ChannelConfig.tsx`：连接类型 BLE / TCP、设备扫描与地址、服务/写/通知 UUID、MTU、保存通道配置（localStorage）、连接/断开/刷新信息、CSV 与采集偏好（含目录选择），布局沿用 GH3036 的 16:8 两栏；
- `src/pages/Gh3220/Gh3220CommandConsole.tsx`：22 条 `gh3220_*` 命令目录表（命令号 Tag + 语义名 + 行内参数 + 行内发送），STOP 二次确认，附最近一次响应与 TX/RX 日志表；
- `ConfigTab.tsx` 收敛为「通道配置 / UProtocol 命令收发」两个折叠区，中英文 i18n 同步；
- `src/stores/menuVisibilityStore.test.ts`：4 个用例覆盖旧页签 key 收敛（单测替代现场改持久化配置，避免污染用户偏好）。

### 7.2 验证环境

- 应用：ComBridge 0.9.0，`npm run tauri dev` 运行中，CDP `127.0.0.1:9222`，standard 档；
- 真机：`GH3220_Nordic`（`BluetoothLE#...f9:ac:fb:db:6e:72`），已连接且**持续采集中**；
- 约束：全程**未下发 STOP**、未断开/重连、未主动触发事件；未单独启动 `scripts/gh3220_sim.py`（GH3220 单会话，切换 TCP 会打断真机采集）。

### 7.3 验证结果

| 项 | 结果 |
|---|---|
| `npx tsc --noEmit` | 通过 |
| `npx vitest run` | 415/415 通过（40 个测试文件），含语义层 8 例、旧页签兼容 4 例 |
| Vite dev 编译 | 新增模块请求均 200（需 `--noproxy '*'` 绕过环境代理，直连 502 是代理干扰） |
| 页签 / 旧配置 | 标题栏 4 个：配置/监控/版本/调试；旧 key 兼容由单测覆盖 |
| 调试页 | 四折叠区全部展开；记录表功能列「心率 HR / 佩戴检测 ADT」；WORK_MODE 下拉「0 · EVK 上位机调试模式」；事件表含「事件含义」列；帧头 `01 03 00 00 00 0F 80 10` 解出 心率 HR / data_type GS+算法结果 / package_type 新数据 / channel_mask 4 CH |
| 监控页 | 功能选择器「佩戴检测 ADT (1 CH)」、当前开启功能「心率 HR」、CRC/LOST/RESYNC 全 0 |
| 版本页 | 全部刷新拉到真机 4 个版本串，maxLen 238、openedFunc 0x2、链路正常 |
| 波形页 | 下拉显示 `gh3220:func0 · 佩戴检测 ADT`、`gh3220:func1 · 心率 HR`；value 仍是 `gh3220:funcN`（MCP `waveform list` 佐证） |
| 配置页新面板 | 通道配置显示真机地址 / GH3220_Nordic / 244 B / 采集中；命令表格 22 行、22 个发送按钮、27 个参数控件，列头「命令 / 参数 / 操作」 |
| 命令收发实测 | 快捷只读 4 条 + 表格行内 1 条全部返回真机数据：版本串、`capturing`/BLE/244 B、229717 帧且 CRC/坏包/丢帧全 0、1147205 记录、CSV 两文件；响应面板与收发日志均正确 |
| 真机 6 项走查 | 连接、版本、采集+CSV（func1 4 通道 + 4 AGC、func0 单通道，活跃文件持续增长）、波形缓冲均通过；WORK_MODE→START 以当前 `capturing` + `openedFunc 0x2` 佐证（未重复下发，避免打断采集）；事件 ACK 由后端自动处理，本次无事件、无重发 |

### 7.4 未执行项

- TCP 模拟器链路（`scripts/gh3220_sim.py`）未单独连接验证：需先断开真机单会话，会中断正在进行的采集；配置页已提供 TCP 通道开关与地址，可后续离线补走查。
- 真机本次未产生 0x16 事件（事件表 0 条），事件语义名的具体取值未现场观察，映射由 `gh3220Functions.test.ts` 覆盖；未知码 UI 兜底原始值。
- 未做断开重连、未下发 STOP（按风险备注执行）。
