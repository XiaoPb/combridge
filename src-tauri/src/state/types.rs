use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, VecDeque};

pub use crate::device::serial::serial_config::{DataBits, Parity, StopBits};

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
    pub entries: VecDeque<BufferEntry>,
    pub total_bytes: usize,
}

impl Default for ChannelBuffer {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            total_bytes: 0,
        }
    }
}

impl ChannelBuffer {
    pub fn add_entry(&mut self, data: &[u8], max_size: usize) {
        let timestamp = current_timestamp();
        self.entries.push_back(BufferEntry {
            timestamp,
            data: data.to_vec(),
        });
        self.total_bytes += data.len();

        while self.total_bytes > max_size {
            if let Some(removed) = self.entries.pop_front() {
                self.total_bytes -= removed.data.len();
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
        channels.insert(
            "tx".to_string(),
            Channel::new("tx".to_string(), ChannelDirection::Write),
        );
        channels.insert(
            "rx".to_string(),
            Channel::new("rx".to_string(), ChannelDirection::Read),
        );

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
pub struct BleDeviceState {
    pub id: String,
    pub name: String,
    pub mac: String,
    pub connected: bool,
    pub connectable: bool,
    pub mtu: u16,
    pub connection_params: ConnectionParams,
    pub channels: HashMap<String, Channel>,
}

impl BleDeviceState {
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

    pub fn add_channel(
        &mut self,
        characteristic_uuid: &str,
        direction: ChannelDirection,
    ) -> String {
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
    Ble(BleDeviceState),
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
    pub timezone: String,
    pub auto_reconnect: bool,
    pub log_level: String,
    pub max_buffer_size: usize,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            language: "zh-CN".to_string(),
            timezone: "Asia/Shanghai".to_string(),
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformPreferences {
    pub display_rows: u32,
    pub refresh_interval: u32,
    pub sidebar_collapsed: bool,
}

impl Default for WaveformPreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl WaveformPreferences {
    pub fn default_values() -> Self {
        Self {
            display_rows: 20,
            refresh_interval: 100,
            sidebar_collapsed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3036ChannelPreferences {
    pub connection_type: String,
    pub serial_port: String,
    pub ble_device: String,
    pub tx_char: String,
    pub rx_char: String,
}

impl Default for Gh3036ChannelPreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl Gh3036ChannelPreferences {
    pub fn default_values() -> Self {
        Self {
            connection_type: "serial".to_string(),
            serial_port: String::new(),
            ble_device: String::new(),
            tx_char: "00000004-0000-1000-8000-00805f9b34fb".to_string(),
            rx_char: "00000003-0000-1000-8000-00805f9b34fb".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gh3036CsvPreferences {
    pub enabled: bool,
    pub output_dir: String,
}

impl Default for Gh3036CsvPreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl Gh3036CsvPreferences {
    pub fn default_values() -> Self {
        let output_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .map(|exe_dir| exe_dir.join("data"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("data"));

        Self {
            enabled: true,
            output_dir,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuGroupVisibility {
    pub visible: bool,
    pub tabs: BTreeMap<String, bool>,
}

impl MenuGroupVisibility {
    pub fn new(visible: bool, tabs: &[(&str, bool)]) -> Self {
        Self {
            visible,
            tabs: tabs
                .iter()
                .map(|(key, visible)| ((*key).to_string(), *visible))
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HomeMenuVisibility {
    pub connection: MenuGroupVisibility,
    pub dashboard: MenuGroupVisibility,
    pub gh3036: MenuGroupVisibility,
    pub protocol: MenuGroupVisibility,
    pub waveform: MenuGroupVisibility,
    pub system: MenuGroupVisibility,
}

impl Default for HomeMenuVisibility {
    fn default() -> Self {
        Self::default_values()
    }
}

impl HomeMenuVisibility {
    pub fn default_values() -> Self {
        Self {
            connection: MenuGroupVisibility::new(true, &[("serial", true), ("ble", true)]),
            dashboard: MenuGroupVisibility::new(
                false,
                &[
                    ("dashboard", false),
                    ("console", false),
                    ("settings", false),
                    ("jsonEditor", false),
                ],
            ),
            gh3036: MenuGroupVisibility::new(
                true,
                &[
                    ("config", true),
                    ("monitor", true),
                    ("version", true),
                    ("factory", true),
                ],
            ),
            protocol: MenuGroupVisibility::new(false, &[("editor", false), ("bind", false)]),
            waveform: MenuGroupVisibility::new(true, &[("realtime", false), ("csvLoader", true)]),
            system: MenuGroupVisibility::new(
                true,
                &[("info", false), ("logs", false), ("settings", true)],
            ),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidebarMenuVisibility {
    pub home: bool,
    pub serial: bool,
    pub ble: bool,
    pub dashboard: bool,
    pub gh3036: bool,
    pub protocol: bool,
    pub waveform: bool,
    pub system: bool,
}

impl Default for SidebarMenuVisibility {
    fn default() -> Self {
        Self::default_values()
    }
}

impl SidebarMenuVisibility {
    pub fn default_values() -> Self {
        Self {
            home: true,
            serial: true,
            ble: true,
            dashboard: false,
            gh3036: true,
            protocol: false,
            waveform: true,
            system: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuVisibilityPreferences {
    pub home: HomeMenuVisibility,
    pub sidebar: SidebarMenuVisibility,
}

impl Default for MenuVisibilityPreferences {
    fn default() -> Self {
        Self::default_values()
    }
}

impl MenuVisibilityPreferences {
    pub fn default_values() -> Self {
        Self {
            home: HomeMenuVisibility::default_values(),
            sidebar: SidebarMenuVisibility::default_values(),
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
    #[serde(default = "WaveformPreferences::default_values")]
    pub waveform: WaveformPreferences,
    #[serde(default = "Gh3036ChannelPreferences::default_values")]
    pub gh3036_channel: Gh3036ChannelPreferences,
    #[serde(default = "Gh3036CsvPreferences::default_values")]
    pub gh3036_csv: Gh3036CsvPreferences,
    #[serde(default = "MenuVisibilityPreferences::default_values")]
    pub menu_visibility: MenuVisibilityPreferences,
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
            waveform: WaveformPreferences::default_values(),
            gh3036_channel: Gh3036ChannelPreferences::default_values(),
            gh3036_csv: Gh3036CsvPreferences::default_values(),
            menu_visibility: MenuVisibilityPreferences::default_values(),
        }
    }
}
