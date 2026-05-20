# ComBridge

<p align="center">
  <strong>GH3036 传感器芯片产测工具</strong>
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

ComBridge 是一款基于 Tauri 2.0 构建的跨平台桌面工具，专为 GH3036 传感器芯片的产线测试场景设计。通过原生 BLE 蓝牙自动连接设备，执行底噪、PPG 噪声、LPCTR、LPLCTR 等自动化测试项目，并根据可配置的卡控规则输出 PASS/FAIL 结果。

## 功能特性

- **BLE 自动连接**：输入设备名称关键词，一键扫描并自动完成连接、GATT 发现、特征订阅、通道配置全流程
- **自动化产测**：顺序执行底噪、PPG 噪声、LPCTR、LPLCTR 测试，支持中途环境切换确认
- **可配置卡控规则**：通过 YAML 配置文件定义每个测试项的通过条件（阈值、范围、单位），无需修改代码
- **逐通道结果展示**：测试完成后展示每个通道的实测值、判断条件和 PASS/FAIL 状态
- **读写寄存器**：通过 CardiffRPC 命令对设备寄存器进行读写测试（顶部"通道配置"标签页）
- **版本信息查询**：读取固件、协议、算法等各类版本号（顶部"版本信息"标签页）

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
- **Rust**: >= 1.88.0（推荐通过 `rustup update stable` 保持最新）
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

#### 前置依赖

所有平台都需要：
- **Node.js** >= 18.0.0
- **Rust** >= 1.88.0（通过 [rustup](https://rustup.rs/) 安装）
- 本地依赖库 `../libs/protocol_rust/`（需与本仓库同级目录存在）

```bash
# 安装/升级 Rust 到最新稳定版
rustup update stable
```

---

#### macOS

**额外依赖：**
```bash
xcode-select --install
```

**编译（Apple Silicon）：**
```bash
# 添加目标架构（首次需要）
rustup target add aarch64-apple-darwin

git clone https://github.com/XiaoPb/combridge.git
cd combridge
npm install
npm run tauri build -- --target aarch64-apple-darwin
```

**编译（Intel）：**
```bash
rustup target add x86_64-apple-darwin
npm run tauri build -- --target x86_64-apple-darwin
```

产物位于 `src-tauri/target/<target>/release/bundle/`，包含 `.app` 和 `.dmg`。

> **注意**：macOS 蓝牙权限需要 `src-tauri/Info.plist` 中的 `NSBluetoothAlwaysUsageDescription`，该文件已包含在仓库中，无需额外配置。

---

#### Windows

**额外依赖：**
- [Microsoft Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选 "Desktop development with C++"）
- WebView2（Windows 10/11 已内置；Windows 7/8 需手动安装）

**编译：**
```bash
git clone https://github.com/XiaoPb/combridge.git
cd combridge
npm install
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`，包含 `.msi` 和 `.exe` 安装包。

---

#### Linux（Ubuntu / Debian）

**额外依赖：**
```bash
sudo apt update
sudo apt install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libdbus-1-dev \
  libudev-dev
```

**编译：**
```bash
git clone https://github.com/XiaoPb/combridge.git
cd combridge
npm install
npm run tauri build
```

产物位于 `src-tauri/target/release/bundle/`，包含 `.AppImage` 和 `.deb` 包。

> **注意**：Linux 上使用原生 BLE 需要 BlueZ >= 5.50，并确保当前用户在 `bluetooth` 用户组中：
> ```bash
> sudo usermod -aG bluetooth $USER
> ```

---

#### 开发模式（所有平台）

```bash
npm install
npm run tauri dev
```

开发模式会同时启动 Vite 开发服务器（端口 1420）和 Tauri 窗口，支持前端热更新。

## 使用指南

### 界面布局

启动后默认进入产测页面。顶部标签栏有三个标签：

- **产测**：主界面，完成 BLE 连接和自动化测试
- **通道配置**：CardiffRPC 寄存器读写，默认折叠
- **版本信息**：查询设备各类版本号

---

### 第一步：准备配置目录

产测需要一个配置目录，目录中包含以下文件：

| 文件 | 说明 |
|------|------|
| `factory_config_GH3036.yaml` | 卡控规则配置，定义各测试项的通过条件 |
| `Base_Noise_TEST1_*.config` | 底噪测试参数 |
| `PPG_Noise_TEST1_*.config` | PPG 噪声测试参数 |
| `LPCTR_TEST1_*.config` | LPCTR 测试参数 |
| `LPLCTR_TEST1_*.config` | LPLCTR 测试参数 |

`factory_config_GH3036.yaml` 示例：

```yaml
project: "GH3036"
version: "1.0"

tests:
  base_noise:
    enabled: true
    description: "底噪测试"
    unit: "uV"
    global_threshold:
      operator: "lt"
      value: 95
      description: "所有通道底噪应小于 95 uV"

  ppg_noise:
    enabled: true
    description: "PPG噪声测试"
    unit: "uV"
    global_threshold:
      operator: "lt"
      value: 280

  lpctr:
    enabled: true
    description: "LPCTR测试"
    unit: "nA/mA"
    global_threshold:
      operator: "range"
      range: [100, 3000]

  lplctr:
    enabled: true
    description: "LPLCTR测试"
    unit: "nA/mA"
    global_threshold:
      operator: "range"
      range: [0, 6]
```

支持的 `operator`：`lt`（小于）、`le`（小于等于）、`gt`（大于）、`ge`（大于等于）、`eq`（等于）、`ne`（不等于）、`range`（范围，需配合 `range: [min, max]`）。

---

### 第二步：连接设备

在产测页面的"蓝牙连接"卡片中：

1. **设备名称过滤**：默认填写 `ChelseaA_OS`，可修改为实际设备名称的关键词（支持模糊匹配）
2. 点击**扫描并连接**，工具会自动完成：
   - 扫描周围 BLE 设备（超时 15 秒）
   - 按名称过滤，连接第一个匹配的设备
   - 发现 GATT 服务
   - 订阅 RX 特征（`00000003-0000-1000-8000-00805f9b34fb`）
   - 配置 TX/RX 通道
3. 连接成功后卡片显示**已连接**状态

**连接失败排查**：点击"扫描到的设备"可展开查看本次扫描到的所有设备列表（含设备名、MAC 地址、RSSI），确认目标设备是否在广播范围内。

---

### 第三步：选择配置目录

点击"配置目录"卡片中的**选择目录**按钮，选择包含上述配置文件的目录。

选择后工具会自动校验配置文件是否完整，每个配置文件旁显示**就绪**（绿色）或**缺失**（红色）状态。

---

### 第四步：执行测试

配置目录就绪且设备已连接后，点击**开始测试**。

测试按以下顺序自动执行：

1. 底噪测试（Base Noise）
2. PPG 噪声测试（PPG Noise）
3. LPCTR 测试
4. **环境切换确认**：弹出对话框，提示切换测试环境（如遮光/开光），确认后继续
5. LPLCTR 测试

进度条实时显示当前进度，状态标签显示当前阶段。

---

### 第五步：查看结果

测试完成后，结果卡片展示：

- **总体结论**：PASS / FAIL
- **芯片初始化状态**
- **UUID**（可复制）
- **各测试项详情**（可展开/折叠）：每个通道的实测值、判断条件、PASS/FAIL

---

### 读写寄存器

切换到顶部**通道配置**标签页，展开"RPC 命令"面板，可对设备寄存器进行读写操作，用于调试验证。

---

### 查询版本信息

切换到顶部**版本信息**标签页，点击**刷新全部**或单独刷新某一项，读取设备固件、协议、算法等版本号。需要设备已连接（TX 通道已配置）。

## 开发指南（内部）

### 开发命令

```bash
# 安装依赖
npm install

# 启动开发服务器（前端热更新 + Tauri 窗口）
npm run tauri dev

# 构建生产版本
npm run tauri build

# 仅构建前端（输出到 dist/）
npm run build

# TypeScript 类型检查
npx tsc --noEmit

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
