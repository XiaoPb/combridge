use serde::{Deserialize, Serialize};

use crate::device::cache::CacheData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChannelType {
    Serial,
    BluetoothCharacteristic,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelType::Serial => write!(f, "serial"),
            ChannelType::BluetoothCharacteristic => write!(f, "ble"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialConfig {
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: u8,
    pub flow_control: String,
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self {
            baud_rate: 115200,
            data_bits: 8,
            parity: "none".to_string(),
            stop_bits: 1,
            flow_control: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BleCharacteristicConfig {
    pub device_address: String,
    pub service_uuid: String,
    pub characteristic_uuid: String,
    pub properties: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BleDeviceConfig {
    pub address: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChannelConfig {
    Serial(SerialConfig),
    BleCharacteristic(BleCharacteristicConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BufferEntry {
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub direction: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceChannel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: ChannelType,
    pub connected: bool,
    pub tx_buffer: ChannelBuffer,
    pub rx_buffer: ChannelBuffer,
    pub config: Option<ChannelConfig>,
    pub created_at: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

impl DeviceChannel {
    pub fn new_serial(id: String, port_name: String) -> Self {
        Self {
            id,
            name: port_name,
            channel_type: ChannelType::Serial,
            connected: false,
            tx_buffer: ChannelBuffer::default(),
            rx_buffer: ChannelBuffer::default(),
            config: Some(ChannelConfig::Serial(SerialConfig::default())),
            created_at: current_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    pub fn new_ble_characteristic(
        id: String,
        device_address: String,
        device_name: Option<String>,
        service_uuid: String,
        characteristic_uuid: String,
    ) -> Self {
        let name = format!(
            "{} - {}",
            device_name.as_deref().unwrap_or(&device_address),
            characteristic_uuid
        );
        Self {
            id,
            name,
            channel_type: ChannelType::BluetoothCharacteristic,
            connected: false,
            tx_buffer: ChannelBuffer::default(),
            rx_buffer: ChannelBuffer::default(),
            config: Some(ChannelConfig::BleCharacteristic(BleCharacteristicConfig {
                device_address,
                service_uuid,
                characteristic_uuid,
                properties: Vec::new(),
            })),
            created_at: current_timestamp(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    pub key: String,
    pub channel_id: String,
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

impl From<CacheData> for ChannelBuffer {
    fn from(cache: CacheData) -> Self {
        Self {
            entries: cache
                .entries
                .into_iter()
                .map(|e| BufferEntry {
                    timestamp: e.timestamp,
                    data: e.data,
                    direction: "receive".to_string(),
                })
                .collect(),
            total_bytes: cache.total_bytes,
        }
    }
}
