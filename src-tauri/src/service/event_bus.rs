use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::broadcast;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventEncoding {
    Json,
    MsgPack,
}

impl Default for EventEncoding {
    fn default() -> Self {
        Self::Json
    }
}

pub type EventCallback = Box<dyn Fn(&str, &[u8], EventEncoding) + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub topic: String,
    pub payload: Vec<u8>,
    pub timestamp: u64,
    pub encoding: EventEncoding,
}

impl Event {
    pub fn new_json(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            payload: payload.into().into_bytes(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            encoding: EventEncoding::Json,
        }
    }

    pub fn new_msgpack(topic: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            topic: topic.into(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            encoding: EventEncoding::MsgPack,
        }
    }

    pub fn new(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self::new_json(topic, payload)
    }

    pub fn new_bytes(topic: impl Into<String>, payload: Vec<u8>, encoding: EventEncoding) -> Self {
        Self {
            topic: topic.into(),
            payload,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            encoding,
        }
    }

    pub fn payload_as_string(&self) -> Option<String> {
        String::from_utf8(self.payload.clone()).ok()
    }
}

type SubscriberMap = Arc<RwLock<HashMap<String, Vec<EventCallback>>>>;

pub struct EventBus {
    sender: broadcast::Sender<Event>,
    subscribers: SubscriberMap,
    capacity: usize,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            capacity,
        }
    }

    pub async fn publish(&self, topic: impl Into<String>, payload: impl Into<String>) {
        let event = Event::new_json(topic, payload);
        let topic = event.topic.clone();
        let encoding = event.encoding;

        if let Err(e) = self.sender.send(event.clone()) {
            tracing::warn!("Failed to broadcast event: {}", e);
        }

        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        if let Some(callbacks) = subscribers.get(&topic) {
            for callback in callbacks {
                callback(&event.topic, &event.payload, encoding);
            }
        }
    }

    pub fn publish_sync(&self, topic: impl Into<String>, payload: impl Into<String>) {
        let event = Event::new_json(topic, payload);
        let topic = event.topic.clone();
        let encoding = event.encoding;

        if let Err(e) = self.sender.send(event.clone()) {
            tracing::warn!("Failed to broadcast event: {}", e);
        }

        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        if let Some(callbacks) = subscribers.get(&topic) {
            for callback in callbacks {
                callback(&event.topic, &event.payload, encoding);
            }
        }
    }

    pub fn publish_msgpack<T: Serialize>(&self, topic: impl Into<String>, payload: &T) {
        let topic_str = topic.into();
        match rmp_serde::to_vec(payload) {
            Ok(bytes) => {
                let event = Event::new_msgpack(&topic_str, bytes);
                let topic = event.topic.clone();
                let encoding = event.encoding;

                tracing::info!(
                    "[EventBus] Publishing msgpack event: topic={}, payload_len={}, sync_subscribers={}, broadcast_receivers={}",
                    topic,
                    event.payload.len(),
                    self.subscriber_count_sync(&topic),
                    self.sender.receiver_count()
                );

                if let Err(e) = self.sender.send(event.clone()) {
                    tracing::warn!("[EventBus] Failed to broadcast event: {}", e);
                }

                let subscribers = self.subscribers.read().unwrap_or_else(|e| {
                    tracing::error!("[EventBus] Failed to acquire read lock: {}", e);
                    panic!("EventBus lock poisoned");
                });
                if let Some(callbacks) = subscribers.get(&topic) {
                    tracing::info!("[EventBus] Invoking {} callbacks for topic={}", callbacks.len(), topic);
                    for callback in callbacks {
                        callback(&event.topic, &event.payload, encoding);
                    }
                }
            }
            Err(e) => {
                tracing::error!("[EventBus] MsgPack serialization failed: {}", e);
            }
        }
    }

    pub async fn publish_msgpack_async<T: Serialize>(&self, topic: impl Into<String>, payload: &T) {
        match rmp_serde::to_vec(payload) {
            Ok(bytes) => {
                let event = Event::new_msgpack(topic, bytes);
                let topic = event.topic.clone();
                let encoding = event.encoding;

                if let Err(e) = self.sender.send(event.clone()) {
                    tracing::warn!("Failed to broadcast event: {}", e);
                }

                let subscribers = self.subscribers.read().unwrap_or_else(|e| {
                    tracing::error!("Failed to acquire read lock: {}", e);
                    panic!("EventBus lock poisoned");
                });
                if let Some(callbacks) = subscribers.get(&topic) {
                    for callback in callbacks {
                        callback(&event.topic, &event.payload, encoding);
                    }
                }
            }
            Err(e) => {
                tracing::error!("MsgPack serialization failed: {}", e);
            }
        }
    }

    pub fn publish_typed<T: Serialize>(&self, topic: impl Into<String>, payload: &T) {
        match serde_json::to_string(payload) {
            Ok(json) => self.publish_sync(topic, json),
            Err(e) => {
                tracing::error!("Failed to serialize typed event payload: {}", e);
            }
        }
    }

    pub async fn publish_typed_async<T: Serialize>(&self, topic: impl Into<String>, payload: &T) {
        match serde_json::to_string(payload) {
            Ok(json) => self.publish(topic, json).await,
            Err(e) => {
                tracing::error!("Failed to serialize typed event payload: {}", e);
            }
        }
    }

    pub async fn subscribe<F>(&self, topic: &str, callback: F)
    where
        F: Fn(&str, &[u8], EventEncoding) + Send + Sync + 'static,
    {
        let mut subscribers = self.subscribers.write().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire write lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    pub fn subscribe_sync<F>(&self, topic: &str, callback: F)
    where
        F: Fn(&str, &[u8], EventEncoding) + Send + Sync + 'static,
    {
        let mut subscribers = self.subscribers.write().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire write lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    pub fn subscribe_json<T, F>(&self, topic: &str, callback: F)
    where
        T: for<'de> Deserialize<'de>,
        F: Fn(&str, T) + Send + Sync + 'static,
    {
        self.subscribe_sync(topic, move |topic, payload, encoding| {
            if encoding == EventEncoding::Json {
                if let Ok(json_str) = std::str::from_utf8(payload) {
                    match serde_json::from_str::<T>(json_str) {
                        Ok(data) => callback(topic, data),
                        Err(e) => tracing::error!("JSON deserialization failed: {}", e),
                    }
                }
            }
        });
    }

    pub fn subscribe_msgpack<T, F>(&self, topic: &str, callback: F)
    where
        T: for<'de> Deserialize<'de>,
        F: Fn(&str, T) + Send + Sync + 'static,
    {
        self.subscribe_sync(topic, move |topic, payload, encoding| {
            if encoding == EventEncoding::MsgPack {
                match rmp_serde::from_slice::<T>(payload) {
                    Ok(data) => callback(topic, data),
                    Err(e) => tracing::error!("MsgPack deserialization failed: {}", e),
                }
            }
        });
    }

    pub async fn unsubscribe(&self, topic: &str) {
        let mut subscribers = self.subscribers.write().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire write lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.remove(topic);
    }

    pub fn unsubscribe_sync(&self, topic: &str) {
        let mut subscribers = self.subscribers.write().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire write lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.remove(topic);
    }

    pub fn subscribe_channel(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub async fn subscriber_count(&self, topic: &str) -> usize {
        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    pub fn subscriber_count_sync(&self, topic: &str) -> usize {
        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    pub async fn topic_count(&self) -> usize {
        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.len()
    }

    pub fn topic_count_sync(&self) -> usize {
        let subscribers = self.subscribers.read().unwrap_or_else(|e| {
            tracing::error!("Failed to acquire read lock: {}", e);
            panic!("EventBus lock poisoned");
        });
        subscribers.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl Debug for EventBus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventBus")
            .field("capacity", &self.capacity)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialDataEvent {
    pub device_id: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl SerialDataEvent {
    pub fn new(device_id: impl Into<String>, data: Vec<u8>) -> Self {
        Self {
            device_id: device_id.into(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialConnectedEvent {
    pub port_name: String,
    pub timestamp: u64,
}

impl SerialConnectedEvent {
    pub fn new(port_name: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerialDisconnectedEvent {
    pub port_name: String,
    pub timestamp: u64,
}

impl SerialDisconnectedEvent {
    pub fn new(port_name: impl Into<String>) -> Self {
        Self {
            port_name: port_name.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleDataEvent {
    pub device_id: String,
    pub address: String,
    pub characteristic_uuid: String,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

impl BleDataEvent {
    pub fn new(
        device_id: impl Into<String>,
        address: impl Into<String>,
        characteristic_uuid: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            device_id: device_id.into(),
            address: address.into(),
            characteristic_uuid: characteristic_uuid.into(),
            data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gh3036FrameEvent {
    pub function_id: u8,
    pub function_name: String,
    pub frame_id: u32,
    pub timestamp: u64,
    pub channel_count: usize,
    pub channels: Vec<f32>,
}

impl Gh3036FrameEvent {
    pub fn new(
        function_id: u8,
        function_name: impl Into<String>,
        frame_id: u32,
        channel_count: usize,
        channels: Vec<f32>,
    ) -> Self {
        Self {
            function_id,
            function_name: function_name.into(),
            frame_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            channel_count,
            channels,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolParsedEvent {
    pub plugin_id: String,
    pub device_id: String,
    pub original_data: Vec<u8>,
    pub parsed_data: serde_json::Value,
    pub timestamp: u64,
}

impl ProtocolParsedEvent {
    pub fn new(
        plugin_id: impl Into<String>,
        device_id: impl Into<String>,
        original_data: Vec<u8>,
        parsed_data: serde_json::Value,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            device_id: device_id.into(),
            original_data,
            parsed_data,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BleConnectionEvent {
    pub address: String,
    pub name: Option<String>,
    pub timestamp: u64,
}

impl BleConnectionEvent {
    pub fn new(address: impl Into<String>, name: Option<String>) -> Self {
        Self {
            address: address.into(),
            name,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }
}

pub mod topics {
    pub const SERIAL_DATA: &str = "serial:data";
    pub const SERIAL_CONNECTED: &str = "serial:connected";
    pub const SERIAL_DISCONNECTED: &str = "serial:disconnected";
    pub const SERIAL_ERROR: &str = "serial:error";

    pub const BLE_DATA: &str = "ble:data";
    pub const BLE_CONNECTED: &str = "ble:connected";
    pub const BLE_DISCONNECTED: &str = "ble:disconnected";
    pub const BLE_DISCOVERED: &str = "ble:discovered";
    pub const BLE_SCAN_STATUS: &str = "ble:scan:status";
    pub const BLE_ERROR: &str = "ble:error";

    pub const GH3036_FRAME: &str = "gh3036:frame";
    pub const GH3036_EVENT: &str = "gh3036:event";
    pub const GH3036_CHANNEL_CHANGED: &str = "gh3036:channel:changed";

    pub const PROTOCOL_PARSED: &str = "protocol:parsed";
    pub const PROTOCOL_ERROR: &str = "protocol:error";

    pub const WAVEFORM_DATA: &str = "waveform:data";
    pub const WAVEFORM_STATUS: &str = "waveform:status";

    pub const DASHBOARD_PARSER_UPDATED: &str = "dashboard:parser:updated";
    pub const DASHBOARD_JSON_UPDATED: &str = "dashboard:json:updated";

    pub const SYSTEM_STARTED: &str = "system:started";
    pub const SYSTEM_SHUTDOWN: &str = "system:shutdown";
    pub const SYSTEM_CONFIG_CHANGED: &str = "system:config:changed";
    pub const SYSTEM_ERROR: &str = "system:error";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_data_event_serialization() {
        let event = SerialDataEvent::new("serial-1", vec![0x01, 0x02, 0x03]);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("serial-1"));
        assert!(json.contains("device_id"));
    }

    #[test]
    fn test_ble_data_event_serialization() {
        let event = BleDataEvent::new(
            "ble-1",
            "00:11:22:33:44:55",
            "00002a37-0000-1000-8000-00805f9b34fb",
            vec![0x01, 0x02],
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ble-1"));
        assert!(json.contains("00:11:22:33:44:55"));
    }

    #[test]
    fn test_gh3036_frame_event_serialization() {
        let event = Gh3036FrameEvent::new(1, "ECG", 100, 4, vec![1.0, 2.0, 3.0, 4.0]);
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ECG"));
        assert!(json.contains("function_id"));
    }

    #[test]
    fn test_protocol_parsed_event_serialization() {
        let event = ProtocolParsedEvent::new(
            "plugin-1",
            "device-1",
            vec![0x01, 0x02],
            serde_json::json!({"value": 42}),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("plugin-1"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_event_bus_publish_sync() {
        let bus = EventBus::new(16);
        bus.publish_sync("test:topic", "test_payload");
    }

    #[test]
    fn test_event_bus_publish_typed() {
        let bus = EventBus::new(16);
        let event = SerialDataEvent::new("serial-1", vec![0x01, 0x02]);
        bus.publish_typed(topics::SERIAL_DATA, &event);
    }

    #[test]
    fn test_event_bus_subscribe_sync() {
        let bus = EventBus::new(16);
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        
        bus.subscribe_sync("test:topic", move |topic, payload, encoding| {
            assert_eq!(topic, "test:topic");
            assert_eq!(encoding, EventEncoding::Json);
            assert_eq!(std::str::from_utf8(payload).unwrap(), "test_payload");
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        bus.publish_sync("test:topic", "test_payload");
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_event_encoding_enum() {
        assert_ne!(EventEncoding::Json, EventEncoding::MsgPack);
        assert_eq!(EventEncoding::Json, EventEncoding::Json);
    }

    #[test]
    fn test_event_new_json() {
        let event = Event::new_json("test:topic", "test_payload");
        assert_eq!(event.topic, "test:topic");
        assert_eq!(event.payload, b"test_payload");
        assert_eq!(event.encoding, EventEncoding::Json);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_event_new_msgpack() {
        let payload = vec![0x01, 0x02, 0x03];
        let event = Event::new_msgpack("test:topic", payload.clone());
        assert_eq!(event.topic, "test:topic");
        assert_eq!(event.payload, payload);
        assert_eq!(event.encoding, EventEncoding::MsgPack);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_event_payload_as_string() {
        let json_event = Event::new_json("test:topic", "test_payload");
        assert_eq!(json_event.payload_as_string(), Some("test_payload".to_string()));

        let msgpack_event = Event::new_msgpack("test:topic", vec![0xFF, 0xFE]);
        assert_eq!(msgpack_event.payload_as_string(), None);
    }

    #[test]
    fn test_event_bus_publish_msgpack() {
        let bus = EventBus::new(16);
        let data = SerialDataEvent::new("serial-1", vec![0x01, 0x02]);
        bus.publish_msgpack(topics::SERIAL_DATA, &data);
    }

    #[test]
    fn test_event_bus_subscribe_json() {
        let bus = EventBus::new(16);
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        
        bus.subscribe_json::<SerialDataEvent, _>(topics::SERIAL_DATA, move |_topic, event| {
            assert_eq!(event.device_id, "serial-1");
            assert_eq!(event.data, vec![0x01, 0x02]);
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let event = SerialDataEvent::new("serial-1", vec![0x01, 0x02]);
        bus.publish_typed(topics::SERIAL_DATA, &event);
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_event_bus_subscribe_msgpack() {
        let bus = EventBus::new(16);
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let called_clone = called.clone();
        
        bus.subscribe_msgpack::<SerialDataEvent, _>(topics::SERIAL_DATA, move |_topic, event| {
            assert_eq!(event.device_id, "serial-1");
            assert_eq!(event.data, vec![0x01, 0x02]);
            called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        let event = SerialDataEvent::new("serial-1", vec![0x01, 0x02]);
        bus.publish_msgpack(topics::SERIAL_DATA, &event);
        assert!(called.load(std::sync::atomic::Ordering::SeqCst));
    }
}
