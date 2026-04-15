use std::sync::Arc;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::broadcast;

use super::event_bus::{Event, EventBus, EventEncoding};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

#[derive(Debug, Clone)]
pub struct EventFilter {
    prefixes: Vec<String>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self {
            prefixes: Vec::new(),
        }
    }

    pub fn with_prefixes(prefixes: Vec<String>) -> Self {
        Self { prefixes }
    }

    pub fn add_prefix(&mut self, prefix: impl Into<String>) {
        self.prefixes.push(prefix.into());
    }

    pub fn matches(&self, topic: &str) -> bool {
        if self.prefixes.is_empty() {
            return true;
        }
        self.prefixes.iter().any(|prefix| topic.starts_with(prefix))
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct EventBridge<R: Runtime> {
    event_bus: Arc<EventBus>,
    app_handle: AppHandle<R>,
    filter: EventFilter,
    shutdown_tx: Option<broadcast::Sender<()>>,
}

impl<R: Runtime> EventBridge<R> {
    pub fn new(event_bus: Arc<EventBus>, app_handle: AppHandle<R>) -> Self {
        Self {
            event_bus,
            app_handle,
            filter: EventFilter::new(),
            shutdown_tx: None,
        }
    }

    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn start(&mut self) {
        let mut receiver = self.event_bus.subscribe_channel();
        let app_handle = self.app_handle.clone();
        let filter = self.filter.clone();

        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        tauri::async_runtime::spawn(async move {
            tracing::info!("EventBridge started, listening for events");

            loop {
                tokio::select! {
                    result = receiver.recv() => {
                        match result {
                            Ok(event) => {
                                if !filter.matches(&event.topic) {
                                    continue;
                                }

                                tracing::info!(
                                    "[EventBridge] Received event: topic={}, encoding={:?}, payload_len={}",
                                    event.topic,
                                    event.encoding,
                                    event.payload.len()
                                );
                                
                                if !filter.matches(&event.topic) {
                                    tracing::debug!("[EventBridge] Event filtered out: topic={}", event.topic);
                                    continue;
                                }

                                if let Err(e) = Self::emit_to_frontend(&app_handle, &event) {
                                    tracing::error!(
                                        "[EventBridge] Failed to emit event to frontend: topic={}, error={}",
                                        event.topic,
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "[EventBridge] Event forwarded to frontend: topic={}, timestamp={}",
                                        event.topic,
                                        event.timestamp
                                    );
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!("EventBus channel closed, stopping EventBridge");
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(
                                    "EventBridge lagged behind by {} messages, continuing",
                                    n
                                );
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("EventBridge received shutdown signal");
                        break;
                    }
                }
            }

            tracing::info!("EventBridge stopped");
        });
    }

    pub fn stop(&mut self) {
        if let Some(tx) = &self.shutdown_tx {
            if let Err(e) = tx.send(()) {
                tracing::warn!("Failed to send shutdown signal to EventBridge: {}", e);
            }
            self.shutdown_tx = None;
        }
    }

    fn emit_to_frontend(app_handle: &AppHandle<R>, event: &Event) -> Result<(), String> {
        let (payload_str, encoding_str) = match event.encoding {
            EventEncoding::Json => {
                let s = String::from_utf8(event.payload.clone())
                    .unwrap_or_else(|_| BASE64.encode(&event.payload));
                (s, "json")
            }
            EventEncoding::MsgPack => {
                let b64 = BASE64.encode(&event.payload);
                (b64, "msgpack+base64")
            }
        };

        let wrapper = serde_json::json!({
            "topic": &event.topic,
            "payload": payload_str,
            "timestamp": event.timestamp,
            "encoding": encoding_str,
        });

        app_handle
            .emit("event-bus", wrapper)
            .map_err(|e| format!("Failed to emit event: {}", e))
    }
}

impl<R: Runtime> Drop for EventBridge<R> {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_empty() {
        let filter = EventFilter::new();
        assert!(filter.matches("serial:data"));
        assert!(filter.matches("ble:data"));
    }

    #[test]
    fn test_event_filter_with_prefixes() {
        let filter = EventFilter::with_prefixes(vec!["serial:".to_string(), "ble:".to_string()]);
        assert!(filter.matches("serial:data"));
        assert!(filter.matches("ble:data"));
        assert!(!filter.matches("other:data"));
    }

    #[test]
    fn test_event_filter_add_prefix() {
        let mut filter = EventFilter::new();
        filter.add_prefix("serial:");
        assert!(filter.matches("serial:data"));
        assert!(!filter.matches("ble:data"));
    }
}
