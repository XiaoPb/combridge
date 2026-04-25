# ComBridge

<p align="center">
  <strong>跨平台串口与 BLE 蓝牙通信调试工具</strong>
</p>

<p align="center">
  <a href="#功能特性">功能特性</a> •
  <a href="#技术栈">技术栈</a> •
  <a href="#环境要求">环境要求</a> •
  <a href="#安装">安装</a> •
  <a href="#使用指南">使用指南</a> •
  <a href="#开发指南">开发指南</a> •
  <a href="#项目结构">项目结构</a>
</p>

---

## 简介

ComBridge 是一款基于 Tauri 2.0 构建的跨平台串口与 BLE 蓝牙通信调试工具，专为嵌入式开发者、物联网工程师和硬件调试场景设计。支持串口通信、原生 BLE 和 AT 指令 BLE 双模式，提供灵活的协议插件系统、可配置的 Dashboard 仪表盘和波形数据展示，帮助开发者高效地进行设备通信调试。

## 功能特性

### 🔌 串口通信
- 自动扫描可用串口设备
- 支持自定义波特率、数据位、校验位、停止位
- 实时数据收发与十六进制/文本显示
- 数据发送历史记录
- 数据导出功能（日志文件 + 原始数据）

### 📡 BLE 蓝牙通信（双模式）
- **原生 BLE 模式**：直接使用操作系统原生蓝牙 API
- **AT 指令 BLE 模式**：通过串口 + AT 指令控制 BLE 模块（支持 ESP32、N32WB 等）
- 设备扫描与信号强度显示
- GATT 服务/特征浏览
- 特征值读写与通知订阅
- 连接参数配置（MTU、PHY、连接间隔）

### 📜 协议插件系统
- Lua 脚本驱动的协议解析
- 支持自定义数据解析钩子
- 协议与设备灵活绑定
- 内置 GH3036 协议支持
- 预置常用协议解析脚本（CSV、JSON、IMU、NMEA）

### 📊 Dashboard 仪表盘
- 可配置的 Widget 展示系统
- Lua 脚本数据解析
- JSON 配置文件定义 Widget 布局
- 支持多种 Widget 类型：
  - 文本显示
  - 仪表盘
  - LED 指示灯
  - 加速度计
  - 指南针
  - 折线图
- 脚本热加载与动态切换

### 📈 波形数据展示
- 多通道实时波形显示
- 支持 CSV、JSON、二进制等多种数据格式
- 可配置的缓冲区大小
- 数据解析器支持
- CSV 文件导入功能

### 🌐 WebSocket 客户端
- 多连接管理
- 自动重连与心跳维护
- 消息收发监控

### 🎨 现代化界面
- 基于 Ant Design v6.3.5 的美观 UI
- 深色/浅色主题切换
- 多标签页管理
- 国际化支持（中文/英文）
- 自定义无标题栏窗口

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 19 + TypeScript | UI 构建 |
| 状态管理 | Zustand | 轻量级状态管理 |
| UI 组件库 | Ant Design v6.3.5 | 企业级 UI 组件 |
| 路由 | React Router v7 | 页面路由管理 |
| 国际化 | react-i18next | 多语言支持 |
| 图表库 | ECharts + Recharts | 数据可视化 |
| 构建工具 | Vite | 快速开发构建 |
| 后端框架 | Tauri 2.0 (Rust) | 跨平台桌面应用 |
| 异步运行时 | Tokio | Rust 异步运行时 |
| 串口通信 | serialport-rs | 跨平台串口库 |
| BLE 通信 | bluest | Rust 蓝牙库 |
| 脚本引擎 | mlua | Lua 脚本支持 |
| 日志系统 | tracing + tracing-subscriber | 结构化日志 |
| 数据序列化 | Serde + Serde JSON + CSV | 数据序列化 |

## 环境要求

### 运行环境
- **Windows**: Windows 10/11 (x64)
- **macOS**: macOS 10.15+ (Intel & Apple Silicon)
- **Linux**: Ubuntu 18.04+ / Debian 10+ (x64)

### 开发环境
- **Node.js**: >= 18.0.0
- **Rust**: >= 1.70.0
- **pnpm/npm/yarn**: 任意包管理器

#### 平台特定依赖

**Windows**:
- Microsoft Visual Studio C++ Build Tools
- WebView2 (Windows 10/11 已内置)

**macOS**:
- Xcode Command Line Tools: `xcode-select --install`

**Linux (Ubuntu/Debian)**:
```bash
sudo apt install libwebkit2gtk-4.0-dev build-essential curl wget file libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## 安装

### 从发布包安装

前往 [Releases](https://github.com/XiaoPb/combridge/releases) 页面下载对应平台的安装包：

- **Windows**: `.msi` 或 `.exe` 安装程序
- **macOS**: `.dmg` 安装包
- **Linux**: `.AppImage` 或 `.deb` 包

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/XiaoPb/combridge.git
cd combridge

# 安装前端依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## 使用指南

### 串口通信

1. 点击左侧导航栏「串口」进入串口页面
2. 点击「扫描」按钮扫描可用串口
3. 选择目标串口，配置波特率等参数
4. 点击「连接」打开串口
5. 在发送面板输入数据，选择发送格式（文本/十六进制）
6. 接收数据将在数据视图中实时显示
7. 可通过导出功能保存通信数据

### BLE 通信

#### 原生 BLE 模式
1. 点击左侧导航栏「BLE」进入 BLE 页面
2. 选择「原生模式」
3. 点击「扫描」搜索周围 BLE 设备
4. 点击目标设备进行连接
5. 浏览 GATT 服务和特征
6. 对特征进行读写或订阅通知

#### AT 指令 BLE 模式
1. 选择「AT 模式」
2. 配置 AT 模块连接的串口
3. 后续操作与原生模式相同

### Dashboard 仪表盘

1. 点击左侧导航栏「Dashboard」进入仪表盘页面
2. 选择数据源（串口/BLE 设备）
3. 选择解析脚本（或使用预置脚本）
4. 导入或编辑 JSON 配置文件定义 Widget 布局
5. Widget 将实时显示解析后的数据

### 波形数据展示

1. 点击左侧导航栏「波形」进入波形页面
2. 配置波形缓冲区参数
3. 选择数据解析器类型（CSV/JSON/二进制）
4. 选择数据源并开始接收数据
5. 波形图将实时显示数据变化
6. 支持导入 CSV 文件进行离线分析

### 协议插件

1. 点击左侧导航栏「协议」进入协议页面
2. 点击「加载协议」选择 Lua 脚本文件
3. 在绑定配置中将协议绑定到设备
4. 设备数据将自动经过协议解析

## 项目结构

```
combridge-rust/
├── src/                          # 前端源码 (React + TypeScript)
│   ├── api/                      # Tauri API 封装
│   │   ├── index.ts              # API 导出
│   │   ├── tauri.ts              # Tauri 命令封装
│   │   ├── events.ts             # 事件监听封装
│   │   ├── stateApi.ts           # 状态管理 API
│   │   ├── waveform.ts           # 波形 API
│   │   ├── gh3036.ts             # GH3036 API
│   │   └── dashboard.ts          # Dashboard API
│   ├── components/               # 公共组件
│   │   ├── Common/               # 通用组件
│   │   ├── Layout/               # 布局组件
│   │   ├── TitleBar/             # 标题栏组件
│   │   └── DataLogger/           # 数据日志组件
│   ├── hooks/                    # 自定义 Hooks
│   │   ├── useSerial.ts          # 串口 Hook
│   │   ├── useBle.ts             # BLE Hook
│   │   ├── useProtocol.ts        # 协议 Hook
│   │   ├── useWaveform.ts        # 波形 Hook
│   │   ├── useAppDispatch.ts     # 应用调度 Hook
│   │   ├── useAppState.ts        # 应用状态 Hook
│   │   ├── useConnectedDevices.ts # 已连接设备 Hook
│   │   ├── useDataParser.ts      # 数据解析 Hook
│   │   ├── useDebounce.ts        # 防抖 Hook
│   │   ├── useLog.ts             # 日志 Hook
│   │   ├── useModuleSubscribe.ts # 模块订阅 Hook
│   │   ├── useNotification.ts    # 通知 Hook
│   │   └── useTheme.ts           # 主题 Hook
│   ├── i18n/                     # 国际化配置
│   ├── locales/                  # 多语言资源
│   │   ├── zh-CN/                # 中文
│   │   └── en-US/                # 英文
│   ├── pages/                    # 页面组件
│   │   ├── Home/                 # 首页
│   │   ├── Serial/               # 串口页面
│   │   ├── Ble/                  # BLE 页面
│   │   ├── Protocol/             # 协议页面
│   │   ├── Dashboard/            # Dashboard 页面
│   │   ├── Waveform/             # 波形页面
│   │   ├── Gh3036/               # GH3036 页面
│   │   └── System/               # 系统页面
│   ├── services/                 # 服务层
│   │   ├── eventListeners.ts     # 事件监听器
│   │   ├── messageParser.ts      # 消息解析
│   │   └── storageService.ts     # 存储服务
│   ├── stores/                   # Zustand 状态管理
│   │   ├── serialStore.ts        # 串口状态
│   │   ├── bleStore.ts           # BLE 状态
│   │   ├── protocolStore.ts      # 协议状态
│   │   ├── dashboardStore.ts     # Dashboard 状态
│   │   ├── waveformStore.ts      # 波形状态
│   │   └── gh3036Store.ts        # GH3036 状态
│   ├── types/                    # TypeScript 类型定义
│   └── utils/                    # 工具函数
│
├── src-tauri/                    # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── main.rs               # 应用入口
│   │   ├── lib.rs                # 模块导出与配置
│   │   ├── compat.rs             # 系统兼容性检测
│   │   ├── error.rs              # 错误定义
│   │   ├── commands/             # Tauri 命令
│   │   │   ├── serial.rs         # 串口命令
│   │   │   ├── ble.rs            # BLE 命令
│   │   │   ├── protocol.rs       # 协议命令
│   │   │   ├── dashboard.rs      # Dashboard 命令
│   │   │   ├── waveform.rs       # 波形命令
│   │   │   ├── gh3036.rs         # GH3036 命令
│   │   │   ├── state.rs          # 状态命令
│   │   │   ├── system.rs         # 系统命令
│   │   │   └── websocket.rs      # WebSocket 命令
│   │   ├── device/               # 设备管理
│   │   │   ├── device_manager.rs # 设备管理器
│   │   │   ├── serial/           # 串口模块
│   │   │   └── ble/              # BLE 模块
│   │   │       ├── ble_manager.rs
│   │   │       ├── native/       # 原生 BLE
│   │   │       └── at/           # AT 指令 BLE
│   │   ├── dashboard/            # Dashboard 模块
│   │   │   ├── commands.rs       # Dashboard 命令
│   │   │   ├── parser_scripts.rs # 解析脚本管理
│   │   │   └── json_config.rs    # JSON 配置管理
│   │   ├── waveform/             # 波形模块
│   │   │   ├── buffer.rs         # 波形缓冲区
│   │   │   └── parser.rs         # 数据解析器
│   │   ├── gh3036/               # GH3036 协议
│   │   │   ├── manager.rs        # GH3036 管理器
│   │   │   ├── types.rs          # 类型定义
│   │   │   ├── config_loader.rs  # 配置加载
│   │   │   └── factory_test.rs   # 工厂测试
│   │   ├── protocol/             # 协议插件
│   │   │   ├── lua_engine.rs     # Lua 引擎
│   │   │   └── plugin_manager.rs # 插件管理
│   │   ├── service/              # 服务层
│   │   │   ├── event_bus.rs      # 事件总线
│   │   │   ├── logger.rs         # 日志服务
│   │   │   └── config.rs         # 配置服务
│   │   ├── state/                # 状态管理
│   │   │   └── app_state.rs      # 应用状态
│   │   └── websocket/            # WebSocket 客户端
│   ├── parser_scripts/           # 预置 Lua 解析脚本
│   │   ├── csv_parser.lua
│   │   ├── json_parser.lua
│   │   ├── imu_parser.lua
│   │   └── nmea_parser.lua
│   ├── capabilities/             # Tauri 权限配置
│   ├── Cargo.toml                # Rust 依赖配置
│   └── tauri.conf.json           # Tauri 配置
│
├── libs/                         # C/C++ 库
│   └── protocol_rust/            # GH3036 协议库
│       ├── gh-rpc/               # GH-RPC 通信库
│       └── rpc/                  # RPC 框架
│
├── config/                       # 配置文件
│   └── factory/                  # 工厂配置
│       └── factory_config_GH3036.yaml
│
└── docs/                         # 文档
    ├── api.md                    # API 文档
    ├── architecture/             # 架构文档
    │   ├── README.md             # 架构概览
    │   ├── backend/              # 后端模块文档
    │   └── frontend/             # 前端模块文档
    └── device-management.md      # 设备管理文档
```

## 开发指南

### 开发命令

```bash
# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# 构建生产版本
npm run tauri build

# 类型检查
npm run build

# Rust 代码检查
cd src-tauri && cargo clippy

# Rust 代码格式化
cd src-tauri && cargo fmt
```

### 代码规范

- **Rust**: 
  - 使用 `rustfmt` 默认配置
  - `clippy` 严格模式
  - 蛇形命名（snake_case）
  - 错误处理使用 `thiserror` + 自定义 `Result` 类型
  
- **TypeScript/React**: 
  - 函数组件 + Hooks
  - 使用 `interface` 定义 Props
  - 驼峰命名（camelCase）
  - 避免使用 `any`

### 添加新的 Tauri 命令

1. 在 `src-tauri/src/commands/` 对应模块添加函数
2. 使用 `#[tauri::command]` 宏标记
3. 在 `src-tauri/src/lib.rs` 的 `.invoke_handler()` 中注册
4. 在前端 `src/api/` 添加封装函数
5. 更新 `docs/api.md` 文档

### 添加新的协议支持

1. 编写 Lua 协议解析脚本
2. 将脚本放入 `src-tauri/parser_scripts/` 目录
3. 通过「协议」页面加载脚本
4. 绑定到目标设备

### 添加新的 Dashboard Widget

1. 在 `src/pages/Dashboard/widgets/` 创建 Widget 组件
2. 定义 Widget 的配置接口
3. 在 `WidgetRenderer.tsx` 中添加渲染逻辑
4. 更新 JSON 配置 Schema

### 测试

- **单元测试**：核心设备管理逻辑（SerialManager、BleManager）有单元测试
- **集成测试**：BLE 双模式切换、协议加载/绑定流程有集成测试
- **前端测试**：关键页面组件（SerialPage、BlePage）有渲染和交互测试

## 架构文档

详细的架构文档请参阅 [docs/architecture/README.md](docs/architecture/README.md)

### 后端模块文档

- [设备管理](docs/architecture/backend/device-manager.md)
- [串口模块](docs/architecture/backend/serial-module.md)
- [BLE 模块](docs/architecture/backend/ble-module.md)
- [协议插件](docs/architecture/backend/protocol-module.md)
- [Dashboard 模块](docs/architecture/backend/dashboard-module.md)
- [波形模块](docs/architecture/backend/waveform-module.md)
- [GH3036 模块](docs/architecture/backend/gh3036-module.md)
- [状态管理](docs/architecture/backend/state-module.md)
- [服务层](docs/architecture/backend/service-module.md)
- [WebSocket](docs/architecture/backend/websocket-module.md)

### 前端模块文档

- [架构概览](docs/architecture/frontend/overview.md)
- [API 层](docs/architecture/frontend/api-layer.md)
- [状态管理层](docs/architecture/frontend/store-layer.md)
- [Hooks 层](docs/architecture/frontend/hooks-layer.md)
- [页面层](docs/architecture/frontend/pages-layer.md)
- [组件层](docs/architecture/frontend/components-layer.md)

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 贡献

欢迎提交 Issue 和 Pull Request！

### 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/XiaoPb">XiaoPb</a>
</p>
