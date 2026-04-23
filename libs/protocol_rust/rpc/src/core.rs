//! RPC Core
//!
//! RPC核心实现，支持：
//! - 命令注册与调用
//! - 异步调用与超时重发
//! - 多帧数据重组
//! - 发布/订阅模式

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Mutex, oneshot, RwLock};
use tokio::time::timeout;

use crate::error::RpcError;
use crate::types::{DEFAULT_TIMEOUT_MS, GHRPC_FRAME_SIZE, MAX_RETRY_COUNT, MAX_SUPPORT_KEY_SIZE};
use crate::frame::{FrameBuilder, FrameParser, ParseResult};
use crate::log::{LogCallback, NullLogger};

const LAST_FRAME_FIX_INDEX: u8 = 255;

#[derive(Debug, Clone)]
pub struct RpcConfig {
    pub timeout_ms: u64,
    pub retry_count: u8,
    pub retry_delay_ms: u64,
    pub frame_size: usize,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            retry_count: MAX_RETRY_COUNT,
            retry_delay_ms: DEFAULT_TIMEOUT_MS,
            frame_size: GHRPC_FRAME_SIZE,
        }
    }
}

#[derive(Clone)]
pub struct InvokeNode {
    pub key: String,
    pub detail: Option<String>,
    pub handler: Option<RpcHandler>,
}

impl std::fmt::Debug for InvokeNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InvokeNode")
            .field("key", &self.key)
            .field("detail", &self.detail)
            .field("handler", &self.handler.is_some())
            .finish()
    }
}

pub type RpcHandler = Arc<dyn Fn(&[u8], usize, &mut InvokeContext) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct InvokeContext {
    pub topic: String,
    pub is_secure: bool,
    pub is_fin: bool,
    pub invoke_idx: u8,
    pub frame_idx: u8,
    response_data: Vec<u8>,
}

impl InvokeContext {
    pub fn new(topic: String) -> Self {
        Self {
            topic,
            is_secure: false,
            is_fin: true,
            invoke_idx: 0,
            frame_idx: LAST_FRAME_FIX_INDEX,
            response_data: Vec::new(),
        }
    }

    pub fn set_response(&mut self, data: Vec<u8>) {
        self.response_data = data;
    }

    pub fn get_response(&self) -> &[u8] {
        &self.response_data
    }
}

#[derive(Debug)]
struct PendingCall {
    invoke_idx: u8,
    key: String,
    tx: oneshot::Sender<Result<Vec<u8>, RpcError>>,
    retry_count: u8,
    frames: Vec<Vec<u8>>,
    current_frame_idx: u8,
}

#[derive(Debug, Clone)]
struct FrameBuffer {
    invoke_idx: u8,
    frame_idx: u8,
    data: Vec<u8>,
}

#[derive(Debug, Default)]
struct MultiFrameBuffer {
    frames: Vec<FrameBuffer>,
    expected_frame_idx: u8,
}

impl MultiFrameBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn add_frame(&mut self, invoke_idx: u8, frame_idx: u8, data: Vec<u8>) -> Result<bool, RpcError> {
        if !self.frames.is_empty() && self.frames[0].invoke_idx != invoke_idx {
            self.frames.clear();
            self.expected_frame_idx = 0;
        }

        if frame_idx == self.expected_frame_idx {
            self.frames.push(FrameBuffer {
                invoke_idx,
                frame_idx,
                data,
            });
            self.expected_frame_idx = self.expected_frame_idx.wrapping_add(1);
            Ok(true)
        } else if frame_idx < self.expected_frame_idx {
            Ok(false)
        } else {
            Err(RpcError::LoseFrame)
        }
    }

    fn is_complete(&self, is_fin: bool) -> bool {
        is_fin && !self.frames.is_empty()
    }

    fn get_all_data(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for frame in &self.frames {
            result.extend_from_slice(&frame.data);
        }
        result
    }

    fn clear(&mut self) {
        self.frames.clear();
        self.expected_frame_idx = 0;
    }
}

pub type SendFunction = Arc<dyn Fn(&[u8]) -> Result<(), RpcError> + Send + Sync>;

pub struct RpcCore {
    config: RpcConfig,
    static_nodes: Arc<RwLock<HashMap<String, InvokeNode>>>,
    dynamic_nodes: Arc<Mutex<HashMap<String, PendingCall>>>,
    frame_parser: Mutex<FrameParser>,
    multi_frame_buffer: Mutex<MultiFrameBuffer>,
    send_function: Mutex<Option<SendFunction>>,
    invoke_index: Mutex<u8>,
    logger: Arc<dyn LogCallback>,
    current_invoke_context: Mutex<Option<InvokeContext>>,
}

impl std::fmt::Debug for RpcCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcCore")
            .field("config", &self.config)
            .field("static_nodes_count", &self.static_nodes.try_read().map(|n| n.len()).unwrap_or(0))
            .field("dynamic_nodes_count", &self.dynamic_nodes.try_lock().map(|n| n.len()).unwrap_or(0))
            .finish()
    }
}

impl RpcCore {
    pub fn new(config: RpcConfig) -> Self {
        Self {
            config,
            static_nodes: Arc::new(RwLock::new(HashMap::new())),
            dynamic_nodes: Arc::new(Mutex::new(HashMap::new())),
            frame_parser: Mutex::new(FrameParser::new()),
            multi_frame_buffer: Mutex::new(MultiFrameBuffer::new()),
            send_function: Mutex::new(None),
            invoke_index: Mutex::new(1),
            logger: Arc::new(NullLogger),
            current_invoke_context: Mutex::new(None),
        }
    }

    pub fn with_logger(mut self, logger: Arc<dyn LogCallback>) -> Self {
        self.logger = logger;
        self
    }

    pub async fn set_send_function(&self, func: SendFunction) {
        let mut send_fn = self.send_function.lock().await;
        *send_fn = Some(func);
    }

    pub async fn register(&self, key: &str, handler: RpcHandler) -> Result<(), RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        let node = InvokeNode {
            key: key.to_string(),
            detail: None,
            handler: Some(handler),
        };

        let mut nodes = self.static_nodes.write().await;
        nodes.insert(key.to_string(), node);

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("Registered command: {}", key),
        );

        Ok(())
    }

    pub async fn unregister(&self, key: &str) -> bool {
        let mut nodes = self.static_nodes.write().await;
        nodes.remove(key).is_some()
    }

    pub async fn publish(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError> {
        let packed_data = crate::package::Package::pack(format, raw_data)
            .map_err(|_| RpcError::FormatError)?;
        self._publish(key, &packed_data).await
    }

    async fn _publish(&self, key: &str, data: &[u8]) -> Result<(), RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("[PUBLISH] key={}, data_len={}, data={:02X?}", key, data.len(), data),
        );

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames(key, data, false)
        };

        let send_fn = self.send_function.lock().await;
        if let Some(ref send) = *send_fn {
            for frame in &frames {
                send(frame)?;
            }
        }

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[PUBLISH] Sent {} frames for key={}", frames.len(), key),
        );

        Ok(())
    }

    pub async fn send(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError> {
        let packed_data = crate::package::Package::pack(format, raw_data)
            .map_err(|_| RpcError::FormatError)?;
        self._send(key, &packed_data).await
    }

    async fn _send(&self, key: &str, data: &[u8]) -> Result<(), RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        let invoke_idx = {
            let mut idx = self.invoke_index.lock().await;
            *idx = idx.wrapping_add(1);
            if *idx == 0 {
                *idx = 1;
            }
            *idx
        };

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("[SEND] key={}, invoke_idx={}, data_len={}, data={:02X?}",
                key, invoke_idx, data.len(), data),
        );

        let (tx, rx) = oneshot::channel();

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames_with_invoke_idx(key, data, true, invoke_idx)
        };

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[SEND] Built {} secure frames for key={}", frames.len(), key),
        );

        let pending = PendingCall {
            invoke_idx,
            key: key.to_string(),
            tx,
            retry_count: 0,
            frames: frames.clone(),
            current_frame_idx: 0,
        };

        {
            let mut dynamic = self.dynamic_nodes.lock().await;
            dynamic.insert(key.to_string(), pending);
            self.logger.log(
                crate::types::LogLevel::Debug,
                "RpcCore",
                &format!("[SEND] Inserted pending call for key={}", key),
            );
        }

        let send_fn = self.send_function.lock().await;
        if let Some(ref send) = *send_fn {
            for frame in &frames {
                if let Err(e) = send(frame) {
                    let mut dynamic = self.dynamic_nodes.lock().await;
                    dynamic.remove(key);
                    self.logger.log(
                        crate::types::LogLevel::Error,
                        "RpcCore",
                        &format!("[SEND] Send failed for key={}: {:?}", key, e),
                    );
                    return Err(e);
                }
            }
        }

        let timeout_duration = Duration::from_millis(self.config.timeout_ms);
        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[SEND] Waiting for ack, key={}, timeout={}ms", key, self.config.timeout_ms),
        );

        match timeout(timeout_duration, rx).await {
            Ok(Ok(Ok(_))) => {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!("[SEND] Ack received for key={}", key),
                );
                Ok(())
            }
            Ok(Ok(Err(e))) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SEND] Response error for key={}: {:?}", key, e),
                );
                Err(e)
            }
            Ok(Err(_)) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SEND] Channel closed for key={}", key),
                );
                Err(RpcError::ChannelClosed)
            }
            Err(_) => {
                let mut dynamic = self.dynamic_nodes.lock().await;
                dynamic.remove(key);
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SEND] Timeout waiting for ack, key={}", key),
                );
                Err(RpcError::Timeout)
            }
        }
    }

    pub async fn call(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<Vec<u8>, RpcError> {
        let packed_data = crate::package::Package::pack(format, raw_data)
            .map_err(|_| RpcError::FormatError)?;
        self._call(key, &packed_data).await
    }

    async fn _call(&self, key: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        let invoke_idx = {
            let mut idx = self.invoke_index.lock().await;
            *idx = idx.wrapping_add(1);
            if *idx == 0 {
                *idx = 1;
            }
            *idx
        };

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("[CALL] key={}, invoke_idx={}, data_len={}, data={:02X?}",
                key, invoke_idx, data.len(), data),
        );

        let (tx, rx) = oneshot::channel();

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames_with_invoke_idx(key, data, false, invoke_idx)
        };

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[CALL] Built {} frames for key={}", frames.len(), key),
        );

        let pending = PendingCall {
            invoke_idx,
            key: key.to_string(),
            tx,
            retry_count: 0,
            frames: frames.clone(),
            current_frame_idx: 0,
        };

        {
            let mut dynamic = self.dynamic_nodes.lock().await;
            dynamic.insert(key.to_string(), pending);
            self.logger.log(
                crate::types::LogLevel::Debug,
                "RpcCore",
                &format!("[CALL] Inserted pending call for key={}", key),
            );
        }

        let send_fn = self.send_function.lock().await;
        if let Some(ref send) = *send_fn {
            for frame in &frames {
                if let Err(e) = send(frame) {
                    let mut dynamic = self.dynamic_nodes.lock().await;
                    dynamic.remove(key);
                    self.logger.log(
                        crate::types::LogLevel::Error,
                        "RpcCore",
                        &format!("[CALL] Send failed for key={}: {:?}", key, e),
                    );
                    return Err(e);
                }
            }
        }

        let timeout_duration = Duration::from_millis(self.config.timeout_ms);
        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[CALL] Waiting for response, key={}, timeout={}ms", key, self.config.timeout_ms),
        );

        match timeout(timeout_duration, rx).await {
            Ok(Ok(Ok(result))) => {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!("[CALL] Response received, key={}, len={}, data={:02X?}",
                        key, result.len(), result),
                );
                Ok(result)
            }
            Ok(Ok(Err(e))) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[CALL] Response error for key={}: {:?}", key, e),
                );
                Err(e)
            }
            Ok(Err(_)) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[CALL] Channel closed for key={}", key),
                );
                Err(RpcError::ChannelClosed)
            }
            Err(_) => {
                let mut dynamic = self.dynamic_nodes.lock().await;
                dynamic.remove(key);
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[CALL] Timeout waiting for response, key={}", key),
                );
                Err(RpcError::Timeout)
            }
        }
    }

    pub async fn sall(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<Vec<u8>, RpcError> {
        let packed_data = crate::package::Package::pack(format, raw_data)
            .map_err(|_| RpcError::FormatError)?;
        self._sall(key, &packed_data).await
    }

    async fn _sall(&self, key: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        let invoke_idx = {
            let mut idx = self.invoke_index.lock().await;
            *idx = idx.wrapping_add(1);
            if *idx == 0 {
                *idx = 1;
            }
            *idx
        };

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("[SALL] key={}, invoke_idx={}, data_len={}, data={:02X?}",
                key, invoke_idx, data.len(), data),
        );

        let (tx, rx) = oneshot::channel();

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames_with_invoke_idx(key, data, true, invoke_idx)
        };

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[SALL] Built {} secure frames for key={}", frames.len(), key),
        );

        let pending = PendingCall {
            invoke_idx,
            key: key.to_string(),
            tx,
            retry_count: 0,
            frames: frames.clone(),
            current_frame_idx: 0,
        };

        {
            let mut dynamic = self.dynamic_nodes.lock().await;
            dynamic.insert(key.to_string(), pending);
            self.logger.log(
                crate::types::LogLevel::Debug,
                "RpcCore",
                &format!("[SALL] Inserted pending call for key={}", key),
            );
        }

        let send_fn = self.send_function.lock().await;
        if let Some(ref send) = *send_fn {
            for frame in &frames {
                if let Err(e) = send(frame) {
                    let mut dynamic = self.dynamic_nodes.lock().await;
                    dynamic.remove(key);
                    self.logger.log(
                        crate::types::LogLevel::Error,
                        "RpcCore",
                        &format!("[SALL] Send failed for key={}: {:?}", key, e),
                    );
                    return Err(e);
                }
            }
        }

        let timeout_duration = Duration::from_millis(self.config.timeout_ms);
        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[SALL] Waiting for response, key={}, timeout={}ms", key, self.config.timeout_ms),
        );

        match timeout(timeout_duration, rx).await {
            Ok(Ok(Ok(result))) => {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!("[SALL] Response received, key={}, len={}, data={:02X?}",
                        key, result.len(), result),
                );
                Ok(result)
            }
            Ok(Ok(Err(e))) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SALL] Response error for key={}: {:?}", key, e),
                );
                Err(e)
            }
            Ok(Err(_)) => {
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SALL] Channel closed for key={}", key),
                );
                Err(RpcError::ChannelClosed)
            }
            Err(_) => {
                let mut dynamic = self.dynamic_nodes.lock().await;
                dynamic.remove(key);
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[SALL] Timeout waiting for response, key={}", key),
                );
                Err(RpcError::Timeout)
            }
        }
    }

    pub async fn process(&self, data: &[u8]) -> Vec<Result<ParseResult, RpcError>> {
        let mut parser = self.frame_parser.lock().await;
        let results = parser.process(data);

        for result in &results {
            if let Ok(parse_result) = result {
                if let Err(e) = self.handle_parse_result(parse_result.clone()).await {
                    self.logger.log(
                        crate::types::LogLevel::Error,
                        "RpcCore",
                        &format!("Error handling frame: {:?}", e),
                    );
                }
            }
        }

        results
    }

    async fn handle_parse_result(&self, result: ParseResult) -> Result<(), RpcError> {
        let key = result.key.clone();
        let is_secure = result.is_secure;
        let is_fin = result.is_fin;
        let invoke_idx = result.invoke_idx;
        let frame_idx = result.frame_idx;

        if is_secure {
            self.handle_secure_frame(&key, invoke_idx, frame_idx, is_fin, &result.param).await?;
        } else {
            self.handle_unsecure_frame(&key, frame_idx, is_fin, &result.param).await?;
        }

        Ok(())
    }

    async fn handle_secure_frame(
        &self,
        key: &str,
        invoke_idx: u8,
        frame_idx: u8,
        is_fin: bool,
        data: &[u8],
    ) -> Result<(), RpcError> {
        let nodes = self.static_nodes.read().await;
        if let Some(node) = nodes.get(key) {
            if let Some(ref handler) = node.handler {
                let mut context = InvokeContext::new(key.to_string());
                context.is_fin = is_fin;
                context.frame_idx = frame_idx;
                context.invoke_idx = invoke_idx;

                handler(data, data.len(), &mut context);

                let response = context.get_response();
                if !response.is_empty() {
                    let mut response_frame = vec![1u8];
                    response_frame.extend_from_slice(response);
                    let frames = {
                        let mut builder = FrameBuilder::new();
                        builder.build_frames_with_invoke_idx(key, &response_frame, true, invoke_idx)
                    };
                    let send_fn = self.send_function.lock().await;
                    if let Some(ref send) = *send_fn {
                        for frame in &frames {
                            if let Err(e) = send(frame) {
                                self.logger.log(
                                    crate::types::LogLevel::Error,
                                    "RpcCore",
                                    &format!("Failed to send response: {}", e),
                                );
                            }
                        }
                    }
                }
            }
        } else {
            let mut dynamic = self.dynamic_nodes.lock().await;
            if let Some(pending) = dynamic.remove(key) {
                if data.len() >= 2 {
                    let msg_type = data[0];
                    match msg_type {
                        0 => {
                            let ack_frame_idx = data.get(1).copied().unwrap_or(0);
                            self.logger.log(
                                crate::types::LogLevel::Debug,
                                "RpcCore",
                                &format!("Received ACK for frame {} on key {}", ack_frame_idx, key),
                            );
                        }
                        1 => {
                            let response_data = if data.len() > 2 {
                                data[2..].to_vec()
                            } else {
                                Vec::new()
                            };
                            let _ = pending.tx.send(Ok(response_data));
                        }
                        2 | 3 => {
                            let _ = pending.tx.send(Err(RpcError::CommandNotFound));
                        }
                        _ => {}
                    }
                }
            } else {
                self.logger.log(
                    crate::types::LogLevel::Warn,
                    "RpcCore",
                    &format!("Command not found: {}", key),
                );
            }
        }

        Ok(())
    }

    async fn handle_unsecure_frame(
        &self,
        key: &str,
        frame_idx: u8,
        is_fin: bool,
        data: &[u8],
    ) -> Result<(), RpcError> {
        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[RECV] Unsecure frame: key={}, frame_idx={}, is_fin={}, data_len={}",
                key, frame_idx, is_fin, data.len()),
        );

        let effective_frame_idx = if frame_idx == LAST_FRAME_FIX_INDEX {
            0
        } else {
            frame_idx
        };

        {
            let mut buffer = self.multi_frame_buffer.lock().await;
            buffer.add_frame(0, effective_frame_idx, data.to_vec())?;

            if !buffer.is_complete(is_fin) {
                self.logger.log(
                    crate::types::LogLevel::Debug,
                    "RpcCore",
                    &format!("[RECV] Multi-frame not complete, key={}, waiting for more frames", key),
                );
                return Ok(());
            }
        }

        let all_data = {
            let mut buffer = self.multi_frame_buffer.lock().await;
            let data = buffer.get_all_data();
            buffer.clear();
            data
        };

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!("[RECV] Complete frame: key={}, total_len={}, data={:02X?}",
                key, all_data.len(), all_data),
        );

        let nodes = self.static_nodes.read().await;
        if let Some(node) = nodes.get(key) {
            if let Some(ref handler) = node.handler {
                let mut context = InvokeContext::new(key.to_string());
                context.is_fin = is_fin;
                context.frame_idx = frame_idx;

                {
                    let mut invoke_ctx = self.current_invoke_context.lock().await;
                    *invoke_ctx = Some(context.clone());
                }

                handler(&all_data, all_data.len(), &mut context);

                if let Some(response) = self.get_and_clear_invoke_context().await {
                    if !response.is_empty() {
                        self._publish(key, &response).await?;
                    }
                }
            }
        } else {
            drop(nodes);
            
            let mut dynamic = self.dynamic_nodes.lock().await;
            if let Some(pending) = dynamic.remove(key) {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!("[RECV] Found pending call for key={}, sending response", key),
                );
                let _ = pending.tx.send(Ok(all_data));
            } else {
                self.logger.log(
                    crate::types::LogLevel::Warn,
                    "RpcCore",
                    &format!("[RECV] No pending call found for key={}", key),
                );
            }
        }

        Ok(())
    }

    async fn get_and_clear_invoke_context(&self) -> Option<Vec<u8>> {
        let mut ctx = self.current_invoke_context.lock().await;
        if let Some(ref mut invoke_ctx) = *ctx {
            let response = invoke_ctx.get_response().to_vec();
            invoke_ctx.response_data.clear();
            Some(response)
        } else {
            None
        }
    }

    pub async fn return_value(&self, data: &[u8]) -> Result<(), RpcError> {
        let mut ctx = self.current_invoke_context.lock().await;
        if let Some(ref mut invoke_ctx) = *ctx {
            invoke_ctx.set_response(data.to_vec());
            Ok(())
        } else {
            Err(RpcError::NotUnderInvoke)
        }
    }

    pub async fn get_registered_commands(&self) -> Vec<String> {
        let nodes = self.static_nodes.read().await;
        nodes.keys().cloned().collect()
    }

    pub async fn has_command(&self, key: &str) -> bool {
        let nodes = self.static_nodes.read().await;
        nodes.contains_key(key)
    }

    pub fn get_config(&self) -> &RpcConfig {
        &self.config
    }
}

impl Default for RpcCore {
    fn default() -> Self {
        Self::new(RpcConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_rpc_core_creation() {
        let core = RpcCore::default();
        assert!(core.get_registered_commands().await.is_empty());
    }

    #[tokio::test]
    async fn test_register_command() {
        let core = RpcCore::default();
        let handler = Arc::new(|_data: &[u8], _size: usize, _ctx: &mut InvokeContext| {});

        let result = core.register("test_cmd", handler).await;
        assert!(result.is_ok());
        assert!(core.has_command("test_cmd").await);
    }

    #[tokio::test]
    async fn test_unregister_command() {
        let core = RpcCore::default();
        let handler = Arc::new(|_data: &[u8], _size: usize, _ctx: &mut InvokeContext| {});

        core.register("test_cmd", handler).await.unwrap();
        assert!(core.has_command("test_cmd").await);

        let removed = core.unregister("test_cmd").await;
        assert!(removed);
        assert!(!core.has_command("test_cmd").await);
    }

    #[tokio::test]
    async fn test_key_too_long() {
        let core = RpcCore::default();
        let handler = Arc::new(|_data: &[u8], _size: usize, _ctx: &mut InvokeContext| {});

        let long_key = "a".repeat(MAX_SUPPORT_KEY_SIZE);
        let result = core.register(&long_key, handler).await;
        assert!(matches!(result, Err(RpcError::KeyOverMaxSize)));
    }

    #[tokio::test]
    async fn test_publish_without_send_function() {
        let core = RpcCore::default();
        let result = core.publish("test", &[1, 2, 3]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invoke_context() {
        let mut ctx = InvokeContext::new("test".to_string());
        assert_eq!(ctx.topic, "test");
        assert!(!ctx.is_secure);
        assert!(ctx.is_fin);

        ctx.set_response(vec![1, 2, 3]);
        assert_eq!(ctx.get_response(), &[1, 2, 3]);
    }

    #[tokio::test]
    async fn test_multi_frame_buffer() {
        let mut buffer = MultiFrameBuffer::new();

        buffer.add_frame(0, 0, vec![1, 2, 3]).unwrap();
        assert!(!buffer.is_complete(false));

        buffer.add_frame(0, 1, vec![4, 5, 6]).unwrap();
        assert!(!buffer.is_complete(false));

        buffer.add_frame(0, 2, vec![7, 8, 9]).unwrap();
        assert!(buffer.is_complete(true));

        let data = buffer.get_all_data();
        assert_eq!(data, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[tokio::test]
    async fn test_multi_frame_buffer_lose_frame() {
        let mut buffer = MultiFrameBuffer::new();

        buffer.add_frame(0, 0, vec![1, 2, 3]).unwrap();
        let result = buffer.add_frame(0, 2, vec![4, 5, 6]);
        assert!(matches!(result, Err(RpcError::LoseFrame)));
    }

    #[tokio::test]
    async fn test_process_frame() {
        let core = RpcCore::default();
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let handler = Arc::new(move |_data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        core.register("G", handler).await.unwrap();

        let mut builder = FrameBuilder::new();
        let frame = builder.build_frame("G", &[1, 2, 3], false, true, 0, 0);

        let results = core.process(&frame).await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_return_value_without_context() {
        let core = RpcCore::default();
        let result = core.return_value(&[1, 2, 3]).await;
        assert!(matches!(result, Err(RpcError::NotUnderInvoke)));
    }

    #[tokio::test]
    async fn test_config_default() {
        let config = RpcConfig::default();
        assert_eq!(config.timeout_ms, DEFAULT_TIMEOUT_MS);
        assert_eq!(config.retry_count, MAX_RETRY_COUNT);
        assert_eq!(config.frame_size, GHRPC_FRAME_SIZE);
    }
}
