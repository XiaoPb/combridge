use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub devices: HashMap<String, Device>,
    pub active_device_id: Option<String>,
    pub settings: AppSettings,
    pub window_state: WindowState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            devices: HashMap::new(),
            active_device_id: None,
            settings: AppSettings::default(),
            window_state: WindowState::default(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    // ==================== 设备管理 ====================

    pub fn add_serial_device(&mut self, id: String, name: String) -> &SerialDevice {
        let device = SerialDevice::new(id.clone(), name);
        self.devices.insert(id.clone(), Device::Serial(device));
        info!("添加串口设备: {}", id);
        self.devices.get(&id).and_then(|d| match d {
            Device::Serial(sd) => Some(sd),
            _ => None,
        }).expect("刚插入的设备必须存在且类型匹配")
    }

    pub fn add_ble_device(&mut self, id: String, name: String, mac: String) -> &BleDeviceState {
        let device = BleDeviceState::new(id.clone(), name, mac.clone());
        self.devices.insert(id.clone(), Device::Ble(device));
        info!("添加蓝牙设备: {} ({})", id, mac);
        self.devices.get(&id).and_then(|d| match d {
            Device::Ble(bd) => Some(bd),
            _ => None,
        }).expect("刚插入的设备必须存在且类型匹配")
    }

    pub fn remove_device(&mut self, device_id: &str) -> Option<Device> {
        let device = self.devices.remove(device_id)?;
        if self.active_device_id.as_deref() == Some(device_id) {
            self.active_device_id = self.devices.keys().next().cloned();
        }
        info!("移除设备: {}", device_id);
        Some(device)
    }

    pub fn get_device(&self, device_id: &str) -> Option<&Device> {
        self.devices.get(device_id)
    }

    pub fn get_device_mut(&mut self, device_id: &str) -> Option<&mut Device> {
        self.devices.get_mut(device_id)
    }

    pub fn get_serial_device(&self, device_id: &str) -> Option<&SerialDevice> {
        self.devices.get(device_id).and_then(|d| match d {
            Device::Serial(sd) => Some(sd),
            _ => None,
        })
    }

    pub fn get_serial_device_mut(&mut self, device_id: &str) -> Option<&mut SerialDevice> {
        self.devices.get_mut(device_id).and_then(|d| match d {
            Device::Serial(sd) => Some(sd),
            _ => None,
        })
    }

    pub fn get_ble_device(&self, device_id: &str) -> Option<&BleDeviceState> {
        self.devices.get(device_id).and_then(|d| match d {
            Device::Ble(bd) => Some(bd),
            _ => None,
        })
    }

    pub fn get_ble_device_mut(&mut self, device_id: &str) -> Option<&mut BleDeviceState> {
        self.devices.get_mut(device_id).and_then(|d| match d {
            Device::Ble(bd) => Some(bd),
            _ => None,
        })
    }

    pub fn set_device_connected(&mut self, device_id: &str, connected: bool) -> bool {
        if let Some(device) = self.devices.get_mut(device_id) {
            device.set_connected(connected);
            debug!("设备 {} 连接状态: {}", device_id, connected);
            true
        } else {
            false
        }
    }

    pub fn get_connected_devices(&self) -> Vec<&Device> {
        self.devices.values().filter(|d| d.connected()).collect()
    }

    pub fn get_devices_by_type(&self, is_serial: bool) -> Vec<&Device> {
        self.devices.values().filter(|d| match d {
            Device::Serial(_) => is_serial,
            Device::Ble(_) => !is_serial,
        }).collect()
    }

    // ==================== 通道管理 ====================

    pub fn add_channel(&mut self, device_id: &str, channel_id: String, direction: ChannelDirection) -> bool {
        if let Some(device) = self.devices.get_mut(device_id) {
            match device {
                Device::Serial(_sd) => {
                    debug!("串口设备 {} 的通道是固定的，无法添加", device_id);
                    false
                }
                Device::Ble(bd) => {
                    bd.add_channel(&channel_id, direction);
                    debug!("为蓝牙设备 {} 添加通道: {}", device_id, channel_id);
                    true
                }
            }
        } else {
            warn!("设备不存在: {}", device_id);
            false
        }
    }

    pub fn get_channel(&self, device_id: &str, channel_id: &str) -> Option<&Channel> {
        self.devices.get(device_id)?.get_channel(channel_id)
    }

    pub fn get_channel_mut(&mut self, device_id: &str, channel_id: &str) -> Option<&mut Channel> {
        self.devices.get_mut(device_id)?.get_channel_mut(channel_id)
    }

    pub fn set_channel_subscribed(&mut self, device_id: &str, channel_id: &str, subscribed: bool) -> bool {
        if let Some(channel) = self.get_channel_mut(device_id, channel_id) {
            channel.subscribed = subscribed;
            debug!("通道 {}/{} 订阅状态: {}", device_id, channel_id, subscribed);
            true
        } else {
            false
        }
    }

    // ==================== 数据操作 ====================

    pub fn add_data_to_channel(&mut self, device_id: &str, channel_id: &str, data: &[u8]) -> bool {
        let max_size = self.settings.max_buffer_size;
        if let Some(channel) = self.get_channel_mut(device_id, channel_id) {
            channel.buffer.add_entry(data, max_size);
            debug!("通道 {}/{} 添加数据: {} 字节", device_id, channel_id, data.len());
            true
        } else {
            warn!("通道不存在: {}/{}", device_id, channel_id);
            false
        }
    }

    pub fn clear_channel_buffer(&mut self, device_id: &str, channel_id: &str) -> bool {
        if let Some(channel) = self.get_channel_mut(device_id, channel_id) {
            channel.buffer.clear();
            debug!("通道 {}/{} 缓冲区已清空", device_id, channel_id);
            true
        } else {
            false
        }
    }

    // ==================== 串口特有操作 ====================

    pub fn add_serial_tx_data(&mut self, device_id: &str, data: &[u8]) -> bool {
        let max_size = self.settings.max_buffer_size;
        if let Some(sd) = self.get_serial_device_mut(device_id) {
            if let Some(tx) = sd.tx_channel_mut() {
                tx.buffer.add_entry(data, max_size);
                debug!("串口 {} TX 数据: {} 字节", device_id, data.len());
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn add_serial_rx_data(&mut self, device_id: &str, data: &[u8]) -> bool {
        let max_size = self.settings.max_buffer_size;
        if let Some(sd) = self.get_serial_device_mut(device_id) {
            if let Some(rx) = sd.rx_channel_mut() {
                rx.buffer.add_entry(data, max_size);
                debug!("串口 {} RX 数据: {} 字节", device_id, data.len());
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn update_serial_config(&mut self, device_id: &str, baud_rate: u32, data_bits: DataBits, parity: Parity, stop_bits: StopBits) -> bool {
        if let Some(sd) = self.get_serial_device_mut(device_id) {
            sd.baud_rate = baud_rate;
            sd.data_bits = data_bits;
            sd.parity = parity;
            sd.stop_bits = stop_bits;
            debug!("串口 {} 配置已更新", device_id);
            true
        } else {
            false
        }
    }

    // ==================== 蓝牙特有操作 ====================

    pub fn update_ble_mtu(&mut self, device_id: &str, mtu: u16) -> bool {
        if let Some(bd) = self.get_ble_device_mut(device_id) {
            bd.mtu = mtu;
            debug!("蓝牙设备 {} MTU: {}", device_id, mtu);
            true
        } else {
            false
        }
    }

    pub fn update_ble_connection_params(&mut self, device_id: &str, params: ConnectionParams) -> bool {
        if let Some(bd) = self.get_ble_device_mut(device_id) {
            bd.connection_params = params;
            debug!("蓝牙设备 {} 连接参数已更新", device_id);
            true
        } else {
            false
        }
    }

    // ==================== 设备切换 ====================

    pub fn switch_device(&mut self, device_id: &str) -> bool {
        if self.devices.contains_key(device_id) {
            self.active_device_id = Some(device_id.to_string());
            debug!("切换到设备: {}", device_id);
            true
        } else {
            warn!("设备不存在: {}", device_id);
            false
        }
    }

    // ==================== TAB 管理 ====================

    pub fn add_tab(&mut self, device_id: String, channel_id: Option<String>, label: String) -> String {
        let key = format!("tab-{}-{}", device_id, current_timestamp());
        let tab = TabState {
            key: key.clone(),
            device_id,
            channel_id,
            label,
            is_active: true,
        };
        
        for t in &mut self.window_state.tabs {
            t.is_active = false;
        }
        
        self.window_state.tabs.push(tab);
        self.window_state.active_tab_key = Some(key.clone());
        debug!("TAB 已添加: {}", key);
        key
    }

    pub fn remove_tab(&mut self, tab_key: &str) -> bool {
        if let Some(pos) = self.window_state.tabs.iter().position(|t| t.key == tab_key) {
            self.window_state.tabs.remove(pos);
            if self.window_state.active_tab_key.as_deref() == Some(tab_key) {
                self.window_state.active_tab_key = self.window_state.tabs.last().map(|t| t.key.clone());
            }
            debug!("TAB 已移除: {}", tab_key);
            true
        } else {
            false
        }
    }

    pub fn switch_tab(&mut self, tab_key: &str) -> bool {
        if self.window_state.tabs.iter().any(|t| t.key == tab_key) {
            for t in &mut self.window_state.tabs {
                t.is_active = t.key == tab_key;
            }
            self.window_state.active_tab_key = Some(tab_key.to_string());
            debug!("TAB 已切换: {}", tab_key);
            true
        } else {
            false
        }
    }
}

pub type AppStateRef = Arc<RwLock<AppState>>;

pub fn create_app_state() -> AppStateRef {
    Arc::new(RwLock::new(AppState::new()))
}
