# 波形图表线统计信息设计

## 背景

CSV 波形图表配置区当前已有“无表头”开关。需要在其后新增“显示线统计”开关，用于控制是否在每个图表上方显示每条曲线的 `max`、`min`、`avg`、`diff`。统计值必须对应当前图表可视区域，并随缩放或拖动实时更新。

## 已确认的产品行为

- 新增控件紧跟在“无表头”控件后方。
- 控件默认关闭。
- 开启后，每个图表的每条曲线独占一行，显示在该图表的折线区域上方。
- 每行显示曲线颜色标识、曲线名以及 `max`、`min`、`avg`、`diff`。
- 统计值按当前可视区域计算，而不是按全部数据计算。
- `diff` 定义为 `max - min`。
- 统计值只使用有限数值；当前区域没有有效数据时显示 `—`。
- 统计区域只属于页面展示，不包含在 PNG/SVG 导出结果中。
- 统计开关属于 CSV 图表状态，保存在现有 CSV 图表 store 中；是否跨应用重启持久化沿用当前项目偏好机制，不为本功能新增持久化体系。

## 现有代码边界

- `src/pages/Waveform/CsvLoaderTab.tsx` 管理 CSV 图表状态、配置控件和 `MultiLineChart` props。
- `src/stores/csvChartStore.ts` 管理 CSV 数据、图表分组和共享 `dataZoomState`。
- `src/pages/Waveform/MultiLineChart.tsx` 创建多个 ECharts 实例，处理图例、缩放同步和页面导出。
- `src/pages/Waveform/chartGroup.ts` 定义图表分组及分组 key。
- `src/pages/Waveform/multiLineChartModel.ts` 已负责将图表分组转换为曲线数据，并限制每图最多 4 条线。

## 推荐架构

在 `MultiLineChart` 内计算和展示统计信息，并将计算逻辑抽到独立纯函数模块 `multiLineChartStats.ts`。

数据流如下：

1. `CsvLoaderTab` 从 `useCsvChartStore` 读取 `showLineStatistics`。
2. 控件变更通过 store action 更新该状态，并将状态作为 `showLineStatistics` prop 传入 `MultiLineChart`。
3. `MultiLineChart` 使用自身的 `localDataZoom`、完整 `rows`、`columns` 和 `chartGroups` 调用统计纯函数。
4. 纯函数把 `start/end` 百分比转换为包含起止点的行索引区间，并对每个分组的有效曲线计算统计值。
5. 每个图表容器在 ECharts 区域前渲染对应统计行。缩放事件更新 `localDataZoom` 后，React 自动重新计算统计区。

这样可以复用现有图表内部的缩放同步机制，不需要从 ECharts 实例读取坐标轴状态，也不需要把统计结果在父组件和多个图表实例之间来回传递。

## 统计规则

- 统计曲线来源与 `buildChartSeries` 一致：使用分组曲线，去重，并遵守每图最多 4 条线的限制。
- 可视区间将百分比转换为完整 `rows` 的索引；索引限制在有效范围内。
- 每条曲线只保留有限数值参与计算。
- `max` 为最大有效值，`min` 为最小有效值，`avg` 为算术平均值，`diff` 为 `max - min`。
- 没有有效值的曲线返回空统计结果，由展示层显示 `—`。
- 展示值沿用现有 `formatActualValue`，避免大数/小数的格式不一致。

## 界面设计

- “显示线统计”与“无表头”使用现有配置区的相同控件样式和国际化机制。
- 统计容器位于每个图表 ECharts 容器上方，统计行按曲线顺序排列。
- 曲线标识和名称使用该曲线颜色，统计标签和值使用主题文本颜色。
- 统计开关关闭时不渲染统计 DOM，不占用图表上方空间。
- 统计信息不接入 PNG/SVG 导出流程，现有导出结果保持不变。

## 边界与错误处理

- 无 CSV 数据、无列、无分组曲线或空可视区间时，不渲染统计行。
- 非有限值、缺失列、行长度不足不会抛异常。
- 缩放值缺失或非法时回退到完整数据范围。
- 统计开关切换、图表分组增删、曲线选择变化和数据重新加载后，统计内容必须与当前 props/状态一致。

## 测试与验收

新增 `multiLineChartStats.test.ts`，覆盖：

- 正常数据的四项统计值。
- `start/end` 到行索引的转换及起止点包含规则。
- 单点、空数据和非法缩放范围。
- `NaN`、`Infinity`、缺失值过滤。
- 多图表、多曲线、重复列名和每图最多 4 条线。

补充 `MultiLineChart.test.ts` 或现有组件测试，覆盖统计开关传递和缩放后统计更新；若现有测试环境不适合挂载 ECharts，则至少验证纯函数和 props 接口，并通过手工验收确认页面布局。

验收命令：

```text
npx tsc --noEmit
npm test -- --run
```

并手工确认：新增控件位置、统计行布局、缩放后刷新、无有效值显示 `—`，以及 PNG/SVG 中不出现统计信息。
