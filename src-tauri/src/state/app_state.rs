use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

use super::types::*;

const DEFAULT_MAX_BUFFER_SIZE: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub channels: Vec<DeviceChannel>,
    pub active_channel_id: Option<String>,
    pub settings: AppSettings,
    pub window_state: WindowState,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            channels: Vec::new(),
            active_channel_id: None,
            settings: AppSettings::default(),
            window_state: WindowState::default(),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_channel(&mut self, channel: DeviceChannel) {
        let id = channel.id.clone();
        if !self.channels.iter().any(|c| c.id == id) {
            self.channels.push(channel);
            debug!("通道已添加: {}", id);
        } else {
            warn!("通道已存在: {}", id);
        }
    }

    pub fn remove_channel(&mut self, id: &str) -> Option<DeviceChannel> {
        if let Some(pos) = self.channels.iter().position(|c| c.id == id) {
            let channel = self.channels.remove(pos);
            if self.active_channel_id.as_deref() == Some(id) {
                self.active_channel_id = self.channels.first().map(|c| c.id.clone());
            }
            debug!("通道已移除: {}", id);
            Some(channel)
        } else {
            None
        }
    }

    pub fn get_channel(&self, id: &str) -> Option<&DeviceChannel> {
        self.channels.iter().find(|c| c.id == id)
    }

    pub fn get_channel_mut(&mut self, id: &str) -> Option<&mut DeviceChannel> {
        self.channels.iter_mut().find(|c| c.id == id)
    }

    pub fn set_channel_connected(&mut self, id: &str, connected: bool) -> bool {
        if let Some(channel) = self.get_channel_mut(id) {
            channel.connected = connected;
            debug!("通道 {} 连接状态: {}", id, connected);
            true
        } else {
            false
        }
    }

    pub fn add_tx_data(&mut self, id: &str, data: &[u8]) -> bool {
        let max_size = self.settings.max_buffer_size;
        
        if let Some(channel) = self.get_channel_mut(id) {
            let entry = BufferEntry {
                timestamp: current_timestamp(),
                data: data.to_vec(),
                direction: "send".to_string(),
            };
            channel.tx_buffer.entries.push(entry);
            channel.tx_buffer.total_bytes += data.len();
            channel.bytes_sent += data.len() as u64;
            
            while channel.tx_buffer.total_bytes > max_size {
                if let Some(removed) = channel.tx_buffer.entries.first() {
                    channel.tx_buffer.total_bytes -= removed.data.len();
                    channel.tx_buffer.entries.remove(0);
                } else {
                    break;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn add_rx_data(&mut self, id: &str, data: &[u8]) -> bool {
        let max_size = self.settings.max_buffer_size;
        
        if let Some(channel) = self.get_channel_mut(id) {
            let entry = BufferEntry {
                timestamp: current_timestamp(),
                data: data.to_vec(),
                direction: "receive".to_string(),
            };
            channel.rx_buffer.entries.push(entry);
            channel.rx_buffer.total_bytes += data.len();
            channel.bytes_received += data.len() as u64;
            
            while channel.rx_buffer.total_bytes > max_size {
                if let Some(removed) = channel.rx_buffer.entries.first() {
                    channel.rx_buffer.total_bytes -= removed.data.len();
                    channel.rx_buffer.entries.remove(0);
                } else {
                    break;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn clear_buffer(&mut self, id: &str, direction: &str) -> bool {
        if let Some(channel) = self.get_channel_mut(id) {
            match direction {
                "tx" => {
                    channel.tx_buffer = ChannelBuffer::default();
                    debug!("通道 {} TX 缓冲区已清空", id);
                }
                "rx" => {
                    channel.rx_buffer = ChannelBuffer::default();
                    debug!("通道 {} RX 缓冲区已清空", id);
                }
                "all" => {
                    channel.tx_buffer = ChannelBuffer::default();
                    channel.rx_buffer = ChannelBuffer::default();
                    debug!("通道 {} 所有缓冲区已清空", id);
                }
                _ => {
                    warn!("未知的缓冲区方向: {}", direction);
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    pub fn add_tab(&mut self, channel_id: String, label: String) -> String {
        let key = format!("tab-{}-{}", channel_id, current_timestamp());
        let tab = TabState {
            key: key.clone(),
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

    pub fn get_connected_channels(&self) -> Vec<&DeviceChannel> {
        self.channels.iter().filter(|c| c.connected).collect()
    }

    pub fn get_channels_by_type(&self, channel_type: ChannelType) -> Vec<&DeviceChannel> {
        self.channels
            .iter()
            .filter(|c| c.channel_type == channel_type)
            .collect()
    }
}

pub type AppStateRef = Arc<RwLock<AppState>>;

pub fn create_app_state() -> AppStateRef {
    Arc::new(RwLock::new(AppState::new()))
}
