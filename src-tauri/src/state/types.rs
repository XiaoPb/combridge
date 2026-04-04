use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelDirection {
    Read,
    Write,
    Notify,
}

impl std::fmt::Display for ChannelDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelDirection::Read => write!(f, "read"),
            ChannelDirection::Write => write!(f, "write"),
            ChannelDirection::Notify => write!(f, "notify"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferEntry {
    pub timestamp: u64,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelBuffer {
    pub entries: Vec<BufferEntry>,
    pub total_bytes: usize,
}

impl Default for ChannelBuffer {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            total_bytes: 0,
        }
    }
}

impl ChannelBuffer {
    pub fn add_entry(&mut self, data: &[u8], max_size: usize) {
        let timestamp = current_timestamp();
        self.entries.push(BufferEntry {
            timestamp,
            data: data.to_vec(),
        });
        self.total_bytes += data.len();
        
        while self.total_bytes > max_size {
            if let Some(removed) = self.entries.first() {
                self.total_bytes -= removed.data.len();
                self.entries.remove(0);
            } else {
                break;
            }
        }
    }
    
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_bytes = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: String,
    pub direction: ChannelDirection,
    pub buffer: ChannelBuffer,
    pub subscribed: bool,
}

impl Channel {
    pub fn new(id: String, direction: ChannelDirection) -> Self {
        Self {
            id,
            direction,
            buffer: ChannelBuffer::default(),
            subscribed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DataBits {
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

impl Default for DataBits {
    fn default() -> Self {
        DataBits::Eight
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl Default for Parity {
    fn default() -> Self {
        Parity::None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StopBits {
    One = 1,
    Two = 2,
}

impl Default for StopBits {
    fn default() -> Self {
        StopBits::One
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionParams {
    pub interval: u16,
    pub latency: u16,
    pub timeout: u16,
}

impl Default for ConnectionParams {
    fn default() -> Self {
        Self {
            interval: 30,
            latency: 0,
            timeout: 500,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialDevice {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub connectable: bool,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub parity: Parity,
    pub stop_bits: StopBits,
    pub channels: HashMap<String, Channel>,
}

impl SerialDevice {
    pub fn new(id: String, name: String) -> Self {
        let mut channels = HashMap::new();
        channels.insert("tx".to_string(), Channel::new("tx".to_string(), ChannelDirection::Write));
        channels.insert("rx".to_string(), Channel::new("rx".to_string(), ChannelDirection::Read));
        
        Self {
            id,
            name,
            connected: false,
            connectable: true,
            baud_rate: 115200,
            data_bits: DataBits::Eight,
            parity: Parity::None,
            stop_bits: StopBits::One,
            channels,
        }
    }
    
    pub fn tx_channel(&self) -> Option<&Channel> {
        self.channels.get("tx")
    }
    
    pub fn rx_channel(&self) -> Option<&Channel> {
        self.channels.get("rx")
    }
    
    pub fn tx_channel_mut(&mut self) -> Option<&mut Channel> {
        self.channels.get_mut("tx")
    }
    
    pub fn rx_channel_mut(&mut self) -> Option<&mut Channel> {
        self.channels.get_mut("rx")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BleDevice {
    pub id: String,
    pub name: String,
    pub mac: String,
    pub connected: bool,
    pub connectable: bool,
    pub mtu: u16,
    pub connection_params: ConnectionParams,
    pub channels: HashMap<String, Channel>,
}

impl BleDevice {
    pub fn new(id: String, name: String, mac: String) -> Self {
        Self {
            id,
            name,
            mac,
            connected: false,
            connectable: true,
            mtu: 23,
            connection_params: ConnectionParams::default(),
            channels: HashMap::new(),
        }
    }
    
    pub fn add_channel(&mut self, characteristic_uuid: &str, direction: ChannelDirection) -> String {
        let channel_id = format!("{}_{}", characteristic_uuid, direction);
        if !self.channels.contains_key(&channel_id) {
            self.channels.insert(
                channel_id.clone(),
                Channel::new(channel_id.clone(), direction),
            );
        }
        channel_id
    }
    
    pub fn get_channel(&self, channel_id: &str) -> Option<&Channel> {
        self.channels.get(channel_id)
    }
    
    pub fn get_channel_mut(&mut self, channel_id: &str) -> Option<&mut Channel> {
        self.channels.get_mut(channel_id)
    }
    
    pub fn remove_channel(&mut self, channel_id: &str) -> Option<Channel> {
        self.channels.remove(channel_id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Device {
    Serial(SerialDevice),
    Ble(BleDevice),
}

impl Device {
    pub fn id(&self) -> &str {
        match self {
            Device::Serial(d) => &d.id,
            Device::Ble(d) => &d.id,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            Device::Serial(d) => &d.name,
            Device::Ble(d) => &d.name,
        }
    }
    
    pub fn connected(&self) -> bool {
        match self {
            Device::Serial(d) => d.connected,
            Device::Ble(d) => d.connected,
        }
    }
    
    pub fn connectable(&self) -> bool {
        match self {
            Device::Serial(d) => d.connectable,
            Device::Ble(d) => d.connectable,
        }
    }
    
    pub fn set_connected(&mut self, connected: bool) {
        match self {
            Device::Serial(d) => d.connected = connected,
            Device::Ble(d) => d.connected = connected,
        }
    }
    
    pub fn get_channel(&self, channel_id: &str) -> Option<&Channel> {
        match self {
            Device::Serial(d) => d.channels.get(channel_id),
            Device::Ble(d) => d.channels.get(channel_id),
        }
    }
    
    pub fn get_channel_mut(&mut self, channel_id: &str) -> Option<&mut Channel> {
        match self {
            Device::Serial(d) => d.channels.get_mut(channel_id),
            Device::Ble(d) => d.channels.get_mut(channel_id),
        }
    }
    
    pub fn channel_count(&self) -> usize {
        match self {
            Device::Serial(d) => d.channels.len(),
            Device::Ble(d) => d.channels.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    pub key: String,
    pub device_id: String,
    pub channel_id: Option<String>,
    pub label: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub tabs: Vec<TabState>,
    pub active_tab_key: Option<String>,
    pub sidebar_width: Option<u32>,
    pub panel_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_reconnect: bool,
    pub log_level: String,
    pub max_buffer_size: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "zh-CN".to_string(),
            auto_reconnect: true,
            log_level: "info".to_string(),
            max_buffer_size: 4 * 1024 * 1024,
        }
    }
}

pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPreferences {
    pub display_format: String,
    pub display_mode: String,
    pub send_format: String,
    pub append_newline: bool,
    pub newline_type: String,
    pub auto_scroll: bool,
}

impl Default for SerialPreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl SerialPreferences {
    pub fn default_values() -> Self {
        Self {
            display_format: "text".to_string(),
            display_mode: "all".to_string(),
            send_format: "text".to_string(),
            append_newline: true,
            newline_type: "lf".to_string(),
            auto_scroll: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlePreferences {
    pub display_format: String,
    pub auto_scroll: bool,
    pub input_format: String,
    pub without_response: bool,
    pub config_collapsed: bool,
    pub gatt_collapsed: bool,
    pub panel_collapsed: bool,
    #[serde(default)]
    pub subscribed_characteristics: HashMap<String, Vec<String>>,
}

impl Default for BlePreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl BlePreferences {
    pub fn default_values() -> Self {
        Self {
            display_format: "text".to_string(),
            auto_scroll: true,
            input_format: "text".to_string(),
            without_response: false,
            config_collapsed: false,
            gatt_collapsed: false,
            panel_collapsed: false,
            subscribed_characteristics: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    #[serde(default = "SerialPreferences::default_values")]
    pub serial: SerialPreferences,
    #[serde(default = "BlePreferences::default_values")]
    pub ble: BlePreferences,
}

impl Default for Preferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl Preferences {
    pub fn default_values() -> Self {
        Self {
            serial: SerialPreferences::default_values(),
            ble: BlePreferences::default_values(),
        }
    }
}
