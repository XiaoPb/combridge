# GH3220 前端页面对齐 GH3036 调整方案

> 目标：GH3220 页面的**页面处理方式、TAB 类型、内容结构**与 GH3036 保持一致，
> 仅后端调用的协议不同（GH3036 走 RPC/G 协议，GH3220 走 UProtocol）。

## 1. 现状差异分析

| 维度 | GH3036（目标模式） | GH3220（现状） |
|---|---|---|
| TAB 类型 | **标题栏页签** `Gh3036TitleTabs`（渲染在 TitleBar 中，按路由 `/gh3036` 切换） | 页内 antd `<Tabs>` 组件 |
| 页签状态 | `pageTabsStore.gh3036ActiveTab` + index.tsx 内 `switch` 渲染 | `gh3220ActiveTab` 仅作为 antd Tabs 的 activeKey |
| 页签可见性 | `menuVisibility.home.gh3036.tabs`（5 项，可配置显隐） | `menuVisibility.home.gh3220.tabs`（4 项，与标题栏体系未打通） |
| 页面骨架 | 初始化 Spin + error Alert + 内容 switch，页面高度 100% flex 布局 | antd Tabs + tabBarExtraContent 状态 Tag |
| 波形展示 | **页内实时波形** `MonitorTab`（复用 `Waveform/MultiLineChart`），顶部生命体征卡片行 | 无页内波形，仅"跳转到 /waveform 页"按钮 |
| 版本信息 | 独立 `VersionTab`：版本类型表 + 单项/全部刷新 | ConnectTab 内 Descriptions 只读展示 |
| 配置 | 独立 `ConfigTab`（Collapse：通道配置 + RPC 命令列表） | 散落在 ControlTab / CaptureTab 内 |
| Store 数据流 | store 内 `listen('event-bus')` 按 topic 分发，`framesData: Map<funcId, 帧缓存>` 环形缓存驱动图表 | store 直接存 `records[]` 最新 100 条，无波形缓存 |

## 2. 页签规划（TAB 类型与 GH3036 一致）

GH3036 页签：`config / monitor / version / factory / threshold`
GH3220 对应页签（内容等价、协议不同）：

| GH3220 页签 | 对应 GH3036 | 内容 |
|---|---|---|
| `config` 配置 | config | Collapse 两区块：① 设备连接（BLE 扫描/连接/断开 + TCP 调试通道）② 采集与 CSV 偏好（funcMask、通道覆盖、CSV 开关/目录/空闲关闭） |
| `monitor` 监控（默认） | monitor | 顶部状态卡片行 + 页内实时波形（MultiLineChart，订阅 `gh3220:data`） |
| `version` 版本 | version | GET_VER 版本类型表（0x01 固件 / 0x0A Demo / 0x0E 协议 / 0x10 驱动 / 0x11 芯片），单项 + 全部刷新 |
| `capture` 采集 | factory（地位对应：核心业务操作页） | 采集启停控制、解码统计、最新记录表、波形跳转按钮 |
| `events` 事件 | threshold（地位对应：结果/记录页） | 现有事件列表（eventId/reportId/ACK 状态） |
| `debug` 调试 | —（GH3036 无） | 保留，仅 DEV 构建显示，**不进标题栏页签**（与 GH3220 特有调试 TCP 配套） |

默认页签：`monitor`（与 GH3036 一致）。

## 3. 实施步骤

### P0 TAB 框架改造（标题栏页签 + 页面骨架）

1. **新增 `src/components/TitleBar/Gh3220TitleTabs.tsx`**
   - 照抄 `Gh3036TitleTabs` 模式：tabs 数组（config/monitor/version/capture/events，带 icon）、
     `menuVisibility.home.gh3220.tabs` 过滤可见项、active 状态类名 `title-bar-tab active`、
     useEffect 校正非法 activeTab。
2. **`src/components/TitleBar/TitleBar.tsx`**
   - `renderSubTabs()` 增加 `case '/gh3220': return <Gh3220TitleTabs />`。
3. **`src/stores/pageTabsStore.ts`**
   - `gh3220ActiveTab` 类型改为 `'config' | 'monitor' | 'version' | 'capture' | 'events' | 'debug'`，
     默认值 `'monitor'`（debug 保留在类型中以便 DEV 直达）。
4. **`src/pages/Gh3220/index.tsx` 骨架对齐**
   - 移除 antd `<Tabs>`；改为 GH3036 骨架：初始化时 Spin、顶部 error Alert（closable）、
     `switch (gh3220ActiveTab)` 渲染内容区，外层 `height:100% flex column overflow:hidden`。
   - 状态 Tag（已连接/采集中）从 tabBarExtraContent 移到监控页状态卡。
5. **`src/stores/menuVisibilityStore.ts`**
   - `gh3220.tabs` 改为 `{ config: true, monitor: true, version: true, capture: true, events: true }`。
6. **`src/components/MenuVisibilityConfigModal.tsx`**
   - `'gh3220'` 分组 key 列表与顺序数组同步为新页签集合。
7. **i18n**：`src/locales/zh-CN/gh3220.json`、`en-US/gh3220.json` 的 `tabs.*`
   增加 `config/monitor/version`，调整 `capture/events` 文案与 GH3036 命名风格一致。

### P1 VersionTab + ConfigTab（纯协议读取，工作量小）

8. **`src/api/gh3220.ts`** 增加 `readVersion(verType)` 批量封装
   （后端 `gh3220_get_device_info` 已具备能力，必要时新增按 type 查询命令）。
9. **新增 `src/pages/Gh3220/VersionTab.tsx`**
   - 对照 `Gh3036/VersionTab`：Table 列 = 类型名 / type 值 / 描述 / 版本串 / 刷新按钮，
     "全部刷新" 循环 GET_VER；未连接时按钮禁用 + Tag 提示。
10. **新增 `src/pages/Gh3220/ConfigTab.tsx`**
    - Collapse 两区块（样式照抄 `Gh3036/ConfigTab`）：
      - 设备连接：迁移现 `ConnectTab` 内容（扫描 Select、连接/断开/刷新信息、TCP 调试通道卡片）；
      - 采集与 CSV 偏好：迁移现 `CaptureTab` 的 csvTitle 卡片 + channelsOverride/autoStop。
    - 原 `ConnectTab.tsx` 删除。

### P2 MonitorTab（页内实时波形，核心增量）

11. **`src/stores/gh3220Store.ts` 扩展波形缓存**（对照 `gh3036Store.framesData` 模式）：
    - `framesData: Map<number, Gh3220FramesState>`，key = functionId；
      value = `{ columns: string[], rows: number[][] (环形, 上限 maxFramesCount), lastTs }`；
      列名 `RAW0..RAWn`（+ `ACC_X/ACC_Y/ACC_Z`，当 data_type bit0 有效）。
    - `gh3220:data` 事件处理器：按记录追加行，超限丢弃头部（与 GH3036 `setMaxFramesCount` 相同策略）。
    - 新增状态/方法：`selectedFunctionId`、`sampleRateConfig`（GH3220 25Hz 默认）、
      `displayDurationSeconds`、`chartLegendSelected`、`clearWaveformData`、`setMaxFramesCount`。
12. **新增 `src/pages/Gh3220/monitorChartData.ts`**
    - `buildRawChartData(frames, selectedId)`：从缓存截取 displayDuration 窗口输出 `{columns, rows}`；
      参考 `Gh3036/monitorChartData.ts` 的对齐策略（多源按最短行数对齐，不补 0）。
13. **新增 `src/pages/Gh3220/MonitorTab.tsx`**（布局照抄 `Gh3036/MonitorTab`）：
    - 顶部卡片行 4 张（Col xs=12 sm=6）：连接状态卡（复用 `StatusCombinedCard` 样式）、
      帧率/记录数卡、解码质量卡（CRC/丢帧/重同步，取自 stats）、采集状态卡；
      —— GH3220 无 HR/SpO2 生命体征算法卡，此为协议差异的合理替换，组件样式保持一致。
    - 数据监控 Card：标题栏右侧控件与 GH3036 完全一致
      （显示时长 s、采样率 Hz、RAW/IPD 切换 → GH3220 改为 raw/agc 切换、functionId Select、清空按钮）；
      body 内 `MultiLineChart`（`legendScope="gh3220"`），chartGroups：
      `gh3220-raw`（RAW 通道组）+ `gh3220-acc`（GS 三轴组）。
    - 空数据 `<Empty>`；流中断 Alert（streamStalled）可后续接入。

### P3 采集/事件页对齐 + 清理验证

14. **`CaptureTab.tsx`**：去除与 ConfigTab 重复的偏好卡片；Card 统一 `size="small"`；
    保留启停控制、统计 Descriptions、最新记录 Table、波形跳转按钮（跳 `/waveform`）。
15. **`EventTab.tsx`**：布局微调对齐（Card size、间距 8px），逻辑不变。
16. **`ControlTab.tsx`**：寄存器/命令控制保留；若内容与 ConfigTab 重合，合并后删除。
17. **`DebugTab.tsx`**：保留 DEV-only，从页签数组中移除（可通过 DEV 下的隐藏入口或 URL 参数进入）；
    或经菜单可见性配置默认隐藏。
18. **删除**：`ConnectTab.tsx`、原页内 Tabs 相关代码与无用 i18n key。

## 4. 涉及文件清单

| 操作 | 文件 |
|---|---|
| 新增 | `src/components/TitleBar/Gh3220TitleTabs.tsx`、`src/pages/Gh3220/{MonitorTab,VersionTab,ConfigTab,monitorChartData}.tsx/.ts` |
| 修改 | `src/pages/Gh3220/index.tsx`、`CaptureTab.tsx`、`EventTab.tsx`、`ControlTab.tsx`、`DebugTab.tsx`、`src/stores/pageTabsStore.ts`、`src/stores/gh3220Store.ts`、`src/stores/menuVisibilityStore.ts`、`src/components/TitleBar/TitleBar.tsx`、`src/components/MenuVisibilityConfigModal.tsx`、`src/api/gh3220.ts`、`src/locales/{zh-CN,en-US}/gh3220.json` |
| 删除 | `src/pages/Gh3220/ConnectTab.tsx` |

后端**无需改动**（`gh3220:data` 事件已含 records/按功能批量数据；版本读取命令已存在）。

## 5. 风险与注意点

- **数据量**：GH3220 采集 25Hz × 多通道，`gh3220:data` 每 60ms/100 条批量推送；
  store 环形缓存需在一次事件内批量 append（参照 gh3036Store 的 batch 处理），避免每条记录触发一次 set()。
- **channelsOverride**：波形列数以实际记录 `raw.length` 为准（解码端可能覆盖通道数）。
- **菜单可见性兼容**：旧偏好文件中 `gh3220.tabs` 为 `{connect,control,capture,events}`，
  merge 逻辑（`mergeMenuVisibility`）已按默认值补齐新 key，旧 key 残留无害；但
  `MenuVisibilityConfigModal` 顺序数组需同步，否则配置界面显示错位。
- **pageTabsStore 类型变更**：`debug` 页签保留类型成员但不在标题栏展示，避免 DEV 环境类型报错。
- **回归范围**：`/waveform` 页对 `gh3220:funcN` 缓冲的展示不受影响（后端波形缓冲机制不变）。

## 6. 验证方式（工具链已就绪）

本会话已搭好无硬件验证环境，改造完成后可直接回归：

```bash
# 终端 1: 应用（已带 CDP 调试端口）
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--remote-debugging-port=9222" npm run tauri dev

# 终端 2: UProtocol 模拟器（25Hz 连续推流）
python scripts/gh3220_sim.py --listen 127.0.0.1:9000 --rate 25

# 终端 3: 页面自动化（点击/取值/截图）
python scripts/cdp.py eval "<js>"   # 操作页面
python scripts/cdp.py shot out.png  # 截图验证
```

验收要点：
1. `/gh3220` 标题栏出现与 GH3036 同款页签，显隐受菜单配置控制，默认落在 monitor；
2. monitor 页：调试通道连接模拟器 → 开始采集 → 页内波形实时滚动（RAW 组 + ACC 组）、
   显示时长/采样率/清空控件与 GH3036 行为一致；
3. version 页：全部刷新拉到模拟器五个版本串；
4. capture/events/config 页功能与改造前等价；
5. `npx tsc --noEmit` 通过；zh/en 两语言无缺 key。
