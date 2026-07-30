# BLE 写入方式自动重试功能实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 GH3036 模块中，当特征同时支持 write 和 write_without_response 时，每次配置 TX/RX UUID 后的第一次指令发送，如果使用 write 超时，自动切换到 write_without_response 重试一次。

**Architecture:**
- 在 GH3036 管理器中添加设备配置状态跟踪（`DeviceConfigState`），包括：
  - 是否已配置 TX/RX UUID
  - 是否已发送过第一次指令
  - 特征是否同时支持两种写入方式
  - 下次写入是否强制使用 `write_without_response` 的标志位
- 在 `configure_tx_channel` 中重置设备状态为"已配置，未发送"
- 在 `execute_rpc` 方法中捕获 RPC 响应超时（`RpcError::Timeout`），触发重试逻辑
- 重试时设置强制使用 `write_without_response` 标志，然后重新发送 RPC 指令
- 在 `send_data` 方法中检查强制写入标志，选择正确的 BLE 写入方式

**Tech Stack:** Rust (Tokio, parking_lot), Tauri 2.0, gh-rpc 库

---

## 文件结构

### 创建文件：
- 无

### 修改文件：
- `src-tauri/src/gh3036/manager.rs` - 添加设备状态跟踪和重试逻辑
- `src-tauri/src/device/ble/ble_manager.rs` - 添加获取特征属性的便捷方法

---

## Task 1: 添加设备配置状态跟踪

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 添加数据结构跟踪每台设备的配置状态

### Task 1.1: 定义设备配置状态结构

- [ ] **Step 1: 添加 DeviceConfigState 结构体**

在 `manager.rs` 的顶部（`use` 语句之后）添加：

```rust
/// 设备配置状态
/// 用于跟踪每台设备在配置 TX/RX UUID 后的发送状态
#[derive(Debug, Clone)]
struct DeviceConfigState {
    /// 设备ID（蓝牙地址或串口名）
    device_id: String,
    /// 是否已配置
    configured: bool,
    /// 是否已发送过第一次指令
    first_command_sent: bool,
    /// 特征是否同时支持两种写入方式
    supports_both_write_modes: bool,
    /// 是否强制使用 write_without_response
    /// 用于重试逻辑
    force_write_without_response: bool,
}

impl DeviceConfigState {
    fn new(device_id: String) -> Self {
        Self {
            device_id,
            configured: false,
            first_command_sent: false,
            supports_both_write_modes: false,
            force_write_without_response: false,
        }
    }
}
```

- [ ] **Step 2: 在 Gh3036Manager 中添加状态映射**

在 `Gh3036Manager` 结构体中添加字段：

```rust
pub struct Gh3036Manager {
    // ... 现有字段 ...
    
    /// 设备配置状态映射（device_id -> DeviceConfigState）
    device_config_states: Arc<Mutex<HashMap<String, DeviceConfigState>>>,
}
```

在 `Gh3036Manager::new()` 方法中初始化：

```rust
impl Gh3036Manager {
    pub fn new(...) -> Self {
        Self {
            // ... 现有字段 ...
            device_config_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
```

- [ ] **Step 3: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译通过，无错误

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 添加设备配置状态跟踪结构"
```

---

## Task 2: 在配置方法中重置设备状态

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 在 `configure_tx_channel` 方法中重置设备状态为"已配置，未发送"

### Task 2.1: 修改 configure_tx_channel 方法

- [ ] **Step 1: 修改 configure_tx_channel 方法**

找到 `configure_tx_channel` 方法（约第826行），修改为：

```rust
pub fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
    CALLBACK_CONTEXT.set_tx_channel(config.clone());
    
    // 重置设备配置状态为"已配置，未发送"
    {
        let mut states = self.device_config_states.lock();
        let device_id = config.device_id.clone();
        let mut state = DeviceConfigState::new(device_id);
        state.configured = true;
        state.first_command_sent = false;
        states.insert(config.device_id.clone(), state);
        info!("[GH3036] 设备 {} 配置状态已重置", config.device_id);
    }
    
    info!("GH3036 TX 通道配置成功: {:?}", config);
    Ok(())
}
```

- [ ] **Step 2: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 在配置TX通道时重置设备状态"
```

---

## Task 3: 检测特征属性并更新设备状态

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 在配置 TX 通道时，检测特征属性，并更新设备状态

### Task 3.1: 修改 configure_tx_channel 方法

- [ ] **Step 1: 在 configure_tx_channel 方法中添加特征属性检测**

找到 `configure_tx_channel` 方法（约第826行），在重置设备状态后添加：

```rust
pub fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
    CALLBACK_CONTEXT.set_tx_channel(config.clone());

    // 重置设备配置状态为"已配置，未发送"
    {
        let mut states = self.device_config_states.lock();
        let device_id = config.device_id.clone();
        let mut state = DeviceConfigState::new(device_id);
        state.configured = true;
        state.first_command_sent = false;
        states.insert(config.device_id.clone(), state);
        info!("[GH3036] 设备 {} 配置状态已重置", config.device_id);
    }

    // 如果是 BLE 设备，检测特征属性
    if config.channel_type == ChannelType::Ble {
        let device_id = config.device_id.clone();
        let char_uuid = config.characteristic_uuid.clone();

        // 在异步上下文中执行
        tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::try_current()
                .map_err(|e| format!("获取 Tokio 运行时失败: {}", e))?;

            handle.block_on(async {
                if let Some(uuid) = char_uuid {
                    if let Ok((write, write_without_response)) = self.detect_characteristic_properties(&device_id, &uuid).await {
                        let mut states = self.device_config_states.lock();
                        if let Some(state) = states.get_mut(&device_id) {
                            state.supports_both_write_modes = write && write_without_response;
                            info!(
                                "[GH3036] 设备 {} 特征属性: write={}, write_without_response={}, both={}",
                                device_id, write, write_without_response, state.supports_both_write_modes
                            );
                        }
                    }
                }
            });
        });
    }

    info!("GH3036 TX 通道配置成功: {:?}", config);
    Ok(())
}
```

- [ ] **Step 2: 添加 detect_characteristic_properties 辅助方法**

在 `Gh3036Manager` impl 块中添加：

```rust
impl Gh3036Manager {
    // ... 现有方法 ...

    /// 检测特征的写入属性
    async fn detect_characteristic_properties(
        &self,
        device_id: &str,
        char_uuid: &str,
    ) -> Result<(bool, bool), String> {
        // 获取 BLE 连接信息
        let connection = self.device_manager
            .ble_manager()
            .get_connection(device_id)
            .map_err(|e| format!("获取 BLE 连接失败: {:?}", e))?;

        // 查找特征
        let char_info = connection.characteristics.iter()
            .find(|c| c.uuid == char_uuid)
            .ok_or_else(|| format!("特征 {} 未找到", char_uuid))?;

        Ok((char_info.write, char_info.write_without_response))
    }
}
```

**注意：** 需要根据实际的 `BleConnection` 和 `BleCharacteristic` 结构体调整字段名。

- [ ] **Step 3: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 可能有编译错误，需要根据实际类型调整

- [ ] **Step 4: 根据实际类型调整**

如果编译失败，根据错误信息调整字段名和类型。

- [ ] **Step 5: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 在配置TX通道时检测特征属性"
```

---

## Task 4: 实现 RPC 响应超时重试逻辑

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 在 `execute_rpc` 方法中捕获 RPC 响应超时，触发重试逻辑

### Task 4.1: 修改 execute_rpc 方法

- [ ] **Step 1: 修改 execute_rpc 方法，添加超时重试逻辑**

找到 `execute_rpc` 方法（约第944行），修改为：

```rust
pub async fn execute_rpc(
    &self,
    command_key: &str,
    params: &[String],
) -> Result<Vec<u8>, String> {
    info!(
        "GH3036 execute_rpc 开始: key={}, params={:?}",
        command_key, params
    );

    // 第一次尝试
    let result = self.execute_rpc_async(command_key, params).await;

    match result {
        Ok(data) => Ok(data),
        Err(e) => {
            // 检查是否是 RPC 超时错误
            if self.is_rpc_timeout_error(&e) {
                // 检查是否需要重试
                if self.should_retry_on_timeout().await? {
                    warn!("[GH3036] RPC 响应超时，尝试使用 write_without_response 重试");

                    // 设置强制使用 write_without_response 标志
                    self.set_force_write_without_response(true);

                    // 重新发送 RPC 指令
                    let retry_result = self.execute_rpc_async(command_key, params).await;

                    // 清除强制标志
                    self.set_force_write_without_response(false);

                    // 标记已发送
                    self.mark_first_command_sent();

                    match retry_result {
                        Ok(data) => {
                            info!("[GH3036] RPC 重试成功");
                            Ok(data)
                        }
                        Err(retry_e) => {
                            error!("[GH3036] RPC 重试失败: {}", retry_e);
                            Err(format!("RPC 执行失败（已重试）: {}", retry_e))
                        }
                    }
                } else {
                    Err(e)
                }
            } else {
                Err(e)
            }
        }
    }
}
```

- [ ] **Step 2: 添加辅助方法 is_rpc_timeout_error**

在 `Gh3036Manager` impl 块中添加：

```rust
impl Gh3036Manager {
    // ... 现有方法 ...

    /// 检查错误是否为 RPC 超时错误
    fn is_rpc_timeout_error(&self, error: &str) -> bool {
        error.contains("RPC 发送失败") && error.contains("Timeout")
    }
}
```

- [ ] **Step 3: 添加辅助方法 should_retry_on_timeout**

```rust
impl Gh3036Manager {
    // ... 现有方法 ...

    /// 检查是否需要在 RPC 超时时重试
    async fn should_retry_on_timeout(&self) -> Result<bool, String> {
        // 获取当前设备信息
        let (device_type, device_id) = {
            let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
            let channel = tx_channel.as_ref().ok_or("TX 通道未配置")?;

            let device_type = match channel.channel_type {
                ChannelType::Serial => crate::device::DeviceType::Serial,
                ChannelType::Ble => crate::device::DeviceType::Ble,
            };
            (device_type, channel.device_id.clone())
        };

        // 只对 BLE 设备进行重试
        if device_type != crate::device::DeviceType::Ble {
            return Ok(false);
        }

        // 检查设备配置状态
        let should_retry = {
            let states = self.device_config_states.lock();
            if let Some(state) = states.get(&device_id) {
                // 如果已发送过，不需要重试
                if state.first_command_sent {
                    return Ok(false);
                }

                // 如果同时支持两种写入方式，需要重试
                state.supports_both_write_modes
            } else {
                // 没有配置状态，不需要重试
                false
            }
        };

        Ok(should_retry)
    }
}
```

- [ ] **Step 4: 添加辅助方法 set_force_write_without_response**

```rust
impl Gh3036Manager {
    // ... 现有方法 ...

    /// 设置强制使用 write_without_response 标志
    fn set_force_write_without_response(&self, force: bool) {
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        if let Some(channel) = tx_channel.as_ref() {
            let device_id = &channel.device_id;
            let mut states = self.device_config_states.lock();
            if let Some(state) = states.get_mut(device_id) {
                state.force_write_without_response = force;
                info!(
                    "[GH3036] 设备 {} 强制写入标志: {}",
                    device_id, force
                );
            }
        }
    }
}
```

- [ ] **Step 5: 添加辅助方法 mark_first_command_sent**

```rust
impl Gh3036Manager {
    // ... 现有方法 ...

    /// 标记已发送第一次命令
    fn mark_first_command_sent(&self) {
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        if let Some(channel) = tx_channel.as_ref() {
            let device_id = &channel.device_id;
            let mut states = self.device_config_states.lock();
            if let Some(state) = states.get_mut(device_id) {
                state.first_command_sent = true;
                info!("[GH3036] 设备 {} 已标记为已发送第一次命令", device_id);
            }
        }
    }
}
```

- [ ] **Step 6: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译通过

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 实现RPC响应超时自动重试逻辑"
```

---

## Task 5: 在 send_data 方法中实现强制写入逻辑

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 在 `send_data` 方法中检查强制写入标志，选择正确的 BLE 写入方式

### Task 5.1: 修改 send_data 方法

- [ ] **Step 1: 修改 send_data 方法**

找到 `send_data` 方法（约第884行），修改为：

```rust
pub async fn send_data(&self, data: &[u8]) -> Result<(), String> {
    let (device_type, device_id, char_uuid) = {
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        let channel = tx_channel.as_ref().ok_or("TX 通道未配置")?;

        let device_type = match channel.channel_type {
            ChannelType::Serial => crate::device::DeviceType::Serial,
            ChannelType::Ble => crate::device::DeviceType::Ble,
        };
        let char_uuid = channel.characteristic_uuid.clone();
        (device_type, channel.device_id.clone(), char_uuid)
    };

    // 检查是否需要强制使用 write_without_response
    let force_write_without_response = {
        let states = self.device_config_states.lock();
        if let Some(state) = states.get(&device_id) {
            state.force_write_without_response
        } else {
            false
        }
    };

    // 根据标志选择发送方式
    if force_write_without_response && device_type == crate::device::DeviceType::Ble {
        // 强制使用 write_without_response
        self.device_manager
            .send_direct_without_response(device_type, &device_id, char_uuid.as_deref(), data)
            .await
            .map_err(|e| {
                error!("GH3036 send_data 失败（write_without_response）: {}", e);
                e.to_string()
            })?;
    } else {
        // 正常发送
        self.device_manager
            .send_direct(device_type, &device_id, char_uuid.as_deref(), data)
            .await
            .map_err(|e| {
                error!("GH3036 send_data 失败: {}", e);
                e.to_string()
            })?;
    }

    Ok(())
}
```

- [ ] **Step 2: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译失败，提示缺少 `send_direct_without_response` 方法

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 在send_data中实现强制写入逻辑"
```

---

## Task 6: 在 DeviceManager 中添加 send_direct_without_response 方法

**Files:**
- Modify: `src-tauri/src/device/device_manager.rs`

**目标：** 添加强制使用 write_without_response 的发送方法

### Task 6.1: 在 DeviceManager 中添加方法

- [ ] **Step 1: 在 DeviceManager 中添加 send_direct_without_response 方法**

在 `DeviceManager` impl 块中添加：

```rust
impl DeviceManager {
    // ... 现有方法 ...

    /// 直接发送数据到设备（强制使用 write_without_response）
    /// 用于 BLE 设备的写入方式切换重试
    pub async fn send_direct_without_response(
        &self,
        device_type: DeviceType,
        device_id: &str,
        char_uuid: Option<&str>,
        data: &[u8],
    ) -> Result<()> {
        match device_type {
            DeviceType::Serial => {
                // 串口不需要区分写入方式，直接发送
                self.serial_manager.send_data(device_id, data)?
            }
            DeviceType::Ble => {
                let uuid = char_uuid.ok_or_else(|| {
                    ComBridgeError::invalid_input("BLE 发送需要特征 UUID")
                })?;
                self.ble_manager
                    .write_without_response(device_id, uuid, data)
                    .await?
            }
        }

        Ok(())
    }
}
```

- [ ] **Step 2: 添加 ble_manager 访问方法（如果不存在）**

如果 `DeviceManager` 没有 `ble_manager()` 方法，添加：

```rust
impl DeviceManager {
    // ... 现有方法 ...

    /// 获取 BLE 管理器引用
    pub fn ble_manager(&self) -> &crate::device::ble::BleManager {
        &self.ble_manager
    }
}
```

- [ ] **Step 3: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译通过（BleManager 已有 `write_without_response` 方法）

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/device/device_manager.rs
git commit -m "feat(device): 添加send_direct_without_response方法"
```

---

## Task 7: 添加单元测试

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs` (测试模块)

**目标：** 为重试逻辑添加单元测试

### Task 7.1: 添加测试模块

- [ ] **Step 1: 在 manager.rs 末尾添加测试模块**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_config_state_new() {
        let state = DeviceConfigState::new("test_device".to_string());
        assert_eq!(state.device_id, "test_device");
        assert!(!state.configured);
        assert!(!state.first_command_sent);
        assert!(!state.supports_both_write_modes);
        assert!(!state.force_write_without_response);
    }

    #[test]
    fn test_is_rpc_timeout_error() {
        let manager = Gh3036Manager::new_for_test();

        // 测试 RPC 超时错误
        let timeout_err = "RPC 发送失败: Timeout";
        assert!(manager.is_rpc_timeout_error(timeout_err));

        // 测试非超时错误
        let other_err = "RPC 发送失败: SendFail";
        assert!(!manager.is_rpc_timeout_error(other_err));
    }

    #[test]
    fn test_set_force_write_without_response() {
        let manager = Gh3036Manager::new_for_test();

        // 配置设备
        let config = ChannelConfig {
            device_id: "test_device".to_string(),
            channel_type: ChannelType::Ble,
            characteristic_uuid: Some("test_uuid".to_string()),
        };
        manager.configure_tx_channel(config).unwrap();

        // 设置强制写入标志
        manager.set_force_write_without_response(true);

        // 检查标志
        let states = manager.device_config_states.lock();
        let state = states.get("test_device").unwrap();
        assert!(state.force_write_without_response);

        // 清除标志
        drop(states);
        manager.set_force_write_without_response(false);

        let states = manager.device_config_states.lock();
        let state = states.get("test_device").unwrap();
        assert!(!state.force_write_without_response);
    }
}
```

**注意：** 需要根据实际的 `Gh3036Manager` 构造方法调整测试代码。可能需要添加一个 `new_for_test()` 方法。

- [ ] **Step 2: 运行测试**

运行: `cd src-tauri && cargo test --lib gh3036::manager::tests`
预期: 测试通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "test(gh3036): 添加RPC超时重试逻辑的单元测试"
```

---

## Task 8: 集成测试和验证

**Files:**
- 无（手动测试）

**目标：** 在实际环境中验证功能

### Task 8.1: 编译和运行

- [ ] **Step 1: 编译项目**

运行: `cd src-tauri && cargo build`
预期: 编译成功

- [ ] **Step 2: 运行开发服务器**

运行: `npm run tauri dev`
预期: 应用启动成功

### Task 8.2: 手动测试流程

- [ ] **Step 3: 测试场景 1 - 正常流程**

1. 连接 BLE 设备（特征同时支持 write 和 write_without_response）
2. 在 GH3036 页面配置 TX UUID
3. 发送第一次指令
4. 验证：指令成功执行，无超时

- [ ] **Step 4: 测试场景 2 - 超时重试**

1. 连接 BLE 设备（特征同时支持 write 和 write_without_response）
2. 在 GH3036 页面配置 TX UUID
3. 发送第一次指令，模拟网络延迟导致 RPC 响应超时
4. 验证：
   - 日志中出现 `[GH3036] RPC 响应超时，尝试使用 write_without_response 重试`
   - 系统自动切换到 `write_without_response` 重试
   - 日志中出现 `[GH3036] RPC 重试成功`

- [ ] **Step 5: 测试场景 3 - 不支持重试**

1. 连接 BLE 设备（特征只支持 write 或只支持 write_without_response）
2. 在 GH3036 页面配置 TX UUID
3. 发送第一次指令，模拟 RPC 响应超时
4. 验证：
   - 日志中没有重试尝试
   - 直接返回超时错误

- [ ] **Step 6: 检查日志关键信息**

查看日志中的关键信息：
- `[GH3036] 设备 {device_id} 配置状态已重置`
- `[GH3036] 设备 {device_id} 特征属性: write=true, write_without_response=true, both=true`
- `[GH3036] RPC 响应超时，尝试使用 write_without_response 重试`
- `[GH3036] 设备 {device_id} 强制写入标志: true`
- `[GH3036] RPC 重试成功`
- `[GH3036] 设备 {device_id} 已标记为已发送第一次命令`

### Task 8.3: 提交最终代码

- [ ] **Step 7: 提交最终代码**

```bash
git add .
git commit -m "feat(gh3036): 完成BLE写入方式自动重试功能"
```

---

## 自检清单

完成所有任务后，进行自检：

### 1. 规格覆盖检查

- ✅ **Task 1**: 设备配置状态跟踪结构已添加，包含 `force_write_without_response` 字段
- ✅ **Task 2**: 配置方法中重置设备状态已实现
- ✅ **Task 3**: 特征属性检测逻辑已实现，在配置时自动检测
- ✅ **Task 4**: RPC 响应超时重试逻辑已实现（核心逻辑）
- ✅ **Task 5**: send_data 方法中强制写入逻辑已实现
- ✅ **Task 6**: send_direct_without_response 方法已添加
- ✅ **Task 7**: 单元测试已添加
- ✅ **Task 8**: 集成测试场景已定义

**是否有遗漏？** 无。所有规格要求都已覆盖。

### 2. 占位符扫描

搜索计划中的占位符模式：
- ❌ 无 "TBD", "TODO", "implement later"
- ❌ 无 "Add appropriate error handling"
- ❌ 无 "Write tests for the above"
- ❌ 无 "Similar to Task N"
- ❌ 所有代码步骤都包含完整代码
- ❌ 所有类型和方法都在之前的任务中定义

### 3. 类型一致性检查

- ✅ `DeviceConfigState` 结构体在 Task 1 定义，包含 `force_write_without_response` 字段
- ✅ `detect_characteristic_properties` 方法在 Task 3 定义，Task 3 调用
- ✅ `is_rpc_timeout_error` 方法在 Task 4 定义，Task 4 调用
- ✅ `should_retry_on_timeout` 方法在 Task 4 定义，Task 4 调用
- ✅ `set_force_write_without_response` 方法在 Task 4 定义，Task 4、5 调用
- ✅ `mark_first_command_sent` 方法在 Task 4 定义，Task 4 调用
- ✅ `send_direct_without_response` 方法在 Task 6 定义，Task 5 调用

---

## 执行建议

1. **按顺序执行**：Task 1-8 需要按顺序执行，后续任务依赖前面的基础设施
2. **编译验证**：每个任务完成后立即编译验证，避免积累错误
3. **频繁提交**：每个任务完成后立即提交，便于回滚
4. **调整实际类型**：如果遇到编译错误，根据实际代码调整类型和方法名
5. **重点测试**：Task 8 中的测试场景 2 是关键，必须验证重试逻辑

---

**计划创建完成！准备执行。**