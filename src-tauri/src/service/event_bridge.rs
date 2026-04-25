use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::broadcast;

use super::event_bus::{Event, EventBus, EventEncoding};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const HEARTBEAT_INTERVAL_SECS: u64 = 10;
const EMIT_CHANNEL_CAPACITY: usize = 256;

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
    running: Arc<AtomicBool>,
}

impl<R: Runtime> EventBridge<R> {
    pub fn new(event_bus: Arc<EventBus>, app_handle: AppHandle<R>) -> Self {
        Self {
            event_bus,
            app_handle,
            filter: EventFilter::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn with_filter(mut self, filter: EventFilter) -> Self {
        self.filter = filter;
        self
    }

    pub fn start(&mut self) {
        let receiver = self.event_bus.subscribe_channel();
        let app_handle = self.app_handle.clone();
        let filter = self.filter.clone();
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let running_clone = running.clone();
        tauri::async_runtime::spawn(async move {
            Self::event_loop(receiver, app_handle, filter, running_clone).await;
        });
    }

    async fn event_loop(
        mut receiver: broadcast::Receiver<Event>,
        app_handle: AppHandle<R>,
        filter: EventFilter,
        running: Arc<AtomicBool>,
    ) {
        tracing::info!(
            "[EventBridge] Started, listening for events (filter prefixes: {:?})",
            filter.prefixes
        );

        let (emit_tx, mut emit_rx) = tokio::sync::mpsc::channel::<Event>(EMIT_CHANNEL_CAPACITY);

        let emit_app_handle = app_handle.clone();
        let emit_running = running.clone();
        let emit_task = tauri::async_runtime::spawn(async move {
            Self::emit_loop(&mut emit_rx, &emit_app_handle, &emit_running).await;
        });

        let mut event_count = 0u64;
        let mut forwarded_count = 0u64;
        let mut filtered_count = 0u64;
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
        heartbeat_interval.tick().await;

        while running.load(Ordering::SeqCst) {
            tokio::select! {
                recv_result = receiver.recv() => {
                    match recv_result {
                        Ok(event) => {
                            event_count += 1;

                            if !filter.matches(&event.topic) {
                                filtered_count += 1;
                                if filtered_count <= 5 || filtered_count % 100 == 0 {
                                    tracing::debug!(
                                        "[EventBridge] Event #{} filtered out: topic={}",
                                        event_count,
                                        event.topic
                                    );
                                }
                            } else {
                                if let Err(e) = emit_tx.send(event).await {
                                    tracing::error!(
                                        "[EventBridge] Failed to send event to emit channel: {}",
                                        e
                                    );
                                } else {
                                    forwarded_count += 1;
                                }
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            tracing::info!(
                                "[EventBridge] EventBus channel closed, stopping (total: {} received, {} forwarded)",
                                event_count,
                                forwarded_count
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
                _ = heartbeat_interval.tick() => {
                    tracing::info!(
                        "[EventBridge] Heartbeat: alive (total: {} received, {} forwarded, {} filtered)",
                        event_count,
                        forwarded_count,
                        filtered_count
                    );
                }
            }
        }

        drop(emit_tx);
        let _ = emit_task.await;

        running.store(false, Ordering::SeqCst);
        tracing::info!(
            "[EventBridge] Stopped (total: {} received, {} forwarded, {} filtered)",
            event_count,
            forwarded_count,
            filtered_count
        );
    }

    async fn emit_loop(
        rx: &mut tokio::sync::mpsc::Receiver<Event>,
        app_handle: &AppHandle<R>,
        running: &AtomicBool,
    ) {
        let mut emitted_count = 0u64;
        let mut failed_count = 0u64;

        while running.load(Ordering::SeqCst) {
            match rx.recv().await {
                Some(event) => {
                    match Self::emit_to_frontend(app_handle, &event) {
                        Ok(()) => {
                            emitted_count += 1;
                            if emitted_count <= 5 || emitted_count % 50 == 0 {
                                tracing::info!(
                                    "[EventBridge] Forwarded to frontend: topic={}, timestamp={}",
                                    event.topic,
                                    event.timestamp
                                );
                            }
                        }
                        Err(e) => {
                            failed_count += 1;
                            tracing::error!(
                                "[EventBridge] Failed to emit to frontend: topic={}, error={}",
                                event.topic,
                                e
                            );
                        }
                    }
                }
                None => {
                    tracing::info!(
                        "[EventBridge] Emit channel closed, stopping (emitted: {}, failed: {})",
                        emitted_count,
                        failed_count
                    );
                    break;
                }
            }
        }

        tracing::info!(
            "[EventBridge] Emit loop stopped (emitted: {}, failed: {})",
            emitted_count,
            failed_count
        );
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("[EventBridge] Stop requested");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
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
        let filter = EventFilter::with_prefixes(vec![
            "serial:".to_string(),
            "ble:".to_string(),
        ]);
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
