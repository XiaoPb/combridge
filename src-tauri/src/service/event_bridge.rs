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

        let (shutdown_tx, shutdown_rx) = broadcast::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        tauri::async_runtime::spawn(async move {
            tracing::info!("[EventBridge] Started, listening for events (filter prefixes: {:?})", filter.prefixes);
            let mut event_count = 0u64;
            let mut forwarded_count = 0u64;
            let mut filtered_count = 0u64;
            let error_count = 0u64;
            let mut shutdown_rx = shutdown_rx;

            loop {
                tokio::select! {
                    result = shutdown_rx.recv() => {
                        match result {
                            Ok(_) => {
                                tracing::info!(
                                    "[EventBridge] Shutdown signal received (total: {} received, {} forwarded, {} filtered, {} errors)",
                                    event_count, forwarded_count, filtered_count, error_count
                                );
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!(
                                    "[EventBridge] Shutdown channel closed (total: {} received, {} forwarded, {} filtered, {} errors)",
                                    event_count, forwarded_count, filtered_count, error_count
                                );
                            }
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                continue;
                            }
                        }
                        break;
                    }
                    result = receiver.recv() => {
                        match result {
                            Ok(event) => {
                                event_count += 1;
                                let matches = filter.matches(&event.topic);

                                if !matches {
                                    filtered_count += 1;
                                    tracing::debug!(
                                        "[EventBridge] Event #{} filtered out: topic={}",
                                        event_count,
                                        event.topic
                                    );
                                    continue;
                                }

                                tracing::info!(
                                    "[EventBridge] Received event #{}: topic={}, encoding={:?}, payload_len={}",
                                    event_count,
                                    event.topic,
                                    event.encoding,
                                    event.payload.len()
                                );

                                let app_handle = app_handle.clone();
                                let event_clone = event.clone();
                                let topic_for_log = event.topic.clone();

                                tokio::task::spawn_blocking(move || {
                                    match Self::emit_to_frontend(&app_handle, &event_clone) {
                                        Ok(()) => {
                                            tracing::info!(
                                                "[EventBridge] Forwarded to frontend: topic={}, timestamp={}",
                                                topic_for_log,
                                                event_clone.timestamp
                                            );
                                        }
                                        Err(e) => {
                                            tracing::error!(
                                                "[EventBridge] Failed to emit to frontend: topic={}, error={}",
                                                topic_for_log,
                                                e
                                            );
                                        }
                                    }
                                });

                                forwarded_count += 1;

                                if forwarded_count % 100 == 0 {
                                    tracing::info!(
                                        "[EventBridge] Stats: {} received, {} forwarded, {} filtered, {} errors",
                                        event_count, forwarded_count, filtered_count, error_count
                                    );
                                }
                            }
                            Err(broadcast::error::RecvError::Closed) => {
                                tracing::info!(
                                    "[EventBridge] EventBus channel closed, stopping (total: {} received, {} forwarded)",
                                    event_count, forwarded_count
                                );
                                break;
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                tracing::warn!(
                                    "[EventBridge] Lagged behind by {} messages (processed {} events), continuing",
                                    n,
                                    event_count
                                );
                            }
                        }
                    }
                }
            }

            tracing::info!("[EventBridge] Stopped (total: {} received, {} forwarded)", event_count, forwarded_count);
        });
    }

    pub fn stop(&mut self) {
        if let Some(tx) = &self.shutdown_tx {
            if let Err(e) = tx.send(()) {
                tracing::warn!("[EventBridge] Failed to send shutdown signal: {}", e);
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

    #[test]
    fn test_event_filter_gh3036() {
        let filter = EventFilter::with_prefixes(vec![
            "serial:".to_string(),
            "ble:".to_string(),
            "gh3036:".to_string(),
            "protocol:".to_string(),
        ]);
        assert!(filter.matches("gh3036:factory_test_progress"));
        assert!(filter.matches("gh3036:frame"));
        assert!(filter.matches("serial:data"));
        assert!(!filter.matches("system:error"));
    }
}
