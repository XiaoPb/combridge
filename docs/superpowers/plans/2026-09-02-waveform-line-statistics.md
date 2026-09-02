# 波形图表线统计信息 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 CSV 波形图表中新增统计显示开关，并在每个图表上方按曲线显示当前可视区域的 max/min/avg/diff。

**Architecture:** `CsvLoaderTab` 从 `csvChartStore` 读取并修改 `showLineStatistics`，将其传递给 `MultiLineChart`。`MultiLineChart` 使用自己的 `localDataZoom` 和完整数据调用独立的纯统计模块，统计模块按现有图表分组及每图最多 4 条线的规则返回结果；统计 DOM 放在 ECharts 容器外，因此不会进入现有 PNG/SVG 导出流程。

**Tech Stack:** React 19、TypeScript、Zustand、Ant Design、ECharts、Vitest、i18next。

---

## 文件变更总览

- Create: `src/pages/Waveform/multiLineChartStats.ts` — 可视区域索引转换和每条曲线统计的纯函数。
- Create: `src/pages/Waveform/multiLineChartStats.test.ts` — 统计函数单元测试。
- Modify: `src/stores/csvChartStore.ts` — 增加统计显示状态及 action。
- Modify: `src/stores/csvChartStore.test.ts` — 验证默认值和 action 状态变更。
- Modify: `src/pages/Waveform/CsvLoaderTab.tsx` — 增加控制项并向图表传递 prop。
- Modify: `src/pages/Waveform/MultiLineChart.tsx` — 计算并渲染统计行，处理图表高度和空值展示。
- Modify: `src/pages/Waveform/MultiLineChart.test.ts` — 验证新增 prop 的类型/行为相关辅助逻辑；若测试环境不能挂载 ECharts，则保持组件测试为静态接口覆盖，主要行为由纯函数测试覆盖。
- Modify: `src/locales/zh-CN/waveform.json` — 增加中文控件及统计标签文案。
- Modify: `src/locales/en-US/waveform.json` — 增加英文控件及统计标签文案。

### Task 1: 为统计计算定义纯函数接口并先写失败测试

**Files:**
- Create: `src/pages/Waveform/multiLineChartStats.ts`
- Create: `src/pages/Waveform/multiLineChartStats.test.ts`

- [ ] **Step 1: 定义测试数据和期望行为**

在测试文件中添加以下测试，先从 `multiLineChartStats.ts` 导入尚未实现的 `getVisibleRowRange` 和 `calculateVisibleLineStats`：

```ts
import { describe, expect, it } from 'vitest';
import {
  calculateVisibleLineStats,
  getVisibleRowRange,
} from './multiLineChartStats';

describe('getVisibleRowRange', () => {
  it('converts percentages to an inclusive row range', () => {
    expect(getVisibleRowRange(5, { start: 25, end: 75 })).toEqual({
      startIndex: 1,
      endIndex: 3,
    });
  });

  it('clamps invalid percentages and uses all rows when zoom is invalid', () => {
    expect(getVisibleRowRange(3, { start: -10, end: 150 })).toEqual({
      startIndex: 0,
      endIndex: 2,
    });
    expect(getVisibleRowRange(3, { start: Number.NaN, end: 50 })).toEqual({
      startIndex: 0,
      endIndex: 2,
    });
  });

  it('returns null for empty rows', () => {
    expect(getVisibleRowRange(0, { start: 0, end: 100 })).toBeNull();
  });
});

describe('calculateVisibleLineStats', () => {
  const columns = ['A', 'B', 'C', 'D', 'E'];
  const rows = [
    [1, 10, Number.NaN, 100, 5],
    [3, 20, 4, 80, 7],
    [5, Number.POSITIVE_INFINITY, 8, 60, 9],
    [7, 40, 12, 40, 11],
  ];

  it('calculates max, min, average, and diff only in the visible range', () => {
    expect(calculateVisibleLineStats(columns, rows, ['A', 'B'], { start: 25, end: 75 }))
      .toEqual([
        { name: 'A', color: '#165DFF', max: 5, min: 3, avg: 4, diff: 2 },
        { name: 'B', color: '#F53F3F', max: 40, min: 20, avg: 30, diff: 20 },
      ]);
  });

  it('filters non-finite values and limits each chart to four unique lines', () => {
    const result = calculateVisibleLineStats(
      columns,
      rows,
      ['A', 'B', 'C', 'D', 'E', 'A'],
      { start: 0, end: 100 },
    );
    expect(result).toHaveLength(4);
    expect(result[1]).toMatchObject({ name: 'B', max: 40, min: 10, avg: 70 / 3, diff: 30 });
    expect(result.map((item) => item.name)).toEqual(['A', 'B', 'C', 'D']);
  });

  it('returns null values when a line has no finite values', () => {
    expect(calculateVisibleLineStats(['A'], [[Number.NaN], [Number.POSITIVE_INFINITY]], ['A'], { start: 0, end: 100 }))
      .toEqual([{ name: 'A', color: '#165DFF', max: null, min: null, avg: null, diff: null }]);
  });
});
```

- [ ] **Step 2: Run the focused test and verify it fails**

Run:

```text
npm test -- --run src/pages/Waveform/multiLineChartStats.test.ts
```

Expected: FAIL because `multiLineChartStats.ts` and its exported functions do not exist yet.

- [ ] **Step 3: Implement the minimal statistics module**

Create the following interfaces and functions in `src/pages/Waveform/multiLineChartStats.ts`:

```ts
import { LINE_COLORS } from './multiLineChartModel';

export interface DataZoomPercent {
  start: number;
  end: number;
}

export interface VisibleRowRange {
  startIndex: number;
  endIndex: number;
}

export interface LineStatistics {
  name: string;
  color: string;
  max: number | null;
  min: number | null;
  avg: number | null;
  diff: number | null;
}

export function getVisibleRowRange(
  rowCount: number,
  zoom: DataZoomPercent,
): VisibleRowRange | null {
  if (!Number.isInteger(rowCount) || rowCount <= 0) return null;
  if (!Number.isFinite(zoom.start) || !Number.isFinite(zoom.end)) {
    return { startIndex: 0, endIndex: rowCount - 1 };
  }
  const startPercent = Math.max(0, Math.min(100, Math.min(zoom.start, zoom.end)));
  const endPercent = Math.max(0, Math.min(100, Math.max(zoom.start, zoom.end)));
  return {
    startIndex: Math.floor((startPercent * (rowCount - 1)) / 100),
    endIndex: Math.ceil((endPercent * (rowCount - 1)) / 100),
  };
}

export function calculateVisibleLineStats(
  columns: string[],
  rows: number[][],
  groupColumns: string[],
  zoom: DataZoomPercent,
  maxLines = 4,
): LineStatistics[] {
  const range = getVisibleRowRange(rows.length, zoom);
  if (!range) return [];
  const selectedColumns = groupColumns
    .slice(0, maxLines)
    .filter((column, index, selected) => selected.indexOf(column) === index);

  return selectedColumns.map((name, lineIndex) => {
    const columnIndex = columns.indexOf(name);
    const values = rows
      .slice(range.startIndex, range.endIndex + 1)
      .map((row) => row[columnIndex])
      .filter((value): value is number => Number.isFinite(value));
    if (values.length === 0) {
      return { name, color: LINE_COLORS[lineIndex % LINE_COLORS.length], max: null, min: null, avg: null, diff: null };
    }
    const max = Math.max(...values);
    const min = Math.min(...values);
    const avg = values.reduce((sum, value) => sum + value, 0) / values.length;
    return { name, color: LINE_COLORS[lineIndex % LINE_COLORS.length], max, min, avg, diff: max - min };
  });
}
```

- [ ] **Step 4: Run the focused test and verify it passes**

Run the same command from Step 2. Expected: PASS for all statistics tests.

- [ ] **Step 5: Commit the statistics module**

```text
git add src/pages/Waveform/multiLineChartStats.ts src/pages/Waveform/multiLineChartStats.test.ts
git commit -m "feat(waveform): 增加可视区域线统计计算"
```

### Task 2: 增加 CSV 图表统计开关状态

**Files:**
- Modify: `src/stores/csvChartStore.ts:8-36,55-104`
- Modify: `src/stores/csvChartStore.test.ts`

- [ ] **Step 1: Add a failing store test**

在现有 store 测试中增加断言：创建/重置 store 后 `showLineStatistics` 为 `false`，调用 `setShowLineStatistics(true)` 后为 `true`，调用 `clearData()` 后恢复 `false`。

```ts
it('stores the CSV line statistics visibility preference', () => {
  const store = useCsvChartStore.getState();
  store.clearData();
  expect(useCsvChartStore.getState().showLineStatistics).toBe(false);
  store.setShowLineStatistics(true);
  expect(useCsvChartStore.getState().showLineStatistics).toBe(true);
  store.clearData();
  expect(useCsvChartStore.getState().showLineStatistics).toBe(false);
});
```

- [ ] **Step 2: Run the store test and verify it fails**

```text
npm test -- --run src/stores/csvChartStore.test.ts
```

Expected: FAIL because the state field and action are not defined.

- [ ] **Step 3: Implement the store field and action**

Extend `CsvChartState` with `showLineStatistics: boolean`, extend `CsvChartActions` with `setShowLineStatistics: (show: boolean) => void`, initialize it to `false`, implement `setShowLineStatistics: (show) => set({ showLineStatistics: show })`, and include `showLineStatistics: false` in the `clearData()` state reset. Do not change CSV parsing or zoom behavior.

- [ ] **Step 4: Run the store test and the full existing store suite**

```text
npm test -- --run src/stores/csvChartStore.test.ts
```

Expected: PASS, including the new preference test and all existing store tests.

- [ ] **Step 5: Commit the store change**

```text
git add src/stores/csvChartStore.ts src/stores/csvChartStore.test.ts
git commit -m "feat(waveform): 保存线统计显示开关"
```

### Task 3: 接入配置区控件和国际化文案

**Files:**
- Modify: `src/pages/Waveform/CsvLoaderTab.tsx:18-35,165-204,275-286`
- Modify: `src/locales/zh-CN/waveform.json`
- Modify: `src/locales/en-US/waveform.json`
- Modify: `src/pages/Waveform/CsvLoaderTab.test.tsx` — 验证新增开关文案、默认关闭状态和切换行为。

- [ ] **Step 1: Add a failing control test**

在现有 `CsvLoaderTab` 测试中加入以下测试，确认中文界面同时渲染“无表头”和“显示线统计”，且新增开关默认为关闭并能切换为开启：

```tsx
it('renders the line statistics switch after the no-header option', () => {
  setCsvData();
  render(<CsvLoaderTab />);

  expect(screen.getByText('无表头')).toBeTruthy();
  expect(screen.getByText('显示线统计')).toBeTruthy();
  const switches = screen.getAllByRole('switch');
  expect(switches.at(-1)).toHaveAttribute('aria-checked', 'false');
  fireEvent.click(switches.at(-1)!);
  expect(useCsvChartStore.getState().showLineStatistics).toBe(true);
});
```

运行 `npm test -- --run src/pages/Waveform/CsvLoaderTab.test.tsx`，预期因 store 字段、locale key 和控件尚未实现而失败。

- [ ] **Step 2: Implement state selection and the new control**

从 `useCsvChartStore()` 解构 `showLineStatistics` 和 `setShowLineStatistics`。在现有 `noHeader` 的 `Space` 后增加同级控件，保持当前配置区的 `Text + Switch` 样式：

```tsx
<Space>
  <Text>{t('csvLoader.showLineStatistics')}</Text>
  <Switch
    checked={showLineStatistics}
    onChange={setShowLineStatistics}
    size="small"
  />
</Space>
```

在 `MultiLineChart` 调用处增加 `showLineStatistics={showLineStatistics}`。控件不应触发重新解析 CSV。

- [ ] **Step 3: Add both locale entries**

在两个 `waveform.json` 的 `csvLoader` 节点加入：

```json
"showLineStatistics": "显示线统计"
```

英文文件使用：

```json
"showLineStatistics": "Show line statistics"
```

统计标签使用固定 key `chart.max`、`chart.min`、`chart.avg`、`chart.diff`；若这些 key 不存在，则在同一 `chart` 节点补充中英文值 `max`、`min`、`avg`、`diff`。

- [ ] **Step 4: Run CSV loader tests and type check**

```text
npm test -- --run src/pages/Waveform/CsvLoaderTab.test.tsx
npx tsc --noEmit
```

Expected: PASS with no TypeScript errors.

- [ ] **Step 5: Commit the control and locale change**

```text
git add src/pages/Waveform/CsvLoaderTab.tsx src/pages/Waveform/CsvLoaderTab.test.tsx src/locales/zh-CN/waveform.json src/locales/en-US/waveform.json
git commit -m "feat(waveform): 增加线统计显示开关"
```

### Task 4: 在 MultiLineChart 中渲染当前可视区域统计

**Files:**
- Modify: `src/pages/Waveform/MultiLineChart.tsx:1-72,203-434,638-792`
- Modify: `src/pages/Waveform/MultiLineChart.test.ts`

- [ ] **Step 1: Extend the chart props and add a rendering helper test**

在 `MultiLineChartProps` 增加 `showLineStatistics?: boolean`，默认值为 `false`。在测试中增加对 `calculateVisibleLineStats` 的集成输入断言，确认 chart props 采用 `{ start: 0, end: 100 }` 时展示完整数据，采用 `{ start: 50, end: 50 }` 时只展示中间数据点。纯函数结果由 Task 1 覆盖，组件测试不依赖真实 ECharts 实例。

- [ ] **Step 2: Calculate statistics from local zoom state**

引入 `calculateVisibleLineStats` 和 `LineStatistics`。在组件参数中解构 `showLineStatistics = false`。增加 memoized 统计结果：

```tsx
const visibleLineStats = useMemo(
  () => chartGroups.map((group) => calculateVisibleLineStats(
    columns,
    rows,
    group.columns,
    localDataZoom,
  )),
  [chartGroups, columns, rows, localDataZoom],
);
```

统计计算不得使用 `effectiveLegendSelected`，因为需求是按当前图表每条配置曲线展示统计；图例隐藏只影响曲线显示，不改变当前分组配置和统计行数量。

- [ ] **Step 3: Add the statistics row renderer**

在 `MultiLineChart.tsx` 内增加 `formatLineStatistic` 和 `LineStatisticsPanel`，使用已有 `formatActualValue` 格式化非空数值，空值返回 `—`。每个图表的外层容器按以下顺序渲染：统计面板（仅 `showLineStatistics` 且有曲线统计结果时）→ ECharts 容器。统计面板的每个 `LineStatistics` 渲染一行；没有有效值的曲线仍渲染该行并显示四个 `—`，使用 `display: grid` 固定名称列和四个统计值列，曲线色标/名称使用 `stat.color`。

```tsx
function formatLineStatistic(value: number | null): string {
  return value === null ? '—' : formatActualValue(value);
}

function LineStatisticsPanel({ stats }: { stats: LineStatistics[] }) {
  if (stats.length === 0) return null;
  return (
    <div style={{ padding: '4px 8px 6px', flexShrink: 0 }}>
      {stats.map((stat) => (
        <div key={stat.name} style={{ display: 'grid', gridTemplateColumns: 'minmax(100px, 1.2fr) repeat(4, minmax(90px, 1fr))', gap: 8, lineHeight: '20px', fontSize: 12 }}>
          <span style={{ color: stat.color }}>■ {stat.name}</span>
          <span>max: {formatLineStatistic(stat.max)}</span>
          <span>min: {formatLineStatistic(stat.min)}</span>
          <span>avg: {formatLineStatistic(stat.avg)}</span>
          <span>diff: {formatLineStatistic(stat.diff)}</span>
        </div>
      ))}
    </div>
  );
}
```

使用图表分组索引取 `visibleLineStats[index]`，不要把统计 panel 放进 `containerRefs` 对应的 ECharts DOM；这样现有实例初始化、resize、右键菜单和导出逻辑保持不变，PNG/SVG 不会包含统计信息。

- [ ] **Step 4: Verify layout and update behavior**

检查 `MultiLineChart` 的每个图表外层容器具备 `display: flex; flex-direction: column; min-height: 0`，ECharts 容器使用 `flex: 1; min-height: 0`。确认 `localDataZoom` 变化会触发 `visibleLineStats` 重新计算，且共享 dataZoom 事件仍由现有 `handleDataZoomEvent` 同步到所有图表。

- [ ] **Step 5: Run focused chart tests and type check**

```text
npm test -- --run src/pages/Waveform/multiLineChartStats.test.ts src/pages/Waveform/MultiLineChart.test.ts
npx tsc --noEmit
```

Expected: PASS and no TypeScript errors.

- [ ] **Step 6: Commit the chart rendering change**

```text
git add src/pages/Waveform/MultiLineChart.tsx src/pages/Waveform/MultiLineChart.test.ts
git commit -m "feat(waveform): 显示当前区域线统计"
```

### Task 5: 完成回归验证和手工验收

**Files:**
- Verify: `src/pages/Waveform/CsvLoaderTab.tsx`
- Verify: `src/pages/Waveform/MultiLineChart.tsx`
- Verify: `src/stores/csvChartStore.ts`
- Verify: `src/locales/zh-CN/waveform.json`
- Verify: `src/locales/en-US/waveform.json`

- [ ] **Step 1: Run the complete frontend test suite**

```text
npm test -- --run
```

Expected: all Vitest tests PASS.

- [ ] **Step 2: Run the production type/build checks**

```text
npx tsc --noEmit
npm run build
```

Expected: TypeScript exits with code 0 and Vite produces a successful production build.

- [ ] **Step 3: Manually verify the CSV chart page**

启动应用并打开 CSV 波形页，逐项确认：

1. “显示线统计”紧跟在“无表头”控件后方，默认关闭。
2. 开启后，每个图表上方每条曲线占一行，并显示 max/min/avg/diff。
3. 拖动或缩放任意图表的 dataZoom，所有图表的统计值随同一可视区域更新。
4. 缩放到单个点时四项值相同，`diff` 为 0。
5. 当前区域只有非法值时显示 `—`，页面不报错。
6. 增删图表分组、修改曲线选择后统计行与当前配置一致。
7. 关闭统计后统计行消失，图表恢复原有布局。
8. 导出 PNG/SVG，确认输出中不包含统计文本。

- [ ] **Step 4: Review the final diff for scope**

```text
git diff HEAD~4 --stat
git status --short
```

确认只包含本功能相关文件；保留用户已有的 `.gitignore` 修改和未跟踪的 `.superpowers/` 内容，不将它们加入本功能提交。
