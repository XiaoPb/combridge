use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use tauri::{AppHandle, Emitter};
use tracing::{error, info};

use crate::device::{BaudRate, BleManagerRef, FlowControl, SerialManagerRef};

use super::action::{Action, ActionResult};
use super::app_state::AppStateRef;
use super::persistence::StatePersistenceRef;
use super::types::*;

const STATE_CHANGE_EVENT: &str = "state-change";
const SAVE_DEBOUNCE_MS: u64 = 500;

enum SaveStrategy {
    Skip,
    Immediate,
    Debounced,
}

pub struct ActionDispatcher {
    state: AppStateRef,
    persistence: StatePersistenceRef,
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
    last_save: Mutex<Option<Instant>>,
}

impl ActionDispatcher {
    pub fn new(
        state: AppStateRef,
        persistence: StatePersistenceRef,
        serial_manager: SerialManagerRef,
        ble_manager: BleManagerRef,
    ) -> Self {
        Self {
            state,
            persistence,
            serial_manager,
            ble_manager,
            last_save: Mutex::new(None),
        }
    }

    pub async fn dispatch(&self, action: Action, app: &AppHandle) -> ActionResult {
        info!("处理 Action: {}", action);

        let save_strategy = Self::get_save_strategy(&action);
        
        let result = match action {
            Action::DeviceAddSerial { id, name, baud_rate } => {
                self.handle_device_add_serial(&id, &name, baud_rate).await
            }
            Action::DeviceAddBle { id, name, mac } => {
                self.handle_device_add_ble(&id, &name, &mac).await
            }
            Action::DeviceRemove { device_id } => {
                self.handle_device_remove(&device_id).await
            }
            Action::DeviceConnect { device_id } => {
                self.handle_device_connect(&device_id, app).await
            }
            Action::DeviceDisconnect { device_id } => {
                self.handle_device_disconnect(&device_id).await
            }
            Action::DeviceUpdateConfig { device_id, config } => {
                self.handle_device_update_config(&device_id, config).await
            }
            Action::ChannelAdd { device_id, channel_id, direction } => {
                self.handle_channel_add(&device_id, &channel_id, &direction).await
            }
            Action::ChannelSubscribe { device_id, channel_id, subscribe } => {
                self.handle_channel_subscribe(&device_id, &channel_id, subscribe).await
            }
            Action::DataSend { device_id, channel_id, data } => {
                self.handle_data_send(&device_id, &channel_id, &data, app).await
            }
            Action::DataReceive { device_id, channel_id, data } => {
                self.handle_data_receive(&device_id, &channel_id, &data).await
            }
            Action::BufferClear { device_id, channel_id } => {
                self.handle_buffer_clear(&device_id, &channel_id).await
            }
            Action::DeviceSwitch { device_id } => {
                self.handle_device_switch(&device_id).await
            }
            Action::TabAdd { device_id, channel_id, label } => {
                self.handle_tab_add(&device_id, channel_id, &label).await
            }
            Action::TabRemove { tab_key } => {
                self.handle_tab_remove(&tab_key).await
            }
            Action::TabSwitch { tab_key } => {
                self.handle_tab_switch(&tab_key).await
            }
            Action::SettingsUpdate { settings } => {
                self.handle_settings_update(settings).await
            }
            Action::StateRestore { window_state } => {
                self.handle_state_restore(window_state).await
            }
        };

        if result.success {
            self.broadcast_state_change(app).await;
            self.save_with_strategy(save_strategy).await;
        }
        
        result
    }

    async fn handle_device_add_serial(&self, id: &str, name: &str, baud_rate: u32) -> ActionResult {
        let mut state = self.state.write().await;
        state.add_serial_device(id.to_string(), name.to_string());
        if let Some(sd) = state.get_serial_device_mut(id) {
            sd.baud_rate = baud_rate;
        }
        ActionResult::success_with_data(serde_json::json!({ "deviceId": id }))
    }

    async fn handle_device_add_ble(&self, id: &str, name: &str, mac: &str) -> ActionResult {
        let mut state = self.state.write().await;
        state.add_ble_device(id.to_string(), name.to_string(), mac.to_string());
        ActionResult::success_with_data(serde_json::json!({ "deviceId": id }))
    }

    async fn handle_device_remove(&self, device_id: &str) -> ActionResult {
        let mut state = self.state.write().await;
        match state.remove_device(device_id) {
            Some(device) => {
                ActionResult::success_with_message(format!("设备 {} 已移除", device.name()))
            }
            None => ActionResult::failure(format!("设备不存在: {}", device_id)),
        }
    }

    async fn handle_device_connect(&self, device_id: &str, app: &AppHandle) -> ActionResult {
        let device_type = {
            let state = self.state.read().await;
            match state.get_device(device_id) {
                Some(device) => match device {
                    Device::Serial(_) => "serial",
                    Device::Ble(_) => "ble",
                },
                None => return ActionResult::failure(format!("设备不存在: {}", device_id)),
            }
        };

        let result = match device_type {
            "serial" => self.connect_serial(device_id, app).await,
            "ble" => self.connect_ble(device_id, app).await,
            _ => ActionResult::failure(format!("未知的设备类型: {}", device_type)),
        };

        if result.success {
            let mut state = self.state.write().await;
            state.set_device_connected(device_id, true);
        }

        result
    }

    async fn connect_serial(&self, device_id: &str, app: &AppHandle) -> ActionResult {
        let (port_name, baud_rate, data_bits, parity, stop_bits) = {
            let state = self.state.read().await;
            match state.get_serial_device(device_id) {
                Some(sd) => (
                    sd.name.clone(),
                    sd.baud_rate,
                    sd.data_bits,
                    sd.parity,
                    sd.stop_bits,
                ),
                None => return ActionResult::failure(format!("串口设备不存在: {}", device_id)),
            }
        };

        let baud_rate_enum = match baud_rate {
            1200 => BaudRate::B1200,
            2400 => BaudRate::B2400,
            4800 => BaudRate::B4800,
            9600 => BaudRate::B9600,
            19200 => BaudRate::B19200,
            38400 => BaudRate::B38400,
            57600 => BaudRate::B57600,
            115200 => BaudRate::B115200,
            230400 => BaudRate::B230400,
            460800 => BaudRate::B460800,
            921600 => BaudRate::B921600,
            _ => BaudRate::B115200,
        };

        let port_config = crate::device::SerialPortConfig {
            port_name: port_name.clone(),
            baud_rate: baud_rate_enum,
            data_bits,
            parity,
            stop_bits,
            flow_control: FlowControl::None,
            timeout_ms: 1000,
            pack_timeout_ms: 50,
        };

        let state = self.state.clone();
        let device_id_owned = device_id.to_string();
        let app_clone = app.clone();

        match self.serial_manager.open_port(port_config, move |_name, data| {
            let state = state.clone();
            let device_id = device_id_owned.clone();
            let app = app_clone.clone();
            let data = data.to_vec();

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut state = state.write().await;
                    state.add_serial_rx_data(&device_id, &data);
                    drop(state);

                    let _ = app.emit(STATE_CHANGE_EVENT, ());
                });
            }
        }) {
            Ok(()) => ActionResult::success_with_message(format!("串口 {} 已连接", port_name)),
            Err(e) => ActionResult::failure(format!("连接串口失败: {}", e)),
        }
    }

    async fn connect_ble(&self, device_id: &str, _app: &AppHandle) -> ActionResult {
        let address = {
            let state = self.state.read().await;
            match state.get_ble_device(device_id) {
                Some(bd) => bd.mac.clone(),
                None => return ActionResult::failure(format!("BLE 设备不存在: {}", device_id)),
            }
        };

        match self.ble_manager.connect(&address).await {
            Ok(_connection) => {
                info!("BLE 设备 {} ({}) 已连接", device_id, address);
                ActionResult::success_with_message(format!("BLE 设备 {} 已连接", address))
            }
            Err(e) => {
                error!("连接 BLE 设备 {} 失败: {}", address, e);
                ActionResult::failure(format!("连接 BLE 设备失败: {}", e))
            }
        }
    }

    async fn disconnect_ble(&self, device_id: &str) -> ActionResult {
        let address = {
            let state = self.state.read().await;
            match state.get_ble_device(device_id) {
                Some(bd) => bd.mac.clone(),
                None => return ActionResult::failure(format!("BLE 设备不存在: {}", device_id)),
            }
        };

        match self.ble_manager.disconnect(&address).await {
            Ok(()) => {
                info!("BLE 设备 {} ({}) 已断开", device_id, address);
                ActionResult::success_with_message(format!("BLE 设备 {} 已断开", address))
            }
            Err(e) => {
                error!("断开 BLE 设备 {} 失败: {}", address, e);
                ActionResult::failure(format!("断开 BLE 设备失败: {}", e))
            }
        }
    }

    async fn handle_device_disconnect(&self, device_id: &str) -> ActionResult {
        let device_type = {
            let state = self.state.read().await;
            match state.get_device(device_id) {
                Some(device) => match device {
                    Device::Serial(_) => "serial",
                    Device::Ble(_) => "ble",
                },
                None => return ActionResult::failure(format!("设备不存在: {}", device_id)),
            }
        };

        let result = match device_type {
            "serial" => {
                let port_name = {
                    let state = self.state.read().await;
                    state.get_serial_device(device_id).map(|sd| sd.name.clone())
                };
                match port_name {
                    Some(name) => match self.serial_manager.close_port(&name) {
                        Ok(()) => ActionResult::success_with_message(format!("串口 {} 已断开", name)),
                        Err(e) => ActionResult::failure(format!("断开串口失败: {}", e)),
                    },
                    None => ActionResult::failure(format!("串口设备不存在: {}", device_id)),
                }
            }
            "ble" => self.disconnect_ble(device_id).await,
            _ => ActionResult::failure(format!("未知的设备类型: {}", device_type)),
        };

        if result.success {
            let mut state = self.state.write().await;
            state.set_device_connected(device_id, false);
        }

        result
    }

    async fn handle_device_update_config(&self, device_id: &str, config: serde_json::Value) -> ActionResult {
        let device_type = {
            let state = self.state.read().await;
            match state.get_device(device_id) {
                Some(device) => match device {
                    Device::Serial(_) => "serial",
                    Device::Ble(_) => "ble",
                },
                None => return ActionResult::failure(format!("设备不存在: {}", device_id)),
            }
        };

        match device_type {
            "serial" => {
                let baud_rate = config.get("baudRate").and_then(|v| v.as_u64()).unwrap_or(115200) as u32;
                let data_bits = match config.get("dataBits").and_then(|v| v.as_u64()).unwrap_or(8) {
                    5 => DataBits::Five,
                    6 => DataBits::Six,
                    7 => DataBits::Seven,
                    _ => DataBits::Eight,
                };
                let parity = match config.get("parity").and_then(|v| v.as_str()).unwrap_or("none") {
                    "odd" => Parity::Odd,
                    "even" => Parity::Even,
                    _ => Parity::None,
                };
                let stop_bits = match config.get("stopBits").and_then(|v| v.as_u64()).unwrap_or(1) {
                    2 => StopBits::Two,
                    _ => StopBits::One,
                };

                let mut state = self.state.write().await;
                if state.update_serial_config(device_id, baud_rate, data_bits, parity, stop_bits) {
                    ActionResult::success()
                } else {
                    ActionResult::failure("更新串口配置失败")
                }
            }
            "ble" => {
                if let Some(mtu) = config.get("mtu").and_then(|v| v.as_u64()) {
                    let mut state = self.state.write().await;
                    if state.update_ble_mtu(device_id, mtu as u16) {
                        ActionResult::success()
                    } else {
                        ActionResult::failure("更新 BLE MTU 失败")
                    }
                } else {
                    ActionResult::failure("无效的 BLE 配置")
                }
            }
            _ => ActionResult::failure(format!("未知的设备类型: {}", device_type)),
        }
    }

    async fn handle_channel_add(&self, device_id: &str, channel_id: &str, direction: &str) -> ActionResult {
        let direction = match direction {
            "read" => ChannelDirection::Read,
            "write" => ChannelDirection::Write,
            "notify" => ChannelDirection::Notify,
            _ => return ActionResult::failure(format!("未知的通道方向: {}", direction)),
        };

        let mut state = self.state.write().await;
        if state.add_channel(device_id, channel_id.to_string(), direction) {
            ActionResult::success()
        } else {
            ActionResult::failure("添加通道失败")
        }
    }

    async fn handle_channel_subscribe(&self, device_id: &str, channel_id: &str, subscribe: bool) -> ActionResult {
        let mut state = self.state.write().await;
        if state.set_channel_subscribed(device_id, channel_id, subscribe) {
            ActionResult::success()
        } else {
            ActionResult::failure("设置订阅状态失败")
        }
    }

    async fn handle_data_send(&self, device_id: &str, channel_id: &str, data: &[u8], app: &AppHandle) -> ActionResult {
        let device_type = {
            let state = self.state.read().await;
            match state.get_device(device_id) {
                Some(device) => match device {
                    Device::Serial(_) => "serial",
                    Device::Ble(_) => "ble",
                },
                None => return ActionResult::failure(format!("设备不存在: {}", device_id)),
            }
        };

        let result = match device_type {
            "serial" => {
                let port_name = {
                    let state = self.state.read().await;
                    state.get_serial_device(device_id).map(|sd| sd.name.clone())
                };
                match port_name {
                    Some(name) => match self.serial_manager.send_data(&name, data) {
                        Ok(bytes) => ActionResult::success_with_data(serde_json::json!({ "bytesSent": bytes })),
                        Err(e) => ActionResult::failure(format!("发送数据失败: {}", e)),
                    },
                    None => ActionResult::failure(format!("串口设备不存在: {}", device_id)),
                }
            }
            "ble" => {
                let address = {
                    let state = self.state.read().await;
                    match state.get_ble_device(device_id) {
                        Some(bd) => bd.mac.clone(),
                        None => return ActionResult::failure(format!("BLE 设备不存在: {}", device_id)),
                    }
                };

                let char_uuid = match channel_id.rsplit_once('_') {
                    Some((uuid, _direction)) => uuid.to_string(),
                    None => channel_id.to_string(),
                };

                match self.ble_manager.write_characteristic(&address, &char_uuid, data).await {
                    Ok(()) => ActionResult::success_with_data(serde_json::json!({ "bytesSent": data.len() })),
                    Err(e) => {
                        error!("BLE 发送数据失败 (设备: {}, 特征: {}): {}", address, char_uuid, e);
                        ActionResult::failure(format!("BLE 发送数据失败: {}", e))
                    }
                }
            }
            _ => ActionResult::failure(format!("未知的设备类型: {}", device_type)),
        };

        if result.success {
            let mut state = self.state.write().await;
            if device_type == "serial" {
                state.add_serial_tx_data(device_id, data);
            } else {
                state.add_data_to_channel(device_id, channel_id, data);
            }
            drop(state);
            self.broadcast_state_change(app).await;
        }

        result
    }

    async fn handle_data_receive(&self, device_id: &str, channel_id: &str, data: &[u8]) -> ActionResult {
        let mut state = self.state.write().await;
        if state.add_data_to_channel(device_id, channel_id, data) {
            ActionResult::success()
        } else {
            ActionResult::failure("添加接收数据失败")
        }
    }

    async fn handle_buffer_clear(&self, device_id: &str, channel_id: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.clear_channel_buffer(device_id, channel_id) {
            ActionResult::success_with_message("缓冲区已清空")
        } else {
            ActionResult::failure("清空缓冲区失败")
        }
    }

    async fn handle_device_switch(&self, device_id: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.switch_device(device_id) {
            ActionResult::success()
        } else {
            ActionResult::failure(format!("设备不存在: {}", device_id))
        }
    }

    async fn handle_tab_add(&self, device_id: &str, channel_id: Option<String>, label: &str) -> ActionResult {
        let mut state = self.state.write().await;
        let tab_key = state.add_tab(device_id.to_string(), channel_id, label.to_string());
        ActionResult::success_with_data(serde_json::json!({ "tabKey": tab_key }))
    }

    async fn handle_tab_remove(&self, tab_key: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.remove_tab(tab_key) {
            ActionResult::success()
        } else {
            ActionResult::failure(format!("TAB 不存在: {}", tab_key))
        }
    }

    async fn handle_tab_switch(&self, tab_key: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.switch_tab(tab_key) {
            ActionResult::success()
        } else {
            ActionResult::failure(format!("TAB 不存在: {}", tab_key))
        }
    }

    async fn handle_settings_update(&self, settings: serde_json::Value) -> ActionResult {
        match serde_json::from_value::<AppSettings>(settings) {
            Ok(new_settings) => {
                let mut state = self.state.write().await;
                state.settings = new_settings;
                ActionResult::success()
            }
            Err(e) => ActionResult::failure(format!("解析设置失败: {}", e)),
        }
    }

    async fn handle_state_restore(&self, window_state: serde_json::Value) -> ActionResult {
        match serde_json::from_value::<WindowState>(window_state) {
            Ok(ws) => {
                let mut state = self.state.write().await;
                state.window_state = ws;
                ActionResult::success()
            }
            Err(e) => ActionResult::failure(format!("解析窗口状态失败: {}", e)),
        }
    }

    async fn broadcast_state_change(&self, app: &AppHandle) {
        let state = self.state.read().await;
        let state_json = match serde_json::to_value(&*state) {
            Ok(json) => json,
            Err(e) => {
                error!("序列化状态失败: {}", e);
                return;
            }
        };

        if let Err(e) = app.emit(STATE_CHANGE_EVENT, state_json) {
            error!("广播状态变更失败: {}", e);
        }
    }

    async fn save_state(&self) {
        let state = self.state.read().await;
        let persistence = self.persistence.read().await;
        if let Err(e) = persistence.save(&state).await {
            error!("保存状态失败: {}", e);
        }
    }

    fn get_save_strategy(action: &Action) -> SaveStrategy {
        match action {
            Action::DataSend { .. } | Action::DataReceive { .. } => SaveStrategy::Skip,
            Action::DeviceConnect { .. }
            | Action::DeviceDisconnect { .. }
            | Action::DeviceAddSerial { .. }
            | Action::DeviceAddBle { .. }
            | Action::DeviceRemove { .. }
            | Action::DeviceUpdateConfig { .. }
            | Action::SettingsUpdate { .. }
            | Action::StateRestore { .. } => SaveStrategy::Immediate,
            _ => SaveStrategy::Debounced,
        }
    }

    async fn save_with_strategy(&self, strategy: SaveStrategy) {
        match strategy {
            SaveStrategy::Skip => {}
            SaveStrategy::Immediate => {
                self.save_state().await;
                if let Ok(mut last) = self.last_save.lock() {
                    *last = Some(Instant::now());
                }
            }
            SaveStrategy::Debounced => {
                let should_save = if let Ok(last) = self.last_save.lock() {
                    match *last {
                        Some(instant) => instant.elapsed().as_millis() as u64 >= SAVE_DEBOUNCE_MS,
                        None => true,
                    }
                } else {
                    true
                };

                if should_save {
                    self.save_state().await;
                    if let Ok(mut last) = self.last_save.lock() {
                        *last = Some(Instant::now());
                    }
                }
            }
        }
    }
}

pub type ActionDispatcherRef = Arc<ActionDispatcher>;

pub fn create_action_dispatcher(
    state: AppStateRef,
    persistence: StatePersistenceRef,
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
) -> ActionDispatcherRef {
    Arc::new(ActionDispatcher::new(state, persistence, serial_manager, ble_manager))
}
