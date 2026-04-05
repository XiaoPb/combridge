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
  <a href="#开发指南">开发指南</a>
</p>

---

## 简介

ComBridge 是一款基于 Tauri 2.0 构建的跨平台串口与 BLE 蓝牙通信调试工具，专为嵌入式开发者、物联网工程师和硬件调试场景设计。支持串口通信、原生 BLE 和 AT 指令 BLE 双模式，提供灵活的协议插件系统，帮助开发者高效地进行设备通信调试。

## 功能特性

### 🔌 串口通信
- 自动扫描可用串口设备
- 支持自定义波特率、数据位、校验位、停止位
- 实时数据收发与十六进制/文本显示
- 数据发送历史记录

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

### 🌐 WebSocket 客户端
- 多连接管理
- 自动重连与心跳维护
- 消息收发监控

### 🎨 现代化界面
- 基于 Ant Design 的美观 UI
- 深色/浅色主题切换
- 多标签页管理
- 国际化支持（中文/英文）

## 技术栈

| 层级 | 技术 | 说明 |
|------|------|------|
| 前端框架 | React 18 + TypeScript | UI 构建 |
| 状态管理 | Zustand | 轻量级状态管理 |
| UI 组件库 | Ant Design v6 | 企业级 UI 组件 |
| 构建工具 | Vite | 快速开发构建 |
| 后端框架 | Tauri 2.0 (Rust) | 跨平台桌面应用 |
| 异步运行时 | Tokio | Rust 异步运行时 |
| 串口通信 | serialport-rs | 跨平台串口库 |
| BLE 通信 | bluest | Rust 蓝牙库 |
| 脚本引擎 | mlua | Lua 脚本支持 |

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

### 协议插件

1. 点击左侧导航栏「协议」进入协议页面
2. 点击「加载协议」选择 Lua 脚本文件
3. 在绑定配置中将协议绑定到设备
4. 设备数据将自动经过协议解析

## 项目结构

```
combridge/
├── src/                      # 前端源码 (React + TypeScript)
│   ├── api/                  # Tauri API 封装
│   ├── components/           # 公共组件
│   ├── hooks/                # 自定义 Hooks
│   ├── pages/                # 页面组件
│   │   ├── Serial/           # 串口页面
│   │   ├── Ble/              # BLE 页面
│   │   ├── Protocol/         # 协议页面
│   │   └── System/           # 系统页面
│   ├── stores/               # Zustand 状态管理
│   ├── types/                # TypeScript 类型定义
│   └── utils/                # 工具函数
│
├── src-tauri/                # Tauri 后端 (Rust)
│   ├── src/
│   │   ├── commands/         # Tauri 命令
│   │   ├── device/           # 设备管理
│   │   │   ├── serial/       # 串口模块
│   │   │   └── ble/          # BLE 模块
│   │   ├── protocol/         # 协议插件
│   │   ├── service/          # 服务层
│   │   └── websocket/        # WebSocket 客户端
│   ├── Cargo.toml            # Rust 依赖配置
│   └── tauri.conf.json       # Tauri 配置
│
├── libs/                     # C/C++ 库
│   └── gh_protocol/          # GH3036 协议库
│
└── docs/                     # 文档
    ├── api.md                # API 文档
    └── ComBridge Tauri 2.0 架构文档.md
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
```

### 代码规范

- **Rust**: 使用 `rustfmt` 默认配置，`clippy` 严格模式
- **TypeScript**: 函数组件 + Hooks，使用 `interface` 定义 Props
- **命名规范**:
  - Rust: 蛇形命名（snake_case）
  - TypeScript: 驼峰命名（camelCase）

### 添加新的 Tauri 命令

1. 在 `src-tauri/src/commands/` 对应模块添加函数
2. 在 `src-tauri/src/lib.rs` 的 `invoke_handler` 中注册
3. 在前端 `src/api/` 添加封装函数

### 添加新的协议支持

1. 编写 Lua 协议解析脚本
2. 通过「协议」页面加载脚本
3. 绑定到目标设备

## 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件。

## 贡献

欢迎提交 Issue 和 Pull Request！

---

<p align="center">
  Made with ❤️ by <a href="https://github.com/XiaoPb">XiaoPb</a>
</p>
