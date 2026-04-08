use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::error::{ComBridgeError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MsgPackMessage {
    pub id: String,
    pub timestamp: u64,
    pub msg_type: String,
    pub payload: Vec<u8>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedMessage {
    pub id: String,
    pub timestamp: u64,
    pub msg_type: MessageType,
    pub data: MessageData,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageType {
    Data,
    Command,
    Response,
    Event,
    Error,
    Heartbeat,
    Custom(String),
}

impl From<&str> for MessageType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "data" => MessageType::Data,
            "command" => MessageType::Command,
            "response" => MessageType::Response,
            "event" => MessageType::Event,
            "error" => MessageType::Error,
            "heartbeat" => MessageType::Heartbeat,
            _ => MessageType::Custom(s.to_string()),
        }
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageType::Data => write!(f, "data"),
            MessageType::Command => write!(f, "command"),
            MessageType::Response => write!(f, "response"),
            MessageType::Event => write!(f, "event"),
            MessageType::Error => write!(f, "error"),
            MessageType::Heartbeat => write!(f, "heartbeat"),
            MessageType::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageData {
    Raw(Vec<u8>),
    Text(String),
    Json(serde_json::Value),
    Binary(Vec<u8>),
}

impl MessageData {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            MessageData::Raw(b) => b.clone(),
            MessageData::Text(s) => s.as_bytes().to_vec(),
            MessageData::Json(v) => v.to_string().as_bytes().to_vec(),
            MessageData::Binary(b) => b.clone(),
        }
    }

    pub fn as_text(&self) -> Option<String> {
        match self {
            MessageData::Text(s) => Some(s.clone()),
            MessageData::Json(v) => Some(v.to_string()),
            MessageData::Raw(b) | MessageData::Binary(b) => {
                String::from_utf8(b.clone()).ok()
            }
        }
    }

    pub fn as_json(&self) -> Option<serde_json::Value> {
        match self {
            MessageData::Json(v) => Some(v.clone()),
            MessageData::Text(s) => serde_json::from_str(s).ok(),
            MessageData::Raw(b) | MessageData::Binary(b) => {
                String::from_utf8(b.clone())
                    .ok()
                    .and_then(|s| serde_json::from_str(&s).ok())
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            MessageData::Raw(b) => b.len(),
            MessageData::Text(s) => s.len(),
            MessageData::Json(v) => v.to_string().len(),
            MessageData::Binary(b) => b.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

const MAGIC_BYTE: u8 = 0xCB;
const VERSION: u8 = 0x01;
const HEADER_SIZE: usize = 8;

pub struct MsgPackHandler {
    buffer: Vec<u8>,
    max_message_size: usize,
}

impl MsgPackHandler {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            max_message_size: 1024 * 1024,
        }
    }

    pub fn with_max_size(max_message_size: usize) -> Self {
        Self {
            buffer: Vec::new(),
            max_message_size,
        }
    }

    pub fn pack(&self, message: &MsgPackMessage) -> Result<Vec<u8>> {
        let mut result = Vec::new();

        result.push(MAGIC_BYTE);
        result.push(VERSION);

        let id_bytes = message.id.as_bytes();
        result.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
        result.extend_from_slice(id_bytes);

        result.extend_from_slice(&message.timestamp.to_be_bytes());

        let type_bytes = message.msg_type.as_bytes();
        result.extend_from_slice(&(type_bytes.len() as u16).to_be_bytes());
        result.extend_from_slice(type_bytes);

        let meta_json = serde_json::to_string(&message.metadata)?;
        let meta_bytes = meta_json.as_bytes();
        result.extend_from_slice(&(meta_bytes.len() as u16).to_be_bytes());
        result.extend_from_slice(meta_bytes);

        result.extend_from_slice(&(message.payload.len() as u32).to_be_bytes());
        result.extend_from_slice(&message.payload);

        let checksum = self.calculate_checksum(&result);
        result.extend_from_slice(&checksum.to_be_bytes());

        Ok(result)
    }

    pub fn unpack(&mut self, data: &[u8]) -> Result<Vec<MsgPackMessage>> {
        self.buffer.extend_from_slice(data);

        let mut messages = Vec::new();
        let mut offset = 0;

        while offset < self.buffer.len() {
            if self.buffer.len() - offset < HEADER_SIZE {
                break;
            }

            if self.buffer[offset] != MAGIC_BYTE {
                warn!("无效的魔数: 0x{:02X}, 期望: 0x{:02X}", self.buffer[offset], MAGIC_BYTE);
                offset += 1;
                continue;
            }

            if self.buffer[offset + 1] != VERSION {
                warn!("不支持的版本: 0x{:02X}", self.buffer[offset + 1]);
                offset += 2;
                continue;
            }

            let mut cursor = offset + 2;

            if self.buffer.len() < cursor + 2 {
                break;
            }
            let id_len = u16::from_be_bytes([self.buffer[cursor], self.buffer[cursor + 1]]) as usize;
            cursor += 2;

            if self.buffer.len() < cursor + id_len {
                break;
            }
            let id = String::from_utf8_lossy(&self.buffer[cursor..cursor + id_len]).to_string();
            cursor += id_len;

            if self.buffer.len() < cursor + 8 {
                break;
            }
            let timestamp = u64::from_be_bytes([
                self.buffer[cursor],
                self.buffer[cursor + 1],
                self.buffer[cursor + 2],
                self.buffer[cursor + 3],
                self.buffer[cursor + 4],
                self.buffer[cursor + 5],
                self.buffer[cursor + 6],
                self.buffer[cursor + 7],
            ]);
            cursor += 8;

            if self.buffer.len() < cursor + 2 {
                break;
            }
            let type_len = u16::from_be_bytes([self.buffer[cursor], self.buffer[cursor + 1]]) as usize;
            cursor += 2;

            if self.buffer.len() < cursor + type_len {
                break;
            }
            let msg_type = String::from_utf8_lossy(&self.buffer[cursor..cursor + type_len]).to_string();
            cursor += type_len;

            if self.buffer.len() < cursor + 2 {
                break;
            }
            let meta_len = u16::from_be_bytes([self.buffer[cursor], self.buffer[cursor + 1]]) as usize;
            cursor += 2;

            if self.buffer.len() < cursor + meta_len {
                break;
            }
            let meta_str = String::from_utf8_lossy(&self.buffer[cursor..cursor + meta_len]);
            let metadata: HashMap<String, String> = serde_json::from_str(&meta_str)?;
            cursor += meta_len;

            if self.buffer.len() < cursor + 4 {
                break;
            }
            let payload_len = u32::from_be_bytes([
                self.buffer[cursor],
                self.buffer[cursor + 1],
                self.buffer[cursor + 2],
                self.buffer[cursor + 3],
            ]) as usize;
            cursor += 4;

            if payload_len > self.max_message_size {
                return Err(ComBridgeError::parse(format!(
                    "消息大小 {} 超过最大限制 {}",
                    payload_len, self.max_message_size
                )));
            }

            if self.buffer.len() < cursor + payload_len + 2 {
                break;
            }
            let payload = self.buffer[cursor..cursor + payload_len].to_vec();
            cursor += payload_len;

            let checksum = u16::from_be_bytes([self.buffer[cursor], self.buffer[cursor + 1]]);
            cursor += 2;

            let expected_checksum = self.calculate_checksum(&self.buffer[offset..cursor - 2]);
            if checksum != expected_checksum {
                warn!("校验和不匹配: 收到 {}, 期望 {}", checksum, expected_checksum);
                offset = cursor;
                continue;
            }

            messages.push(MsgPackMessage {
                id,
                timestamp,
                msg_type,
                payload,
                metadata,
            });

            offset = cursor;
        }

        if offset > 0 {
            self.buffer.drain(..offset);
        }

        debug!("解析完成，共 {} 条消息，剩余 {} 字节", messages.len(), self.buffer.len());
        Ok(messages)
    }

    fn calculate_checksum(&self, data: &[u8]) -> u16 {
        let mut sum: u16 = 0;
        for &byte in data {
            sum = sum.wrapping_add(byte as u16);
        }
        sum
    }

    pub fn pack_simple(&self, msg_type: &str, payload: &[u8]) -> Result<Vec<u8>> {
        let message = MsgPackMessage {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            msg_type: msg_type.to_string(),
            payload: payload.to_vec(),
            metadata: HashMap::new(),
        };
        self.pack(&message)
    }

    pub fn pack_json<T: Serialize>(&self, msg_type: &str, data: &T) -> Result<Vec<u8>> {
        let payload = serde_json::to_vec(data)?;
        self.pack_simple(msg_type, &payload)
    }

    pub fn parse_json<T: for<'de> Deserialize<'de>>(&self, message: &MsgPackMessage) -> Result<T> {
        serde_json::from_slice(&message.payload)
            .map_err(|e| ComBridgeError::parse(format!("JSON解析失败: {}", e)))
    }

    pub fn clear_buffer(&mut self) {
        self.buffer.clear();
    }

    pub fn buffer_len(&self) -> usize {
        self.buffer.len()
    }
}

impl Default for MsgPackHandler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn create_data_message(payload: Vec<u8>) -> MsgPackMessage {
    MsgPackMessage {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        msg_type: "data".to_string(),
        payload,
        metadata: HashMap::new(),
    }
}

pub fn create_command_message(command: &str, params: serde_json::Value) -> MsgPackMessage {
    MsgPackMessage {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        msg_type: "command".to_string(),
        payload: serde_json::to_vec(&serde_json::json!({
            "command": command,
            "params": params
        }))
        .unwrap_or_default(),
        metadata: HashMap::new(),
    }
}

pub fn create_response_message(request_id: &str, success: bool, data: serde_json::Value) -> MsgPackMessage {
    let mut metadata = HashMap::new();
    metadata.insert("request_id".to_string(), request_id.to_string());

    MsgPackMessage {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        msg_type: "response".to_string(),
        payload: serde_json::to_vec(&serde_json::json!({
            "success": success,
            "data": data
        }))
        .unwrap_or_default(),
        metadata,
    }
}

pub fn create_heartbeat_message() -> MsgPackMessage {
    MsgPackMessage {
        id: uuid::Uuid::new_v4().to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        msg_type: "heartbeat".to_string(),
        payload: vec![],
        metadata: HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_unpack() {
        let handler = MsgPackHandler::new();
        let original = create_data_message(vec![1, 2, 3, 4, 5]);

        let packed = handler.pack(&original).unwrap();
        assert!(!packed.is_empty());

        let mut handler2 = MsgPackHandler::new();
        let messages = handler2.unpack(&packed).unwrap();
        assert_eq!(messages.len(), 1);

        let unpacked = &messages[0];
        assert_eq!(unpacked.id, original.id);
        assert_eq!(unpacked.msg_type, original.msg_type);
        assert_eq!(unpacked.payload, original.payload);
    }

    #[test]
    fn test_pack_json() {
        let handler = MsgPackHandler::new();
        let data = serde_json::json!({
            "key": "value",
            "number": 42
        });

        let packed = handler.pack_json("test", &data).unwrap();
        assert!(!packed.is_empty());
    }

    #[test]
    fn test_message_type() {
        assert_eq!(MessageType::from("data"), MessageType::Data);
        assert_eq!(MessageType::from("command"), MessageType::Command);
        assert_eq!(MessageType::from("custom_type"), MessageType::Custom("custom_type".to_string()));
    }

    #[test]
    fn test_message_data() {
        let raw = MessageData::Raw(vec![1, 2, 3]);
        assert_eq!(raw.as_bytes(), vec![1, 2, 3]);
        assert_eq!(raw.len(), 3);

        let text = MessageData::Text("hello".to_string());
        assert_eq!(text.as_text(), Some("hello".to_string()));

        let json = MessageData::Json(serde_json::json!({"key": "value"}));
        assert!(json.as_json().is_some());
    }
}
