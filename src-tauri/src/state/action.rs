use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    DeviceAddSerial {
        id: String,
        name: String,
        baud_rate: u32,
    },

    #[serde(rename_all = "camelCase")]
    DeviceAddBle {
        id: String,
        name: String,
        mac: String,
    },

    #[serde(rename_all = "camelCase")]
    DeviceRemove { device_id: String },

    #[serde(rename_all = "camelCase")]
    DeviceConnect { device_id: String },

    #[serde(rename_all = "camelCase")]
    DeviceDisconnect { device_id: String },

    #[serde(rename_all = "camelCase")]
    DeviceUpdateConfig {
        device_id: String,
        config: serde_json::Value,
    },

    #[serde(rename_all = "camelCase")]
    ChannelAdd {
        device_id: String,
        channel_id: String,
        direction: String,
    },

    #[serde(rename_all = "camelCase")]
    ChannelSubscribe {
        device_id: String,
        channel_id: String,
        subscribe: bool,
    },

    #[serde(rename_all = "camelCase")]
    DataSend {
        device_id: String,
        channel_id: String,
        data: Vec<u8>,
    },

    #[serde(rename_all = "camelCase")]
    DataReceive {
        device_id: String,
        channel_id: String,
        data: Vec<u8>,
    },

    #[serde(rename_all = "camelCase")]
    BufferClear {
        device_id: String,
        channel_id: String,
    },

    #[serde(rename_all = "camelCase")]
    DeviceSwitch { device_id: String },

    #[serde(rename_all = "camelCase")]
    TabAdd {
        device_id: String,
        channel_id: Option<String>,
        label: String,
    },

    #[serde(rename_all = "camelCase")]
    TabRemove { tab_key: String },

    #[serde(rename_all = "camelCase")]
    TabSwitch { tab_key: String },

    #[serde(rename_all = "camelCase")]
    SettingsUpdate { settings: serde_json::Value },

    #[serde(rename_all = "camelCase")]
    StateRestore { window_state: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl ActionResult {
    pub fn success() -> Self {
        Self {
            success: true,
            message: None,
            data: None,
        }
    }

    pub fn success_with_data(data: serde_json::Value) -> Self {
        Self {
            success: true,
            message: None,
            data: Some(data),
        }
    }

    pub fn success_with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
            data: None,
        }
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: None,
        }
    }

    pub fn failure_with_data(message: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
            data: Some(data),
        }
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Action::DeviceAddSerial { name, .. } => write!(f, "DEVICE_ADD_SERIAL({})", name),
            Action::DeviceAddBle { name, .. } => write!(f, "DEVICE_ADD_BLE({})", name),
            Action::DeviceRemove { device_id } => write!(f, "DEVICE_REMOVE({})", device_id),
            Action::DeviceConnect { device_id } => write!(f, "DEVICE_CONNECT({})", device_id),
            Action::DeviceDisconnect { device_id } => write!(f, "DEVICE_DISCONNECT({})", device_id),
            Action::DeviceUpdateConfig { device_id, .. } => {
                write!(f, "DEVICE_UPDATE_CONFIG({})", device_id)
            }
            Action::ChannelAdd {
                device_id,
                channel_id,
                ..
            } => write!(f, "CHANNEL_ADD({}/{})", device_id, channel_id),
            Action::ChannelSubscribe {
                device_id,
                channel_id,
                ..
            } => write!(f, "CHANNEL_SUBSCRIBE({}/{})", device_id, channel_id),
            Action::DataSend {
                device_id,
                channel_id,
                ..
            } => write!(f, "DATA_SEND({}/{})", device_id, channel_id),
            Action::DataReceive {
                device_id,
                channel_id,
                ..
            } => write!(f, "DATA_RECEIVE({}/{})", device_id, channel_id),
            Action::BufferClear {
                device_id,
                channel_id,
            } => write!(f, "BUFFER_CLEAR({}/{})", device_id, channel_id),
            Action::DeviceSwitch { device_id } => write!(f, "DEVICE_SWITCH({})", device_id),
            Action::TabAdd { label, .. } => write!(f, "TAB_ADD({})", label),
            Action::TabRemove { tab_key } => write!(f, "TAB_REMOVE({})", tab_key),
            Action::TabSwitch { tab_key } => write!(f, "TAB_SWITCH({})", tab_key),
            Action::SettingsUpdate { .. } => write!(f, "SETTINGS_UPDATE"),
            Action::StateRestore { .. } => write!(f, "STATE_RESTORE"),
        }
    }
}
