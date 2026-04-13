# Dashboard 调用流程文档

> **注意**: 本文档已整合到模块化架构文档体系中。详细的后端架构请参阅 [Dashboard 模块文档](./architecture/backend/dashboard-module.md)，前端页面架构请参阅 [前端页面层文档](./architecture/frontend/pages-layer.md)。

## 一、概述

本文档描述了 ComBridge Dashboard 模块的数据流和调用流程，包括前端组件交互、后端服务处理、以及数据解析流程。

## 二、模块架构

```
┌─────────────────────────────────────────────────────────────────────┐
│                         前端 (React + TypeScript)                    │
├─────────────────────────────────────────────────────────────────────┤
│  DashboardPage                                                       │
│  ├── DashboardToolbar (工具栏)                                       │
│  │   ├── Dashboard选择器                                             │
│  │   ├── DataSourceSelector (数据源选择)                             │
│  │   └── ParserSelector (解析器选择)                                 │
│  ├── DashboardCanvas (画布)                                          │
│  │   └── WidgetRenderer → Widget组件                                 │
│  └── DashboardPanel (侧边面板)                                        │
│      ├── 数据视图                                                    │
│      ├── 原始数据                                                    │
│      └── 组件配置                                                    │
├─────────────────────────────────────────────────────────────────────┤
│  dashboardStore (Zustand 状态管理)                                   │
│  ├── currentDashboard: 当前Dashboard配置                             │
│  ├── savedDashboards: 已保存的Dashboard列表                          │
│  ├── dataBuffer: 数据缓冲区                                          │
│  └── parserScripts: 解析脚本列表                                     │
├─────────────────────────────────────────────────────────────────────┤
│                         Tauri IPC 桥接                               │
├─────────────────────────────────────────────────────────────────────┤
│                         后端 (Rust)                                  │
│  ├── dashboard/commands.rs - Dashboard命令                          │
│  ├── dashboard/parser_scripts.rs - 解析脚本管理                      │
│  └── serial/ble - 数据源                                             │
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
    │              │  onSerialData/onBleData     │              │
    │              │<─────────────────────────────│              │
    │              │              │              │              │
    │              │ parseData()  │              │              │
    │              │──────┐       │              │              │
    │              │      │ 解析  │              │              │
    │              │<─────┘       │              │              │
    │              │              │              │              │
    │              │ addDataPoint │              │              │
    │              │─────────────>│              │              │
    │              │              │              │              │
    │              │  Widget订阅dataBuffer       │              │
    │              │<─────────────│              │              │
    │              │              │              │              │
    │  更新显示    │              │              │              │
    │<─────────────│              │              │              │
    │              │              │              │              │
```

### 3.2 组件添加流程

```
┌────────┐     ┌────────┐     ┌────────┐     ┌────────┐
│ 用户   │     │ Canvas │     │Selector│     │ Store  │
└───┬────┘     └───┬────┘     └───┬────┘     └───┬────┘
    │              │              │              │
    │ 点击添加组件 │              │              │
    │─────────────>│              │              │
    │              │ 打开Selector │              │
    │              │─────────────>│              │
    │              │              │              │
    │ 选择组件类型 │              │              │
    │─────────────────────────────>│              │
    │              │              │              │
    │ 配置组件参数 │              │              │
    │─────────────────────────────>│              │
    │              │              │ addWidget    │
    │              │              │─────────────>│
    │              │              │              │
    │              │ 更新显示     │              │
    │              │<─────────────────────────────│
    │              │              │              │
    │  显示新组件  │              │              │
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
解析层      [parseData]──┬──[JSON解析]──┬──[Lua脚本]──┬──[分隔符]──>
                             │              │              │
                             └──────────────┴──────────────┘
                                            │
                                            ▼
存储层      [addDataPoint]──────────────────────────────────────────>
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
  if (isRunning && dataSourceType === 'serial') {
    const unlisten = onSerialData(handleSerialData);
    return () => { unlisten.then(f => f()); };
  }
}, [isRunning, dataSourceType]);

// 数据解析
const parseData = (rawData: string): Record<string, number> => {
  // 1. JSON解析
  // 2. Lua脚本解析
  // 3. 分隔符解析
};
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
  parserType: 'json' | 'lua' | 'delimiter';
  parserScript: string | null;
  parserScripts: ParserScriptInfo[];
  
  // 运行时状态
  isRunning: boolean;
  dataBuffer: DataPoint[];
  isEditMode: boolean;
  selectedWidget: string | null;
}
```

### 5.3 WidgetRenderer (组件渲染)

**支持的组件类型**:
| 类型 | 用途 | 配置参数 |
|------|------|----------|
| lineChart | 趋势图 | dataKey, min, max, color |
| gauge | 仪表盘 | dataKey, min, max, unit |
| text | 文本显示 | dataKey, unit |
| led | 状态指示 | dataKey, color |
| compass | 方向显示 | dataKey |
| accelerometer | 三轴显示 | dataKey |

## 六、后端API

### 6.1 Dashboard命令

| 命令 | 说明 | 参数 |
|------|------|------|
| `get_parser_scripts` | 获取解析脚本列表 | - |
| `save_parser_script` | 保存解析脚本 | name, content |
| `delete_parser_script` | 删除解析脚本 | name |
| `execute_parser_script` | 执行解析脚本 | name, data |
| `generate_parser_from_json` | 从JSON生成脚本 | json, name |

### 6.2 数据事件

| 事件 | 触发时机 | 数据格式 |
|------|----------|----------|
| `serial-data` | 串口收到数据 | `{ data: string, timestamp: number }` |
| `ble-data` | BLE收到通知 | `{ data: string, timestamp: number }` |

## 七、多Dashboard布局支持

### 7.1 Dashboard配置结构

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
    type: 'json' | 'lua' | 'delimiter';
    scriptName?: string;
    config: Record<string, unknown>;
  };
  widgets: WidgetConfig[];
  refreshRate: number;
}
```

### 7.2 多Dashboard操作流程

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

## 八、错误处理

### 8.1 前端错误处理

```typescript
// 数据解析错误
try {
  const parsed = parseData(rawData);
  addDataPoint({ timestamp: Date.now(), values: parsed });
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

### 8.2 后端错误处理

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
}
```

## 九、性能优化建议

1. **数据缓冲区限制**: `maxBufferSize: 1000` 防止内存溢出
2. **组件懒加载**: Widget组件按需渲染
3. **事件节流**: 高频数据使用 throttle 降低更新频率
4. **虚拟滚动**: 大量数据点时使用虚拟列表

## 十、扩展指南

### 10.1 添加新Widget类型

1. 在 `src/types/dashboard.ts` 添加类型定义
2. 在 `src/pages/Dashboard/widgets/` 创建组件
3. 在 `WidgetRenderer.tsx` 添加渲染逻辑
4. 在翻译文件添加国际化文本

### 10.2 添加新数据源

1. 在 `dataSourceType` 添加新类型
2. 在 `index.tsx` 添加事件监听逻辑
3. 在 `DataSourceSelector.tsx` 添加UI选项
