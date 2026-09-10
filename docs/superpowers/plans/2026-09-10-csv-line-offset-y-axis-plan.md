# CSV 折线采样点偏移与 Y 轴模式 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 CSV 加载页面支持按列配置采样点偏移，并让每个图表组独立切换共用 Y 轴或每条折线独立 Y 轴，同时保证图表、统计和导出使用一致的显示数据。

**Architecture:** 保持 `csvData.rows` 不可变，在 `multiLineChartModel.ts` 增加固定长度的纯函数偏移变换，并由 `buildChartSeries` 和线统计共同调用。将 `yAxisMode` 存在图表组配置中，将 `lineOffsets` 存在 CSV Zustand store 中；`CsvLoaderTab` 只编辑这些配置，`MultiLineChart` 根据配置生成 ECharts option。

**Tech Stack:** React 19, TypeScript, Zustand, Ant Design, ECharts 6, Vitest, Testing Library。

---

## 文件地图

| 文件 | 责任 | 计划改动 |
| --- | --- | --- |
| `src/pages/Waveform/chartGroup.ts` | 图表组类型和默认值 | 增加 `YAxisMode` 和默认 `shared` |
| `src/pages/Waveform/multiLineChartModel.ts` | 折线序列纯函数 | 增加偏移规范化、固定长度偏移、可选偏移配置 |
| `src/pages/Waveform/multiLineChartStats.ts` | 可视区域统计 | 接收偏移配置并统计偏移后的值 |
| `src/stores/csvChartStore.ts` | CSV 页面状态生命周期 | 增加 `lineOffsets`、setter、加载过滤和清空重置 |
| `src/pages/Waveform/CsvLoaderTab.tsx` | CSV 配置交互 | 增加 Y 轴模式和每列偏移控件 |
| `src/pages/Waveform/MultiLineChart.tsx` | ECharts option 和渲染 | 接收偏移配置，按组生成 shared/per-line Y 轴 |
| `src/locales/zh-CN/waveform.json` | 中文文案 | 增加模式、偏移标签 |
| `src/locales/en-US/waveform.json` | 英文文案 | 增加对应英文文案 |
| `src/pages/Waveform/multiLineChartModel.test.ts` | 模型单测 | 覆盖偏移边界和原始数据不可变 |
| `src/pages/Waveform/chartGroup.test.ts` | 图表组单测 | 覆盖默认和旧对象兼容 |
| `src/stores/csvChartStore.test.ts` | store 单测 | 覆盖偏移 setter、加载过滤、清空 |
| `src/pages/Waveform/multiLineChartStats.test.ts` | 统计单测 | 覆盖统计使用偏移后的数据 |
| `src/pages/Waveform/CsvLoaderTab.test.tsx` | 页面测试 | 覆盖控件渲染和交互绑定 |
| `src/pages/Waveform/MultiLineChart.test.ts` | 图表测试 | 覆盖 shared/per-line option 结构 |

## Task 1: 添加图表组 Y 轴模式类型

**Files:**
- Modify: `src/pages/Waveform/chartGroup.ts`
- Test: `src/pages/Waveform/chartGroup.test.ts`

- [ ] **Step 1: Write the failing tests**

在 `chartGroup.test.ts` 增加：

```ts
it('defaults new chart groups to a shared Y axis', () => {
  expect(createChartGroup('图表1').yAxisMode).toBe('shared');
});

it('keeps legacy chart group objects valid without a mode', () => {
  const legacy: ChartGroupConfig = { name: '旧图表', columns: ['A'] };
  expect(legacy.yAxisMode).toBeUndefined();
});
```

运行：`npm test -- src/pages/Waveform/chartGroup.test.ts`

预期：新测试失败，因为类型和默认值尚不存在。

- [ ] **Step 2: Implement the type and default**

在 `chartGroup.ts` 导出：

```ts
export type YAxisMode = 'shared' | 'per-line';
```

在 `ChartGroupConfig` 增加 `yAxisMode?: YAxisMode`，在 `createChartGroup` 返回对象中增加 `yAxisMode: 'shared'`。调用方读取时使用 `group.yAxisMode ?? 'shared'`，保持旧对象兼容。

- [ ] **Step 3: Run the focused test**

运行：`npm test -- src/pages/Waveform/chartGroup.test.ts`

预期：PASS。

- [ ] **Step 4: Commit**

```bash
git add src/pages/Waveform/chartGroup.ts src/pages/Waveform/chartGroup.test.ts
git commit -m "feat(waveform): 增加图表组Y轴模式"
```

## Task 2: 实现采样点偏移纯函数和序列模型

**Files:**
- Modify: `src/pages/Waveform/multiLineChartModel.ts`
- Test: `src/pages/Waveform/multiLineChartModel.test.ts`

- [ ] **Step 1: Write failing tests for offset semantics**

增加以下测试：

```ts
it('shifts a line right and fills the leading gap with zero', () => {
  expect(applyLineOffset([1, 2, 3], 1)).toEqual([0, 1, 2]);
});

it('shifts a line left and fills the trailing gap with zero', () => {
  expect(applyLineOffset([1, 2, 3], -1)).toEqual([2, 3, 0]);
});

it('returns a same-length zero line when the offset exceeds the data', () => {
  expect(applyLineOffset([1, 2, 3], 3)).toEqual([0, 0, 0]);
  expect(applyLineOffset([1, 2, 3], -4)).toEqual([0, 0, 0]);
});

it('normalizes invalid offsets to a bounded integer', () => {
  expect(normalizeLineOffset(Number.NaN)).toBe(0);
  expect(normalizeLineOffset(2.8)).toBe(3);
  expect(normalizeLineOffset(Number.POSITIVE_INFINITY)).toBe(0);
});

it('applies offsets without mutating source rows', () => {
  const rows = [[1, 10], [2, 20], [3, 30]];
  const series = buildChartSeries(['A', 'B'], rows, ['A'], MAX_LINES_PER_CHART, { A: 1 });
  expect(series[0].data).toEqual([0, 1, 2]);
  expect(rows).toEqual([[1, 10], [2, 20], [3, 30]]);
});
```

运行：`npm test -- src/pages/Waveform/multiLineChartModel.test.ts`

预期：FAIL，因为新导出函数和 `buildChartSeries` 参数尚不存在。

- [ ] **Step 2: Implement bounded offset helpers**

在 `multiLineChartModel.ts` 增加：

```ts
export const MAX_LINE_OFFSET_POINTS = 1_000_000;

export function normalizeLineOffset(offset: number): number {
  if (!Number.isFinite(offset)) return 0;
  return Math.max(-MAX_LINE_OFFSET_POINTS, Math.min(MAX_LINE_OFFSET_POINTS, Math.round(offset)));
}

export function applyLineOffset(data: readonly number[], offset: number): number[] {
  const normalized = normalizeLineOffset(offset);
  const length = data.length;
  if (length === 0 || Math.abs(normalized) >= length) return Array(length).fill(0);
  if (normalized > 0) {
    return Array(normalized).fill(0).concat(data.slice(0, length - normalized));
  }
  if (normalized < 0) {
    return data.slice(-normalized).concat(Array(-normalized).fill(0));
  }
  return [...data];
}
```

Update `buildChartSeries` to accept an optional fifth argument `lineOffsets?: Readonly<Record<string, number>>`; normalize sparse/missing cells to `0`, then call `applyLineOffset` with `lineOffsets?.[name] ?? 0`. Preserve all existing callers by leaving `maxLines` as the fourth argument.

- [ ] **Step 3: Run model tests**

运行：`npm test -- src/pages/Waveform/multiLineChartModel.test.ts`

预期：PASS，且既有颜色、去重和四线限制测试继续通过。

- [ ] **Step 4: Commit**

```bash
git add src/pages/Waveform/multiLineChartModel.ts src/pages/Waveform/multiLineChartModel.test.ts
git commit -m "feat(waveform): 增加折线采样点偏移模型"
```

## Task 3: 让统计复用偏移后的显示序列

**Files:**
- Modify: `src/pages/Waveform/multiLineChartStats.ts`
- Test: `src/pages/Waveform/multiLineChartStats.test.ts`

- [ ] **Step 1: Add the failing statistics test**

新增测试，验证右移后的首点为 0 并参与当前可视区统计：

```ts
it('calculates visible statistics from offset line values', () => {
  const stats = calculateVisibleLineStats(
    ['A'],
    [[1], [2], [3]],
    ['A'],
    { start: 0, end: 100 },
    4,
    { A: 1 },
  );
  expect(stats[0]).toMatchObject({ min: 0, max: 2, diff: 2 });
});
```

运行：`npm test -- src/pages/Waveform/multiLineChartStats.test.ts`

预期：FAIL。

- [ ] **Step 2: Implement the optional offsets argument**

给 `calculateVisibleLineStats` 增加最后一个可选参数 `lineOffsets?: Readonly<Record<string, number>>`。在计算每条线时先用 `buildChartSeries(columns, rows, groupColumns, maxLines, lineOffsets)`，再按可视行范围读取对应 `series.data[rowIndex]`。保持原函数前五个参数和现有统计算法不变。

- [ ] **Step 3: Run focused tests**

运行：`npm test -- src/pages/Waveform/multiLineChartStats.test.ts src/pages/Waveform/multiLineChartModel.test.ts`

预期：PASS。

- [ ] **Step 4: Commit**

```bash
git add src/pages/Waveform/multiLineChartStats.ts src/pages/Waveform/multiLineChartStats.test.ts
git commit -m "feat(waveform): 让线统计使用偏移后数据"
```

## Task 4: 扩展 CSV store 的偏移状态生命周期

**Files:**
- Modify: `src/stores/csvChartStore.ts`
- Test: `src/stores/csvChartStore.test.ts`

- [ ] **Step 1: Add failing store tests**

增加：

```ts
it('normalizes line offsets through the store action', () => {
  useCsvChartStore.getState().setLineOffset('A', 2.8);
  expect(useCsvChartStore.getState().lineOffsets).toEqual({ A: 3 });
  useCsvChartStore.getState().setLineOffset('A', Number.NaN);
  expect(useCsvChartStore.getState().lineOffsets).toEqual({ A: 0 });
});

it('keeps matching offsets and removes offsets for columns absent from a new CSV', async () => {
  useCsvChartStore.setState({ lineOffsets: { A: 2, OLD: -1 } });
  readCsvFile.mockResolvedValueOnce({ columns: ['A', 'B'], rows: [[1, 2]] });
  await useCsvChartStore.getState().loadCsvFile('new.csv');
  expect(useCsvChartStore.getState().lineOffsets).toEqual({ A: 2 });
});

it('clears line offsets with CSV data', () => {
  useCsvChartStore.setState({ lineOffsets: { A: 2 } });
  useCsvChartStore.getState().clearData();
  expect(useCsvChartStore.getState().lineOffsets).toEqual({});
});
```

运行：`npm test -- src/stores/csvChartStore.test.ts`

预期：FAIL。

- [ ] **Step 2: Add state and actions**

导入 `normalizeLineOffset`，在 state/action 类型中加入：

```ts
lineOffsets: Record<string, number>;
setLineOffset: (column: string, offset: number) => void;
```

初始值为 `{}`；action 使用 `normalizeLineOffset(offset)` 写入列名 key。成功加载 CSV 时用 `Object.fromEntries(Object.entries(get().lineOffsets).filter(([column]) => csvData.columns.includes(column)))` 写入过滤后的配置；失败路径不改动旧配置。`clearData` 同时重置为 `{}`。

- [ ] **Step 3: Run store tests**

运行：`npm test -- src/stores/csvChartStore.test.ts`

预期：PASS，且既有异步加载顺序、缩放保留和失败保留测试继续通过。

- [ ] **Step 4: Commit**

```bash
git add src/stores/csvChartStore.ts src/stores/csvChartStore.test.ts
git commit -m "feat(waveform): 管理CSV折线偏移配置"
```

## Task 5: 将偏移和 Y 轴模式接入 MultiLineChart

**Files:**
- Modify: `src/pages/Waveform/MultiLineChart.tsx`
- Test: `src/pages/Waveform/MultiLineChart.test.ts`

- [ ] **Step 1: Add pure option-builder tests**

将当前内联的 Y 轴布局提取为可测试的导出函数（建议命名 `buildYAxisOptions`），测试：

```ts
it('builds one visible axis and maps every series to axis zero in shared mode', () => {
  const result = buildYAxisOptions('shared', [
    { name: 'A', data: [1], color: '#165DFF' },
    { name: 'B', data: [100], color: '#F53F3F' },
  ], 300);
  expect(result.yAxis).toHaveLength(1);
  expect(result.series.map((item) => item.yAxisIndex)).toEqual([0, 0]);
});

it('builds one axis per series in per-line mode', () => {
  const result = buildYAxisOptions('per-line', [
    { name: 'A', data: [1], color: '#165DFF' },
    { name: 'B', data: [100], color: '#F53F3F' },
  ], 300);
  expect(result.yAxis).toHaveLength(2);
  expect(result.series.map((item) => item.yAxisIndex)).toEqual([0, 1]);
});
```

运行：`npm test -- src/pages/Waveform/MultiLineChart.test.ts`

预期：FAIL，函数尚不存在。

- [ ] **Step 2: Extend props and display data flow**

给 `MultiLineChartProps` 增加：

```ts
lineOffsets?: Readonly<Record<string, number>>;
```

在 `visibleLineStats` 调用中传入 `lineOffsets`；在 `getChartOption` 调用 `buildChartSeries(..., lineOffsets)`。把 `lineOffsets` 加入相关 `useMemo`/`useCallback` 依赖，确保修改输入后重绘。

- [ ] **Step 3: Extract and implement Y-axis option builder**

新增 `buildYAxisOptions(mode, seriesData, height)`：

- `shared` 或缺省模式：创建一个可见 value axis，所有 series 的 `yAxisIndex` 为 `0`，只显示一套 split line。
- `per-line`：按现有左 0、右 0、左 `Y_AXIS_WIDTH`、右 `Y_AXIS_WIDTH` 的顺序为序列建轴，序列索引与轴索引一致。
- 返回 `{ yAxis, series }`，保留现有轴颜色、科学计数法和线样式。

将 `getChartOption` 改为合并 builder 返回值；shared 模式使用较小的左右 grid 边距，per-line 模式保留当前双侧多轴边距。tooltip、legend、dataZoom、导出代码不改变。

- [ ] **Step 4: Run chart tests and type check**

运行：`npm test -- src/pages/Waveform/MultiLineChart.test.ts src/pages/Waveform/multiLineChartModel.test.ts src/pages/Waveform/multiLineChartStats.test.ts`。

预期：PASS。

运行：`npx tsc --noEmit`。

预期：无 TypeScript 错误。

- [ ] **Step 5: Commit**

```bash
git add src/pages/Waveform/MultiLineChart.tsx src/pages/Waveform/MultiLineChart.test.ts
git commit -m "feat(waveform): 支持图表组Y轴模式"
```

## Task 6: 增加 CSV 页面配置控件和本地化文案

**Files:**
- Modify: `src/pages/Waveform/CsvLoaderTab.tsx`
- Modify: `src/locales/zh-CN/waveform.json`
- Modify: `src/locales/en-US/waveform.json`
- Test: `src/pages/Waveform/CsvLoaderTab.test.tsx`

- [ ] **Step 1: Add failing component tests**

在 CSV 测试中设置包含 `A`、`B` 的图表组和 CSV 数据，验证：

```ts
expect(screen.getByText(i18n.t('waveform:csvLoader.yAxisMode'))).toBeInTheDocument();
expect(screen.getByText(i18n.t('waveform:csvLoader.lineOffset'))).toBeInTheDocument();
```

触发 Y 轴模式 `Select` 的 `onChange('per-line')` 和偏移 `InputNumber` 的 `onChange(2)`，断言 `useCsvChartStore.getState().chartGroups[0].yAxisMode` 和 `lineOffsets.A` 更新。

运行：`npm test -- src/pages/Waveform/CsvLoaderTab.test.tsx`

预期：FAIL。

- [ ] **Step 2: Add translation keys**

在两个 waveform locale 的 `csvLoader` 下增加：

```json
"yAxisMode": "Y轴模式",
"sharedYAxis": "共用 Y 轴",
"perLineYAxis": "每条折线独立 Y 轴",
"lineOffset": "采样点偏移",
"offsetPoints": "偏移点数"
```

英文对应：`Y-axis mode`, `Shared Y axis`, `Independent Y axis per line`, `Line offset`, `Offset points`。

- [ ] **Step 3: Add controls to each chart-group editor**

从 store 解构 `lineOffsets`、`setLineOffset`，从图表组编辑中使用 `updateChartGroup`。每组在列选择器旁增加：

```tsx
<Select
  size="small"
  value={group.yAxisMode ?? 'shared'}
  onChange={(yAxisMode) => updateChartGroup(group.id, { yAxisMode })}
  options={[
    { label: t('csvLoader.sharedYAxis'), value: 'shared' },
    { label: t('csvLoader.perLineYAxis'), value: 'per-line' },
  ]}
/>
```

对 `group.columns.slice(0, MAX_LINES_PER_CHART)` 渲染列名和 `InputNumber`：`min={-MAX_LINE_OFFSET_POINTS}`, `max={MAX_LINE_OFFSET_POINTS}`, `precision={0}`, `step={1}`, `value={lineOffsets[column] ?? 0}`，变更时调用 `setLineOffset(column, value ?? 0)`。使用紧凑换行布局，避免配置区横向溢出。

- [ ] **Step 4: Run component tests and type check**

运行：`npm test -- src/pages/Waveform/CsvLoaderTab.test.tsx`

预期：PASS。

运行：`npx tsc --noEmit`

预期：无 TypeScript 错误。

- [ ] **Step 5: Commit**

```bash
git add src/pages/Waveform/CsvLoaderTab.tsx src/locales/zh-CN/waveform.json src/locales/en-US/waveform.json src/pages/Waveform/CsvLoaderTab.test.tsx
git commit -m "feat(waveform): 增加CSV偏移和Y轴配置控件"
```

## Task 7: 将 CSV store 配置传入图表并完成回归验证

**Files:**
- Modify: `src/pages/Waveform/CsvLoaderTab.tsx`
- Verify: `src/pages/Waveform/MultiLineChart.tsx` (确认 Task 5 的 `lineOffsets` prop 已接通)
- Test: `src/pages/Waveform/CsvLoaderTab.test.tsx`

- [ ] **Step 1: Pass line offsets through the CSV chart entry**

在 `CsvLoaderTab` 的 `CsvMultiLineChart` 调用中增加：

```tsx
lineOffsets={lineOffsets}
```

确认实时波形和 GH3036 调用方不传该 prop 时仍得到零偏移；确认旧 `ChartGroupConfig` 不传 `yAxisMode` 时使用 shared。

- [ ] **Step 2: Add an integration assertion**

在页面测试中设置 `{ A: 1 }` 偏移并确认 mock `MultiLineChart` 收到同一对象；切换第一组 Y 轴模式后确认第二组仍保持 `shared`。

- [ ] **Step 3: Run the full frontend verification suite**

运行：

```bash
npm test -- src/pages/Waveform src/stores/csvChartStore.test.ts
npx tsc --noEmit
```

预期：所有 Waveform/store 测试 PASS，TypeScript 无错误。

- [ ] **Step 4: Run the production build**

运行：`npm run build`

预期：`tsc` 和 Vite build 均成功完成。

- [ ] **Step 5: Review the final diff and commit**

运行：`git diff HEAD~6..HEAD --stat` 和 `git status --short`。确认只包含本功能文件，未修改 `.worktree/` 等用户未跟踪内容。

```bash
git add src/pages/Waveform src/stores/csvChartStore.ts src/locales/zh-CN/waveform.json src/locales/en-US/waveform.json
git commit -m "test(waveform): 完成CSV折线配置回归验证"
```

## 验收清单

- [ ] 正偏移右移并以 0 填充开头，负偏移左移并以 0 填充末尾，长度不变。
- [ ] 原始 `csvData.rows` 不发生修改。
- [ ] 每个图表组可独立选择 shared/per-line Y 轴。
- [ ] 图表、线统计、PNG/SVG 导出使用同一份偏移后的序列。
- [ ] 同一文件重载保留仍存在列的偏移，新文件过滤不存在列，清空数据重置偏移。
- [ ] 实时波形、GH3036 图表和既有 CSV 功能回归测试通过。
