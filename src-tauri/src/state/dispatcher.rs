use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tracing::{error, info};

use crate::device::{BaudRate, BleManagerRef, DataBits, FlowControl, Parity, SerialManagerRef, StopBits};

use super::action::{Action, ActionResult};
use super::app_state::AppStateRef;
use super::persistence::StatePersistenceRef;
use super::types::*;

const STATE_CHANGE_EVENT: &str = "state-change";

pub struct ActionDispatcher {
    state: AppStateRef,
    persistence: StatePersistenceRef,
    serial_manager: SerialManagerRef,
    ble_manager: BleManagerRef,
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
        }
    }

    pub async fn dispatch(&self, action: Action, app: &AppHandle) -> ActionResult {
        info!("处理 Action: {}", action);
        
        let result = match action {
            Action::ChannelAdd { name, channel_type, config } => {
                self.handle_channel_add(&name, &channel_type, config).await
            }
            Action::ChannelRemove { id } => {
                self.handle_channel_remove(&id).await
            }
            Action::ChannelConnect { id, config } => {
                self.handle_channel_connect(&id, config, app).await
            }
            Action::ChannelDisconnect { id } => {
                self.handle_channel_disconnect(&id).await
            }
            Action::DataSend { channel_id, data } => {
                self.handle_data_send(&channel_id, &data, app).await
            }
            Action::ChannelSwitch { channel_id } => {
                self.handle_channel_switch(&channel_id).await
            }
            Action::BufferClear { channel_id, direction } => {
                self.handle_buffer_clear(&channel_id, &direction).await
            }
            Action::TabAdd { channel_id, label } => {
                self.handle_tab_add(&channel_id, &label).await
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
            self.save_state().await;
        }
        
        result
    }

    async fn handle_channel_add(
        &self,
        name: &str,
        channel_type: &str,
        config: Option<serde_json::Value>,
    ) -> ActionResult {
        let channel_type = match channel_type.to_lowercase().as_str() {
            "serial" => ChannelType::Serial,
            "ble" => ChannelType::BluetoothCharacteristic,
            _ => return ActionResult::failure(format!("未知的通道类型: {}", channel_type)),
        };
        
        let id = match channel_type {
            ChannelType::Serial => format!("serial-{}", name),
            ChannelType::BluetoothCharacteristic => format!("ble-{}-{}", name, current_timestamp()),
        };
        
        let mut channel = match channel_type {
            ChannelType::Serial => DeviceChannel::new_serial(id.clone(), name.to_string()),
            ChannelType::BluetoothCharacteristic => {
                DeviceChannel::new_ble_characteristic(
                    id.clone(),
                    name.to_string(),
                    None,
                    "".to_string(),
                    "".to_string(),
                )
            }
        };
        
        if let Some(cfg) = config {
            if let Ok(serial_config) = serde_json::from_value::<SerialConfig>(cfg.clone()) {
                channel.config = Some(ChannelConfig::Serial(serial_config));
            }
        }
        
        let mut state = self.state.write().await;
        state.add_channel(channel);
        
        ActionResult::success_with_data(serde_json::json!({ "channelId": id }))
    }

    async fn handle_channel_remove(&self, id: &str) -> ActionResult {
        let mut state = self.state.write().await;
        match state.remove_channel(id) {
            Some(channel) => {
                ActionResult::success_with_message(format!("通道 {} 已移除", channel.name))
            }
            None => ActionResult::failure(format!("通道不存在: {}", id)),
        }
    }

    async fn handle_channel_connect(
        &self,
        id: &str,
        config: Option<serde_json::Value>,
        app: &AppHandle,
    ) -> ActionResult {
        let channel_type = {
            let state = self.state.read().await;
            match state.get_channel(id) {
                Some(channel) => channel.channel_type,
                None => return ActionResult::failure(format!("通道不存在: {}", id)),
            }
        };
        
        let result = match channel_type {
            ChannelType::Serial => {
                self.connect_serial(id, config, app).await
            }
            ChannelType::BluetoothCharacteristic => {
                self.connect_ble(id, config, app).await
            }
        };
        
        if result.success {
            let mut state = self.state.write().await;
            state.set_channel_connected(id, true);
        }
        
        result
    }

    async fn connect_serial(
        &self,
        id: &str,
        config: Option<serde_json::Value>,
        app: &AppHandle,
    ) -> ActionResult {
        let port_name = id.strip_prefix("serial-").unwrap_or(id).to_string();
        
        let serial_config = config
            .and_then(|c| serde_json::from_value::<SerialConfig>(c).ok())
            .unwrap_or_default();
        
        let baud_rate = match serial_config.baud_rate {
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
        
        let data_bits = match serial_config.data_bits {
            5 => DataBits::Five,
            6 => DataBits::Six,
            7 => DataBits::Seven,
            8 => DataBits::Eight,
            _ => DataBits::Eight,
        };
        
        let parity = match serial_config.parity.to_lowercase().as_str() {
            "none" => Parity::None,
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => Parity::None,
        };
        
        let stop_bits = match serial_config.stop_bits {
            1 => StopBits::One,
            2 => StopBits::Two,
            _ => StopBits::One,
        };
        
        let flow_control = match serial_config.flow_control.to_lowercase().as_str() {
            "none" => FlowControl::None,
            "software" => FlowControl::Software,
            "hardware" => FlowControl::Hardware,
            _ => FlowControl::None,
        };
        
        let port_config = crate::device::SerialPortConfig {
            port_name: port_name.clone(),
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            flow_control,
            timeout_ms: 1000,
            pack_timeout_ms: 50,
        };
        
        let state = self.state.clone();
        let channel_id = id.to_string();
        let app_clone = app.clone();
        
        match self.serial_manager.open_port(port_config, move |_name, data| {
            let state = state.clone();
            let channel_id = channel_id.clone();
            let app = app_clone.clone();
            let data = data.to_vec();
            
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let mut state = state.write().await;
                    state.add_rx_data(&channel_id, &data);
                    drop(state);
                    
                    let _ = app.emit(STATE_CHANGE_EVENT, ());
                });
            }
        }) {
            Ok(()) => ActionResult::success_with_message(format!("串口 {} 已连接", port_name)),
            Err(e) => ActionResult::failure(format!("连接串口失败: {}", e)),
        }
    }

    async fn connect_ble(
        &self,
        _id: &str,
        _config: Option<serde_json::Value>,
        _app: &AppHandle,
    ) -> ActionResult {
        ActionResult::failure("BLE 连接暂未实现")
    }

    async fn handle_channel_disconnect(&self, id: &str) -> ActionResult {
        let channel_type = {
            let state = self.state.read().await;
            match state.get_channel(id) {
                Some(channel) => channel.channel_type,
                None => return ActionResult::failure(format!("通道不存在: {}", id)),
            }
        };
        
        let result = match channel_type {
            ChannelType::Serial => {
                let port_name = id.strip_prefix("serial-").unwrap_or(id);
                match self.serial_manager.close_port(port_name) {
                    Ok(()) => ActionResult::success_with_message(format!("串口 {} 已断开", port_name)),
                    Err(e) => ActionResult::failure(format!("断开串口失败: {}", e)),
                }
            }
            ChannelType::BluetoothCharacteristic => {
                ActionResult::failure("BLE 断开暂未实现")
            }
        };
        
        if result.success {
            let mut state = self.state.write().await;
            state.set_channel_connected(id, false);
        }
        
        result
    }

    async fn handle_data_send(
        &self,
        channel_id: &str,
        data: &[u8],
        app: &AppHandle,
    ) -> ActionResult {
        let (channel_type, name) = {
            let state = self.state.read().await;
            match state.get_channel(channel_id) {
                Some(channel) => (channel.channel_type, channel.name.clone()),
                None => return ActionResult::failure(format!("通道不存在: {}", channel_id)),
            }
        };
        
        let result = match channel_type {
            ChannelType::Serial => {
                let port_name = channel_id.strip_prefix("serial-").unwrap_or(&name);
                match self.serial_manager.send_data(port_name, data) {
                    Ok(bytes) => ActionResult::success_with_data(serde_json::json!({ "bytesSent": bytes })),
                    Err(e) => ActionResult::failure(format!("发送数据失败: {}", e)),
                }
            }
            ChannelType::BluetoothCharacteristic => {
                ActionResult::failure("BLE 发送暂未实现")
            }
        };
        
        if result.success {
            let mut state = self.state.write().await;
            state.add_tx_data(channel_id, data);
            drop(state);
            self.broadcast_state_change(app).await;
        }
        
        result
    }

    async fn handle_channel_switch(&self, channel_id: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.get_channel(channel_id).is_some() {
            state.active_channel_id = Some(channel_id.to_string());
            ActionResult::success()
        } else {
            ActionResult::failure(format!("通道不存在: {}", channel_id))
        }
    }

    async fn handle_buffer_clear(&self, channel_id: &str, direction: &str) -> ActionResult {
        let mut state = self.state.write().await;
        if state.clear_buffer(channel_id, direction) {
            ActionResult::success_with_message(format!("缓冲区已清空: {}", direction))
        } else {
            ActionResult::failure(format!("通道不存在: {}", channel_id))
        }
    }

    async fn handle_tab_add(&self, channel_id: &str, label: &str) -> ActionResult {
        let mut state = self.state.write().await;
        let tab_key = state.add_tab(channel_id.to_string(), label.to_string());
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
