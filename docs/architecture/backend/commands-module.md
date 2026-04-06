# 命令层模块

## 概述

命令层模块定义了所有 Tauri 命令，作为前端调用后端的入口点。每个命令模块对应特定的功能领域。

## 模块位置

- 源码路径：`src-tauri/src/commands/`
- 主要文件：
  - `mod.rs` - 模块导出
  - `serial.rs` - 串口命令
  - `ble.rs` - BLE 命令
  - `protocol.rs` - 协议命令
  - `system.rs` - 系统命令
  - `websocket.rs` - WebSocket 命令
  - `state.rs` - 状态命令
  - `preferences.rs` - 偏好设置命令
  - `gh3036.rs` - GH3036 命令
  - `waveform.rs` - 波形命令

## 命令注册

在 `lib.rs` 中注册所有命令：

```rust
.invoke_handler(tauri::generate_handler![
    // 串口命令
    commands::serial::scan_serial_ports,
    commands::serial::open_serial_port,
    commands::serial::close_serial_port,
    commands::serial::send_serial_data,
    commands::serial::get_open_ports,
    commands::serial::is_port_open,
    commands::serial::export_serial_data,
    
    // BLE 命令
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
    
    // WebSocket 命令
    commands::websocket::connect_websocket,
    commands::websocket::send_websocket_message,
    commands::websocket::disconnect_websocket,
    commands::websocket::get_websocket_status,
    commands::websocket::get_all_websocket_connections,
    commands::websocket::get_all_websocket_status,
    
    // 协议命令
    commands::protocol::load_protocol,
    commands::protocol::unload_protocol,
    commands::protocol::enable_protocol,
    commands::protocol::disable_protocol,
    commands::protocol::bind_protocol,
    commands::protocol::unbind_protocol,
    commands::protocol::list_protocols,
    commands::protocol::get_protocol,
    commands::protocol::get_bound_protocols,
    
    // 系统命令
    commands::system::get_system_info,
    commands::system::get_system_status,
    commands::system::configure_log,
    commands::system::get_log_config,
    commands::system::get_runtime_status,
    commands::system::get_app_version,
    commands::system::get_platform,
    commands::system::open_url,
    commands::system::show_in_folder,
    commands::system::show_main_window,
    
    // 状态命令
    commands::state::dispatch_action,
    commands::state::get_state,
    commands::state::get_channel_data,
    commands::state::restore_state,
    commands::state::save_state,
    commands::state::get_connected_devices,
    commands::state::get_window_state,
    
    // 偏好设置命令
    commands::preferences::get_preferences,
    commands::preferences::save_preferences,
    commands::preferences::update_serial_preferences,
    commands::preferences::update_ble_preferences,
    
    // GH3036 命令
    commands::gh3036::gh3036_init,
    commands::gh3036::gh3036_is_initialized,
    commands::gh3036::gh3036_configure_tx_channel,
    commands::gh3036::gh3036_configure_rx_channel,
    commands::gh3036::gh3036_get_channels,
    commands::gh3036::gh3036_send_data,
    commands::gh3036::gh3036_set_csv_config,
    commands::gh3036::gh3036_get_csv_config,
    commands::gh3036::gh3036_get_rpc_commands,
    commands::gh3036::gh3036_execute_rpc,
    commands::gh3036::gh3036_subscribe_events,
    commands::gh3036::gh3036_get_library_status,
    commands::gh3036::gh3036_on_rx_data,
    
    // 波形命令
    commands::waveform::waveform_create_buffer,
    commands::waveform::waveform_remove_buffer,
    commands::waveform::waveform_configure_parser,
    commands::waveform::waveform_parse_and_store,
    commands::waveform::waveform_read_data,
    commands::waveform::waveform_get_status,
    commands::waveform::waveform_clear_buffer,
    commands::waveform::waveform_list_buffers,
])
```

## 串口命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `scan_serial_ports` | 无 | `Vec<PortInfo>` | 扫描可用串口 |
| `open_serial_port` | `config: SerialPortConfig` | `()` | 打开串口 |
| `close_serial_port` | `port_name: String` | `()` | 关闭串口 |
| `send_serial_data` | `port_name: String, data: Vec<u8>` | `usize` | 发送数据 |
| `get_open_ports` | 无 | `Vec<String>` | 获取已打开端口 |
| `is_port_open` | `port_name: String` | `bool` | 检查端口状态 |
| `export_serial_data` | `port_name: String, path: String` | `()` | 导出数据 |

## BLE 命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `configure_ble` | `mode: BleMode, config: Option<AtConfig>` | `()` | 配置 BLE 模式 |
| `scan_ble_devices` | `duration_ms: u64` | `Vec<BleDevice>` | 扫描设备 |
| `stop_ble_scan` | 无 | `Vec<BleDevice>` | 停止扫描 |
| `connect_ble` | `address: String` | `BleConnection` | 连接设备 |
| `disconnect_ble` | `address: String` | `()` | 断开连接 |
| `get_ble_connections` | 无 | `Vec<BleConnection>` | 获取连接列表 |
| `discover_ble_services` | `address: String` | `Vec<BleService>` | 发现服务 |
| `discover_ble_characteristics` | `address: String, service_uuid: String` | `Vec<BleCharacteristic>` | 发现特征 |
| `read_ble_characteristic` | `address: String, char_uuid: String` | `Vec<u8>` | 读取特征 |
| `write_ble_characteristic` | `address: String, char_uuid: String, data: Vec<u8>` | `()` | 写入特征 |
| `write_ble_without_response` | `address: String, char_uuid: String, data: Vec<u8>` | `()` | 无响应写入 |
| `subscribe_ble_notify` | `address: String, char_uuid: String` | `()` | 订阅通知 |
| `unsubscribe_ble_notify` | `address: String, char_uuid: String` | `()` | 取消订阅 |
| `get_ble_rssi` | `address: String` | `i16` | 获取 RSSI |
| `get_ble_mode` | 无 | `BleMode` | 获取当前模式 |
| `is_ble_configured` | 无 | `bool` | 检查配置状态 |
| `set_ble_mtu` | `address: String, mtu: u16` | `u16` | 设置 MTU |
| `get_ble_subscriptions` | `address: String` | `Vec<String>` | 获取订阅列表 |

## WebSocket 命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `connect_websocket` | `id: String, url: String` | `()` | 连接服务器 |
| `send_websocket_message` | `id: String, message: String` | `()` | 发送消息 |
| `disconnect_websocket` | `id: String` | `()` | 断开连接 |
| `get_websocket_status` | `id: String` | `ConnectionStatus` | 获取状态 |
| `get_all_websocket_connections` | 无 | `Vec<String>` | 获取所有连接 |
| `get_all_websocket_status` | 无 | `HashMap<String, ConnectionStatus>` | 获取所有状态 |

## 协议命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `load_protocol` | `path: String` | `String` | 加载协议 |
| `unload_protocol` | `id: String` | `()` | 卸载协议 |
| `enable_protocol` | `id: String` | `()` | 启用协议 |
| `disable_protocol` | `id: String` | `()` | 禁用协议 |
| `bind_protocol` | `device_id: String, protocol_id: String` | `()` | 绑定协议 |
| `unbind_protocol` | `device_id: String` | `()` | 解绑协议 |
| `list_protocols` | 无 | `Vec<PluginInfo>` | 列出协议 |
| `get_protocol` | `id: String` | `Option<PluginInfo>` | 获取协议 |
| `get_bound_protocols` | 无 | `HashMap<String, String>` | 获取绑定 |

## 系统命令

| 命令 | 参数 | 返回值 | 说明 |
|------|------|--------|------|
| `get_system_info` | 无 | `SystemInfo` | 获取系统信息 |
| `get_system_status` | 无 | `SystemStatus` | 获取运行状态 |
| `configure_log` | `level: String` | `()` | 配置日志 |
| `get_log_config` | 无 | `LogConfig` | 获取日志配置 |
| `get_runtime_status` | 无 | `RuntimeStatus` | 获取运行时状态 |
| `get_app_version` | 无 | `String` | 获取版本 |
| `get_platform` | 无 | `String` | 获取平台 |
| `open_url` | `url: String` | `()` | 打开 URL |
| `show_in_folder` | `path: String` | `()` | 在文件夹显示 |
| `show_main_window` | 无 | `()` | 显示主窗口 |

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
