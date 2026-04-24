# GH3036 产测模块工作流程文档

## 1. 模块概述

GH3036 产测模块实现 GH3036/GH3038 芯片的自动化产线测试流程，涵盖芯片初始化、UUID 读取、底噪/PPG 噪声测试、LPCTR/LPLCTR 测试等关键环节。模块采用 Rust 后端 + React 前端的 Tauri 2 架构，通过 EventBus 事件总线实现前后端实时通信。

## 2. 架构总览

```
┌─────────────────────────────────────────────────────────────────┐
│                        前端 (React + TypeScript)                  │
│  FactoryTestTab.tsx ←→ gh3036Store.ts ←→ gh3036.ts (API)       │
│         ↑                         ↑                              │
│    UI 渲染更新              事件监听/状态管理                       │
└─────────┬───────────────────────┬────────────────────────────────┘
          │ Tauri invoke          │ Tauri event (event-bus)
          ↓                       ↓
┌─────────────────────────────────────────────────────────────────┐
│                        后端 (Rust + Tauri 2)                     │
│  commands/gh3036.rs → Gh3036Manager → FactoryTestManager        │
│                                    ↓                             │
│                              EventBus (publish_msgpack)          │
│                                    ↓                             │
│                           EventBridge → Tauri emit → 前端       │
│                                    ↓                             │
│                           RpcCore → 串口/BLE 通信                │
└─────────────────────────────────────────────────────────────────┘
```

## 3. 数据类型定义

### 3.1 产测步骤 (FactoryTestStep)

| 枚举值 | 序号 | 进度区间 | 说明 |
|--------|------|----------|------|
| `Idle` | - | - | 空闲状态 |
| `Prepare` | 1 | 0.00~0.05 | 准备阶段：切换工作模式、关闭功能 |
| `ChipInit` | 2 | 0.05~0.15 | 芯片初始化：FS 0x01 → FG 0x01 |
| `Uuid` | 3 | 0.15~0.25 | UUID 读取：FS 0x02 → FG 0x02 |
| `BaseNoise` | 4 | 0.25~0.40 | 底噪测试：FS 0x04 → 配置下载 → 采集 → FG 0x04 |
| `PpgNoise` | 5 | 0.40~0.55 | PPG 噪声测试：FS 0x08 → 配置下载 → 采集 → FG 0x08 |
| `Lpctr` | 6 | 0.55~0.70 | LPCTR 测试：FS 0x10 → 配置下载 → 采集 → FG 0x10 |
| `EnvironmentSwitch` | 7 | 0.70~0.75 | 环境切换：等待操作员确认 |
| `Lplctr` | 8 | 0.75~0.90 | LPLCTR 测试：FS 0x20 → 配置下载 → 采集 → FG 0x20 |
| `Cleanup` | 9 | 0.90~0.95 | 清理：关闭功能、恢复工作模式 |
| `Completed` | 10 | 0.95~1.00 | 完成 |

### 3.2 产测状态 (FactoryTestStatus)

| 枚举值 | 前端值 | 说明 |
|--------|--------|------|
| `Idle` | `idle` | 空闲 |
| `Running` | `running` | 运行中 |
| `WaitingForEnvironmentSwitch` | `waiting_for_environment_switch` | 等待环境切换确认 |
| `Completed` | `completed` | 已完成 |
| `Failed` | `failed` | 失败 |
| `Stopped` | `stopped` | 已停止 |

### 3.3 产测模式位掩码

| 常量 | 值 | 说明 |
|------|-----|------|
| `FACTORY_TEST_MODE_CHIP_INIT` | 0x01 | 芯片初始化 |
| `FACTORY_TEST_MODE_CHIP_UID` | 0x02 | UUID 读取 |
| `FACTORY_TEST_MODE_BASE_NOISE` | 0x04 | 底噪测试 |
| `FACTORY_TEST_MODE_PPG_NOISE` | 0x08 | PPG 噪声测试 |
| `FACTORY_TEST_MODE_LPCTR` | 0x10 | LPCTR 测试 |
| `FACTORY_TEST_MODE_LPLCTR` | 0x20 | LPLCTR 测试 |

### 3.4 核心数据结构

```rust
struct FactoryTestResult {
    chip_init_status: u16,      // 芯片初始化状态码 (1=成功)
    uuid: Vec<u8>,              // 芯片 UUID (16字节)
    base_noise: Vec<u16>,       // 底噪数据 (每通道 u16)
    ppg_noise: Vec<u16>,        // PPG 噪声数据
    lpctr: Vec<u16>,            // LPCTR 数据
    lplctr: Vec<u16>,           // LPLCTR 数据
    overall_result: String,     // 总体结果 ("PASS"/"FAIL")
    timestamp: u64,             // Unix 时间戳 (毫秒)
}

struct FactoryTestProgressEvent {
    current_step: FactoryTestStep,
    status: FactoryTestStatus,
    step_result: Option<FactoryTestStepResult>,
    progress: f32,              // 0.0~1.0
    message: String,
}

struct ConfigValidationResult {
    base_noise_config: Option<String>,  // 配置文件路径
    ppg_noise_config: Option<String>,
    lpctr_config: Option<String>,
    lplctr_config: Option<String>,
    errors: Vec<String>,
    is_valid: bool,
}
```

## 4. IPC 命令接口

### 4.1 Tauri 命令 (Rust → 前端 invoke)

| 命令名 | 参数 | 返回值 | 说明 |
|--------|------|--------|------|
| `gh3036_factory_test_start` | - | `Result<(), ErrorResponse>` | 启动产测 |
| `gh3036_factory_test_stop` | - | `Result<(), ErrorResponse>` | 停止产测 |
| `gh3036_factory_test_status` | - | `Result<(FactoryTestStatus, FactoryTestStep), ErrorResponse>` | 查询状态 |
| `gh3036_factory_test_continue` | - | `Result<(), ErrorResponse>` | 环境切换后继续 |
| `gh3036_factory_test_set_config_dir` | `config_dir: String` | `Result<(), ErrorResponse>` | 设置配置目录 |
| `gh3036_factory_test_validate_config` | - | `Result<ConfigValidationResult, ErrorResponse>` | 校验配置 |
| `gh3036_factory_test_get_result` | - | `Result<Option<FactoryTestResult>, ErrorResponse>` | 获取结果 |

### 4.2 事件通道 (后端 → 前端 push)

| Topic | 编码 | 数据结构 | 说明 |
|-------|------|----------|------|
| `gh3036:factory_test_progress` | MsgPack+Base64 | `FactoryTestProgressEvent` | 产测进度推送 |

### 4.3 RPC 命令 (产测内部通信)

| Key | 名称 | 参数 | 说明 |
|-----|------|------|------|
| `M` | GHSetWorkModeCmd | mode: u8 | 设置工作模式 |
| `S` | GH3X_SwFunctionCmd | mode: u32, ctrl: u8 | 软件功能开关 |
| `FS` | F_SetMode | mode: u8 | 产测模式设置 |
| `FG` | F_GetMode | mode: u8 | 产测结果获取 |
| `D` | download_config | stage: u8 | 下载配置阶段 |
| `L` | GH3X_RegsListWriteCmd | reg_values: Vec<u16> | 寄存器列表写入 |

## 5. 完整工作流程

### 5.1 流程状态机

```
                    ┌──────────┐
                    │   Idle   │
                    └────┬─────┘
                         │ start_test()
                         ↓
                    ┌──────────┐
              ┌────→│ Running  │←────┐
              │     └────┬─────┘     │
              │          │           │
              │   步骤执行完成       │ continue_test()
              │   (非 EnvironmentSwitch)
              │          │           │
              │          ↓           │
              │  ┌───────────────────┤
              │  │ EnvironmentSwitch │
              │  └───────┬──────────┘
              │          │
              │          ↓
              │  ┌──────────────────────┐
              │  │WaitingForEnvironment │
              │  │      Switch          │
              │  └───────┬──────────────┘
              │          │
              │          └──────────────┘
              │
              │ 步骤失败/stop_test()
              ↓
     ┌──────────────┐
     │ Failed/Stopped│
     └──────┬───────┘
            │
            ↓
     ┌───────────┐
     │ Completed │
     └───────────┘
```

### 5.2 详细步骤流程

#### 步骤 1: Prepare (准备)

```
输入: 无
处理:
  1. 发送 RPC 命令 M(2) → 切换工作模式为 2
  2. 等待 ACK 响应
  3. 延时 100ms
  4. 发送 RPC 命令 S(0x0, 0) → 关闭全部功能
  5. 等待 ACK 响应
输出: FactoryTestStepResult { success: true, message: "准备步骤完成" }
异常: RPC 超时 → 返回 Failed 状态
```

#### 步骤 2: ChipInit (芯片初始化)

```
输入: 无
处理:
  1. 发送 RPC 命令 FS(0x01) → 设置产测模式为 CHIP_INIT
  2. 等待 ACK 响应
  3. 延时 100ms
  4. 发送 RPC 命令 FG(0x01) → 获取芯片初始化结果
  5. 解包响应: U16Array → 取第一个值作为 chip_init_status
输出: FactoryTestStepResult { success: true, data: [chip_init_status] }
      chip_init_status = 1 表示成功
异常: RPC 超时 → 返回 Failed 状态
```

#### 步骤 3: Uuid (UUID 读取)

```
输入: 无
处理:
  1. 发送 RPC 命令 FS(0x02) → 设置产测模式为 CHIP_UID
  2. 等待 ACK 响应
  3. 延时 100ms
  4. 发送 RPC 命令 FG(0x02) → 获取 UUID 结果
  5. 解包响应: U16Array → 转换为字节数组 (每 u16 拆为 2 字节)
输出: FactoryTestStepResult { success: true, data: [uuid_bytes as u16] }
      UUID 格式: XX:XX:XX:XX:... (16字节，冒号分隔的十六进制)
异常: RPC 超时 → 返回 Failed 状态
```

#### 步骤 4: BaseNoise (底噪测试)

```
输入: 配置目录中的 base_noise 配置文件
处理:
  1. 查找配置文件 (文件名包含 "base_noise"，扩展名 .config/.ini)
  2. 解析配置文件 → 获取寄存器列表
  3. 发送 RPC 命令 FS(0x04) → 设置产测模式为 BASE_NOISE
  4. 发送 RPC 命令 D(0) → 下载配置阶段 0
  5. 发送 RPC 命令 L(reg_values) → 写入寄存器列表
  6. 发送 RPC 命令 D(1) → 下载配置阶段 1
  7. 发送 RPC 命令 S(0x1, 1) → 启动 TEST1 功能
  8. 延时 3 秒 (采集数据)
  9. 发送 RPC 命令 S(0x1, 0) → 停止 TEST1 功能
  10. 延时 1 秒
  11. 发送 RPC 命令 FG(0x04) → 获取底噪结果
  12. 解包响应: 每 2 字节组合为 u16 (小端序)
输出: FactoryTestStepResult { success: true, data: [base_noise_channels] }
异常: 配置文件缺失/寄存器列表为空/RPC 超时 → 返回 Failed 状态
```

#### 步骤 5: PpgNoise (PPG 噪声测试)

```
输入: 配置目录中的 ppg_noise 配置文件
处理:
  1. 查找配置文件 (文件名包含 "ppg_noise"，扩展名 .config/.ini)
  2. 解析配置文件 → 获取寄存器列表
  3. 发送 RPC 命令 FS(0x08) → 设置产测模式为 PPG_NOISE
  4. 发送 RPC 命令 D(0) → 下载配置阶段 0
  5. 发送 RPC 命令 L(reg_values) → 写入寄存器列表
  6. 发送 RPC 命令 D(1) → 下载配置阶段 1
  7. 发送 RPC 命令 S(0x1, 1) → 启动 TEST1 功能
  8. 延时 3 秒 (采集数据)
  9. 发送 RPC 命令 S(0x1, 0) → 停止 TEST1 功能
  10. 延时 1 秒
  11. 发送 RPC 命令 FG(0x08) → 获取 PPG 噪声结果
  12. 解包响应: 每 2 字节组合为 u16 (小端序)
输出: FactoryTestStepResult { success: true, data: [ppg_noise_channels] }
异常: 配置文件缺失/寄存器列表为空/RPC 超时 → 返回 Failed 状态
```

#### 步骤 6: Lpctr (LPCTR 测试)

```
输入: 配置目录中的 lpctr 配置文件
处理:
  1. 查找配置文件 (文件名包含 "lpctr"，扩展名 .config/.ini)
  2. 解析配置文件 → 获取寄存器列表
  3. 发送 RPC 命令 FS(0x10) → 设置产测模式为 LPCTR
  4. 发送 RPC 命令 D(0) → 下载配置阶段 0
  5. 发送 RPC 命令 L(reg_values) → 写入寄存器列表
  6. 发送 RPC 命令 D(1) → 下载配置阶段 1
  7. 发送 RPC 命令 S(0x1, 1) → 启动 TEST1 功能
  8. 延时 3 秒 (采集数据)
  9. 发送 RPC 命令 S(0x1, 0) → 停止 TEST1 功能
  10. 延时 1 秒
  11. 发送 RPC 命令 FG(0x10) → 获取 LPCTR 结果
  12. 解包响应: 每 2 字节组合为 u16 (小端序)
输出: FactoryTestStepResult { success: true, data: [lpctr_channels] }
异常: 配置文件缺失/寄存器列表为空/RPC 超时 → 返回 Failed 状态
```

#### 步骤 7: EnvironmentSwitch (环境切换)

```
输入: 无
处理:
  1. execute_step 返回 Ok(None) (无实际操作)
  2. 外层循环将状态设置为 WaitingForEnvironmentSwitch
  3. 发布进度事件: step=EnvironmentSwitch, status=WaitingForEnvironmentSwitch
  4. 进入轮询等待 (每 100ms 检查一次状态)
  5. 前端弹出环境切换确认弹窗
  6. 操作员确认后调用 gh3036_factory_test_continue
  7. 状态恢复为 Running，轮询退出
输出: 无 (跳过步骤)
异常: stop_test() → running 标志置 false，轮询退出
```

#### 步骤 8: Lplctr (LPLCTR 测试)

```
输入: 配置目录中的 lplctr 配置文件
处理:
  1. 查找配置文件 (文件名包含 "lplctr"，扩展名 .config/.ini)
  2. 解析配置文件 → 获取寄存器列表
  3. 发送 RPC 命令 FS(0x20) → 设置产测模式为 LPLCTR
  4. 发送 RPC 命令 D(0) → 下载配置阶段 0
  5. 发送 RPC 命令 L(reg_values) → 写入寄存器列表
  6. 发送 RPC 命令 D(1) → 下载配置阶段 1
  7. 发送 RPC 命令 S(0x1, 1) → 启动 TEST1 功能
  8. 延时 3 秒 (采集数据)
  9. 发送 RPC 命令 S(0x1, 0) → 停止 TEST1 功能
  10. 延时 1 秒
  11. 发送 RPC 命令 FG(0x20) → 获取 LPLCTR 结果
  12. 解包响应: 每 2 字节组合为 u16 (小端序)
输出: FactoryTestStepResult { success: true, data: [lplctr_channels] }
异常: 配置文件缺失/寄存器列表为空/RPC 超时 → 返回 Failed 状态
```

#### 步骤 9: Cleanup (清理)

```
输入: 无
处理:
  1. 发送 RPC 命令 S(0x0, 0) → 关闭全部功能
  2. 延时 100ms
  3. 发送 RPC 命令 M(0) → 切换工作模式回 0
输出: FactoryTestStepResult { success: true, message: "清理步骤完成" }
异常: RPC 超时 → 返回 Failed 状态
```

#### 步骤 10: Completed (完成)

```
处理:
  1. 发布完成事件: step=Completed, status=Completed, progress=1.0
  2. 保存结果到 CSV 文件: data/factory/factory_YYYY-MM-DD.csv
  3. 将结果存入 result_clone 供前端查询
  4. 状态恢复为 Idle
```

## 6. 事件数据流转路径

### 6.1 进度事件流转

```
FactoryTestManager.publish_progress_static()
    ↓ 构造 FactoryTestProgressEvent
    ↓ rmp_serde::to_vec() 序列化为 MsgPack
EventBus.publish_msgpack("gh3036:factory_test_progress", &event)
    ↓ broadcast::Sender::send() 发送到 channel
    ↓ 遍历 subscribers 回调 (Rust 后端内部订阅者)
EventBridge (tokio 异步任务)
    ↓ broadcast::Receiver::recv() 接收
    ↓ EventFilter.matches() 过滤
    ↓ Base64 编码 MsgPack 数据
    ↓ 构造 JSON wrapper: { topic, payload, timestamp, encoding }
Tauri AppHandle.emit("event-bus", wrapper)
    ↓ Tauri IPC 事件系统
前端 gh3036Store.subscribeFactoryTestEvents()
    ↓ listen('event-bus', callback) 接收
    ↓ 检查 topic === 'gh3036:factory_test_progress'
    ↓ decodePayload<FactoryTestProgressEvent>() 解码
    ↓ Base64 解码 → MsgPack 反序列化 → 数组转对象映射
Zustand Store set()
    ↓ 更新 factoryTest 状态
React 组件重渲染
    ↓ FactoryTestTab 读取 store 状态
UI 更新 (进度条、状态标签、结果展示)
```

### 6.2 MsgPack 数组转对象映射

```typescript
// msgpack.ts 中的映射逻辑
case 'gh3036:factory_test_progress':
    return {
        current_step: arr[0],    // FactoryTestStep
        status: arr[1],          // FactoryTestStatus
        step_result: arr[2],     // Option<FactoryTestStepResult>
        progress: arr[3],        // f32
        message: arr[4],         // String
    };
```

### 6.3 RPC 命令执行路径

```
FactoryTestManager.execute_step()
    ↓ rt.block_on() 在独立线程的 Tokio runtime 中执行
Gh3036Manager.execute_rpc(key, params)
    ↓ 匹配命令 key
    ↓ 构造命令数据
RpcCore.send_command() / call_command() / send_all_and_wait()
    ↓ 帧封装 (协议帧头 + 数据 + 校验)
    ↓ 通过 TX 通道发送
DeviceManager.send_direct()
    ↓ 串口写入 / BLE 特征值写入
硬件设备
    ↓ 响应数据
SerialPort 读取线程
    ↓ 组包 (50ms 超时)
EventBus.publish_msgpack("serial:data")
    ↓
Gh3036Manager 事件回调
    ↓ 解析协议帧
RpcCore.process(data)
    ↓ 帧解析 → handle_parse_result
    ↓ 匹配 pending call / static node
    ↓ oneshot::Sender::send() 返回结果
FactoryTestManager 收到响应
```

## 7. 配置文件规范

### 7.1 配置目录结构

```
config_dir/
├── Base_Noise_TEST1_100Hz_0327.config   # 底噪测试配置
├── PPG_Noise_TEST1_100Hz_0327.config    # PPG 噪声测试配置
├── LPCTR_TEST1_100Hz_0327.config        # LPCTR 测试配置
└── LPLCTR_TEST1_100Hz_0327.config       # LPLCTR 测试配置
```

### 7.2 配置文件查找规则

- 在指定目录中搜索文件名包含关键字（不区分大小写）的文件
- 关键字: `base_noise`、`ppg_noise`、`lpctr`、`lplctr`
- 扩展名: `.config` 或 `.ini`
- 每个关键字必须唯一匹配一个文件，多个匹配视为错误

### 7.3 配置文件格式

配置文件由 ConfigLoader 解析，每行包含一个寄存器地址-值对，格式为：
```
0xADDR 0xVALUE
```
最后一行以 `0xFFFF 0x0001` 结尾作为结束标记。

## 8. 结果存储

### 8.1 CSV 文件格式

- 路径: `<可执行文件目录>/data/factory/factory_YYYY-MM-DD.csv`
- 追加模式写入，同一天的结果追加到同一文件
- 首次创建时写入表头

### 8.2 CSV 列定义

| 列名 | 类型 | 说明 |
|------|------|------|
| timestamp | u64 | Unix 时间戳 (毫秒) |
| overall_result | String | "PASS" 或 "FAIL" |
| chip_init_status | u16 | 芯片初始化状态码 |
| uuid | String | UUID 十六进制 (冒号分隔) |
| base_noise | String | 底噪数据 (竖线分隔) |
| ppg_noise | String | PPG 噪声数据 (竖线分隔) |
| lpctr | String | LPCTR 数据 (竖线分隔) |
| lplctr | String | LPLCTR 数据 (竖线分隔) |

## 9. 异常处理机制

### 9.1 错误分类

| 错误类型 | 处理方式 | 状态转换 |
|----------|----------|----------|
| 配置文件缺失 | start_test 前校验，拒绝启动 | Idle → Idle |
| 配置文件重复 | 返回错误信息 | Idle → Idle |
| RPC 超时 (1s) | 步骤失败，终止流程 | Running → Failed |
| RPC 发送失败 | 步骤失败，终止流程 | Running → Failed |
| 寄存器列表为空 | 步骤失败，终止流程 | Running → Failed |
| 用户手动停止 | 设置 running=false | Running → Stopped |
| 串口断开 | RPC 发送失败，步骤失败 | Running → Failed |

### 9.2 状态码定义

| 状态码 | 含义 |
|--------|------|
| chip_init_status = 1 | 芯片初始化成功 |
| chip_init_status = 0 | 芯片初始化失败 |
| overall_result = "PASS" | 全部步骤通过 |
| overall_result = "FAIL" | 任一步骤失败 |

### 9.3 线程安全机制

- `running: Arc<AtomicBool>` — 控制产测线程运行/停止
- `status: Mutex<FactoryTestStatus>` — 主状态锁
- `status_clone: Arc<Mutex<FactoryTestStatus>>` — 线程间状态同步（用于环境切换等待）
- `result: Mutex<Option<FactoryTestResult>>` — 结果锁
- `result_clone: Arc<Mutex<Option<FactoryTestResult>>>` — 线程间结果同步
- `thread_handle: Mutex<Option<JoinHandle>>` — 线程句柄（用于 join 等待）

## 10. 已知问题与技术债

### 10.1 EventBus subscriber_count=0

产测进度事件发布时 `subscriber_count=0` 是预期行为。EventBus 的 subscriber_count 仅统计 Rust 后端的 `subscribe_sync` 回调数，前端通过 EventBridge 的 broadcast channel 接收事件，不计入此统计。

### 10.2 RPC 帧合并响应

当串口组包将多个协议帧合并为一次读取时，RPC 核心会依次处理每个帧。第一个帧匹配 pending call 并消费，后续帧产生 "No pending call found" 日志（已降级为 Debug 级别）。这是正常行为，不影响功能。

### 10.3 Antd message 静态方法

当前使用 `message.success()`/`message.error()` 静态方法，Antd v6 建议使用 `App.useApp()` hook 获取 message 实例以支持动态主题。此为技术债，需后续统一迁移。

### 10.4 React StrictMode 双重渲染

开发模式下 React StrictMode 会导致 FactoryTestTab 组件双重挂载/卸载，表现为事件订阅/取消订阅的重复调用。生产构建不受影响。

## 11. 文件索引

| 文件路径 | 职责 |
|----------|------|
| `src-tauri/src/gh3036/factory_test.rs` | 产测流程核心逻辑 |
| `src-tauri/src/gh3036/types.rs` | 产测数据类型定义 |
| `src-tauri/src/gh3036/manager.rs` | GH3036 管理器（产测集成入口） |
| `src-tauri/src/gh3036/config_loader.rs` | 配置文件加载器 |
| `src-tauri/src/commands/gh3036.rs` | Tauri IPC 命令层 |
| `src-tauri/src/service/event_bus.rs` | 事件总线 |
| `src-tauri/src/service/event_bridge.rs` | 事件桥接（后端→前端） |
| `src/pages/Gh3036/FactoryTestTab.tsx` | 产测 UI 组件 |
| `src/stores/gh3036Store.ts` | Zustand 状态管理 |
| `src/api/gh3036.ts` | API 调用层 |
| `src/api/types.ts` | TypeScript 类型定义 |
| `src/utils/msgpack.ts` | MsgPack 解码工具 |
| `src/services/eventListeners.ts` | 事件监听服务 |
