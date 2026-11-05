# 芯片工作台可复用 UI 框架设计

> 目标：接任何新芯片（GH32xx / GH30xx / 其它厂商）时，UI 层只写"接入描述文件 + 数据映射"，
> 不再重复开发页面骨架、页签体系、波形、版本、统计等通用能力。
> 本设计是 [`gh3220-frontend-alignment-plan.md`](gh3220-frontend-alignment-plan.md) 的上层演进。

---

## 1. 三个架构问题的决策

### Q1：要不要使用相同的接口？—— **要，定义"芯片模块契约"**

现状痛点：GH3036 与 GH3220 的 store/api/页面结构相似但互不相认，第三个芯片还要再抄一遍。
方案：抽一层 **前端能力契约**，通用 UI 只认契约，芯片只做适配：

```ts
// src/chipworkbench/contract.ts
/** 每个芯片注册一个 descriptor，一切通用 UI 由它驱动 */
export interface ChipModuleDescriptor {
  id: string;                    // 'gh3220'
  route: string;                 // '/gh3220'
  name: string;                  // 显示名（i18n key）
  icon: ReactNode;
  i18nNs: string;                // 'gh3220'
  menuKey: HomeMenuKey;          // menuVisibility 分组 key
  defaultTab: string;
  tabs: WorkbenchTabDescriptor[];// 页签集合（key/label/icon/component/可见性）
  capabilities: ChipCapabilities;// 能力声明，通用组件按需渲染
}

/** 能力块：芯片实现多少就开多少，UI 不写死 */
export interface ChipCapabilities {
  connection: ChipConnectionAdapter;   // 连接/断开/状态 + 参数描述(BLE/TCP/串口)
  waveform?: ChipWaveformAdapter;      // 归一化波形流
  version?: ChipVersionAdapter;        // 版本条目列表 + 读取方法
  capture?: ChipCaptureAdapter;        // 启停控制 + 统计
  events?: ChipEventAdapter;           // 事件流
  registers?: ChipRegisterAdapter;     // 寄存器读写（可选）
  config?: ChipConfigAdapter;          // 偏好配置
}

/** 归一化数据事件 —— 各芯片 store 负责把私有 topic 映射成这个形状 */
export interface ChipWaveformFrame {
  chipId: string;
  functionId: number;
  frameId: number;
  channels: number[][];          // raw 通道
  agc?: number[][];
  acc?: [number, number, number];
  algo?: Array<[number, number]>;
  tsMs: number;
}
```

**关键点**：契约只存在于前端。后端不动 —— 各芯片继续用独立 topic 前缀
（`gh3036:*`、`gh3220:*`、未来 `ghXXXX:*`），由各芯片 store 里的
**事件映射器**（十几行代码）把私有 payload 转成 `ChipWaveformFrame`。

### Q2：顶部切换器 还是 不同页面？—— **不同页面（路由隔离）+ 注册表驱动 + 顶部快捷切换器**

| 方案 | 评估 |
|---|---|
| A. 单页面 + 顶部芯片下拉切换同一套组件 | ❌ 切换时要 teardown/重建订阅与 store，波形缓存、连接状态、菜单可见性、MCP 域都纠缠在一起；GH3036 的生命体征卡与 GH3220 的统计卡差异大，"同页面换数据源"会让组件 props 泛滥 |
| B. **每芯片独立路由（现状延伸）+ registry 驱动** | ✅ 页面/订阅/状态天然隔离；`/gh3036`、`/gh3220` 深链接可分享；现有 menuVisibility、pageTabsStore、TitleBar 模式无缝对接；MCP 工具域也按芯片划分，认知一致 |
| C. 一个应用一个产品，多开 | ❌ 见 Q3 |

采纳 B，并**加一个顶部"工作台切换器"作为快捷导航**（不是页面替换）：
TitleBar 左侧 App 名旁放一个下拉，列出 registry 中所有芯片 + 连接状态小圆点，
点击跳对应路由 —— 回答"顶部要不要有切换"：有，但它切换的是**页面**，不是替换数据源。

### Q3：一个应用一个产品、靠多开测试？—— **不推荐，单实例多工作台**

多开的问题：
- Windows 蓝牙适配器是系统级独占资源，两个实例抢 BLE 会互相失败；
- MCP 端口（47821）、preferences.yaml、日志文件锁、WebView2 用户数据目录都会冲突；
- 无法做"同一时刻多产品数据并排"——这恰是多开想达到的目的。

正确路径：
1. **现在**：单实例多工作台。EventBus 本来就按 topic 隔离并行数据流，
   切到别的芯片页面时，后台 store 常驻、采集不断、CSV 继续落盘；
2. **远期（有真实并排需求再做）**：Tauri 多窗口（`WebviewWindow`），
   每窗口一个芯片工作台，**共享同一个后端进程与 BLE 栈** —— 既并行又不多开。

---

## 2. 框架分层

```
┌─ 芯片接入层（每芯片 ~1 个 descriptor + 差异组件）─────────────┐
│  GH3036 descriptor      GH3220 descriptor      未来芯片 …      │
│  （页签注册/能力声明/事件映射器/专属组件如生命体征卡）          │
├─ 可复用工作台框架 src/chipworkbench/ ─────────────────────────┤
│  contract.ts        契约 + 归一化事件类型                      │
│  registry.ts        芯片注册表（Sidebar/TitleBar/菜单派生源）  │
│  components/        WorkbenchPage / ChipTitleTabs /           │
│                     WaveformPanel / VersionTable /            │
│                     StatsCard / CaptureControlCard /          │
│                     ConnectionCard / EventListCard            │
│  hooks/             useChipWaveformBuffer / useChipTopics     │
├─ 后端（不改动）───────────────────────────────────────────────┤
│  EventBus topic 前缀隔离：gh3036:* │ gh3220:* │ ghXXXX:*      │
└───────────────────────────────────────────────────────────────┘
```

- **WorkbenchPage**：通用页面骨架（Spin 初始化 + error Alert + 按 descriptor.tabs 渲染），
  即 GH3036 `index.tsx` 的泛化版；
- **ChipTitleTabs**：`Gh3036TitleTabs` 的泛化版，tabs 数组来自 descriptor，
  `menuVisibility.home[menuKey].tabs` 控制显隐，`pageTabsStore` 按 `chipId` 分域；
- **WaveformPanel**：内置环形缓冲 hook（maxFrames 随显示时长/采样率计算）+ MultiLineChart +
  chartGroups 分组，props 由 `ChipWaveformAdapter` 提供；
- **VersionTable / StatsCard / CaptureControlCard / EventListCard / ConnectionCard**：
  全部 props 驱动，GH3220/GH3036 共用同一套实现。

---

## 3. 目录结构（目标态）

```
src/
├── chipworkbench/               # 通用框架（不认识任何具体芯片）
│   ├── contract.ts
│   ├── registry.ts              # registerChipModule() / getChipModules()
│   ├── components/
│   │   ├── WorkbenchPage.tsx
│   │   ├── ChipTitleTabs.tsx
│   │   ├── ChipSwitcher.tsx     # 顶部工作台切换下拉
│   │   ├── WaveformPanel.tsx
│   │   ├── VersionTable.tsx
│   │   ├── StatsCard.tsx
│   │   ├── CaptureControlCard.tsx
│   │   ├── ConnectionCard.tsx
│   │   └── EventListCard.tsx
│   └── hooks/
│       ├── useChipWaveformBuffer.ts
│       └── useChipTopics.ts     # listen(event-bus) + topic 前缀分发
├── pages/Gh3220/
│   ├── descriptor.tsx           # 芯片接入文件（唯一新增入口）
│   ├── tabs/…                   # 专属页签组件（MonitorTab 特化等）
│   └── gh3220EventMapper.ts     # gh3220:data → ChipWaveformFrame
├── pages/Gh3036/
│   ├── descriptor.tsx
│   └── …                        # 生命体征卡等专属组件保留
└── stores/pageTabsStore.ts      # 改为 Record<chipId, string> 动态分域
```

---

## 4. 新芯片接入 SOP（预期 ≤1 天 UI 工作量）

1. 后端：新增 `src-tauri/src/ghXXXX/` + topic 前缀 `ghXXXX:*` + Tauri commands（后端规范不变）；
2. 前端 API/store：实现 `ChipCapabilities` 各适配器 + 事件映射器（私有 payload → `ChipWaveformFrame`）；
3. 页签组件：通用页签直接用（Version/Stats/Events…），差异页签自写（如生命体征卡）；
4. 写 `descriptor.tsx`，在 `registry.ts` 注册一行；
5. i18n 新增 `ghXXXX.json`（只需本芯片差异文案，通用文案走 `chipworkbench` 命名空间）；
6. menuVisibility 默认值加一组；模拟器 + cdp.py 工具链按 chipId 参数化回归。

**对比现状**：GH3220 接入写了 9 个前端文件 ≈1400 行；框架就位后新芯片预期
descriptor + 映射器 + 1~2 个差异页签 ≈300~500 行。

---

## 5. 实施路径（与 GH3220 对齐方案衔接）

| 阶段 | 内容 | 说明 |
|---|---|---|
| **A** | 按 alignment plan 完成 GH3220 对齐（P0~P3） | 不做框架，先让两个芯片"长得一样"，避免过早抽象 |
| **B** | 抽取框架：GH3220 作为第一个接入方迁移到 chipworkbench，GH3036 第二个回归 | 从两份相似实现中提炼，通用组件有真实双用例验证 |
| **C** | registry 驱动 Sidebar/TitleBar/menuVisibility + 顶部 ChipSwitcher | 导航代码从硬编码 switch 改为注册表遍历 |
| **D**（可选） | Tauri 多窗口并行对比 | 有真实需求再做；后端已天然支持 |

## 6. 风险与注意点

- **pageTabsStore 泛化**：`gh3220ActiveTab` 等 5 个字段改为
  `tabs: Record<string, string>` + `setActiveTab(chipId, tab)`，老字段保留过渡期别名；
- **menuVisibility 兼容**：`home[chipKey].tabs` 结构不变，registry 只读不改结构，
  旧偏好文件零迁移；
- **zustand store 不合并**：各芯片 store 保持独立（状态隔离是 Q2 选 B 的根基），
  框架只约束"暴露适配器所需的 selector"；
- **不要过早抽 Hook**：阶段 B 之前禁止动手抽框架 —— GH3036 的 RPC 流、GH3220 的
  UProtocol 流差异仍在消化中，先让第二份实现落地再提炼；
- **i18n 双命名空间**：`chipworkbench`（通用）+ `ghXXXX`（差异），避免通用组件文案散落各芯片文件。

## 7. 验证

沿用已搭好的工具链（`scripts/gh3220_sim.py` + `scripts/cdp.py`），按 chipId 参数化模拟器：
`python scripts/gh3220_sim.py --chip gh3220`；验收在 alignment plan 第 6 节基础上增加：
切换工作台时另一芯片采集不中断、顶部切换器状态点正确、新芯片模拟接入演练（可用
gh3220 descriptor 复制改名做冒烟测试）。
