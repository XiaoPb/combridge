# CSV 自动分文件增强实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在启动/停止命令执行、蓝牙断链以及用户手动点击按钮时自动创建新的CSV文件，实现更智能的分文件逻辑

**Architecture:** 在 CsvWriter 添加强制创建新文件的方法，在 GlobalContext 提供统一的触发接口，在启动/停止命令执行、设备断开事件处理和前端按钮点击中调用该接口

**Tech Stack:** Rust, Tauri 2.0, parking_lot::Mutex, crossbeam-channel, React 19, TypeScript, Ant Design

---

## 文件结构

**修改文件：**
- `src-tauri/src/gh3036/csv_writer.rs` - 添加强制创建新文件的方法
- `src-tauri/src/gh3036/manager.rs` - 在启动/停止命令、设备断开和手动触发时创建新文件
- `src-tauri/src/commands/gh3036.rs` - 添加手动触发新文件的Tauri命令
- `src-tauri/src/lib.rs` - 注册新的Tauri命令
- `src/api/gh3036.ts` - 添加前端API调用方法
- `src/pages/Protocol/Gh3036DataView.tsx` - 添加手动触发按钮

**测试文件：**
- `src-tauri/src/gh3036/csv_writer.rs` (tests 模块) - 单元测试
- `src-tauri/src/gh3036/manager.rs` (tests 模块) - 集成测试

---

### Task 1: 在 CsvWriter 添加强制创建新文件的方法

**Files:**
- Modify: `src-tauri/src/gh3036/csv_writer.rs`

- [ ] **Step 1: 在 CsvWriter 结构体添加 force_new_file 方法**

在 `impl CsvWriter` 块中添加以下方法（在 `write_frame` 方法之后）：

```rust
/// 强制创建新的 CSV 文件
///
/// 用于在特定事件（如启动/停止命令、设备断开）发生时，
/// 确保后续数据写入新文件
pub fn force_new_file(&mut self) -> std::io::Result<()> {
    // 先刷新当前文件（如果有）
    {
        let mut writer_guard = self.writer.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(ref mut writer) = *writer_guard {
            writer.flush()?;
        }
        // 清空 writer，下次写入时会创建新文件
        *writer_guard = None;
    }

    self.last_frame_id = -1;
    self.rows_since_flush = 0;

    info!("[CsvWriter] 强制创建新文件标记已设置");
    Ok(())
}
```

- [ ] **Step 2: 在 csv_writer.rs 的测试模块添加单元测试**

在 `#[cfg(test)] mod tests` 块中添加以下测试：

```rust
#[test]
fn force_new_file_creates_separate_files() {
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let output_dir = temp_dir.path().to_path_buf();
    let mut writer = CsvWriter::new(output_dir, 2, "SPO2".to_string());

    // 写入第一帧
    let frame1 = Gh3036FrameData {
        function_id: 2,
        function_name: "SPO2".to_string(),
        frame_id: 1,
        timestamp: 100,
        gs_data: vec![1, 2, 3, 4, 5, 6],
        rawdata: vec![100],
        flags: vec![0],
        ref_data: vec![0],
        algo_data: vec![98],
        agc_info: vec![0],
        phy_value: vec![200],
        led_info: vec![0],
    };
    writer.write_frame(&frame1).unwrap();

    // 强制创建新文件
    writer.force_new_file().unwrap();

    // 写入第二帧（应该在新文件中）
    let frame2 = Gh3036FrameData {
        frame_id: 2,
        timestamp: 200,
        ..frame1.clone()
    };
    writer.write_frame(&frame2).unwrap();

    // 验证：检查输出目录中至少有两个 CSV 文件
    let csv_files: Vec<_> = std::fs::read_dir(output_dir.join("SPO2"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|e| e == "csv").unwrap_or(false))
        .collect();

    assert!(csv_files.len() >= 2, "应该创建至少两个CSV文件");
}
```

- [ ] **Step 3: 运行测试验证功能**

Run: `cd src-tauri && cargo test --lib gh3036::csv_writer::tests::force_new_file -- --nocapture`
Expected: PASS

- [ ] **Step 4: 提交代码**

```bash
git add src-tauri/src/gh3036/csv_writer.rs
git commit -m "feat(gh3036): CsvWriter 添加强制创建新文件的方法"
```

---

### Task 2: 在 GlobalContext 添加触发新文件创建的方法

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

- [ ] **Step 1: 在 GlobalContext impl 块添加 trigger_new_csv_file 方法**

在 `impl GlobalContext` 块中，`save_frame_to_csv` 方法之后添加：

```rust
/// 触发所有 CSV writer 创建新文件
///
/// 在启动/停止命令执行或设备断开时调用，
/// 确保每个功能的 CSV writer 都创建新文件
fn trigger_new_csv_file(&self) {
    let csv_config = self.csv_config.lock();
    if !csv_config.enabled {
        return;
    }
    drop(csv_config);

    let mut writers = self.csv_writers.lock();
    for (function_id, writer) in writers.iter_mut() {
        if let Err(e) = writer.force_new_file() {
            error!("[GH3036] 功能 {} 强制创建新CSV文件失败: {}", function_id, e);
        } else {
            info!("[GH3036] 功能 {} 已标记创建新CSV文件", function_id);
        }
    }
}
```

- [ ] **Step 2: 在 manager.rs 的测试模块添加单元测试**

在 `#[cfg(test)] mod tests` 块中添加以下测试：

```rust
#[test]
fn trigger_new_csv_file_creates_new_files_for_all_writers() {
    use tempfile::TempDir;

    let context = GlobalContext::new();
    let temp_dir = TempDir::new().unwrap();

    context.set_csv_config(CsvConfig {
        enabled: true,
        output_dir: temp_dir.path().to_string_lossy().to_string(),
    });

    // 模拟已有 writer
    let frame = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
    context.save_frame_to_csv(&frame);

    // 触发新文件创建
    context.trigger_new_csv_file();

    // 再次写入，应该在新文件中
    let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
    context.save_frame_to_csv(&frame2);

    // 验证：检查输出目录中至少有两个 CSV 文件
    let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|e| e == "csv").unwrap_or(false))
        .collect();

    assert!(csv_files.len() >= 2, "应该创建至少两个CSV文件");
}
```

- [ ] **Step 3: 运行测试验证功能**

Run: `cd src-tauri && cargo test --lib gh3036::manager::tests::trigger_new_csv_file -- --nocapture`
Expected: PASS

- [ ] **Step 4: 提交代码**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): GlobalContext 添加触发新CSV文件创建的方法"
```

---

### Task 3: 在启动/停止命令执行后触发新文件创建

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

- [ ] **Step 1: 修改 execute_sw_function_cmd_async 方法**

将现有的 `execute_sw_function_cmd_async` 方法修改为：

```rust
async fn execute_sw_function_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
    let target_func_mode: u32 = params
        .first()
        .and_then(|s| {
            if s.starts_with("0x") || s.starts_with("0X") {
                u32::from_str_radix(&s[2..], 16).ok()
            } else {
                s.parse().ok()
            }
        })
        .ok_or("缺少目标功能模式参数")?;

    let ctrl_type: u8 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

    info!(
        "软件功能命令: mode=0x{:08X}, ctrl={}",
        target_func_mode, ctrl_type
    );

    let mut data = Vec::new();
    data.extend_from_slice(&target_func_mode.to_le_bytes());
    data.push(ctrl_type);

    self.send_command(KEY_GH3X_SW_FUNCTION_CMD, FMT_GH3X_SW_FUNCTION_CMD, &data)
        .await?;

    // 启动（ctrl_type=0）或停止（ctrl_type=1）命令执行后，触发新CSV文件创建
    if ctrl_type == 0 || ctrl_type == 1 {
        CALLBACK_CONTEXT.trigger_new_csv_file();
        info!(
            "[GH3036] 软件{}命令执行完成，已触发新CSV文件创建",
            if ctrl_type == 0 { "启动" } else { "停止" }
        );
    }

    Ok(vec![])
}
```

- [ ] **Step 2: 添加集成测试验证启动/停止命令触发新文件**

在测试模块添加：

```rust
#[test]
fn sw_function_start_stop_triggers_new_csv_file() {
    use tempfile::TempDir;

    let context = GlobalContext::new();
    let temp_dir = TempDir::new().unwrap();

    context.set_csv_config(CsvConfig {
        enabled: true,
        output_dir: temp_dir.path().to_string_lossy().to_string(),
    });

    // 写入初始帧
    let frame1 = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
    context.save_frame_to_csv(&frame1);

    // 模拟执行启动命令（ctrl_type=0）后触发新文件
    context.trigger_new_csv_file();

    // 写入后续帧（应该在新文件中）
    let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
    context.save_frame_to_csv(&frame2);

    // 验证：检查输出目录中至少有两个 CSV 文件
    let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|e| e == "csv").unwrap_or(false))
        .collect();

    assert!(csv_files.len() >= 2, "启动命令后应该创建新CSV文件");
}
```

- [ ] **Step 3: 运行测试验证功能**

Run: `cd src-tauri && cargo test --lib gh3036::manager::tests::sw_function -- --nocapture`
Expected: PASS

- [ ] **Step 4: 提交代码**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 启动/停止命令执行后触发新CSV文件创建"
```

---

### Task 4: 在蓝牙断链时触发新文件创建

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`

- [ ] **Step 1: 修改 handle_device_disconnected 方法**

将现有的 `handle_device_disconnected` 方法修改为：

```rust
fn handle_device_disconnected(device_id: &str) {
    {
        let mut rx_channel = CALLBACK_CONTEXT.rx_channel.lock();
        if rx_channel
            .as_ref()
            .is_some_and(|channel| channel.device_id == device_id)
        {
            *rx_channel = None;
            if let Some(ref sender) = *CALLBACK_CONTEXT.rpc_data_sender.lock() {
                if let Err(error) = sender.send(RpcInput::Reset) {
                    warn!("[GH3036] 设备断开时重置 RPC 接收状态失败: {}", error);
                }
            }
            info!("GH3036 RX 通道已清理: 设备 {} 已断开", device_id);

            // 设备断开时触发新CSV文件创建
            CALLBACK_CONTEXT.trigger_new_csv_file();
            info!("[GH3036] 设备断开，已触发新CSV文件创建");
        }
    }

    let mut tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
    if tx_channel
        .as_ref()
        .is_some_and(|channel| channel.device_id == device_id)
    {
        *tx_channel = None;
        info!("GH3036 TX 通道已清理: 设备 {} 已断开", device_id);
    }
}
```

- [ ] **Step 2: 添加单元测试验证设备断开触发新文件**

在测试模块添加：

```rust
#[test]
fn device_disconnect_triggers_new_csv_file() {
    use tempfile::TempDir;

    let context = GlobalContext::new();
    let temp_dir = TempDir::new().unwrap();

    context.set_csv_config(CsvConfig {
        enabled: true,
        output_dir: temp_dir.path().to_string_lossy().to_string(),
    });

    // 配置 RX 通道
    context.set_rx_channel(ChannelConfig {
        channel_type: ChannelType::Ble,
        device_id: "test-device".to_string(),
        characteristic_uuid: Some("test-char".to_string()),
    });

    // 写入初始帧
    let frame1 = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
    context.save_frame_to_csv(&frame1);

    // 模拟设备断开
    Gh3036Manager::handle_device_disconnected("test-device");

    // 写入后续帧（应该在新文件中）
    let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
    context.save_frame_to_csv(&frame2);

    // 验证：检查输出目录中至少有两个 CSV 文件
    let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map(|e| e == "csv").unwrap_or(false))
        .collect();

    assert!(csv_files.len() >= 2, "设备断开后应该创建新CSV文件");
}
```

- [ ] **Step 3: 运行测试验证功能**

Run: `cd src-tauri && cargo test --lib gh3036::manager::tests::device_disconnect -- --nocapture`
Expected: PASS

- [ ] **Step 4: 提交代码**

```bash
git add src-tauri/src/gh3036/manager.rs
git commit -m "feat(gh3036): 设备断开时触发新CSV文件创建"
```

---

### Task 5: 后端添加手动触发新文件的Tauri命令

**Files:**
- Modify: `src-tauri/src/gh3036/manager.rs`
- Modify: `src-tauri/src/commands/gh3036.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: 在 Gh3036Manager 添加公开方法**

在 `impl Gh3036Manager` 块中，`set_csv_config` 方法之后添加：

```rust
pub fn force_new_csv_file(&self) -> Result<(), String> {
    CALLBACK_CONTEXT.trigger_new_csv_file();
    info!("[GH3036] 手动触发新CSV文件创建");
    Ok(())
}
```

- [ ] **Step 2: 在 commands/gh3036.rs 添加 Tauri 命令**

在文件末尾添加：

```rust
#[tauri::command]
pub async fn gh3036_force_new_csv_file(
    state: tauri::State<'_, Arc<tokio::sync::RwLock<crate::state::AppState>>>,
) -> Result<(), String> {
    let state = state.read().await;
    let manager = state.gh3036_manager.lock();
    manager.force_new_csv_file()
}
```

- [ ] **Step 3: 在 lib.rs 注册新命令**

在 `invoke_handler` 中的命令列表里添加：

```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令 ...
    crate::commands::gh3036::gh3036_force_new_csv_file,
])
```

- [ ] **Step 4: 提交代码**

```bash
git add src-tauri/src/gh3036/manager.rs src-tauri/src/commands/gh3036.rs src-tauri/src/lib.rs
git commit -m "feat(gh3036): 添加手动触发新CSV文件的Tauri命令"
```

---

### Task 6: 前端API添加调用方法

**Files:**
- Modify: `src/api/gh3036.ts`

- [ ] **Step 1: 在 src/api/gh3036.ts 添加 API 方法**

在文件末尾（导出区域）添加：

```typescript
/**
 * 手动触发创建新的CSV文件
 * 用于用户在前端主动点击按钮时创建新文件
 */
export const gh3036ForceNewCsvFile = async (): Promise<void> => {
  await invoke<void>('gh3036_force_new_csv_file');
};
```

- [ ] **Step 2: 提交代码**

```bash
git add src/api/gh3036.ts
git commit -m "feat(gh3036): 前端API添加手动触发新CSV文件的方法"
```

---

### Task 7: 前端页面添加手动触发按钮

**Files:**
- Modify: `src/pages/Protocol/Gh3036DataView.tsx`

- [ ] **Step 1: 修改 Gh3036DataView 组件**

在现有的 `Gh3036DataView` 组件中，在"清空波形"按钮旁边添加新按钮：

```tsx
import React, { useMemo, useCallback } from 'react';
import { Button, Empty, Card, Select, Space, Row, Col, message } from 'antd';
import { ClearOutlined, FileAddOutlined } from '@ant-design/icons';
import { useTranslation } from 'react-i18next';
import { useGh3036Store } from '../../stores/gh3036Store';
import { gh3036ForceNewCsvFile } from '../../api/gh3036';
import MultiLineChart from '../Waveform/MultiLineChart';

const Gh3036DataView: React.FC = () => {
  const { t } = useTranslation('protocol');
  const {
    framesData,
    selectedFunctionId,
    chartGroups,
    clearWaveformData,
    setSelectedFunctionId,
    setChartGroups
  } = useGh3036Store();

  // 新增：手动触发新CSV文件
  const handleForceNewCsvFile = useCallback(async () => {
    try {
      await gh3036ForceNewCsvFile();
      message.success(t('gh3036.newCsvFileCreated'));
    } catch (error) {
      message.error(t('gh3036.newCsvFileFailed'));
      console.error('[GH3036] 手动创建新CSV文件失败:', error);
    }
  }, [t]);

  // ... 其他现有代码 ...

  return (
    <Card
      size="small"
      title={t('gh3036.dataView')}
      extra={
        <Space>
          <Button
            size="small"
            icon={<FileAddOutlined />}
            onClick={handleForceNewCsvFile}
          >
            {t('gh3036.newCsvFile')}
          </Button>
          <Button
            size="small"
            icon={<ClearOutlined />}
            onClick={clearWaveformData}
          >
            {t('common.clear')}
          </Button>
        </Space>
      }
      style={{ height: '100%' }}
      styles={{ body: { padding: 8, height: 'calc(100% - 40px)', overflow: 'auto' } }}
    >
      {/* ... 现有的组件内容 ... */}
    </Card>
  );
};

export default Gh3036DataView;
```

- [ ] **Step 2: 添加国际化文本（如需要）**

在 `public/locales/zh-CN/protocol.json` 和 `public/locales/en-US/protocol.json` 中添加：

```json
{
  "gh3036": {
    "newCsvFile": "新建CSV文件",
    "newCsvFileCreated": "新CSV文件创建成功",
    "newCsvFileFailed": "创建新CSV文件失败"
  }
}
```

- [ ] **Step 3: 提交代码**

```bash
git add src/pages/Protocol/Gh3036DataView.tsx public/locales
git commit -m "feat(gh3036): 数据监控页面添加手动触发新CSV文件按钮"
```

---

### Task 8: 运行完整测试套件并验证

**Files:**
- 无文件修改

- [ ] **Step 1: 运行所有 GH3036 模块测试**

Run: `cd src-tauri && cargo test --lib gh3036:: -- --nocapture`
Expected: 所有测试 PASS

- [ ] **Step 2: 运行 Rust 代码检查**

Run: `cd src-tauri && cargo clippy --all-targets --all-features -- -D warnings`
Expected: 无警告

- [ ] **Step 3: 运行 TypeScript 类型检查**

Run: `npx tsc --noEmit`
Expected: 无类型错误

- [ ] **Step 4: 运行代码格式化检查**

Run: `cd src-tauri && cargo fmt -- --check`
Expected: 格式正确

- [ ] **Step 5: 提交最终版本**

```bash
git add .
git commit -m "test(gh3036): 完善CSV自动分文件功能的测试覆盖"
```

---

## Self-Review Checklist

### Spec Coverage

✅ **需求1: 调用启动和停止命令时，文件需要重新保存**
- Task 3 实现了在 `execute_sw_function_cmd_async` 中检测 `ctrl_type` 为 0（启动）或 1（停止）时触发新文件创建

✅ **需求2: 蓝牙断链之后，文件需要重新保存**
- Task 4 实现了在 `handle_device_disconnected` 中触发新文件创建

✅ **需求3: 前端手动点击按钮触发重新保存**
- Task 5 添加了后端Tauri命令 `gh3036_force_new_csv_file`
- Task 6 添加了前端API调用方法 `gh3036ForceNewCsvFile`
- Task 7 在数据监控页面添加了触发按钮

### Placeholder Scan

✅ 所有代码块都包含完整实现
✅ 无 "TODO"、"TBD"、"implement later" 等占位符
✅ 每个测试都包含具体的断言逻辑

### Type Consistency

✅ `force_new_file` 方法签名在 Task 1 定义，Task 2 使用
✅ `trigger_new_csv_file` 方法签名在 Task 2 定义，Task 3、Task 4 和 Task 5 使用
✅ `force_new_csv_file` Tauri命令签名在 Task 5 定义，Task 6 使用
✅ 所有方法调用参数类型一致