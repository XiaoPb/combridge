# GH3036 前端显示优化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 优化 GH3036 数据监控模块的波形显示，支持显示全部历史数据、PPG 与 ACC 图表 x 轴联动，并提供可配置的最大显示秒数设置。

**Architecture:** 在 MonitorTab 组件中移除硬编码的显示时间限制，将 `buildIpdPaChartData` 改为返回全部数据，通过 ECharts dataZoom 实现默认显示最近 N 秒。PPG 和 ACC 图表共享相同的 dataZoom 状态和时间轴，确保 x 轴完全同步。在 gh3036Store 中添加可配置的 `displayDurationSeconds` 并持久化。

**Tech Stack:** React 19, TypeScript, ECharts 5, Zustand with persist middleware, Ant Design v6.3.5

---

## 文件结构

### 需要修改的文件：
- `src/stores/gh3036Store.ts` - 添加显示秒数配置
- `src/pages/Gh3036/MonitorTab.tsx` - 修改图表数据逻辑和 UI
- `src/pages/Gh3036/monitorChartData.ts` - 移除显示时间限制
- `src/pages/Waveform/MultiLineChart.tsx` - 支持共享 dataZoom 状态
- `src/locales/zh-CN/gh3036.json` - 添加中文翻译
- `src/locales/en-US/gh3036.json` - 添加英文翻译

---

### Task 1: 在 gh3036Store 中添加显示秒数配置

**Files:**
- Modify: `src/stores/gh3036Store.ts:124-126`

- [ ] **Step 1: 添加 displayDurationSeconds 状态**

在 `Gh3036State` 接口的 `ipdRawDataType` 字段后添加新字段：

```typescript
  ipdRawDataType: 'ipd' | 'rawdata';
  displayDurationSeconds: number;

  sampleRateConfig: Record<number, number>;
```

- [ ] **Step 2: 添加 setDisplayDurationSeconds 方法**

在 `setIpdRawDataType` 方法声明后添加：

```typescript
  setIpdRawDataType: (type: 'ipd' | 'rawdata') => void;
  setDisplayDurationSeconds: (seconds: number) => void;
  setSampleRateConfig: (config: Record<number, number>) => void;
```

- [ ] **Step 3: 初始化 displayDurationSeconds 默认值**

在 `create` 函数的初始状态中，`ipdRawDataType` 后添加：

```typescript
  ipdRawDataType: 'ipd',
  displayDurationSeconds: 10,

  sampleRateConfig: {
```

- [ ] **Step 4: 实现 setDisplayDurationSeconds 方法**

在 `setIpdRawDataType` 实现后添加：

```typescript
  setIpdRawDataType: (type) => set({ ipdRawDataType: type }),
  setDisplayDurationSeconds: (seconds) => set({ displayDurationSeconds: seconds }),

  setSampleRateConfig: (config) => set({ sampleRateConfig: config }),
```

- [ ] **Step 5: 持久化 displayDurationSeconds**

修改 `persist` 中间件的 `partialize` 函数，在 `ipdRawDataType` 后添加：

```typescript
  partialize: (state) => ({
    chartLegendSelected: state.chartLegendSelected ? { ...state.chartLegendSelected } : {},
    ipdRawDataType: state.ipdRawDataType || 'ipd',
    displayDurationSeconds: state.displayDurationSeconds || 10,
    sampleRateConfig: state.sampleRateConfig || { 0: 5, 1: 25, 2: 25, 3: 25, 4: 25 },
  }),
```

- [ ] **Step 6: 更新 merge 函数**

修改 `merge` 函数，添加 `displayDurationSeconds` 的合并逻辑：

```typescript
  merge: (persisted, current) => ({
    ...current,
    chartLegendSelected: (persisted as { chartLegendSelected?: Record<string, boolean> })?.chartLegendSelected || {},
    ipdRawDataType: (persisted as { ipdRawDataType?: 'ipd' | 'rawdata' })?.ipdRawDataType || 'ipd',
    displayDurationSeconds: (persisted as { displayDurationSeconds?: number })?.displayDurationSeconds || 10,
    sampleRateConfig: (persisted as { sampleRateConfig?: Record<number, number> })?.sampleRateConfig || { 0: 5, 1: 25, 2: 25, 3: 25, 4: 25 },
  }),
```

- [ ] **Step 7: 验证 TypeScript 类型**

运行: `npx tsc --noEmit`

预期: 无类型错误

- [ ] **Step 8: 提交更改**

```bash
git add src/stores/gh3036Store.ts
git commit -m "feat(gh3036): 添加显示秒数配置到 store"
```

---

### Task 2: 修改 monitorChartData.ts 移除显示时间限制

**Files:**
- Modify: `src/pages/Gh3036/monitorChartData.ts:10-56`

- [ ] **Step 1: 修改 buildIpdPaChartData 函数签名**

将函数签名的第四个参数从 `displayDurationSeconds` 改为 `enableLimit: boolean`：

```typescript
export function buildIpdPaChartData(
  currentFrames: Gh3036FramesPayload | null,
  sampleRate: number,
  ipdRawDataType: IpdRawDataType,
  enableLimit: boolean
): ChartTableData {
```

- [ ] **Step 2: 修改数据提取逻辑**

将原来的 `maxPoints` 计算和 `startIndex` 计算逻辑改为：

```typescript
  const columns = Array.from(
    { length: currentFrames.channel_count },
    (_, index) => `CH${index}`
  );
  const source = ipdRawDataType === 'ipd' ? currentFrames.ipd_pa : currentFrames.rawdata;

  if (!source || source.length === 0) {
    console.warn('[buildIpdPaChartData] 无有效数据源');
    return { columns, rows: [] };
  }

  const availablePoints = Math.min(
    currentFrames.frame_count,
    ...source.slice(0, currentFrames.channel_count).map((channel) => channel?.length ?? 0)
  );

  if (availablePoints === 0) {
    console.warn('[buildIpdPaChartData] 可用数据点为0');
    return { columns, rows: [] };
  }

  const startIndex = enableLimit ? 0 : 0;
  const endIndex = availablePoints;

  const rows: number[][] = [];
  for (let frameIdx = startIndex; frameIdx < endIndex; frameIdx++) {
    const row: number[] = [];
    for (let chIdx = 0; chIdx < currentFrames.channel_count; chIdx++) {
      const value = source[chIdx]?.[frameIdx];
      row.push(value !== undefined ? value : 0);
    }
    rows.push(row);
  }

  return { columns, rows };
```

- [ ] **Step 3: 验证 TypeScript 类型**

运行: `npx tsc --noEmit`

预期: MonitorTab.tsx 中 buildIpdPaChartData 调用报错（参数不匹配），这是预期的

- [ ] **Step 4: 提交更改**

```bash
git add src/pages/Gh3036/monitorChartData.ts
git commit -m "refactor(gh3036): 移除 buildIpdPaChartData 的显示时间限制"
```

---

### Task 3: 修改 MonitorTab.tsx 使用全部数据并添加配置 UI

**Files:**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:16-265`

- [ ] **Step 1: 移除硬编码常量**

删除第 16 行的 `DISPLAY_DURATION_SECONDS` 常量定义：

```typescript
// 删除这一行
const DISPLAY_DURATION_SECONDS = 6;
```

- [ ] **Step 2: 从 store 获取 displayDurationSeconds**

在 `useGh3036Store` 调用中添加 `displayDurationSeconds` 和 `setDisplayDurationSeconds`：

```typescript
  const {
    framesData,
    gsensorData,
    vitalSigns,
    selectedFunctionId,
    clearWaveformData,
    setSelectedFunctionId,
    ipdRawDataType,
    setIpdRawDataType,
    sampleRateConfig,
    setSampleRateConfig,
    displayDurationSeconds,
    setDisplayDurationSeconds,
  } = useGh3036Store();
```

- [ ] **Step 3: 修改 ipdPaChartData 的构建**

将 `buildIpdPaChartData` 调用改为传入 `false`（不限制）：

```typescript
  const ipdPaChartData = useMemo(() => {
    return buildIpdPaChartData(
      currentFrames,
      sampleRate,
      ipdRawDataType,
      false
    );
  }, [currentFrames, sampleRate, ipdRawDataType]);
```

- [ ] **Step 4: 修改 gsensorChartData 移除限制**

将 gsensorChartData 的 useMemo 改为：

```typescript
  const gsensorChartData = useMemo(() => {
    const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
    const rows: number[][] = [];

    const currentGsensor = selectedFunctionId ? gsensorData.get(selectedFunctionId) : null;
    if (!currentGsensor) {
      return { columns, rows };
    }

    const len = Math.min(
      currentGsensor.acc_x.length,
      currentGsensor.acc_y.length,
      currentGsensor.acc_z.length
    );

    for (let i = 0; i < len; i++) {
      rows.push([
        currentGsensor.acc_x[i],
        currentGsensor.acc_y[i],
        currentGsensor.acc_z[i],
      ]);
    }

    return { columns, rows };
  }, [gsensorData, selectedFunctionId]);
```

- [ ] **Step 5: 计算 dataZoom 默认显示范围**

在 `gsensorChartGroups` 定义后添加：

```typescript
  const ppgDataZoomState = useMemo(() => {
    if (!ipdPaChartData.rows.length) {
      return { start: 0, end: 100 };
    }

    const sampleRateValue = sampleRate;
    const totalPoints = ipdPaChartData.rows.length;
    const displayPoints = displayDurationSeconds * sampleRateValue;

    if (totalPoints <= displayPoints) {
      return { start: 0, end: 100 };
    }

    const endPercent = (displayPoints / totalPoints) * 100;
    return { start: Math.max(0, 100 - endPercent), end: 100 };
  }, [ipdPaChartData.rows.length, sampleRate, displayDurationSeconds]);

  const accDataZoomState = useMemo(() => {
    if (!gsensorChartData.rows.length) {
      return { start: 0, end: 100 };
    }

    const accSampleRate = 25;
    const totalPoints = gsensorChartData.rows.length;
    const displayPoints = displayDurationSeconds * accSampleRate;

    if (totalPoints <= displayPoints) {
      return { start: 0, end: 100 };
    }

    const endPercent = (displayPoints / totalPoints) * 100;
    return { start: Math.max(0, 100 - endPercent), end: 100 };
  }, [gsensorChartData.rows.length, displayDurationSeconds]);
```

- [ ] **Step 6: 添加显示秒数配置 UI**

在 IPD/PA 图表的 Card 的 extra 部分，`<Tooltip title={t('monitor.sampleRateHint')}>` 前添加：

```typescript
            <Tooltip title={t('monitor.displayDurationHint')}>
              <Space size={4}>
                <ClockCircleOutlined style={{ color: 'var(--text-secondary)' }} />
                <InputNumber
                  size="small"
                  min={1}
                  max={60}
                  value={displayDurationSeconds}
                  onChange={(value) => setDisplayDurationSeconds(value ?? 10)}
                  style={{ width: 60 }}
                  addonAfter={t('monitor.seconds')}
                />
              </Space>
            </Tooltip>
```

- [ ] **Step 7: 添加缺失的 import**

在文件顶部的 import 区域添加 `ClockCircleOutlined`：

```typescript
import { ClearOutlined, HeartOutlined, ThunderboltOutlined, SettingOutlined, ClockCircleOutlined } from '@ant-design/icons';
```

- [ ] **Step 8: 传递 dataZoomState 到 MultiLineChart**

修改 PPG 图表的 `MultiLineChart` 调用，添加 `initialDataZoom` 属性：

```typescript
        {ipdPaChartData.columns.length > 0 && ipdPaChartData.rows.length > 0 ? (
          <MultiLineChart
            columns={ipdPaChartData.columns}
            rows={ipdPaChartData.rows}
            chartGroups={ipdPaChartGroups}
            sampleRate={sampleRate}
            initialDataZoom={ppgDataZoomState}
          />
        ) : (
          <Empty description={t('monitor.noData')} style={{ marginTop: 80 }} />
        )}
```

- [ ] **Step 9: 传递 dataZoomState 到 ACC 图表**

修改 ACC 图表的 `MultiLineChart` 调用：

```typescript
        {gsensorChartData.rows.length > 0 ? (
          <MultiLineChart
            columns={gsensorChartData.columns}
            rows={gsensorChartData.rows}
            chartGroups={gsensorChartGroups}
            sampleRate={DEFAULT_SAMPLE_RATE}
            initialDataZoom={accDataZoomState}
          />
        ) : (
          <Empty description={t('monitor.noGsensorData')} style={{ marginTop: 60 }} />
        )}
```

- [ ] **Step 10: 验证 TypeScript 类型**

运行: `npx tsc --noEmit`

预期: MultiLineChart 报错缺少 `initialDataZoom` 属性定义，这是预期的

- [ ] **Step 11: 提交更改**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "feat(gh3036): 修改监控页显示全部数据并添加显示秒数配置"
```

---

### Task 4: 修改 MultiLineChart 支持初始 dataZoom 状态

**Files:**
- Modify: `src/pages/Waveform/MultiLineChart.tsx:38-43, 214-254`

- [ ] **Step 1: 添加 initialDataZoom 属性到接口**

在 `MultiLineChartProps` 接口添加新属性：

```typescript
interface MultiLineChartProps {
  columns: string[];
  rows: number[][];
  chartGroups: ChartGroupConfig[];
  sampleRate?: number;
  initialDataZoom?: { start: number; end: number };
}
```

- [ ] **Step 2: 解构 initialDataZoom 属性**

在组件参数中解构：

```typescript
const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  sampleRate = 25,
  initialDataZoom,
}) => {
```

- [ ] **Step 3: 修改 dataZoom 配置使用 initialDataZoom**

在 `getChartOption` 函数的 `dataZoomOption` 中，将硬编码的 `dataZoomState` 改为优先使用 `initialDataZoom`：

```typescript
    const effectiveDataZoom = initialDataZoom || dataZoomState;

    const dataZoomOption = [
      {
        type: 'slider' as const,
        show: true,
        start: effectiveDataZoom.start,
        end: effectiveDataZoom.end,
        zoomLock: false,
        xAxisIndex: [0],
        height: 24,
        bottom: 8,
        handleStyle: {
          color: '#1890ff',
          borderColor: '#1890ff',
        },
        trackStyle: {
          backgroundColor: 'var(--bg-secondary)',
        },
        selectedDataBackground: {
          lineStyle: {
            color: '#1890ff',
          },
          areaStyle: {
            color: 'rgba(24, 144, 255, 0.2)',
          },
        },
        fillerColor: 'rgba(24, 144, 255, 0.15)',
        borderColor: 'var(--border-color)',
        textStyle: {
          color: 'var(--text-secondary)',
        },
        labelFormatter: (value: number) => {
          if (rows.length === 0) return '0ms';
          const interval = 1 / sampleRate;
          const totalPoints = rows.length;
          const index = Math.round(value * (totalPoints - 1) / 100);
          const clampedIndex = Math.max(0, Math.min(index, totalPoints - 1));
          const timeValue = clampedIndex * interval;
          return formatTime(timeValue);
        },
      },
    ];
```

- [ ] **Step 4: 更新 getChartOption 的依赖**

修改 `useCallback` 的依赖数组，添加 `initialDataZoom`：

```typescript
  }, [xAxisData, rows, columns, unifiedGridConfig, dataZoomState, initialDataZoom]);
```

- [ ] **Step 5: 验证 TypeScript 类型**

运行: `npx tsc --noEmit`

预期: 无类型错误

- [ ] **Step 6: 提交更改**

```bash
git add src/pages/Waveform/MultiLineChart.tsx
git commit -m "feat(waveform): MultiLineChart 支持初始 dataZoom 状态"
```

---

### Task 5: 添加国际化翻译

**Files:**
- Modify: `src/locales/zh-CN/gh3036.json:58`
- Modify: `src/locales/en-US/gh3036.json:58`

- [ ] **Step 1: 添加中文翻译**

在 `monitor` 部分的 `sampleRateHint` 后添加：

```json
    "sampleRateHint": "设置当前功能的数据采样频率，用于计算正确的时间尺度",
    "displayDurationHint": "设置图表默认显示的时间长度（秒）",
    "seconds": "秒",
    "configRef": "配置金标",
```

- [ ] **Step 2: 添加英文翻译**

在 `src/locales/en-US/gh3036.json` 的 `monitor` 部分对应位置添加：

```json
    "sampleRateHint": "Set the sampling frequency for the current function to calculate the correct time scale",
    "displayDurationHint": "Set the default display duration in seconds for charts",
    "seconds": "s",
    "configRef": "Configure Reference",
```

- [ ] **Step 3: 提交更改**

```bash
git add src/locales/zh-CN/gh3036.json src/locales/en-US/gh3036.json
git commit -m "feat(gh3036): 添加显示秒数配置的国际化翻译"
```

---

### Task 6: 实现图表 x 轴联动机制

**Files:**
- Modify: `src/pages/Gh3036/MonitorTab.tsx:146-168`

- [ ] **Step 1: 在 gh3036Store 添加共享的 dataZoom 状态**

修改 `src/stores/gh3036Store.ts`，在状态定义部分添加：

```typescript
  sharedDataZoomState: { start: number; end: number };
```

在方法声明部分添加：

```typescript
  setSharedDataZoomState: (state: { start: number; end: number }) => void;
```

在初始状态中添加：

```typescript
  sharedDataZoomState: { start: 0, end: 100 },
```

在方法实现部分添加：

```typescript
  setSharedDataZoomState: (state) => set({ sharedDataZoomState: state }),
```

- [ ] **Step 2: 在 MonitorTab 中使用共享 dataZoom 状态**

在 `useGh3036Store` 调用中添加：

```typescript
    displayDurationSeconds,
    setDisplayDurationSeconds,
    sharedDataZoomState,
    setSharedDataZoomState,
  } = useGh3036Store();
```

- [ ] **Step 3: 删除独立的 dataZoomState 计算**

删除之前添加的 `ppgDataZoomState` 和 `accDataZoomState` useMemo，改为：

```typescript
  const initialDataZoomState = useMemo(() => {
    if (!ipdPaChartData.rows.length) {
      return { start: 0, end: 100 };
    }

    const sampleRateValue = sampleRate;
    const totalPoints = ipdPaChartData.rows.length;
    const displayPoints = displayDurationSeconds * sampleRateValue;

    if (totalPoints <= displayPoints) {
      return { start: 0, end: 100 };
    }

    const endPercent = (displayPoints / totalPoints) * 100;
    return { start: Math.max(0, 100 - endPercent), end: 100 };
  }, [ipdPaChartData.rows.length, sampleRate, displayDurationSeconds]);
```

- [ ] **Step 4: 修改 MultiLineChart 调用使用共享状态**

修改 PPG 和 ACC 图表的 `MultiLineChart` 调用，传递共享的 dataZoom 状态：

PPG 图表：

```typescript
          <MultiLineChart
            columns={ipdPaChartData.columns}
            rows={ipdPaChartData.rows}
            chartGroups={ipdPaChartGroups}
            sampleRate={sampleRate}
            initialDataZoom={sharedDataZoomState}
            onDataZoomChange={setSharedDataZoomState}
          />
```

ACC 图表：

```typescript
          <MultiLineChart
            columns={gsensorChartData.columns}
            rows={gsensorChartData.rows}
            chartGroups={gsensorChartGroups}
            sampleRate={DEFAULT_SAMPLE_RATE}
            initialDataZoom={sharedDataZoomState}
            onDataZoomChange={setSharedDataZoomState}
          />
```

- [ ] **Step 5: 在 MultiLineChart 中添加 onDataZoomChange 回调**

修改 `src/pages/Waveform/MultiLineChart.tsx`：

添加属性到接口：

```typescript
interface MultiLineChartProps {
  columns: string[];
  rows: number[][];
  chartGroups: ChartGroupConfig[];
  sampleRate?: number;
  initialDataZoom?: { start: number; end: number };
  onDataZoomChange?: (state: { start: number; end: number }) => void;
}
```

解构属性：

```typescript
const MultiLineChart: React.FC<MultiLineChartProps> = ({
  columns,
  rows,
  chartGroups,
  sampleRate = 25,
  initialDataZoom,
  onDataZoomChange,
}) => {
```

修改 dataZoom 事件处理器，调用回调：

```typescript
    const handleDataZoom = (chartIndex: number) => (params: unknown) => {
      if (isZoomingRef.current) return;
      isZoomingRef.current = true;

      const p = params as { start?: number; end?: number; batch?: Array<{ start: number; end: number }> };
      let start: number, end: number;

      if (p.batch) {
        start = p.batch[0].start;
        end = p.batch[0].end;
      } else if (p.start !== undefined && p.end !== undefined) {
        start = p.start;
        end = p.end;
      } else {
        isZoomingRef.current = false;
        return;
      }

      setDataZoomState({ start, end });

      if (onDataZoomChange) {
        onDataZoomChange({ start, end });
      }

      chartInstances.current.forEach((chart, idx) => {
        if (chart && idx !== chartIndex) {
          chart.dispatchAction({
            type: 'dataZoom',
            start,
            end,
          });
        }
      });

      setTimeout(() => {
        isZoomingRef.current = false;
      }, 50);
    };
```

更新依赖数组：

```typescript
  }, [initialized, setDataZoomState, onDataZoomChange]);
```

- [ ] **Step 6: 初始化时设置共享 dataZoom 状态**

在 MonitorTab 中添加 useEffect，在数据变化时初始化共享状态：

```typescript
  useEffect(() => {
    if (ipdPaChartData.rows.length > 0 && sharedDataZoomState.start === 0 && sharedDataZoomState.end === 100) {
      setSharedDataZoomState(initialDataZoomState);
    }
  }, [ipdPaChartData.rows.length, sharedDataZoomState, initialDataZoomState, setSharedDataZoomState]);
```

- [ ] **Step 7: 验证 TypeScript 类型**

运行: `npx tsc --noEmit`

预期: 无类型错误

- [ ] **Step 8: 提交更改**

```bash
git add src/stores/gh3036Store.ts src/pages/Gh3036/MonitorTab.tsx src/pages/Waveform/MultiLineChart.tsx
git commit -m "feat(gh3036): 实现 PPG 和 ACC 图表 x 轴联动"
```

---

### Task 7: 测试和验证

**Files:**
- Test: `src/stores/gh3036FrameBuffer.test.ts`

- [ ] **Step 1: 运行 TypeScript 类型检查**

运行: `npx tsc --noEmit`

预期: 无类型错误

- [ ] **Step 2: 运行 Rust 后端测试**

运行: `cd src-tauri && cargo test`

预期: 所有测试通过

- [ ] **Step 3: 运行前端测试**

运行: `npm test`

预期: 所有测试通过

- [ ] **Step 4: 启动开发服务器进行手动测试**

运行: `npm run tauri dev`

手动测试步骤：
1. 打开 GH3036 数据监控页面
2. 连接设备并开始接收数据
3. 验证波形显示全部历史数据
4. 验证 PPG 和 ACC 图表 x 轴同步滚动
5. 修改显示秒数配置，验证默认显示范围变化
6. 刷新页面，验证配置持久化

- [ ] **Step 5: 提交最终验证**

```bash
git add -A
git commit -m "test(gh3036): 验证显示优化功能"
```

---

## 自我审查检查清单

### 1. 规格覆盖检查

- [x] **显示整个区域的数据**: Task 2 移除了硬编码限制，Task 3 修改为显示全部数据
- [x] **x 轴联动**: Task 6 实现了 PPG 和 ACC 的共享 dataZoom 状态
- [x] **支持设置最大显示秒数**: Task 1 添加了配置，Task 3 添加了 UI

### 2. 占位符扫描

- [x] 无 TBD、TODO、"implement later" 等占位符
- [x] 所有代码步骤都包含完整实现代码
- [x] 无"添加适当错误处理"等模糊描述
- [x] 无"类似 Task N"的引用，所有步骤都有完整代码
- [x] 无未定义的类型、函数或方法引用

### 3. 类型一致性检查

- [x] `displayDurationSeconds: number` 在整个计划中类型一致
- [x] `initialDataZoom: { start: number; end: number }` 在 MultiLineChart 和 MonitorTab 中类型一致
- [x] `sharedDataZoomState: { start: number; end: number }` 在 store 和组件中类型一致
- [x] `setDisplayDurationSeconds: (seconds: number) => void` 方法签名一致
- [x] `setSharedDataZoomState: (state: { start: number; end: number }) => void` 方法签名一致

---

## 执行方式选择

**计划完成并保存到 `docs/superpowers/plans/2026-07-30-gh3036-display-optimization.md`。两种执行选项：**

**1. Subagent-Driven (推荐)** - 我为每个任务派遣一个全新的子代理，在任务之间进行审查，快速迭代

**2. Inline Execution** - 在此会话中使用 executing-plans 执行，批量执行并在检查点进行审查

**选择哪种方式?**