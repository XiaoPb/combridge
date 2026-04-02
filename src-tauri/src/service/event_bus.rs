use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

pub type EventCallback = Box<dyn Fn(&str, &str) + Send + Sync>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub topic: String,
    pub payload: String,
    pub timestamp: u64,
}

impl Event {
    pub fn new(topic: impl Into<String>, payload: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            payload: payload.into(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
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
        let event = Event::new(topic, payload);
        let topic = event.topic.clone();

        if let Err(e) = self.sender.send(event.clone()) {
            tracing::warn!("Failed to broadcast event: {}", e);
        }

        let subscribers = self.subscribers.read().await;
        if let Some(callbacks) = subscribers.get(&topic) {
            for callback in callbacks {
                callback(&event.topic, &event.payload);
            }
        }
    }

    pub async fn subscribe<F>(&self, topic: &str, callback: F)
    where
        F: Fn(&str, &str) + Send + Sync + 'static,
    {
        let mut subscribers = self.subscribers.write().await;
        subscribers
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(Box::new(callback));
    }

    pub async fn unsubscribe(&self, topic: &str) {
        let mut subscribers = self.subscribers.write().await;
        subscribers.remove(topic);
    }

    pub fn subscribe_channel(&self) -> broadcast::Receiver<Event> {
        self.sender.subscribe()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub async fn subscriber_count(&self, topic: &str) -> usize {
        let subscribers = self.subscribers.read().await;
        subscribers.get(topic).map(|v| v.len()).unwrap_or(0)
    }

    pub async fn topic_count(&self) -> usize {
        let subscribers = self.subscribers.read().await;
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
