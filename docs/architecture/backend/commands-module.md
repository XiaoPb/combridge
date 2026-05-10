# 命令层模块

## 概述

命令层模块定义了所有 Tauri 命令，作为前端调用后端的入口点。每个命令模块对应特定的功能领域。所有命令在 `lib.rs` 中通过 `invoke_handler` 统一注册。

## 模块位置

- 源码路径：`src-tauri/src/commands/`
- 主要文件：
  - `mod.rs` - 模块导出
  - `serial.rs` - 串口命令
  - `ble.rs` - BLE 命令（含 AT 专用命令）
  - `protocol.rs` - 协议命令
  - `system.rs` - 系统命令
  - `state.rs` - 状态命令
  - `preferences.rs` - 偏好设置命令
  - `gh3036.rs` - GH3036 命令
  - `waveform.rs` - 波形命令

## 命令注册

在 `lib.rs` 中注册所有命令（完整列表）：

```rust
.invoke_handler(tauri::generate_handler![
    // 串口命令 (8)
    commands::serial::scan_serial_ports,
    commands::serial::open_serial_port,
    commands::serial::close_serial_port,
    commands::serial::send_serial_data,
    commands::serial::get_open_ports,
    commands::serial::is_port_open,
    commands::serial::export_serial_data,
    commands::serial::get_serial_cache,

    // BLE 通用命令 (19)
    commands::ble::configure_ble,
    commands::ble::scan_ble_devices,
    commands::ble::stop_ble_scan,
    commands::ble::connect_ble,
    commands::ble::disconnect_ble,
    commands::ble::get_ble_connections,
    commands::ble::discover_ble_services,
    commands::ble::discover_ble_characteristics,
    commands::ble::read_ble_characteristic,
    commands::ble::write_ble_characteristic,
    commands::ble::write_ble_without_response,
    commands::ble::subscribe_ble_notify,
    commands::ble::unsubscribe_ble_notify,
    commands::ble::get_ble_rssi,
    commands::ble::get_ble_mode,
    commands::ble::is_ble_configured,
    commands::ble::set_ble_mtu,
    commands::ble::get_ble_subscriptions,
    commands::ble::get_ble_cache,

    // AT 专用 BLE 命令 (7)
    commands::ble::get_at_config,
    commands::ble::update_at_uuid_config,
    commands::ble::get_at_tabs,
    commands::ble::get_at_tab,
    commands::ble::clear_at_tab_data,
    commands::ble::remove_at_tab,
    commands::ble::send_at_data,


    // 协议命令 (9)
    commands::protocol::load_protocol,
    commands::protocol::unload_protocol,
    commands::protocol::enable_protocol,
    commands::protocol::disable_protocol,
    commands::protocol::bind_protocol,
    commands::protocol::unbind_protocol,
    commands::protocol::list_protocols,
    commands::protocol::get_protocol,
    commands::protocol::get_bound_protocols,

    // 系统命令 (14 + 2 feature-gated)
    commands::system::get_system_info,
    commands::system::get_system_status,
    commands::system::configure_log,
    commands::system::get_log_config,
    commands::system::get_runtime_status,
    commands::system::get_app_version,
    commands::system::set_timezone_config,
    commands::system::get_timezone_config,
    commands::system::get_platform,
    commands::system::open_url,
    commands::system::show_in_folder,
    commands::system::show_main_window,
    commands::system::get_window_status,
    #[cfg(feature = "devtools")]
    commands::system::open_devtools,
    #[cfg(feature = "devtools")]
    commands::system::close_devtools,

    // 状态命令 (7)
    commands::state::dispatch_action,
    commands::state::get_state,
    commands::state::get_channel_data,
    commands::state::restore_state,
    commands::state::save_state,
    commands::state::get_connected_devices,
    commands::state::get_window_state,

    // 偏好设置命令 (6)
    commands::preferences::get_preferences,
    commands::preferences::save_preferences,
    commands::preferences::update_serial_preferences,
    commands::preferences::update_ble_preferences,
    commands::preferences::update_waveform_preferences,
    commands::preferences::update_gh3036_channel_preferences,

    // GH3036 命令 (26)
    commands::gh3036::gh3036_init,
    commands::gh3036::gh3036_is_initialized,
    commands::gh3036::gh3036_configure_tx_channel,
    commands::gh3036::gh3036_configure_rx_channel,
    commands::gh3036::gh3036_get_channels,
    commands::gh3036::gh3036_send_data,
    commands::gh3036::gh3036_set_csv_config,
    commands::gh3036::gh3036_get_csv_config,
    commands::gh3036::gh3036_get_rpc_commands,
    commands::gh3036::gh3036_get_version_types,
    commands::gh3036::gh3036_execute_rpc,
    commands::gh3036::gh3036_subscribe_events,
    commands::gh3036::gh3036_get_library_status,
    commands::gh3036::gh3036_load_config_file,
    commands::gh3036::gh3036_factory_test_start,
    commands::gh3036::gh3036_factory_test_stop,
    commands::gh3036::gh3036_factory_test_status,
    commands::gh3036::gh3036_factory_test_continue,
    commands::gh3036::gh3036_factory_test_set_config_dir,
    commands::gh3036::gh3036_factory_test_validate_config,
    commands::gh3036::gh3036_factory_test_get_result,
    commands::gh3036::gh3036_validate_threshold_config,
    commands::gh3036::gh3036_get_threshold_config,
    commands::gh3036::gh3036_get_evaluation_result,

    // 波形命令 (8)
    commands::waveform::waveform_create_buffer,
    commands::waveform::waveform_remove_buffer,
    commands::waveform::waveform_configure_parser,
    commands::waveform::waveform_parse_and_store,
    commands::waveform::waveform_read_data,
    commands::waveform::waveform_get_status,
    commands::waveform::waveform_clear_buffer,
    commands::waveform::waveform_list_buffers,

    // Dashboard 命令 (14)
    dashboard::get_parser_scripts,
    dashboard::get_parser_script_content,
    dashboard::save_parser_script,
    dashboard::delete_parser_script,
    dashboard::execute_parser_script,
    dashboard::init_default_parser_scripts,
    dashboard::analyze_json_structure,
    dashboard::generate_parser_from_json,
    dashboard::get_parser_defined_fields,
    dashboard::merge_json_to_parser,
    dashboard::get_json_files,
    dashboard::save_json_file,
    dashboard::delete_json_file,
    dashboard::load_json_file,
])
```

## 串口命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `scan_serial_ports` | 无 | `Vec<PortInfo>` | 扫描可用串口 |
| `open_serial_port` | `config: SerialPortConfigDto` | `()` | 打开串口 |
| `close_serial_port` | `port_name: String` | `()` | 关闭串口 |
| `send_serial_data` | `port_name: String, data: Vec<u8>` | `usize` | 发送数据 |
| `get_open_ports` | 无 | `Vec<String>` | 获取已打开端口 |
| `is_port_open` | `port_name: String` | `bool` | 检查端口状态 |
| `export_serial_data` | `port_name: String, all_data: Vec<ExportDataEntry>, rx_data: Vec<u8>` | `ExportResult` | 导出数据到日志文件 |
| `get_serial_cache` | `port_name: String` | `CacheData` | 获取串口缓存数据 |

## BLE 通用命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `configure_ble` | `config: BleConfigDto` | `()` | 配置 BLE 模式（native/at） |
| `scan_ble_devices` | `duration_ms: u64` | `Vec<BleDevice>` | 扫描设备 |
| `stop_ble_scan` | 无 | `Vec<BleDevice>` | 停止扫描 |
| `connect_ble` | `device_id: String` | `BleConnection` | 连接设备 |
| `disconnect_ble` | `device_id: String` | `()` | 断开连接 |
| `get_ble_connections` | 无 | `Vec<BleConnection>` | 获取连接列表 |
| `discover_ble_services` | `device_id: String` | `Vec<BleService>` | 发现服务 |
| `discover_ble_characteristics` | `device_id: String, service_uuid: String` | `Vec<BleCharacteristic>` | 发现特征 |
| `read_ble_characteristic` | `device_id: String, characteristic_uuid: String` | `Vec<u8>` | 读取特征 |
| `write_ble_characteristic` | `device_id: String, characteristic_uuid: String, data: Vec<u8>` | `()` | 写入特征 |
| `write_ble_without_response` | `device_id: String, characteristic_uuid: String, data: Vec<u8>` | `()` | 无响应写入 |
| `subscribe_ble_notify` | `device_id: String, characteristic_uuid: String` | `()` | 订阅通知 |
| `unsubscribe_ble_notify` | `device_id: String, characteristic_uuid: String` | `()` | 取消订阅 |
| `get_ble_rssi` | `address: String` | `i16` | 获取 RSSI |
| `get_ble_mode` | 无 | `String` | 获取当前模式 |
| `is_ble_configured` | 无 | `bool` | 检查配置状态 |
| `set_ble_mtu` | `device_id: String, mtu: u16` | `u16` | 设置 MTU |
| `get_ble_subscriptions` | `device_id: String` | `Vec<String>` | 获取订阅列表 |
| `get_ble_cache` | `characteristic_uuid: String` | `CacheData` | 获取 BLE 缓存数据 |

## AT 专用 BLE 命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_at_config` | 无 | `AtConfig` | 获取 AT 模式配置 |
| `update_at_uuid_config` | `tx_uuid: Option<String>, rx_uuid: Option<String>, srv_uuid: Option<String>` | `()` | 更新 AT UUID 配置 |
| `get_at_tabs` | 无 | `Vec<AtConnectionTab>` | 获取 AT 连接 TAB 列表 |
| `get_at_tab` | `tab_id: String` | `Option<AtConnectionTab>` | 获取指定 AT 连接 TAB |
| `clear_at_tab_data` | `tab_id: String` | `()` | 清空 AT TAB 收发数据 |
| `remove_at_tab` | `tab_id: String` | `()` | 移除 AT 连接 TAB |
| `send_at_data` | `device_id: String, data: Vec<u8>` | `()` | AT 透传数据发送 |

## 协议命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `load_protocol` | `plugin_id: String, path: String` | `PluginInfo` | 加载协议 |
| `unload_protocol` | `plugin_id: String` | `()` | 卸载协议 |
| `enable_protocol` | `plugin_id: String` | `()` | 启用协议 |
| `disable_protocol` | `plugin_id: String` | `()` | 禁用协议 |
| `bind_protocol` | `plugin_id: String, device_id: String` | `()` | 绑定协议到设备 |
| `unbind_protocol` | `plugin_id: String, device_id: String` | `()` | 解绑协议 |
| `list_protocols` | 无 | `Vec<PluginInfo>` | 列出协议 |
| `get_protocol` | `plugin_id: String` | `PluginInfo` | 获取协议信息 |
| `get_bound_protocols` | `device_id: String` | `Vec<PluginInfo>` | 获取设备绑定的协议列表 |

## 系统命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_system_info` | 无 | `SystemInfo` | 获取系统信息 |
| `get_system_status` | 无 | `SystemStatus` | 获取运行状态 |
| `configure_log` | `config: LogConfig` | `()` | 配置日志 |
| `get_log_config` | 无 | `LogConfig` | 获取日志配置 |
| `get_runtime_status` | 无 | `RuntimeStatus` | 获取运行时状态 |
| `get_app_version` | 无 | `String` | 获取版本 |
| `set_timezone_config` | `config: TimezoneConfig` | `()` | 设置时区配置 |
| `get_timezone_config` | 无 | `String` | 获取时区配置 |
| `get_platform` | 无 | `String` | 获取平台 |
| `open_url` | `url: String` | `()` | 打开 URL |
| `show_in_folder` | `path: String` | `()` | 在文件夹显示 |
| `show_main_window` | 无 | `()` | 显示主窗口 |
| `get_window_status` | 无 | `WindowStatus` | 获取窗口状态 |
| `open_devtools` | 无 | `()` | 打开开发者工具（需 `devtools` feature） |
| `close_devtools` | 无 | `()` | 关闭开发者工具（需 `devtools` feature） |

## 状态命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `dispatch_action` | `action: Action` | `ActionResult` | 分发状态变更动作 |
| `get_state` | 无 | `AppState` | 获取应用状态 |
| `get_channel_data` | `device_id: String, channel_id: String` | `Option<ChannelData>` | 获取通道数据 |
| `restore_state` | 无 | `AppState` | 恢复状态 |
| `save_state` | 无 | `()` | 保存状态 |
| `get_connected_devices` | 无 | `Vec<DeviceInfo>` | 获取连接设备 |
| `get_window_state` | 无 | `WindowState` | 获取窗口状态 |

## 偏好设置命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_preferences` | 无 | `Preferences` | 获取偏好设置 |
| `save_preferences` | `prefs: Preferences` | `()` | 保存偏好设置 |
| `update_serial_preferences` | `display_format, display_mode, send_format, append_newline, newline_type, auto_scroll` | `()` | 更新串口偏好 |
| `update_ble_preferences` | `display_format, auto_scroll, input_format, without_response, config_collapsed, gatt_collapsed, panel_collapsed` | `()` | 更新 BLE 偏好 |
| `update_waveform_preferences` | `display_rows, refresh_interval, sidebar_collapsed` | `()` | 更新波形偏好 |
| `update_gh3036_channel_preferences` | `connection_type, serial_port, ble_device, tx_char, rx_char` | `()` | 更新 GH3036 通道偏好 |

## GH3036 命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `gh3036_init` | 无 | `()` | 初始化 GH3036 |
| `gh3036_is_initialized` | 无 | `bool` | 检查初始化状态 |
| `gh3036_configure_tx_channel` | `channel_type, device_id, characteristic_uuid` | `()` | 配置 TX 通道 |
| `gh3036_configure_rx_channel` | `channel_type, device_id, characteristic_uuid` | `()` | 配置 RX 通道 |
| `gh3036_get_channels` | 无 | `(Option<ChannelConfig>, Option<ChannelConfig>)` | 获取通道配置 |
| `gh3036_send_data` | `data: Vec<u8>` | `()` | 发送数据 |
| `gh3036_set_csv_config` | `enabled, output_dir` | `()` | 设置 CSV 配置 |
| `gh3036_get_csv_config` | 无 | `CsvConfig` | 获取 CSV 配置 |
| `gh3036_get_rpc_commands` | 无 | `Vec<RpcCommand>` | 获取 RPC 命令列表 |
| `gh3036_get_version_types` | 无 | `Vec<VersionTypeConfig>` | 获取版本类型列表 |
| `gh3036_execute_rpc` | `command_key, params` | `Vec<u8>` | 执行 RPC 命令 |
| `gh3036_subscribe_events` | 无 | `bool` | 订阅事件 |
| `gh3036_get_library_status` | 无 | `(bool, bool)` | 获取库状态 |
| `gh3036_load_config_file` | `file_path: String` | `Vec<String>` | 加载配置文件 |
| `gh3036_factory_test_start` | 无 | `()` | 启动工厂测试 |
| `gh3036_factory_test_stop` | 无 | `()` | 停止工厂测试 |
| `gh3036_factory_test_status` | 无 | `(FactoryTestStatus, FactoryTestStep)` | 获取工厂测试状态 |
| `gh3036_factory_test_continue` | 无 | `()` | 继续工厂测试 |
| `gh3036_factory_test_set_config_dir` | `config_dir: String` | `()` | 设置工厂测试配置目录 |
| `gh3036_factory_test_validate_config` | 无 | `ConfigValidationResult` | 验证工厂测试配置 |
| `gh3036_factory_test_get_result` | 无 | `Option<FactoryTestResult>` | 获取工厂测试结果 |
| `gh3036_validate_threshold_config` | 无 | `ThresholdConfigValidation` | 验证阈值配置 |
| `gh3036_get_threshold_config` | 无 | `Option<FactoryThresholdConfig>` | 获取阈值配置 |
| `gh3036_get_evaluation_result` | 无 | `Option<FactoryEvaluationResult>` | 获取评估结果 |

## 波形命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `waveform_create_buffer` | `buffer_id: String, config: WaveformBufferConfig` | `()` | 创建缓冲区 |
| `waveform_remove_buffer` | `buffer_id: String` | `()` | 移除缓冲区 |
| `waveform_configure_parser` | `buffer_id: String, config: ParserConfig` | `()` | 配置解析器 |
| `waveform_parse_and_store` | `buffer_id: String, data: String` | `()` | 解析并存储数据 |
| `waveform_read_data` | `buffer_id: String, rows: usize` | `WaveformData` | 读取数据 |
| `waveform_get_status` | `buffer_id: String` | `WaveformStatus` | 获取状态 |
| `waveform_clear_buffer` | `buffer_id: String` | `()` | 清空缓冲区 |
| `waveform_list_buffers` | 无 | `Vec<String>` | 列出所有缓冲区 |

## Dashboard 命令

详见 [Dashboard 模块文档](./dashboard-module.md)

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_parser_scripts` | 无 | `Vec<ParserScriptInfo>` | 获取解析脚本列表 |
| `get_parser_script_content` | `name: String` | `String` | 获取脚本内容 |
| `save_parser_script` | `name: String, content: String` | `()` | 保存脚本 |
| `delete_parser_script` | `name: String` | `()` | 删除脚本 |
| `execute_parser_script` | `name: String, data: String` | `HashMap<String, f64>` | 执行脚本 |
| `init_default_parser_scripts` | 无 | `()` | 初始化默认脚本 |
| `analyze_json_structure` | `json_content: String` | `JsonStructureInfo` | 分析 JSON 结构 |
| `generate_parser_from_json` | `json_content, script_name, selected_fields` | `String` | 生成解析脚本 |
| `get_parser_defined_fields` | `script_name: String` | `Vec<FieldDefinition>` | 获取脚本字段定义 |
| `merge_json_to_parser` | `json_content, script_name, selected_fields` | `String` | 合并 JSON 到脚本 |
| `get_json_files` | 无 | `Vec<String>` | 获取 JSON 配置文件列表 |
| `save_json_file` | `file_name: String, config: DashboardJsonConfig` | `()` | 保存 JSON 配置 |
| `delete_json_file` | `file_name: String` | `()` | 删除 JSON 配置 |
| `load_json_file` | `file_name: String` | `DashboardJsonConfig` | 加载 JSON 配置 |

## 命令统计

| 模块 | 数量 | 说明 |
|------|------|------|
| 串口 | 8 | 基础串口操作（含缓存获取） |
| BLE 通用 | 19 | 原生/AT 通用 BLE 操作（含缓存获取） |
| AT 专用 | 7 | AT 模式特有命令 |
| 协议 | 9 | Lua 协议插件管理 |
| 系统 | 16 | 系统信息与窗口管理（含 2 个 feature-gated） |
| 状态 | 7 | 应用状态管理 |
| 偏好设置 | 6 | 用户偏好配置 |
| GH3036 | 26 | GH3036 芯片操作（含工厂测试） |
| 波形 | 8 | 波形数据缓冲 |
| Dashboard | 14 | 数据仪表盘 |
| **总计** | **126** | - |

## 命令实现示例

```rust
#[tauri::command]
pub async fn scan_serial_ports(
    serial_manager: State<'_, SerialManagerRef>,
) -> Result<Vec<PortInfo>, ErrorResponse> {
    serial_manager
        .scan_ports()
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn open_serial_port(
    serial_manager: State<'_, SerialManagerRef>,
    config: SerialPortConfig,
    app_handle: tauri::AppHandle,
) -> Result<(), ErrorResponse> {
    let app_handle_clone = app_handle.clone();

    serial_manager
        .open_port(config, move |port_name, data| {
            let _ = app_handle_clone.emit("serial-data", serde_json::json!({
                "port_name": port_name,
                "data": data,
            }));
        })
        .map_err(|e| e.to_error_response())
}
```

## 相关模块

- [错误处理](./error-handling.md) - 错误响应格式
- [设备管理](./device-manager.md) - 设备操作
- [状态管理](./state-module.md) - 状态命令
- [Dashboard](./dashboard-module.md) - Dashboard 命令详情
- [波形模块](./waveform-module.md) - 波形命令详情
