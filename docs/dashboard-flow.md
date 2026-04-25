# Dashboard 调用流程文档

> **注意**: 本文档已整合到模块化架构文档体系中。详细的后端架构请参阅 [Dashboard 模块文档](./architecture/backend/dashboard-module.md)，前端页面架构请参阅 [前端页面层文档](./architecture/frontend/pages-layer.md)。

## 一、概述

本文档描述了 ComBridge Dashboard 模块的数据流和调用流程，包括前端组件交互、后端服务处理、以及数据解析流程。

## 二、模块架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         前端 (React + TypeScript)                    │
├─────────────────────────────────────────────────────────────────────┤
│  DashboardPage (index.tsx)                                          │
│  ├── DashboardToolbar (工具栏)                                       │
│  │   ├── Dashboard选择器                                             │
│  │   ├── DataSourceSelector (数据源选择)                             │
│  │   └── ParserSelector (解析器选择)                                 │
│  ├── DashboardCanvas (画布)                                          │
│  │   └── WidgetRenderer → Widget组件                                 │
│  ├── ConsolePanel (控制台面板)                                        │
│  ├── SettingsPanel (设置面板)                                        │
│  └── JsonEditor (JSON编辑器)                                         │
│      ├── DatasetEditor                                              │
│      ├── FrameConfigEditor                                          │
│      ├── GroupEditor                                                │
│      └── JsonPreview                                                │
├─────────────────────────────────────────────────────────────────────┤
│  dashboardStore (Zustand 状态管理)                                   │
│  ├── currentDashboard: 当前Dashboard配置                             │
│  ├── savedDashboards: 已保存的Dashboard列表                          │
│  ├── jsonConfig: JSON配置                                           │
│  ├── jsonFiles: JSON文件列表                                         │
│  ├── rawDataBuffer: 原始数据缓冲区                                    │
│  ├── parsedDataBuffer: 解析后数据缓冲区                               │
│  ├── parserScripts: 解析脚本列表                                     │
│  └── activeTabs: 活动标签页                                          │
├─────────────────────────────────────────────────────────────────────┤
│                         Tauri IPC 桥接                               │
├─────────────────────────────────────────────────────────────────────┤
│                         后端 (Rust)                                  │
│  ├── dashboard/commands.rs - Dashboard命令                          │
│  ├── dashboard/parser_scripts.rs - 解析脚本管理                      │
│  └── dashboard/json_config.rs - JSON配置管理                         │
└─────────────────────────────────────────────────────────────────────┘
```

## 三、数据流时序图

### 3.1 数据接收流程

```
┌────────┐     ┌────────┐     ┌────────┐     ┌────────┐     ┌────────┐
│ 用户   │     │ 前端   │     │ Store  │     │ Tauri  │     │ 后端   │
└───┬────┘     └───┬────┘     └───┬────┘     └───┬────┘     └───┬────┘
    │              │              │              │              │
    │ 点击开始     │              │              │              │
    │─────────────>│              │              │              │
    │              │ setIsRunning │              │              │
    │              │─────────────>│              │              │
    │              │              │              │              │
    │              │ useEffect监听isRunning      │              │
    │              │─────────────────────────────>│              │
    │              │              │              │ 注册事件监听 │
    │              │              │              │─────────────>│
    │              │              │              │              │
    │              │              │  数据到达    │              │
    │              │              │<─────────────│<─────────────│
    │              │              │              │              │
    │              │  onSerialData/onBleData/onParsedData       │
    │              │<─────────────────────────────│              │
    │              │              │              │              │
    │              │ addRawDataPoint/addParsedDataPoint         │
    │              │─────────────>│              │              │
    │              │              │              │              │
    │              │  Widget订阅parsedDataBuffer │              │
    │              │<─────────────│              │              │
    │              │              │              │              │
    │  更新显示    │              │              │              │
    │<─────────────│              │              │              │
    │              │              │              │              │
```

### 3.2 JSON配置加载流程

```
┌────────┐     ┌────────┐     ┌────────┐     ┌────────┐
│ 用户   │     │ 前端   │     │ Store  │     │ 后端   │
└───┬────┘     └───┬────┘     └───┬────┘     └───┬────┘
    │              │              │              │
    │ 选择JSON文件 │              │              │
    │─────────────>│              │              │
    │              │ load_json_file              │
    │              │─────────────────────────────>│
    │              │              │              │
    │              │  DashboardJsonConfig        │
    │              │<─────────────────────────────│
    │              │              │              │
    │              │ setJsonConfig│              │
    │              │─────────────>│              │
    │              │              │              │
    │              │ setSelectedJsonFile         │
    │              │─────────────>│              │
    │              │              │              │
    │  渲染组件    │              │              │
    │<─────────────│              │              │
    │              │              │              │
```

## 四、甘特图 - 数据处理流程

```
时间轴 ──────────────────────────────────────────────────────────────>

数据源层    [串口数据]──────────────────────────────────────────────>
                 │
                 ▼
事件层      [onSerialData]──────────────────────────────────────────>
                 │
                 ▼
解析层      [Lua脚本解析]──────────────────────────────────────────>
                 │
                 ▼
存储层      [addParsedDataPoint]────────────────────────────────────>
                 │
                 ▼
渲染层      [Widget更新]────────────────────────────────────────────>
```

## 五、核心组件说明

### 5.1 DashboardPage (index.tsx)

**职责**: 主页面容器，管理数据流桥接

**关键逻辑**:
```typescript
// 监听运行状态，注册/注销事件监听
useEffect(() => {
  const setupDataListeners = async () => {
    if (!isRunning) return;

    if (dataSourceType === 'serial') {
      listenersRef.current.serialData = await onSerialData((event) => {
        addRawDataPoint({
          timestamp: event.timestamp ?? Date.now(),
          data: event.data,
          direction: 'RX',
        });
      });
    } else if (dataSourceType === 'ble') {
      listenersRef.current.bleData = await onBleData((event) => {
        addRawDataPoint({
          timestamp: event.timestamp ?? Date.now(),
          data: event.data,
          direction: 'RX',
        });
      });
    }

    listenersRef.current.parsedData = await onParsedData((event) => {
      addParsedDataPoint({
        timestamp: event.timestamp,
        values: event.values,
      });
    });
  };

  setupDataListeners();

  return () => {
    // 清理监听器
  };
}, [isRunning, dataSourceType, connectedDevice]);
```

### 5.2 dashboardStore (状态管理)

**状态结构**:
```typescript
interface DashboardState {
  // Dashboard管理
  currentDashboard: DashboardConfig | null;
  savedDashboards: DashboardConfig[];
  
  // 数据源配置
  dataSourceType: 'serial' | 'ble' | 'file' | 'manual';
  connectedDevice: string | null;
  
  // 解析器配置
  parserType: 'json' | 'csv' | 'delimiter' | 'regex' | 'lua';
  parserScript: string | null;
  parserScripts: ParserScriptInfo[];
  
  // JSON配置
  jsonConfig: DashboardJsonConfig;
  jsonFiles: string[];
  selectedJsonFile: string | null;
  
  // 运行时状态
  isRunning: boolean;
  rawDataBuffer: RawDataPoint[];
  parsedDataBuffer: DataPoint[];
  maxBufferSize: number;
  isEditMode: boolean;
  selectedWidget: string | null;
  activeTabs: TabType[];
  lastError: string | null;
  
  // 设备配置
  serialConfig: SerialConfig;
  serialPort: string;
  bleConfig: BleConnectionConfig | null;
}
```

### 5.3 DashboardCanvas (画布渲染)

**职责**: 根据 JSON 配置渲染 Widget 组件

**支持的组件类型**:
| 类型 | 用途 | 配置参数 |
|------|------|----------|
| lineChart | 趋势图 | dataKey, min, max, color |
| gauge | 仪表盘 | dataKey, min, max, unit |
| text | 文本显示 | dataKey, unit |
| led | 状态指示 | dataKey, color, ledHigh |
| compass | 方向显示 | dataKey |
| accelerometer | 三轴显示 | dataKey, min, max |
| bar | 进度条 | dataKey, min, max, unit |
| x/y/z | 坐标显示 | dataKey, units |

### 5.4 JsonEditor (JSON编辑器)

**职责**: 编辑 Dashboard JSON 配置

**子组件**:
- `DatasetEditor` - 数据集编辑器
- `FrameConfigEditor` - 帧配置编辑器
- `GroupEditor` - 组件组编辑器
- `JsonPreview` - JSON 预览

## 六、后端API

### 6.1 Dashboard命令

| 命令 | 说明 | 参数 |
|------|------|------|
| `get_parser_scripts` | 获取解析脚本列表 | - |
| `get_parser_script_content` | 获取脚本内容 | name |
| `save_parser_script` | 保存解析脚本 | name, content |
| `delete_parser_script` | 删除解析脚本 | name |
| `execute_parser_script` | 执行解析脚本 | name, data |
| `init_default_parser_scripts` | 初始化默认脚本 | - |
| `analyze_json_structure` | 分析JSON结构 | json_content |
| `generate_parser_from_json` | 从JSON生成脚本 | json_content, script_name, selected_fields |
| `get_parser_defined_fields` | 获取脚本定义字段 | script_name |
| `merge_json_to_parser` | 合并JSON到脚本 | json_content, script_name, selected_fields |

### 6.2 JSON配置命令

| 命令 | 说明 | 参数 |
|------|------|------|
| `get_json_files` | 获取JSON文件列表 | - |
| `save_json_file` | 保存JSON文件 | file_name, config |
| `delete_json_file` | 删除JSON文件 | file_name |
| `load_json_file` | 加载JSON文件 | file_name |

### 6.3 数据事件

| 事件 | 触发时机 | 数据格式 |
|------|----------|----------|
| `serial-data` | 串口收到数据 | `{ device_id, data, timestamp }` |
| `ble-data` | BLE收到通知 | `{ device_id, address, characteristic_uuid, data, timestamp }` |
| `parsed-data` | 数据解析完成 | `{ timestamp, values }` |

## 七、JSON配置结构

### 7.1 DashboardJsonConfig

```typescript
interface DashboardJsonConfig {
  title: string;
  decoder: number;
  frameDetection: number;
  frameStart: string;
  frameEnd: string;
  frameParser: string;
  groups: WidgetGroup[];
  mapTilerApiKey?: string;
  thunderforestApiKey?: string;
}
```

### 7.2 WidgetGroup

```typescript
interface WidgetGroup {
  title: string;
  widget: string;
  datasets: DatasetConfig[];
}
```

### 7.3 DatasetConfig

```typescript
interface DatasetConfig {
  index: number;
  title: string;
  units: string;
  widget: string;
  graph: boolean;
  min: number;
  max: number;
  color?: string;
  led: boolean;
  ledHigh: number;
  log: boolean;
  alarm: number;
  fft: boolean;
  fftSamples: number;
  fftSamplingRate: number;
  value: string;
}
```

## 八、多Dashboard布局支持

### 8.1 Dashboard配置结构

```typescript
interface DashboardConfig {
  id: string;
  name: string;
  dataSource: {
    type: 'serial' | 'ble' | 'file' | 'manual';
    deviceId?: string;
    filePath?: string;
  };
  parser: {
    type: 'json' | 'csv' | 'delimiter' | 'regex' | 'lua';
    scriptName?: string;
    config: Record<string, unknown>;
  };
  widgets: WidgetConfig[];
  refreshRate: number;
}
```

### 8.2 多Dashboard操作流程

```
┌─────────────────────────────────────────────────────────────────┐
│                     Dashboard管理流程                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐      │
│  │ 新建    │───>│ 配置    │───>│ 添加组件│───>│ 保存    │      │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘      │
│       │              │              │              │            │
│       ▼              ▼              ▼              ▼            │
│  createNewDashboard  │         addWidget    saveDashboard      │
│       │              │              │              │            │
│       └──────────────┴──────────────┴──────────────┘            │
│                              │                                  │
│                              ▼                                  │
│                    savedDashboards[]                            │
│                              │                                  │
│              ┌───────────────┼───────────────┐                  │
│              ▼               ▼               ▼                  │
│        [Dashboard 1]   [Dashboard 2]   [Dashboard N]            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## 九、错误处理

### 9.1 前端错误处理

```typescript
// 数据解析错误
try {
  const parsed = await executeParserScript(scriptName, data);
  addParsedDataPoint({ timestamp: Date.now(), values: parsed });
} catch (error) {
  setLastError(`解析失败: ${error.message}`);
  console.error('Parse error:', error);
}

// 事件监听错误
onSerialData(handleData).catch(error => {
  console.error('Failed to register serial listener:', error);
  setLastError('无法监听串口数据');
});
```

### 9.2 后端错误处理

```rust
// Rust 后端使用 thiserror 定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("Script not found: {0}")]
    ScriptNotFound(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
```

## 十、性能优化建议

1. **数据缓冲区限制**: `maxBufferSize: 1000` 防止内存溢出
2. **组件懒加载**: Widget组件按需渲染
3. **事件节流**: 高频数据使用 throttle 降低更新频率
4. **虚拟滚动**: 大量数据点时使用虚拟列表

## 十一、扩展指南

### 11.1 添加新Widget类型

1. 在 `src/types/dashboard.ts` 添加类型定义
2. 在 `src/pages/Dashboard/widgets/` 创建组件
3. 在 `DashboardCanvas.tsx` 添加渲染逻辑
4. 在 `WIDGET_SUPPORT_MATRIX` 添加配置支持
5. 在翻译文件添加国际化文本

### 11.2 添加新数据源

1. 在 `DataSourceType` 添加新类型
2. 在 `index.tsx` 添加事件监听逻辑
3. 在 `DataSourceSelector.tsx` 添加UI选项
4. 在后端添加对应的事件发布逻辑

### 11.3 添加新解析器类型

1. 在 `ParserType` 添加新类型
2. 在后端 `parser_scripts.rs` 添加解析逻辑
3. 在 `ParserSelector.tsx` 添加UI选项
4. 添加默认解析脚本模板
