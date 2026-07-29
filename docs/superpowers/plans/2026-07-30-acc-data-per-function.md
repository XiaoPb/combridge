# ACC 数据按功能分组实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 ACC 数据从全局单一缓冲区改为按 function_id 分组存储，每个功能维护独立的 ACC 数据流。

**Architecture:** 将 gsensorData 从单一对象改为 Map<function_id, GsensorData>，与现有的 framesData 存储模式一致。前端修改 gh3036Store 和 MonitorTab 组件，后端无需修改。

**Tech Stack:** React 19, TypeScript, Zustand, ECharts

---

## 范围检查

本计划仅涉及前端数据存储和显示逻辑修改，属于单一子系统。后端已按帧推送 ACC 数据，无需修改。

---

## 文件结构

**前端文件：**
- `src/stores/gh3036Store.ts` - 状态管理（核心修改）
- `src/pages/Gh3036/MonitorTab.tsx` - 显示组件（核心修改）
- `src/pages/Gh3036/monitorChartData.ts` - 数据构建（可选优化）

**后端文件：**
- 无需修改

**测试文件：**
- `src/stores/gh3036Store.test.ts` - 现有测试文件（需更新）

---

## Task 1: 修改 gh3036Store 数据结构

**问题分析：**
当前 gsensorData 是全局单一对象，需要改为按 function_id 分组的 Map 结构。

### Task 1.1: 修改类型定义和初始状态

**Files:**
- Modify: `src/stores/gh3036Store.ts`

- [ ] **Step 1: 修改 Gh3036State 类型定义**

在 `src/stores/gh3036Store.ts` 第110-117行，修改类型定义：

```typescript
// 修改前
gsensorData: {
  acc_x: number[];
  acc_y: number[];
  acc_z: number[];
  gyro_x: number[];
  gyro_y: number[];
  gyro_z: number[];
};
maxGsensorCount: number;

// 修改后
gsensorData: Map<number, {
  acc_x: number[];
  acc_y: number[];
  acc_z: number[];
  gyro_x: number[];
  gyro_y: number[];
  gyro_z: number[];
}>;
maxGsensorCount: number;
```

- [ ] **Step 2: 修改初始状态**

在 `src/stores/gh3036Store.ts` 第259-266行，修改初始状态：

```typescript
// 修改前
gsensorData: {
  acc_x: [],
  acc_y: [],
  acc_z: [],
  gyro_x: [],
  gyro_y: [],
  gyro_z: [],
},

// 修改后
gsensorData: new Map(),
```

- [ ] **Step 3: 提交 Task 1.1 的修改**

```bash
git add src/stores/gh3036Store.ts
git commit -m "refactor(gh3036): 修改gsensorData为Map结构"
```

### Task 1.2: 修改 updateFrames 方法

**Files:**
- Modify: `src/stores/gh3036Store.ts`

- [ ] **Step 1: 修改 updateFrames 方法中的 ACC 数据处理**

在 `src/stores/gh3036Store.ts` 第450-501行，修改 ACC 数据更新逻辑：

```typescript
// 在 updateFrames 方法中，修改 gsensorData 的处理逻辑

// 1. 获取或创建当前 function_id 的 gsensorData
let currentGsensorData = newGsensorData.get(frames.function_id);
if (!currentGsensorData) {
  currentGsensorData = {
    acc_x: [],
    acc_y: [],
    acc_z: [],
    gyro_x: [],
    gyro_y: [],
    gyro_z: [],
  };
  newGsensorData.set(frames.function_id, currentGsensorData);
}

// 2. 更新当前功能的 ACC 数据
for (const frame of frames.frames) {
  currentGsensorData.acc_x.push(frame.gsensor_data.acc[0]);
  currentGsensorData.acc_y.push(frame.gsensor_data.acc[1]);
  currentGsensorData.acc_z.push(frame.gsensor_data.acc[2]);
  currentGsensorData.gyro_x.push(0);
  currentGsensorData.gyro_y.push(0);
  currentGsensorData.gyro_z.push(0);
}

// 3. 应用容量限制（与 framesData 一致）
const maxPoints = get().maxGsensorCount;
if (currentGsensorData.acc_x.length > maxPoints) {
  const startIndex = currentGsensorData.acc_x.length - maxPoints;
  currentGsensorData.acc_x = currentGsensorData.acc_x.slice(startIndex);
  currentGsensorData.acc_y = currentGsensorData.acc_y.slice(startIndex);
  currentGsensorData.acc_z = currentGsensorData.acc_z.slice(startIndex);
  currentGsensorData.gyro_x = currentGsensorData.gyro_x.slice(startIndex);
  currentGsensorData.gyro_y = currentGsensorData.gyro_y.slice(startIndex);
  currentGsensorData.gyro_z = currentGsensorData.gyro_z.slice(startIndex);
}
```

- [ ] **Step 2: 提交 Task 1.2 的修改**

```bash
git add src/stores/gh3036Store.ts
git commit -m "feat(gh3036): updateFrames支持按function_id更新ACC数据"
```

### Task 1.3: 修改 clearWaveformData 方法

**Files:**
- Modify: `src/stores/gh3036Store.ts`

- [ ] **Step 1: 修改 clearWaveformData 方法**

在 `src/stores/gh3036Store.ts` 第503-529行，修改清理逻辑：

```typescript
// 修改前
clearWaveformData: () => set({
  framesData: new Map(),
  chartGroups: [],
  selectedFunctionId: null,
  gsensorData: {
    acc_x: [],
    acc_y: [],
    acc_z: [],
    gyro_x: [],
    gyro_y: [],
    gyro_z: [],
  },
  // ... 其他字段
}),

// 修改后
clearWaveformData: () => set({
  framesData: new Map(),
  chartGroups: [],
  selectedFunctionId: null,
  gsensorData: new Map(),
  // ... 其他字段
}),
```

- [ ] **Step 2: 提交 Task 1.3 的修改**

```bash
git add src/stores/gh3036Store.ts
git commit -m "fix(gh3036): clearWaveformData清理Map结构gsensorData"
```

---

## Task 2: 修改 MonitorTab 组件显示逻辑

**问题分析：**
MonitorTab 需要根据 selectedFunctionId 从 Map 中获取对应的 ACC 数据。

### Task 2.1: 修改 gsensorChartData 计算逻辑

**Files:**
- Modify: `src/pages/Gh3036/MonitorTab.tsx`

- [ ] **Step 1: 修改 gsensorChartData useMemo**

在 `src/pages/Gh3036/MonitorTab.tsx` 第112-139行，修改计算逻辑：

```typescript
// 修改前
const gsensorChartData = useMemo(() => {
  const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
  const rows: number[][] = [];

  const maxPoints = DISPLAY_DURATION_SECONDS * DEFAULT_SAMPLE_RATE;
  const len = Math.min(
    gsensorData.acc_x.length,
    gsensorData.acc_y.length,
    gsensorData.acc_z.length,
    maxPoints
  );

  const startIndex = Math.max(0, Math.min(
    gsensorData.acc_x.length,
    gsensorData.acc_y.length,
    gsensorData.acc_z.length
  ) - maxPoints);

  for (let i = startIndex; i < startIndex + len; i++) {
    rows.push([
      gsensorData.acc_x[i],
      gsensorData.acc_y[i],
      gsensorData.acc_z[i],
    ]);
  }

  return { columns, rows };
}, [gsensorData]);

// 修改后
const gsensorChartData = useMemo(() => {
  const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
  const rows: number[][] = [];

  // 根据 selectedFunctionId 获取对应的 gsensorData
  const currentGsensorData = selectedFunctionId
    ? gsensorData.get(selectedFunctionId)
    : null;

  if (!currentGsensorData) {
    return { columns, rows: [] };
  }

  const maxPoints = DISPLAY_DURATION_SECONDS * DEFAULT_SAMPLE_RATE;
  const len = Math.min(
    currentGsensorData.acc_x.length,
    currentGsensorData.acc_y.length,
    currentGsensorData.acc_z.length,
    maxPoints
  );

  const startIndex = Math.max(0, Math.min(
    currentGsensorData.acc_x.length,
    currentGsensorData.acc_y.length,
    currentGsensorData.acc_z.length
  ) - maxPoints);

  for (let i = startIndex; i < startIndex + len; i++) {
    rows.push([
      currentGsensorData.acc_x[i],
      currentGsensorData.acc_y[i],
      currentGsensorData.acc_z[i],
    ]);
  }

  return { columns, rows };
}, [gsensorData, selectedFunctionId]);
```

- [ ] **Step 2: 提交 Task 2.1 的修改**

```bash
git add src/pages/Gh3036/MonitorTab.tsx
git commit -m "feat(gh3036): MonitorTab根据selectedFunctionId显示ACC数据"
```

---

## Task 3: 添加数据构建辅助函数（可选优化）

**问题分析：**
为了代码一致性和可测试性，可以将 gsensorChartData 的构建逻辑提取为独立函数。

### Task 3.1: 添加 buildGsensorChartData 函数

**Files:**
- Modify: `src/pages/Gh3036/monitorChartData.ts`

- [ ] **Step 1: 添加 buildGsensorChartData 函数**

在 `src/pages/Gh3036/monitorChartData.ts` 文件末尾添加：

```typescript
/**
 * 构建 ACC（加速度计）波形图表数据
 * @param gsensorData - 当前功能的 gsensor 数据
 * @param displayDurationSeconds - 显示时长（秒）
 * @param sampleRate - 采样率
 * @returns ChartTableData - 图表数据
 */
export function buildGsensorChartData(
  gsensorData: {
    acc_x: number[];
    acc_y: number[];
    acc_z: number[];
  } | null,
  displayDurationSeconds: number,
  sampleRate: number
): ChartTableData {
  if (!gsensorData) {
    return { columns: ['ACC_X', 'ACC_Y', 'ACC_Z'], rows: [] };
  }

  const columns = ['ACC_X', 'ACC_Y', 'ACC_Z'];
  const rows: number[][] = [];

  const maxPoints = Math.max(1, Math.floor(displayDurationSeconds * sampleRate));
  const availablePoints = Math.min(
    gsensorData.acc_x.length,
    gsensorData.acc_y.length,
    gsensorData.acc_z.length
  );

  if (availablePoints === 0) {
    return { columns, rows: [] };
  }

  const startIndex = Math.max(0, availablePoints - maxPoints);

  for (let i = startIndex; i < availablePoints; i++) {
    rows.push([
      gsensorData.acc_x[i] ?? 0,
      gsensorData.acc_y[i] ?? 0,
      gsensorData.acc_z[i] ?? 0,
    ]);
  }

  return { columns, rows };
}
```

- [ ] **Step 2: 修改 MonitorTab 使用新函数**

在 `src/pages/Gh3036/MonitorTab.tsx` 第112-139行，修改为：

```typescript
const gsensorChartData = useMemo(() => {
  const currentGsensorData = selectedFunctionId
    ? gsensorData.get(selectedFunctionId)
    : null;

  return buildGsensorChartData(
    currentGsensorData,
    DISPLAY_DURATION_SECONDS,
    sampleRate
  );
}, [gsensorData, selectedFunctionId, sampleRate]);
```

- [ ] **Step 3: 提交 Task 3.1 的修改**

```bash
git add src/pages/Gh3036/monitorChartData.ts src/pages/Gh3036/MonitorTab.tsx
git commit -m "refactor(gh3036): 提取buildGsensorChartData函数"
```

---

## Task 4: 更新测试

**问题分析：**
现有测试需要更新以适应新的数据结构。

### Task 4.1: 更新 gh3036Store 测试

**Files:**
- Modify: `src/stores/gh3036Store.test.ts`

- [ ] **Step 1: 更新测试中的 gsensorData 断言**

在测试文件中，将所有 `gsensorData.acc_x` 访问改为 `gsensorData.get(functionId)?.acc_x`。

- [ ] **Step 2: 运行测试验证**

```bash
npm test src/stores/gh3036Store.test.ts
```

Expected: PASS

- [ ] **Step 3: 提交 Task 4.1 的修改**

```bash
git add src/stores/gh3036Store.test.ts
git commit -m "test(gh3036): 更新测试以适应Map结构gsensorData"
```

---

## 自我审查

### 1. 需求覆盖检查

✅ **独立存储** - Task 1 实现了按 function_id 分组存储
✅ **数据长度一致** - 使用相同的容量限制逻辑
✅ **实时更新** - Task 1.2 在 updateFrames 中实时更新
✅ **无缝切换** - Task 2.1 根据 selectedFunctionId 立即显示

### 2. 占位符扫描

✅ 无 "TBD"、"TODO"、"implement later" 等占位符
✅ 所有代码步骤都包含完整代码
✅ 无模糊描述

### 3. 类型一致性检查

✅ `gsensorData` 类型在所有任务中一致使用 `Map<number, GsensorData>`
✅ `selectedFunctionId` 类型始终为 `number | null`
✅ 数据访问方式一致：`gsensorData.get(selectedFunctionId)`

---

## 执行移交

计划已完成并保存到 `docs/superpowers/plans/2026-07-30-acc-data-per-function.md`。

**两种执行选项：**

**1. Subagent-Driven（推荐）** - 为每个任务派发新的子代理，任务间审查，快速迭代

**2. Inline Execution** - 在当前会话中使用 executing-plans 执行，批量执行带检查点

**选择哪种方式？**