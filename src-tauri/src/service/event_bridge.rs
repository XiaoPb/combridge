use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};
use tokio::sync::broadcast;

use super::event_bus::{Event, EventBus, EventEncoding};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const HEARTBEAT_INTERVAL_SECS: u64 = 10;
const POLL_INTERVAL_MS: u64 = 5;
const EMIT_BATCH_SIZE: usize = 32;

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

#[derive(Debug)]
pub struct EventBridgeStats {
    pub received: u64,
    pub forwarded: u64,
    pub filtered: u64,
    pub emit_failed: u64,
}

pub struct EventBridge<R: Runtime> {
    event_bus: Arc<EventBus>,
    app_handle: AppHandle<R>,
    filter: EventFilter,
    running: Arc<AtomicBool>,
    received_count: Arc<AtomicU64>,
    forwarded_count: Arc<AtomicU64>,
    filtered_count: Arc<AtomicU64>,
    emit_failed_count: Arc<AtomicU64>,
}

impl<R: Runtime> EventBridge<R> {
    pub fn new(event_bus: Arc<EventBus>, app_handle: AppHandle<R>) -> Self {
        Self {
            event_bus,
            app_handle,
            filter: EventFilter::new(),
            running: Arc::new(AtomicBool::new(false)),
            received_count: Arc::new(AtomicU64::new(0)),
            forwarded_count: Arc::new(AtomicU64::new(0)),
            filtered_count: Arc::new(AtomicU64::new(0)),
            emit_failed_count: Arc::new(AtomicU64::new(0)),
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
        let received_count = self.received_count.clone();
        let forwarded_count = self.forwarded_count.clone();
        let filtered_count = self.filtered_count.clone();
        let emit_failed_count = self.emit_failed_count.clone();
        running.store(true, Ordering::SeqCst);

        std::thread::Builder::new()
            .name("event-bridge".to_string())
            .spawn(move || {
                Self::run_loop(
                    receiver,
                    &app_handle,
                    &filter,
                    &running,
                    &received_count,
                    &forwarded_count,
                    &filtered_count,
                    &emit_failed_count,
                );
            })
            .expect("Failed to spawn event-bridge thread");
    }

    fn run_loop(
        mut receiver: broadcast::Receiver<Event>,
        app_handle: &AppHandle<R>,
        filter: &EventFilter,
        running: &AtomicBool,
        received_count: &AtomicU64,
        forwarded_count: &AtomicU64,
        filtered_count: &AtomicU64,
        emit_failed_count: &AtomicU64,
    ) {
        tracing::info!(
            "[EventBridge] Started on dedicated thread (filter prefixes: {:?})",
            filter.prefixes
        );

        let mut last_heartbeat = std::time::Instant::now();
        let heartbeat_duration = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
        let poll_duration = Duration::from_millis(POLL_INTERVAL_MS);

        while running.load(Ordering::SeqCst) {
            let mut batch_count = 0usize;
            loop {
                match receiver.try_recv() {
                    Ok(event) => {
                        received_count.fetch_add(1, Ordering::Relaxed);

                        if !filter.matches(&event.topic) {
                            filtered_count.fetch_add(1, Ordering::Relaxed);
                        } else {
                            match Self::emit_to_frontend(app_handle, &event) {
                                Ok(()) => {
                                    forwarded_count.fetch_add(1, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    emit_failed_count.fetch_add(1, Ordering::Relaxed);
                                    tracing::error!(
                                        "[EventBridge] Failed to emit: topic={}, error={}",
                                        event.topic,
                                        e
                                    );
                                }
                            }
                        }

                        batch_count += 1;
                        if batch_count >= EMIT_BATCH_SIZE {
                            break;
                        }
                    }
                    Err(broadcast::error::TryRecvError::Empty) => {
                        break;
                    }
                    Err(broadcast::error::TryRecvError::Closed) => {
                        tracing::info!("[EventBridge] Broadcast channel closed, stopping");
                        running.store(false, Ordering::SeqCst);
                        return;
                    }
                    Err(broadcast::error::TryRecvError::Lagged(n)) => {
                        tracing::warn!(
                            "[EventBridge] Lagged behind by {} messages, continuing",
                            n
                        );
                    }
                }
            }

            if batch_count == 0 {
                std::thread::sleep(poll_duration);
            } else {
                std::thread::yield_now();
            }

            let now = std::time::Instant::now();
            if now.duration_since(last_heartbeat) >= heartbeat_duration {
                last_heartbeat = now;
                let recv = received_count.load(Ordering::Relaxed);
                let fwd = forwarded_count.load(Ordering::Relaxed);
                let filt = filtered_count.load(Ordering::Relaxed);
                let failed = emit_failed_count.load(Ordering::Relaxed);
                #[cfg(debug_assertions)]
                tracing::debug!(
                    "[EventBridge] Heartbeat: alive (received={}, forwarded={}, filtered={}, failed={})",
                    recv, fwd, filt, failed
                );
            }
        }

        let recv = received_count.load(Ordering::Relaxed);
        let fwd = forwarded_count.load(Ordering::Relaxed);
        let filt = filtered_count.load(Ordering::Relaxed);
        tracing::info!(
            "[EventBridge] Stopped (received={}, forwarded={}, filtered={})",
            recv, fwd, filt
        );
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        tracing::info!("[EventBridge] Stop requested");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn get_stats(&self) -> EventBridgeStats {
        EventBridgeStats {
            received: self.received_count.load(Ordering::Relaxed),
            forwarded: self.forwarded_count.load(Ordering::Relaxed),
            filtered: self.filtered_count.load(Ordering::Relaxed),
            emit_failed: self.emit_failed_count.load(Ordering::Relaxed),
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
