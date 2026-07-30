# GH3036 图表重构实施计划

> **对于代理工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务执行此计划。步骤使用复选框（`- [ ]`）语法进行跟踪。

**目标：** 将 PPG 和 ACC 图表合并到一个 MultiLineChart 组件中，实现自动联动，简化代码结构。

**架构：** 采用 CsvLoaderTab 的方式，使用一个 MultiLineChart 组件，内部通过 echarts.connect 实现多个图表组的自动联动。合并 ipdPaChartData 和 gsensorChartData，创建统一的 chartGroups 配置，删除手动状态管理代码。

**技术栈：** React + TypeScript + ECharts + Zustand

---

## 文件结构

**修改文件：**
- `src/pages/Gh3036/MonitorTab.tsx` - 合并数据和图表，简化布局
- `src/stores/gh3036Store.ts` - 删除 sharedDataZoomState 状态

**影响范围：** 仅限前端代码，不影响后端和数据流

---

### Task 1: 删除 sharedDataZoomState 状态

**文件：**
- Modify: `src/stores/gh3036Store.ts:126,339,595`

- [ ] **Step 1: 从接口定义中删除 sharedDataZoomState**

找到第 126 行，删除这一行：

```typescript
// 删除这一行
sharedDataZoomState: { start: number; end: number };
```

- [ ] **Step 2: 从初始状态中删除 sharedDataZoomState**

找到第 339 行，删除这一行：

```typescript
// 删除这一行
sharedDataZoomState: { start: 0, end: 100 },
```

- [ ] **Step 3: 删除 setSharedDataZoomState 方法**

找到第 595 行，删除这一行：

```typescript
// 删除这一行
setSharedDataZoomState: (state) => set({ sharedDataZoomState: state }),
```

- [ ] **Step 4: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`
Expected: 编译错误，提示 MonitorTab.tsx 中使用了未定义的 sharedDataZoomState

- [ ] **Step 5: 提交**

```bash
git add src/stores/gh3036Store.ts
git commit -m "refactor(gh3036): 删除 sharedDataZoomState 状态"
```

---

### Task 2: 合并 PPG 和 ACC 数据

**文件：**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:124-178`

- [ ] **Step 1: 创建合并的 chartGroups**

替换原来的 `ipdPaChartGroups` 和 `gsensorChartGroups`（第 157-178 行）：

```typescript
const chartGroups = useMemo(() => {
  const groups = [];

  // PPG/IPD 数据图表组
  if (ipdPaChartData.columns.length > 0 && ipdPaChartData.rows.length > 0) {
    const ppgColumns: string[] = [];
    for (let i = 0; i < Math.min(currentFrames?.channel_count ?? 0, 4); i++) {
      ppgColumns.push(`CH${i}`);
    }
    if (ppgColumns.length > 0) {
      groups.push({
        name: ipdRawDataType === 'ipd' ? t('monitor.ipdPaChart') : t('monitor.rawdataChart'),
        columns: ppgColumns,
        height: 250,
      });
    }
  }

  // ACC 数据图表组
  if (gsensorChartData.columns.length > 0 && gsensorChartData.rows.length > 0) {
    groups.push({
      name: t('monitor.gsensorChart'),
      columns: gsensorChartData.columns,
      height: 200,
    });
  }

  return groups;
}, [currentFrames, ipdRawDataType, gsensorChartData, t, ipdPaChartData]);
```

- [ ] **Step 2: 创建合并的 allChartData**

在 `gsensorChartData` 后面添加（约第 156 行后）：

```typescript
const allChartData = useMemo(() => {
  const columns: string[] = [];
  const rows: number[][] = [];

  // 合并 PPG 列
  if (ipdPaChartData.columns.length > 0) {
    columns.push(...ipdPaChartData.columns);
  }

  // 合并 ACC 列
  if (gsensorChartData.columns.length > 0) {
    columns.push(...gsensorChartData.columns);
  }

  // 合并数据行（以较长的数据为准，缺失的数据填充为 0）
  const maxRows = Math.max(
    ipdPaChartData.rows.length,
    gsensorChartData.rows.length
  );

  for (let i = 0; i < maxRows; i++) {
    const row: number[] = [];

    // 添加 PPG 数据
    if (ipdPaChartData.rows[i]) {
      row.push(...ipdPaChartData.rows[i]);
    } else {
      row.push(...Array(ipdPaChartData.columns.length).fill(0));
    }

    // 添加 ACC 数据
    if (gsensorChartData.rows[i]) {
      row.push(...gsensorChartData.rows[i]);
    } else {
      row.push(...Array(gsensorChartData.columns.length).fill(0));
    }

    rows.push(row);
  }

  return { columns, rows };
}, [ipdPaChartData, gsensorChartData]);
```

- [ ] **Step 3: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`
Expected: 通过，但可能有未使用变量的警告

- [ ] **Step 4: 提交**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "refactor(gh3036): 合并 PPG 和 ACC 数据为统一的 chartGroups"
```

---

### Task 3: 删除 sharedDataZoomState 使用和初始化逻辑

**文件：**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:50-51,180-212`

- [ ] **Step 1: 从 useGh3036Store 删除 sharedDataZoomState 导入**

找到第 50-51 行，删除这两行：

```typescript
// 删除这两行
sharedDataZoomState,
setSharedDataZoomState,
```

- [ ] **Step 2: 删除 initialDataZoomState 和相关逻辑**

删除第 180-212 行的所有代码：

```typescript
// 删除所有这些代码
const initialDataZoomState = useMemo(() => {
  if (!ipdPaChartData.rows.length) {
    return { start: 0, end: 100 };
  }

  // 始终显示 100% 的数据，displayDurationSeconds 只影响缓存大小
  console.log('[MonitorTab] 默认显示全部数据:', {
    totalPoints: ipdPaChartData.rows.length,
    sampleRate,
    displayDurationSeconds,
  });

  return { start: 0, end: 100 };
}, [ipdPaChartData.rows.length, sampleRate, displayDurationSeconds]);

// 使用 ref 跟踪是否已初始化，避免重复设置
const isDataZoomInitializedRef = useRef(false);

useEffect(() => {
  console.log('[MonitorTab] useEffect 触发:', {
    rowsLength: ipdPaChartData.rows.length,
    sharedDataZoomState,
    initialDataZoomState,
    isInitialized: isDataZoomInitializedRef.current,
  });
  
  // 只在数据首次到达时设置初始状态
  if (ipdPaChartData.rows.length > 0 && !isDataZoomInitializedRef.current) {
    console.log('[MonitorTab] 首次初始化 sharedDataZoomState:', initialDataZoomState);
    setSharedDataZoomState(initialDataZoomState);
    isDataZoomInitializedRef.current = true;
  }
}, [ipdPaChartData.rows.length, initialDataZoomState, setSharedDataZoomState]);
```

- [ ] **Step 3: 从 import 中删除未使用的 useRef**

找到第 1 行，修改为：

```typescript
import React, { useMemo, useState, useEffect } from 'react';
```

删除 `useRef`，因为不再需要。

- [ ] **Step 4: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`
Expected: 通过

- [ ] **Step 5: 提交**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "refactor(gh3036): 删除 sharedDataZoomState 使用和初始化逻辑"
```

---

### Task 4: 简化布局，合并两个 Card 为一个

**文件：**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:266-350`

- [ ] **Step 1: 替换两个 Card 为一个合并的 Card**

找到第 266-350 行，将两个 Card（PPG 和 ACC）替换为一个：

```typescript
<Card
  size="small"
  title={
    <Space>
      <span>{t('monitor.dataMonitor')}</span>
      <Tooltip title={t('monitor.displayDurationTooltip')}>
        <Space size={4}>
          <ClockCircleOutlined />
          <InputNumber
            size="small"
            min={1}
            max={60}
            value={displayDurationSeconds}
            onChange={handleDisplayDurationChange}
            style={{ width: 60 }}
            addonAfter="s"
            disabled={selectedFunctionId === null}
          />
        </Space>
      </Tooltip>
      <Tooltip title={t('monitor.sampleRateTooltip')}>
        <Space size={4}>
          <SettingOutlined />
          <InputNumber
            size="small"
            min={1}
            max={1000}
            value={sampleRate}
            onChange={handleSampleRateChange}
            style={{ width: 70 }}
            addonAfter="Hz"
            disabled={selectedFunctionId === null}
          />
        </Space>
      </Tooltip>
      <Select
        size="small"
        style={{ width: 100 }}
        value={ipdRawDataType}
        onChange={setIpdRawDataType}
        options={[
          { value: 'ipd', label: t('monitor.ipd') },
          { value: 'rawdata', label: t('monitor.rawdata') },
        ]}
      />
      <Select
        size="small"
        style={{ width: 150 }}
        value={selectedFunctionId}
        onChange={setSelectedFunctionId}
        options={functionOptions}
        placeholder={t('monitor.selectFunction')}
      />
      <Button
        size="small"
        icon={<ClearOutlined />}
        onClick={clearWaveformData}
      >
        {t('monitor.clearData')}
      </Button>
    </Space>
  }
  style={{ flex: '0 0 auto' }}
  styles={{ body: { padding: 8, height: 510 } }}
>
  {allChartData.columns.length > 0 && allChartData.rows.length > 0 ? (
    <MultiLineChart
      columns={allChartData.columns}
      rows={allChartData.rows}
      chartGroups={chartGroups}
      sampleRate={sampleRate}
    />
  ) : (
    <Empty description={t('monitor.noData')} style={{ marginTop: 200 }} />
  )}
</Card>
```

- [ ] **Step 2: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`
Expected: 通过

- [ ] **Step 3: 提交**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "refactor(gh3036): 合并 PPG 和 ACC 图表为一个统一的组件"
```

---

### Task 5: 清理未使用的导入和常量

**文件：**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:1-16`

- [ ] **Step 1: 删除未使用的 DEFAULT_SAMPLE_RATE 常量**

找到第 16 行，删除：

```typescript
// 删除这一行
const DEFAULT_SAMPLE_RATE = 25;
```

- [ ] **Step 2: 运行 TypeScript 检查**

Run: `npx tsc --noEmit`
Expected: 通过

- [ ] **Step 3: 提交**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "refactor(gh3036): 删除未使用的 DEFAULT_SAMPLE_RATE 常量"
```

---

### Task 6: 运行完整测试

**文件：**
- 无文件修改

- [ ] **Step 1: 运行 TypeScript 类型检查**

Run: `npx tsc --noEmit`
Expected: 通过，无错误

- [ ] **Step 2: 运行前端测试**

Run: `npm run test -- --passWithNoTests`
Expected: 所有测试通过

- [ ] **Step 3: 手动测试验证**

启动应用：`npm run tauri dev`

测试点：
1. 选择功能后，PPG 和 ACC 数据是否显示在同一个图表中
2. 拖动 dataZoom 滑块，两个图表是否自动联动
3. 修改显示秒数，缓存大小是否正确调整
4. 清除数据后，图表是否正确清空

- [ ] **Step 4: 提交**

```bash
git add .
git commit -m "test(gh3036): 验证图表重构功能"
```

---

## 自我审查清单

**1. 规格覆盖：**
- ✅ 合并 ipdPaChartData 和 gsensorChartData → Task 2
- ✅ 创建统一的 chartGroups → Task 2
- ✅ 删除 sharedDataZoomState → Task 1, Task 3
- ✅ 简化布局 → Task 4

**2. 占位符扫描：**
- ✅ 无 "TBD"、"TODO"、"implement later"
- ✅ 无 "Add appropriate error handling"
- ✅ 无 "Write tests for the above"
- ✅ 无 "Similar to Task N"
- ✅ 所有代码步骤都有完整代码块

**3. 类型一致性：**
- ✅ `allChartData` 类型：`{ columns: string[]; rows: number[][] }`
- ✅ `chartGroups` 类型：`Array<{ name: string; columns: string[]; height: number }>`
- ✅ 所有地方使用一致的类型定义

---

## 执行选项

**计划完成并保存到 `docs/superpowers/plans/2026-07-30-gh3036-chart-refactor.md`。两种执行选项：**

**1. Subagent-Driven（推荐）** - 每个任务派遣新的子代理，任务之间审查，快速迭代

**2. Inline Execution** - 在此会话中使用 executing-plans 执行，批量执行并在检查点审查

**选择哪种方式？**