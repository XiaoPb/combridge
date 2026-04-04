use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Action {
    #[serde(rename_all = "camelCase")]
    ChannelAdd {
        name: String,
        channel_type: String,
        config: Option<serde_json::Value>,
    },
    #[serde(rename_all = "camelCase")]
    ChannelRemove { id: String },
    #[serde(rename_all = "camelCase")]
    ChannelConnect {
        id: String,
        config: Option<serde_json::Value>,
    },
    #[serde(rename_all = "camelCase")]
    ChannelDisconnect { id: String },
    #[serde(rename_all = "camelCase")]
    DataSend {
        channel_id: String,
        data: Vec<u8>,
    },
    #[serde(rename_all = "camelCase")]
    ChannelSwitch { channel_id: String },
    #[serde(rename_all = "camelCase")]
    BufferClear {
        channel_id: String,
        direction: String,
    },
    #[serde(rename_all = "camelCase")]
    TabAdd {
        channel_id: String,
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
            Action::ChannelAdd { name, .. } => write!(f, "CHANNEL_ADD({})", name),
            Action::ChannelRemove { id } => write!(f, "CHANNEL_REMOVE({})", id),
            Action::ChannelConnect { id, .. } => write!(f, "CHANNEL_CONNECT({})", id),
            Action::ChannelDisconnect { id } => write!(f, "CHANNEL_DISCONNECT({})", id),
            Action::DataSend { channel_id, .. } => write!(f, "DATA_SEND({})", channel_id),
            Action::ChannelSwitch { channel_id } => write!(f, "CHANNEL_SWITCH({})", channel_id),
            Action::BufferClear { channel_id, .. } => write!(f, "BUFFER_CLEAR({})", channel_id),
            Action::TabAdd { label, .. } => write!(f, "TAB_ADD({})", label),
            Action::TabRemove { tab_key } => write!(f, "TAB_REMOVE({})", tab_key),
            Action::TabSwitch { tab_key } => write!(f, "TAB_SWITCH({})", tab_key),
            Action::SettingsUpdate { .. } => write!(f, "SETTINGS_UPDATE"),
            Action::StateRestore { .. } => write!(f, "STATE_RESTORE"),
        }
    }
}
