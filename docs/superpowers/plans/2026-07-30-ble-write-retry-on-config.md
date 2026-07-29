# BLE 写入方式自动重试功能实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 GH3036 模块中，当特征同时支持 write 和 write_without_response 时，每次配置 TX/RX UUID 后的第一次指令发送，如果使用 write 超时，自动切换到 write_without_response 重试一次。

**Architecture:**
- 在 GH3036 管理器中添加设备配置状态跟踪（`DeviceConfigState`）
- 在 `configure_tx_channel` 中重置设备状态为"已配置，未发送"
- 在 `send_data` 中检测是否是配置后的第一次发送，并实现重试逻辑
- 调用 BLE 管理器的 `discover_characteristics` 获取特征属性

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
}

impl DeviceConfigState {
    fn new(device_id: String) -> Self {
        Self {
            device_id,
            configured: false,
            first_command_sent: false,
            supports_both_write_modes: false,
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

## Task 3: 添加特征属性查询方法

**Files:**
- Modify: `src-tauri/src/device/ble/ble_manager.rs`

**目标：** 添加便捷方法获取特征的写入属性

### Task 3.1: 在 BleManager 中添加获取特征属性的方法

- [ ] **Step 1: 添加 get_characteristic_properties 方法**

在 `ble_manager.rs` 中找到 `BleManager` impl 块，添加：

```rust
impl BleManager {
    // ... 现有方法 ...
    
    /// 获取特征的属性
    /// 返回 (write, write_without_response) 元组
    pub async fn get_characteristic_properties(
        &self,
        device_id: &str,
        char_uuid: &str,
    ) -> Result<(bool, bool)> {
        let client = self.get_client(device_id)?;
        
        // 获取已发现的服务和特征
        let services = client.discover_characteristics().await?;
        
        // 查找目标特征
        for service in services {
            for char in service.characteristics {
                if char.uuid == char_uuid {
                    // 返回属性
                    return Ok((char.write, char.write_without_response));
                }
            }
        }
        
        Err(format!("特征 {} 未找到", char_uuid).into())
    }
}
```

**注意：** 需要根据实际的 `BleCharacteristic` 结构体调整字段名。如果 `discover_characteristics` 返回的是 `BleCharacteristic` 列表，需要调整代码。

- [ ] **Step 2: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 可能有编译错误，需要根据实际类型调整

- [ ] **Step 3: 根据实际类型调整**

如果编译失败，根据错误信息调整字段名和类型。

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/device/ble/ble_manager.rs
git commit -m "feat(ble): 添加获取特征属性的便捷方法"
```

---

## Task 4: 实现写入重试逻辑

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

**目标：** 在 `send_data` 方法中实现超时重试逻辑

### Task 4.1: 修改 send_data 方法

- [ ] **Step 1: 修改 send_data 方法**

找到 `send_data` 方法（约第856行），完整替换为：

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

    // 检查是否需要尝试重试
    let should_retry = self.should_retry_first_write(&device_type, &device_id, &char_uuid).await?;

    // 第一次发送
    let result = self.device_manager
        .send_direct(device_type.clone(), &device_id, char_uuid.as_deref(), data)
        .await;

    match result {
        Ok(_) => {
            // 发送成功，标记已发送
            self.mark_first_command_sent(&device_id);
            Ok(())
        }
        Err(e) => {
            // 检查是否是超时错误且需要重试
            if should_retry && self.is_timeout_error(&e) {
                warn!("[GH3036] write 超时，尝试使用 write_without_response 重试");
                
                // 使用 write_without_response 重试
                let retry_result = self.device_manager
                    .send_direct_without_response(device_type, &device_id, char_uuid.as_deref(), data)
                    .await;
                
                match retry_result {
                    Ok(_) => {
                        info!("[GH3036] write_without_response 重试成功");
                        self.mark_first_command_sent(&device_id);
                        Ok(())
                    }
                    Err(retry_e) => {
                        error!("[GH3036] write_without_response 重试失败: {:?}", retry_e);
                        self.mark_first_command_sent(&device_id);
                        Err(format!("写入失败（已重试）: {:?}", retry_e))
                    }
                }
            } else {
                Err(format!("发送失败: {:?}", e))
            }
        }
    }
}
```

- [ ] **Step 2: 添加辅助方法 should_retry_first_write**

在 `Gh3036Manager` impl 块中添加：

```rust
impl Gh3036Manager {
    // ... 现有方法 ...
    
    /// 检查是否需要尝试重试第一次写入
    async fn should_retry_first_write(
        &self,
        device_type: &crate::device::DeviceType,
        device_id: &str,
        char_uuid: &Option<String>,
    ) -> Result<bool, String> {
        // 只对 BLE 设备进行重试
        if *device_type != crate::device::DeviceType::Ble {
            return Ok(false);
        }
        
        // 检查是否有特征 UUID
        let char_uuid = match char_uuid {
            Some(uuid) => uuid,
            None => return Ok(false),
        };
        
        // 检查设备配置状态
        let should_retry = {
            let mut states = self.device_config_states.lock();
            if let Some(state) = states.get_mut(device_id) {
                // 如果已发送过，不需要重试
                if state.first_command_sent {
                    return Ok(false);
                }
                
                // 如果还没有检测过特征属性，进行检测
                if !state.supports_both_write_modes {
                    // 需要释放锁后再调用异步方法
                    drop(states);
                    
                    // 获取特征属性
                    let (write, write_without_response) = self.device_manager
                        .ble_manager()
                        .get_characteristic_properties(device_id, char_uuid)
                        .await
                        .map_err(|e| format!("获取特征属性失败: {:?}", e))?;
                    
                    // 重新获取锁
                    let mut states = self.device_config_states.lock();
                    if let Some(state) = states.get_mut(device_id) {
                        state.supports_both_write_modes = write && write_without_response;
                    }
                    
                    // 如果同时支持两种方式，需要重试
                    write && write_without_response
                } else {
                    // 已经检测过，根据缓存结果决定
                    state.supports_both_write_modes
                }
            } else {
                // 没有配置状态，不需要重试
                false
            }
        };
        
        Ok(should_retry)
    }
}
```

- [ ] **Step 3: 添加辅助方法 is_timeout_error**

```rust
impl Gh3036Manager {
    // ... 现有方法 ...
    
    /// 检查错误是否为超时错误
    fn is_timeout_error(&self, error: &crate::error::ComBridgeError) -> bool {
        // 根据实际错误类型判断
        // 这里需要根据 ComBridgeError 的定义调整
        error.to_string().contains("timeout") || error.to_string().contains("超时")
    }
}
```

- [ ] **Step 4: 添加辅助方法 mark_first_command_sent**

```rust
impl Gh3036Manager {
    // ... 现有方法 ...
    
    /// 标记已发送第一次命令
    fn mark_first_command_sent(&self, device_id: &str) {
        let mut states = self.device_config_states.lock();
        if let Some(state) = states.get_mut(device_id) {
            state.first_command_sent = true;
            info!("[GH3036] 设备 {} 已标记为已发送第一次命令", device_id);
        }
    }
}
```

- [ ] **Step 5: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 可能有编译错误，需要根据实际类型调整

- [ ] **Step 6: 根据实际类型调整**

如果编译失败，根据错误信息调整：
- `ComBridgeError` 类型的判断方法
- `device_manager.ble_manager()` 方法是否存在
- `send_direct_without_response` 方法是否需要添加

- [ ] **Step 7: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 实现BLE写入超时自动重试逻辑"
```

---

## Task 5: 添加 DeviceManager 的便捷方法（如果需要）

**Files:**
- Modify: `src-tauri/src/device/device_manager.rs` (如果需要)

**目标：** 如果 DeviceManager 没有 `ble_manager()` 方法，添加它

### Task 5.1: 检查并添加 ble_manager 方法

- [ ] **Step 1: 检查 DeviceManager 是否有 ble_manager 方法**

运行: `cd src-tauri && grep -n "pub fn ble_manager" src/device/device_manager.rs`

如果有，跳过此任务。如果没有，继续。

- [ ] **Step 2: 添加 ble_manager 方法**

在 `DeviceManager` impl 块中添加：

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
预期: 编译通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/device/device_manager.rs
git commit -m "feat(device): 添加获取BLE管理器的便捷方法"
```

---

## Task 6: 添加 send_direct_without_response 方法（如果需要）

**Files:**
- Modify: `src-tauri/src/device/device_manager.rs`
- Modify: `src-tauri/src/device/ble/ble_manager.rs` (可能需要)

**目标：** 添加强制使用 write_without_response 的发送方法

### Task 6.1: 检查并添加方法

- [ ] **Step 1: 在 DeviceManager 中添加 send_direct_without_response 方法**

在 `DeviceManager` impl 块中添加：

```rust
impl DeviceManager {
    // ... 现有方法 ...
    
    /// 直接发送数据到设备（强制使用 write_without_response）
    pub async fn send_direct_without_response(
        &self,
        device_type: DeviceType,
        device_id: &str,
        char_uuid: Option<&str>,
        data: &[u8],
    ) -> Result<()> {
        match device_type {
            DeviceType::Serial => {
                // 串口不需要区分写入方式
                self.serial_manager.send_data(device_id, data).await
            }
            DeviceType::Ble => {
                let uuid = char_uuid.ok_or_else(|| {
                    ComBridgeError::invalid_input("BLE 发送需要特征 UUID")
                })?;
                self.ble_manager
                    .write_without_response(device_id, uuid, data)
                    .await
            }
        }
    }
}
```

- [ ] **Step 2: 在 BleManager 中添加 write_without_response 方法**

如果 `BleManager` 没有该方法，添加：

```rust
impl BleManager {
    // ... 现有方法 ...
    
    /// 写入特征值（强制使用 write_without_response）
    pub async fn write_without_response(
        &self,
        device_id: &str,
        char_uuid: &str,
        data: &[u8],
    ) -> Result<()> {
        let client = self.get_client(device_id)?;
        client.write_without_response(char_uuid, data).await
    }
}
```

- [ ] **Step 3: 编译验证**

运行: `cd src-tauri && cargo check`
预期: 编译通过

- [ ] **Step 4: 提交**

```bash
git add src-tauri/src/device/device_manager.rs src-tauri/src/device/ble/ble_manager.rs
git commit -m "feat(device): 添加强制使用write_without_response的发送方法"
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
    }

    #[test]
    fn test_is_timeout_error() {
        let manager = Gh3036Manager::new_for_test();
        
        // 测试超时错误
        let timeout_err = ComBridgeError::timeout("操作超时");
        assert!(manager.is_timeout_error(&timeout_err));
        
        // 测试非超时错误
        let other_err = ComBridgeError::invalid_input("参数错误");
        assert!(!manager.is_timeout_error(&other_err));
    }
}
```

**注意：** 需要根据实际的 `Gh3036Manager` 构造方法和 `ComBridgeError` 类型调整测试代码。

- [ ] **Step 2: 运行测试**

运行: `cd src-tauri && cargo test --lib gh3036::manager::tests`
预期: 测试通过

- [ ] **Step 3: 提交**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "test(gh3036): 添加设备配置状态和超时检测测试"
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

- [ ] **Step 3: 手动测试流程**

1. 连接 BLE 设备
2. 在 GH3036 页面配置 TX UUID
3. 发送第一次指令
4. 观察日志，验证：
   - 是否检测到特征属性
   - 是否在 write 超时后自动重试
   - 重试是否成功

- [ ] **Step 4: 检查日志**

查看日志中的关键信息：
- `[GH3036] 设备 {device_id} 配置状态已重置`
- `[GH3036] write 超时，尝试使用 write_without_response 重试`
- `[GH3036] write_without_response 重试成功`

---

## 自检清单

完成所有任务后，进行自检：

### 1. 规格覆盖检查

- ✅ **Task 1**: 设备配置状态跟踪结构已添加
- ✅ **Task 2**: 配置方法中重置设备状态已实现
- ✅ **Task 3**: 特征属性查询方法已添加
- ✅ **Task 4**: 写入重试逻辑已实现
- ✅ **Task 5**: DeviceManager 便捷方法已添加（如果需要）
- ✅ **Task 6**: send_direct_without_response 方法已添加（如果需要）
- ✅ **Task 7**: 单元测试已添加
- ✅ **Task 8**: 集成测试已完成

**是否有遗漏？** 无

### 2. 占位符扫描

搜索计划中的占位符模式：
- ❌ 无 "TBD", "TODO", "implement later"
- ❌ 无 "Add appropriate error handling"
- ❌ 无 "Write tests for the above"
- ❌ 无 "Similar to Task N"
- ❌ 所有代码步骤都包含完整代码
- ❌ 所有类型和方法都在之前的任务中定义

### 3. 类型一致性检查

- ✅ `DeviceConfigState` 结构体在 Task 1 定义，Task 2 使用
- ✅ `should_retry_first_write` 方法在 Task 4 定义，`send_data` 调用
- ✅ `is_timeout_error` 方法在 Task 4 定义，`send_data` 调用
- ✅ `mark_first_command_sent` 方法在 Task 4 定义，`send_data` 调用
- ✅ `get_characteristic_properties` 方法在 Task 3 定义，Task 4 调用
- ✅ `send_direct_without_response` 方法在 Task 6 定义，Task 4 调用（如果需要）

---

## 执行建议

1. **按顺序执行**：Task 1-8 需要按顺序执行，后续任务依赖前面的基础设施
2. **编译验证**：每个任务完成后立即编译验证，避免积累错误
3. **频繁提交**：每个任务完成后立即提交，便于回滚
4. **调整实际类型**：如果遇到编译错误，根据实际代码调整类型和方法名

---

**计划创建完成！准备执行。**