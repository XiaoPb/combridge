//! RPC Core
//!
//! RPC核心实现，支持：
//! - 命令注册与调用
//! - 异步调用与超时重发
//! - 多帧数据重组
//! - 发布/订阅模式

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tokio::time::{timeout, timeout_at, Instant};

use crate::error::RpcError;
use crate::frame::{FrameBuilder, FrameParser, ParseResult};
use crate::log::{LogCallback, NullLogger};
use crate::types::{DEFAULT_TIMEOUT_MS, GHRPC_FRAME_SIZE, MAX_RETRY_COUNT, MAX_SUPPORT_KEY_SIZE};

const LAST_FRAME_FIX_INDEX: u8 = 255;
const HEX_PREVIEW_LIMIT: usize = 512;

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
enum PendingCall {
    Call {
        tx: oneshot::Sender<Result<Vec<u8>, RpcError>>,
    },
    Secure {
        tx: mpsc::UnboundedSender<SecureResponse>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SecureResponse {
    FrameAck { invoke_idx: u8, frame_idx: u8 },
    CommandNotFound { invoke_idx: u8 },
    Return { invoke_idx: u8, data: Vec<u8> },
    Error { invoke_idx: u8 },
}

impl SecureResponse {
    fn decode(data: &[u8]) -> Result<Self, RpcError> {
        fn read_u8(data: &[u8], offset: &mut usize) -> Result<u8, RpcError> {
            if *offset + 2 > data.len() || data[*offset] & 0x3f != 0x19 {
                return Err(RpcError::UnpackageError);
            }
            let value = data[*offset + 1];
            *offset += 2;
            Ok(value)
        }

        let mut offset = 0;
        let response_type = read_u8(data, &mut offset)?;
        let invoke_idx = read_u8(data, &mut offset)?;

        match response_type {
            0 => Ok(Self::FrameAck {
                invoke_idx,
                frame_idx: read_u8(data, &mut offset)?,
            }),
            1 => Ok(Self::CommandNotFound { invoke_idx }),
            2 => Ok(Self::Return {
                invoke_idx,
                data: data[offset..].to_vec(),
            }),
            3 => Ok(Self::Error { invoke_idx }),
            _ => Err(RpcError::UnpackageError),
        }
    }

    fn invoke_idx(&self) -> u8 {
        match self {
            Self::FrameAck { invoke_idx, .. }
            | Self::CommandNotFound { invoke_idx }
            | Self::Return { invoke_idx, .. }
            | Self::Error { invoke_idx } => *invoke_idx,
        }
    }
}

#[derive(Debug, Clone)]
struct FrameBuffer {
    invoke_idx: u8,
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

    fn add_frame(
        &mut self,
        invoke_idx: u8,
        frame_idx: u8,
        data: Vec<u8>,
    ) -> Result<bool, RpcError> {
        if frame_idx == 0 && self.expected_frame_idx != 0 {
            self.clear();
        }

        if frame_idx != self.expected_frame_idx {
            self.clear();
            Err(RpcError::LoseFrame)
        } else {
            self.frames.push(FrameBuffer { invoke_idx, data });
            self.expected_frame_idx = self.expected_frame_idx.wrapping_add(1);
            Ok(true)
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

pub type SendFuture = Pin<Box<dyn Future<Output = Result<(), RpcError>> + Send>>;
pub type SendFunction = Arc<dyn Fn(Vec<u8>) -> SendFuture + Send + Sync>;

pub struct RpcCore {
    config: RpcConfig,
    static_nodes: Arc<RwLock<HashMap<String, InvokeNode>>>,
    dynamic_nodes: Arc<Mutex<HashMap<(String, u8), PendingCall>>>,
    frame_parser: Mutex<FrameParser>,
    multi_frame_buffer: Mutex<HashMap<String, MultiFrameBuffer>>,
    send_function: Mutex<Option<SendFunction>>,
    send_lock: Mutex<()>,
    invoke_index: Mutex<u8>,
    logger: Arc<dyn LogCallback>,
    current_invoke_context: Mutex<Option<InvokeContext>>,
}

impl std::fmt::Debug for RpcCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcCore")
            .field("config", &self.config)
            .field(
                "static_nodes_count",
                &self.static_nodes.try_read().map(|n| n.len()).unwrap_or(0),
            )
            .field(
                "dynamic_nodes_count",
                &self.dynamic_nodes.try_lock().map(|n| n.len()).unwrap_or(0),
            )
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
            multi_frame_buffer: Mutex::new(HashMap::new()),
            send_function: Mutex::new(None),
            send_lock: Mutex::new(()),
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

    fn hex_bytes(data: &[u8]) -> String {
        Self::hex_bytes_with_limit(data, HEX_PREVIEW_LIMIT)
    }

    fn hex_bytes_with_limit(data: &[u8], limit: usize) -> String {
        let shown = data.len().min(limit);
        let mut output = String::new();
        for (idx, byte) in data.iter().take(shown).enumerate() {
            if idx > 0 {
                output.push(' ');
            }
            output.push_str(&format!("{:02X}", byte));
        }
        if data.len() > shown {
            output.push_str(&format!(" ...(+{} bytes)", data.len() - shown));
        }
        output
    }

    fn log_tx_frames(&self, op: &str, key: &str, frames: &[Vec<u8>]) {
        for (idx, frame) in frames.iter().enumerate() {
            self.logger.log(
                crate::types::LogLevel::Debug,
                "rpc_core",
                &format!(
                    "[RpcCore][TX_FRAME] op={}, key={}, index={}/{}, len={}, bytes={}",
                    op,
                    key,
                    idx + 1,
                    frames.len(),
                    frame.len(),
                    Self::hex_bytes(frame)
                ),
            );
        }
    }

    async fn send_frame(
        &self,
        op: &str,
        key: &str,
        frame_idx: usize,
        frame_count: usize,
        frame: &[u8],
    ) -> Result<(), RpcError> {
        let send = self
            .send_function
            .lock()
            .await
            .clone()
            .ok_or(RpcError::ChannelClosed)?;

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!(
                "[{}] TX start key={}, frame={}/{}, len={}",
                op,
                key,
                frame_idx + 1,
                frame_count,
                frame.len()
            ),
        );
        send(frame.to_vec()).await?;
        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!(
                "[{}] TX complete key={}, frame={}/{}",
                op,
                key,
                frame_idx + 1,
                frame_count
            ),
        );
        Ok(())
    }

    async fn retry_delay(&self) {
        if self.config.retry_delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
        }
    }

    async fn wait_secure_ack(
        &self,
        key: &str,
        invoke_idx: u8,
        expected_frame_idx: u8,
        rx: &mut mpsc::UnboundedReceiver<SecureResponse>,
    ) -> Result<(), RpcError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.timeout_ms);
        loop {
            if Instant::now() >= deadline {
                return Err(RpcError::Timeout);
            }
            let response = timeout_at(deadline, rx.recv())
                .await
                .map_err(|_| RpcError::Timeout)?
                .ok_or(RpcError::ChannelClosed)?;

            match response {
                SecureResponse::FrameAck {
                    invoke_idx: response_invoke,
                    frame_idx,
                } if response_invoke == invoke_idx && frame_idx == expected_frame_idx => {
                    self.logger.log(
                        crate::types::LogLevel::Info,
                        "RpcCore",
                        &format!(
                            "[SECURE] RX frame ack key={}, invoke_idx={}, frame_idx={}",
                            key, invoke_idx, frame_idx
                        ),
                    );
                    return Ok(());
                }
                SecureResponse::CommandNotFound { .. } => return Err(RpcError::CommandNotFound),
                SecureResponse::Error { .. } => return Err(RpcError::RemoteError),
                unexpected => {
                    self.logger.log(
                        crate::types::LogLevel::Warn,
                        "RpcCore",
                        &format!(
                            "[SECURE] Ignoring unexpected response while waiting frame ack: key={}, invoke_idx={}, response={:?}",
                            key, invoke_idx, unexpected
                        ),
                    );
                }
            }
        }
    }

    async fn wait_secure_return(
        &self,
        key: &str,
        invoke_idx: u8,
        rx: &mut mpsc::UnboundedReceiver<SecureResponse>,
    ) -> Result<Vec<u8>, RpcError> {
        let deadline = Instant::now() + Duration::from_millis(self.config.timeout_ms);
        loop {
            if Instant::now() >= deadline {
                return Err(RpcError::Timeout);
            }
            let response = timeout_at(deadline, rx.recv())
                .await
                .map_err(|_| RpcError::Timeout)?
                .ok_or(RpcError::ChannelClosed)?;

            match response {
                SecureResponse::Return {
                    invoke_idx: response_invoke,
                    data,
                } if response_invoke == invoke_idx => {
                    self.logger.log(
                        crate::types::LogLevel::Info,
                        "RpcCore",
                        &format!(
                            "[SECURE] RX final return key={}, invoke_idx={}, len={}",
                            key,
                            invoke_idx,
                            data.len()
                        ),
                    );
                    return Ok(data);
                }
                SecureResponse::CommandNotFound { .. } => return Err(RpcError::CommandNotFound),
                SecureResponse::Error { .. } => return Err(RpcError::RemoteError),
                unexpected => {
                    self.logger.log(
                        crate::types::LogLevel::Warn,
                        "RpcCore",
                        &format!(
                            "[SECURE] Ignoring unexpected response while waiting final return: key={}, invoke_idx={}, response={:?}",
                            key, invoke_idx, unexpected
                        ),
                    );
                }
            }
        }
    }

    pub async fn unregister(&self, key: &str) -> bool {
        let mut nodes = self.static_nodes.write().await;
        nodes.remove(key).is_some()
    }

    pub async fn publish(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError> {
        let packed_data =
            crate::package::Package::pack(format, raw_data).map_err(|_| RpcError::FormatError)?;
        self._publish(key, &packed_data).await
    }

    async fn _publish(&self, key: &str, data: &[u8]) -> Result<(), RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!(
                "[PUBLISH] key={}, data_len={}, data={:02X?}",
                key,
                data.len(),
                data
            ),
        );

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames(key, data, false)
        };
        self.log_tx_frames("PUBLISH", key, &frames);

        let _send_guard = self.send_lock.lock().await;
        for (frame_idx, frame) in frames.iter().enumerate() {
            self.send_frame("PUBLISH", key, frame_idx, frames.len(), frame)
                .await?;
        }

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[PUBLISH] Sent {} frames for key={}", frames.len(), key),
        );

        Ok(())
    }

    pub async fn send(&self, key: &str, format: &str, raw_data: &[u8]) -> Result<(), RpcError> {
        let packed_data =
            crate::package::Package::pack(format, raw_data).map_err(|_| RpcError::FormatError)?;
        self._send(key, &packed_data).await
    }

    async fn _send(&self, key: &str, data: &[u8]) -> Result<(), RpcError> {
        self._secure_request("SEND", key, data).await.map(|_| ())
    }

    async fn _secure_request(&self, op: &str, key: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }

        let _send_guard = self.send_lock.lock().await;
        let invoke_idx = {
            let mut idx = self.invoke_index.lock().await;
            *idx = idx.wrapping_add(1);
            if *idx == 0 {
                *idx = 1;
            }
            *idx
        };
        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames_with_invoke_idx(key, data, true, invoke_idx)
        };
        self.log_tx_frames(op, key, &frames);
        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!(
                "[{}] key={}, invoke_idx={}, data_len={}, frame_count={}",
                op,
                key,
                invoke_idx,
                data.len(),
                frames.len()
            ),
        );

        let (tx, mut rx) = mpsc::unbounded_channel();
        let pending_key = (key.to_string(), invoke_idx);
        self.dynamic_nodes
            .lock()
            .await
            .insert(pending_key.clone(), PendingCall::Secure { tx });

        let result = async {
            for (frame_idx, frame) in frames.iter().enumerate() {
                let is_final = frame_idx + 1 == frames.len();
                let mut frame_confirmed = false;

                for attempt in 0..=self.config.retry_count {
                    if attempt > 0 {
                        self.logger.log(
                            crate::types::LogLevel::Warn,
                            "RpcCore",
                            &format!(
                                "[{}] Retry key={}, invoke_idx={}, frame_idx={}, attempt={}/{}",
                                op, key, invoke_idx, frame_idx, attempt, self.config.retry_count
                            ),
                        );
                        self.retry_delay().await;
                    }

                    self.send_frame(op, key, frame_idx, frames.len(), frame)
                        .await?;
                    let response = if is_final {
                        self.wait_secure_return(key, invoke_idx, &mut rx).await
                    } else {
                        self.wait_secure_ack(key, invoke_idx, frame_idx as u8, &mut rx)
                            .await
                            .map(|_| Vec::new())
                    };

                    match response {
                        Ok(data) => {
                            if is_final {
                                return Ok(data);
                            }
                            frame_confirmed = true;
                            break;
                        }
                        Err(RpcError::Timeout) => {}
                        Err(error) => return Err(error),
                    }
                }

                if !is_final && !frame_confirmed {
                    return Err(RpcError::MaxRetryExceeded);
                }
                if is_final {
                    return Err(RpcError::MaxRetryExceeded);
                }
            }

            Err(RpcError::SendFail)
        }
        .await;

        self.dynamic_nodes.lock().await.remove(&pending_key);
        result
    }

    pub async fn call(
        &self,
        key: &str,
        format: &str,
        raw_data: &[u8],
    ) -> Result<Vec<u8>, RpcError> {
        let packed_data =
            crate::package::Package::pack(format, raw_data).map_err(|_| RpcError::FormatError)?;
        self._call(key, &packed_data).await
    }

    async fn _call(&self, key: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        if key.len() >= MAX_SUPPORT_KEY_SIZE {
            return Err(RpcError::KeyOverMaxSize);
        }
        let _send_guard = self.send_lock.lock().await;

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
            &format!(
                "[CALL] key={}, invoke_idx={}, data_len={}, data={:02X?}",
                key,
                invoke_idx,
                data.len(),
                data
            ),
        );

        let (tx, rx) = oneshot::channel();

        let frames = {
            let mut builder = FrameBuilder::new();
            builder.build_frames_with_invoke_idx(key, data, false, invoke_idx)
        };
        self.log_tx_frames("CALL", key, &frames);

        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!("[CALL] Built {} frames for key={}", frames.len(), key),
        );

        let pending_key = (key.to_string(), invoke_idx);

        {
            let mut dynamic = self.dynamic_nodes.lock().await;
            dynamic.insert(pending_key.clone(), PendingCall::Call { tx });
            self.logger.log(
                crate::types::LogLevel::Debug,
                "RpcCore",
                &format!("[CALL] Inserted pending call for key={}", key),
            );
        }

        for (frame_idx, frame) in frames.iter().enumerate() {
            if let Err(error) = self
                .send_frame("CALL", key, frame_idx, frames.len(), frame)
                .await
            {
                self.dynamic_nodes.lock().await.remove(&pending_key);
                return Err(error);
            }
        }

        let timeout_duration = Duration::from_millis(self.config.timeout_ms);
        self.logger.log(
            crate::types::LogLevel::Debug,
            "RpcCore",
            &format!(
                "[CALL] Waiting for response, key={}, timeout={}ms",
                key, self.config.timeout_ms
            ),
        );

        match timeout(timeout_duration, rx).await {
            Ok(Ok(Ok(result))) => {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!(
                        "[CALL] Response received, key={}, len={}, data={:02X?}",
                        key,
                        result.len(),
                        result
                    ),
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
                dynamic.remove(&pending_key);
                self.logger.log(
                    crate::types::LogLevel::Error,
                    "RpcCore",
                    &format!("[CALL] Timeout waiting for response, key={}", key),
                );
                Err(RpcError::Timeout)
            }
        }
    }

    pub async fn sall(
        &self,
        key: &str,
        format: &str,
        raw_data: &[u8],
    ) -> Result<Vec<u8>, RpcError> {
        let packed_data =
            crate::package::Package::pack(format, raw_data).map_err(|_| RpcError::FormatError)?;
        self._sall(key, &packed_data).await
    }

    async fn _sall(&self, key: &str, data: &[u8]) -> Result<Vec<u8>, RpcError> {
        self._secure_request("SALL", key, data).await
    }

    pub async fn process(&self, data: &[u8]) -> Vec<Result<ParseResult, RpcError>> {
        self.logger.log(
            crate::types::LogLevel::Debug,
            "rpc_core",
            &format!(
                "[RpcCore][RX_RAW] len={}, bytes={}",
                data.len(),
                Self::hex_bytes(data)
            ),
        );

        let mut parser = self.frame_parser.lock().await;
        let results = parser.process(data);
        drop(parser);

        for result in &results {
            match result {
                Ok(parse_result) => {
                    self.logger.log(
                        crate::types::LogLevel::Debug,
                        "rpc_core",
                        &format!(
                            "[RpcCore][RX_FRAME] key={}, secure={}, fin={}, invoke_idx={}, frame_idx={}, param_len={}, param={}",
                            parse_result.key,
                            parse_result.is_secure,
                            parse_result.is_fin,
                            parse_result.invoke_idx,
                            parse_result.frame_idx,
                            parse_result.param.len(),
                            Self::hex_bytes(&parse_result.param)
                        ),
                    );
                    if let Err(e) = self.handle_parse_result(parse_result.clone()).await {
                        self.multi_frame_buffer
                            .lock()
                            .await
                            .remove(&parse_result.key);
                        self.logger.log(
                            crate::types::LogLevel::Error,
                            "RpcCore",
                            &format!("Error handling frame: {:?}", e),
                        );
                    }
                }
                Err(error) => {
                    self.multi_frame_buffer.lock().await.clear();
                    self.logger.log(
                        crate::types::LogLevel::Warn,
                        "RpcCore",
                        &format!(
                            "Frame parsing failed, receive state resynchronized: {:?}",
                            error
                        ),
                    );
                }
            }
        }

        results
    }

    pub async fn reset_receive_state(&self) {
        self.frame_parser.lock().await.reset();
        self.multi_frame_buffer.lock().await.clear();
    }

    async fn handle_parse_result(&self, result: ParseResult) -> Result<(), RpcError> {
        let key = result.key.clone();
        let is_secure = result.is_secure;
        let is_fin = result.is_fin;
        let invoke_idx = result.invoke_idx;
        let frame_idx = result.frame_idx;

        if is_secure {
            self.handle_secure_frame(&key, invoke_idx, frame_idx, is_fin, &result.param)
                .await?;
        } else {
            self.handle_unsecure_frame(&key, frame_idx, is_fin, &result.param)
                .await?;
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
                    let send_fn = self.send_function.lock().await.clone();
                    if let Some(send) = send_fn {
                        for frame in frames {
                            if let Err(e) = send(frame).await {
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
            let dynamic = self.dynamic_nodes.lock().await;
            let pending_key = (key.to_string(), invoke_idx);
            if let Some(PendingCall::Secure { tx }) = dynamic.get(&pending_key) {
                if !data.is_empty() {
                    let response = match data[0] {
                        0 if data.len() >= 2 => SecureResponse::FrameAck {
                            invoke_idx,
                            frame_idx: data[1],
                        },
                        1 => SecureResponse::CommandNotFound { invoke_idx },
                        2 => SecureResponse::Return {
                            invoke_idx,
                            data: data[1..].to_vec(),
                        },
                        3 => SecureResponse::Error { invoke_idx },
                        _ => return Ok(()),
                    };
                    let _ = tx.send(response);
                }
            } else {
                self.logger.log(
                    crate::types::LogLevel::Debug,
                    "RpcCore",
                    &format!(
                        "Command not found: {} (response already consumed or unexpected ACK)",
                        key
                    ),
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
            &format!(
                "[RECV] Unsecure frame: key={}, frame_idx={}, is_fin={}, data_len={}",
                key,
                frame_idx,
                is_fin,
                data.len()
            ),
        );

        {
            let mut buffers = self.multi_frame_buffer.lock().await;
            let buffer = buffers
                .entry(key.to_string())
                .or_insert_with(MultiFrameBuffer::new);
            let effective_frame_idx = if frame_idx == LAST_FRAME_FIX_INDEX {
                buffer.expected_frame_idx
            } else {
                frame_idx
            };
            buffer.add_frame(0, effective_frame_idx, data.to_vec())?;

            if !buffer.is_complete(is_fin) {
                self.logger.log(
                    crate::types::LogLevel::Debug,
                    "RpcCore",
                    &format!(
                        "[RECV] Multi-frame not complete, key={}, waiting for more frames",
                        key
                    ),
                );
                return Ok(());
            }
        }

        let all_data = {
            let mut buffers = self.multi_frame_buffer.lock().await;
            buffers
                .remove(key)
                .map(|buffer| buffer.get_all_data())
                .unwrap_or_default()
        };

        self.logger.log(
            crate::types::LogLevel::Info,
            "RpcCore",
            &format!(
                "[RECV] Complete frame: key={}, total_len={}, data={:02X?}",
                key,
                all_data.len(),
                all_data
            ),
        );
        self.logger.log(
            crate::types::LogLevel::Debug,
            "rpc_core",
            &format!(
                "[RpcCore][RX_COMPLETE] key={}, total_len={}, data={}",
                key,
                all_data.len(),
                Self::hex_bytes(&all_data)
            ),
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
            if let Ok(response) = SecureResponse::decode(&all_data) {
                let pending_key = (key.to_string(), response.invoke_idx());
                if let Some(PendingCall::Secure { tx }) = dynamic.get(&pending_key) {
                    self.logger.log(
                        crate::types::LogLevel::Info,
                        "RpcCore",
                        &format!(
                            "[SECURE] RX key={}, invoke_idx={}, response={:?}",
                            key,
                            response.invoke_idx(),
                            response
                        ),
                    );
                    let _ = tx.send(response);
                } else {
                    self.logger.log(
                        crate::types::LogLevel::Warn,
                        "RpcCore",
                        &format!(
                            "[SECURE] Ignoring stale response key={}, invoke_idx={}",
                            key,
                            response.invoke_idx()
                        ),
                    );
                }
                return Ok(());
            }

            let call_key = dynamic.iter().find_map(|(pending_key, pending)| {
                if pending_key.0 == key && matches!(pending, PendingCall::Call { .. }) {
                    Some(pending_key.clone())
                } else {
                    None
                }
            });
            if let Some(call_key) = call_key {
                self.logger.log(
                    crate::types::LogLevel::Info,
                    "RpcCore",
                    &format!(
                        "[RECV] Found pending call for key={}, sending response",
                        key
                    ),
                );
                if let Some(PendingCall::Call { tx }) = dynamic.remove(&call_key) {
                    let _ = tx.send(Ok(all_data));
                }
            } else {
                self.logger.log(
                    crate::types::LogLevel::Debug,
                    "RpcCore",
                    &format!("[RECV] No pending call found for key={} (response already consumed or unexpected ACK)", key),
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

pub fn format_hex_preview(data: &[u8], limit: usize) -> String {
    RpcCore::hex_bytes_with_limit(data, limit)
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
    use std::sync::Mutex as StdMutex;

    fn parse_frame(frame: &[u8]) -> ParseResult {
        let mut parser = FrameParser::new();
        parser
            .process(frame)
            .into_iter()
            .next()
            .expect("frame result")
            .expect("valid frame")
    }

    fn secure_control_frame(
        key: &str,
        invoke_idx: u8,
        response_type: u8,
        frame_idx: Option<u8>,
    ) -> Vec<u8> {
        secure_control_frame_with_data(key, invoke_idx, response_type, frame_idx, &[])
    }

    fn secure_control_frame_with_data(
        key: &str,
        invoke_idx: u8,
        response_type: u8,
        frame_idx: Option<u8>,
        response_data: &[u8],
    ) -> Vec<u8> {
        let mut data = match frame_idx {
            Some(frame_idx) => vec![0x19, response_type, 0x19, invoke_idx, 0x59, frame_idx],
            None => vec![0x19, response_type, 0x19, invoke_idx],
        };
        data.extend_from_slice(response_data);
        FrameBuilder::new().build_frame(key, &data, false, true, 0, 0)
    }

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
        let result = core.publish("test", "<u8*>", &[3, 0, 1, 2, 3]).await;
        assert_eq!(result, Err(RpcError::ChannelClosed));
    }

    #[test]
    fn secure_response_decodes_captured_frame_ack() {
        let response = SecureResponse::decode(&[0x19, 0x00, 0x19, 0x11, 0x59, 0x00]).unwrap();
        assert_eq!(
            response,
            SecureResponse::FrameAck {
                invoke_idx: 0x11,
                frame_idx: 0,
            }
        );
    }

    #[test]
    fn secure_response_decodes_captured_final_return() {
        let response = SecureResponse::decode(&[0x19, 0x02, 0x19, 0x10]).unwrap();
        assert_eq!(
            response,
            SecureResponse::Return {
                invoke_idx: 0x10,
                data: Vec::new(),
            }
        );
    }

    #[test]
    fn secure_response_decodes_remote_failures() {
        assert_eq!(
            SecureResponse::decode(&[0x19, 0x01, 0x19, 0x20]).unwrap(),
            SecureResponse::CommandNotFound { invoke_idx: 0x20 }
        );
        assert_eq!(
            SecureResponse::decode(&[0x19, 0x03, 0x19, 0x21]).unwrap(),
            SecureResponse::Error { invoke_idx: 0x21 }
        );
    }

    #[tokio::test]
    async fn secure_multi_frame_waits_for_ack_before_sending_final_frame() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 200,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let send_fn: SendFunction = Arc::new(move |data: Vec<u8>| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(data).map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        });
        core.set_send_function(send_fn).await;

        let values: Vec<u8> = (0..182u16).flat_map(|value| value.to_le_bytes()).collect();
        let mut params = Vec::with_capacity(values.len() + 2);
        params.extend_from_slice(&182u16.to_le_bytes());
        params.extend_from_slice(&values);

        let task_core = Arc::clone(&core);
        let task = tokio::spawn(async move {
            task_core
                .send("GH3X_RegsListWriteCmd", "<u16*>", &params)
                .await
        });

        let first_frame = timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect("first frame timeout")
            .expect("first frame");
        let first = parse_frame(&first_frame);
        assert!(first.is_secure);
        assert!(!first.is_fin);
        assert_eq!(first.frame_idx, 0);
        assert!(timeout(Duration::from_millis(30), rx.recv()).await.is_err());
        assert!(!task.is_finished());

        core.process(&secure_control_frame(
            "GH3X_RegsListWriteCmd",
            first.invoke_idx,
            0,
            Some(0),
        ))
        .await;

        let final_frame = timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect("final frame timeout")
            .expect("final frame");
        let final_parsed = parse_frame(&final_frame);
        assert!(final_parsed.is_secure);
        assert!(final_parsed.is_fin);
        assert_eq!(final_parsed.invoke_idx, first.invoke_idx);
        assert!(!task.is_finished());

        core.process(&secure_control_frame(
            "GH3X_RegsListWriteCmd",
            first.invoke_idx,
            2,
            None,
        ))
        .await;

        assert_eq!(task.await.unwrap(), Ok(()));
    }

    #[tokio::test]
    async fn secure_frame_ack_timeout_fails_the_command() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 20,
            retry_count: 1,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let send_fn: SendFunction = Arc::new(move |data: Vec<u8>| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(data).map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        });
        core.set_send_function(send_fn).await;

        let params = vec![0u8; 400];
        let task_core = Arc::clone(&core);
        let task =
            tokio::spawn(async move { task_core._secure_request("SEND", "large", &params).await });

        let first = rx.recv().await.unwrap();
        let retry = rx.recv().await.unwrap();
        assert_eq!(first, retry);
        assert_eq!(task.await.unwrap(), Err(RpcError::MaxRetryExceeded));
    }

    #[tokio::test]
    async fn secure_final_return_timeout_fails_the_command() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 20,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        let send_fn: SendFunction = Arc::new(move |data: Vec<u8>| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(data).map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        });
        core.set_send_function(send_fn).await;

        let task_core = Arc::clone(&core);
        let task = tokio::spawn(async move { task_core.send("small", "<u8>", &[1]).await });

        let frame = rx.recv().await.unwrap();
        assert!(parse_frame(&frame).is_fin);
        assert_eq!(task.await.unwrap(), Err(RpcError::MaxRetryExceeded));
    }

    #[tokio::test]
    async fn secure_mismatched_ack_does_not_advance_or_extend_timeout() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 40,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        core.set_send_function(Arc::new(move |data| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(data).map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        }))
        .await;

        let task_core = Arc::clone(&core);
        let task = tokio::spawn(async move {
            task_core
                ._secure_request("SEND", "large", &[0; 400])
                .await
        });
        let first = parse_frame(&rx.recv().await.unwrap());

        for _ in 0..3 {
            core.process(&secure_control_frame(
                "large",
                first.invoke_idx,
                0,
                Some(first.frame_idx.wrapping_add(1)),
            ))
            .await;
            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        assert!(timeout(Duration::from_millis(20), rx.recv()).await.is_err());
        assert_eq!(task.await.unwrap(), Err(RpcError::MaxRetryExceeded));
    }

    #[tokio::test]
    async fn sall_returns_final_type_two_payload() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 100,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let (tx, mut rx) = mpsc::unbounded_channel();
        core.set_send_function(Arc::new(move |data| {
            let tx = tx.clone();
            Box::pin(async move {
                tx.send(data).map_err(|_| RpcError::SendFail)?;
                Ok(())
            })
        }))
        .await;

        let task_core = Arc::clone(&core);
        let task = tokio::spawn(async move { task_core._sall("query", &[1]).await });
        let request = parse_frame(&rx.recv().await.unwrap());
        let payload = [0x59, 0x2A];
        core.process(&secure_control_frame_with_data(
            "query",
            request.invoke_idx,
            2,
            None,
            &payload,
        ))
        .await;

        assert_eq!(task.await.unwrap(), Ok(payload.to_vec()));
    }

    #[tokio::test]
    async fn secure_remote_failure_types_fail_the_live_request() {
        for (response_type, expected) in [
            (1, RpcError::CommandNotFound),
            (3, RpcError::RemoteError),
        ] {
            let core = Arc::new(RpcCore::new(RpcConfig {
                timeout_ms: 100,
                retry_count: 0,
                retry_delay_ms: 0,
                ..RpcConfig::default()
            }));
            let (tx, mut rx) = mpsc::unbounded_channel();
            core.set_send_function(Arc::new(move |data| {
                let tx = tx.clone();
                Box::pin(async move {
                    tx.send(data).map_err(|_| RpcError::SendFail)?;
                    Ok(())
                })
            }))
            .await;

            let task_core = Arc::clone(&core);
            let task = tokio::spawn(async move { task_core._send("failure", &[1]).await });
            let request = parse_frame(&rx.recv().await.unwrap());
            core.process(&secure_control_frame(
                "failure",
                request.invoke_idx,
                response_type,
                None,
            ))
            .await;

            assert_eq!(task.await.unwrap(), Err(expected));
        }
    }

    #[tokio::test]
    async fn publish_waits_for_each_transport_write_without_rx() {
        let core = RpcCore::default();
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let writes = Arc::new(AtomicUsize::new(0));
        let active_for_send = Arc::clone(&active);
        let max_for_send = Arc::clone(&max_active);
        let writes_for_send = Arc::clone(&writes);
        core.set_send_function(Arc::new(move |_data| {
            let active = Arc::clone(&active_for_send);
            let max_active = Arc::clone(&max_for_send);
            let writes = Arc::clone(&writes_for_send);
            Box::pin(async move {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(2)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                writes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }))
        .await;

        assert_eq!(core._publish("large", &[0; 500]).await, Ok(()));
        assert!(writes.load(Ordering::SeqCst) > 1);
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn call_sends_frames_sequentially_then_times_out_without_rx() {
        let core = RpcCore::new(RpcConfig {
            timeout_ms: 20,
            ..RpcConfig::default()
        });
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let writes = Arc::new(AtomicUsize::new(0));
        let active_for_send = Arc::clone(&active);
        let max_for_send = Arc::clone(&max_active);
        let writes_for_send = Arc::clone(&writes);
        core.set_send_function(Arc::new(move |_data| {
            let active = Arc::clone(&active_for_send);
            let max_active = Arc::clone(&max_for_send);
            let writes = Arc::clone(&writes_for_send);
            Box::pin(async move {
                let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                max_active.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(2)).await;
                active.fetch_sub(1, Ordering::SeqCst);
                writes.fetch_add(1, Ordering::SeqCst);
                Ok(())
            })
        }))
        .await;

        assert_eq!(core._call("large", &[0; 500]).await, Err(RpcError::Timeout));
        assert!(writes.load(Ordering::SeqCst) > 1);
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_commands_do_not_interleave_frame_batches() {
        let core = Arc::new(RpcCore::default());
        let sent_keys = Arc::new(StdMutex::new(Vec::new()));
        let sent_keys_for_send = Arc::clone(&sent_keys);
        core.set_send_function(Arc::new(move |data| {
            let sent_keys = Arc::clone(&sent_keys_for_send);
            Box::pin(async move {
                sent_keys.lock().unwrap().push(parse_frame(&data).key);
                tokio::time::sleep(Duration::from_millis(1)).await;
                Ok(())
            })
        }))
        .await;

        let first_core = Arc::clone(&core);
        let second_core = Arc::clone(&core);
        let (first_result, second_result) = tokio::join!(
            first_core._publish("first", &[0; 500]),
            second_core._publish("second", &[0; 500]),
        );

        assert_eq!(first_result, Ok(()));
        assert_eq!(second_result, Ok(()));
        let keys = sent_keys.lock().unwrap();
        let transitions = keys.windows(2).filter(|pair| pair[0] != pair[1]).count();
        assert!(keys.iter().any(|key| key == "first"));
        assert!(keys.iter().any(|key| key == "second"));
        assert_eq!(transitions, 1);
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
        assert!(buffer.frames.is_empty());
        assert_eq!(buffer.expected_frame_idx, 0);

        buffer.add_frame(0, 0, vec![7, 8, 9]).unwrap();
        assert_eq!(buffer.get_all_data(), vec![7, 8, 9]);
    }

    #[tokio::test]
    async fn reset_receive_state_discards_partial_frame() {
        let core = RpcCore::default();
        let stale = FrameBuilder::new().build_frame("G", &[0x10], false, true, 0, 0);
        let fresh = FrameBuilder::new().build_frame("G", &[0x20], false, true, 0, 0);

        assert!(core.process(&stale[..4]).await.is_empty());
        core.reset_receive_state().await;
        let results = core.process(&fresh).await;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_ref().unwrap().param, vec![0x20]);
    }

    #[tokio::test]
    async fn parser_error_clears_multi_frame_buffer() {
        let core = RpcCore::default();
        let first = FrameBuilder::new().build_frame("G", &[0x01], false, false, 0, 0);
        let mut invalid = FrameBuilder::new().build_frame("G", &[0x02], false, true, 0, 0);
        *invalid.last_mut().unwrap() ^= 0xFF;

        core.process(&first).await;
        assert_eq!(core.multi_frame_buffer.lock().await["G"].frames.len(), 1);
        let results = core.process(&invalid).await;

        assert!(matches!(results.first(), Some(Err(RpcError::CrcMismatch))));
        assert!(core.multi_frame_buffer.lock().await.is_empty());
    }

    #[tokio::test]
    async fn unsecure_final_frame_is_appended_to_multi_frame_sequence() {
        let core = RpcCore::default();
        let received = Arc::new(StdMutex::new(Vec::new()));
        let received_for_handler = Arc::clone(&received);
        core.register(
            "G",
            Arc::new(move |data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                *received_for_handler.lock().unwrap() = data.to_vec();
            }),
        )
        .await
        .unwrap();
        let payload: Vec<u8> = (0..300).map(|index| (index & 0xFF) as u8).collect();
        let frames = FrameBuilder::new().build_frames("G", &payload, false);

        for frame in frames {
            let results = core.process(&frame).await;
            assert!(results.iter().all(Result::is_ok));
        }

        assert_eq!(*received.lock().unwrap(), payload);
    }

    #[tokio::test]
    async fn interleaved_unsecure_keys_are_reassembled_independently() {
        let core = RpcCore::default();
        let first_received = Arc::new(StdMutex::new(Vec::new()));
        let second_received = Arc::new(StdMutex::new(Vec::new()));

        let first_for_handler = Arc::clone(&first_received);
        core.register(
            "telemetry_a",
            Arc::new(move |data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                *first_for_handler.lock().unwrap() = data.to_vec();
            }),
        )
        .await
        .unwrap();
        let second_for_handler = Arc::clone(&second_received);
        core.register(
            "telemetry_b",
            Arc::new(move |data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                *second_for_handler.lock().unwrap() = data.to_vec();
            }),
        )
        .await
        .unwrap();

        let mut builder = FrameBuilder::new();
        let first_start = builder.build_frame("telemetry_a", &[0xA0], false, false, 0, 0);
        let second_start = builder.build_frame("telemetry_b", &[0xB0], false, false, 0, 0);
        let first_end = builder.build_frame("telemetry_a", &[0xA1], false, true, 0, 0);
        let second_end = builder.build_frame("telemetry_b", &[0xB1], false, true, 0, 0);

        for frame in [first_start, second_start, first_end, second_end] {
            let results = core.process(&frame).await;
            assert!(results.iter().all(Result::is_ok));
        }

        assert_eq!(*first_received.lock().unwrap(), vec![0xA0, 0xA1]);
        assert_eq!(*second_received.lock().unwrap(), vec![0xB0, 0xB1]);
    }

    #[tokio::test]
    async fn telemetry_does_not_drop_an_interleaved_call_response() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 100,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let telemetry_received = Arc::new(StdMutex::new(Vec::new()));
        let telemetry_for_handler = Arc::clone(&telemetry_received);
        core.register(
            "telemetry",
            Arc::new(move |data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                *telemetry_for_handler.lock().unwrap() = data.to_vec();
            }),
        )
        .await
        .unwrap();

        let (response_tx, response_rx) = oneshot::channel();
        core.dynamic_nodes.lock().await.insert(
            (String::from("call"), 7),
            PendingCall::Call { tx: response_tx },
        );

        let mut builder = FrameBuilder::new();
        let telemetry_start = builder.build_frame("telemetry", &[0x10], false, false, 0, 0);
        let call_response = builder.build_frame("call", &[0xCC], false, true, 0, 0);
        let telemetry_end = builder.build_frame("telemetry", &[0x11], false, true, 0, 0);

        core.process(&telemetry_start).await;
        let call_results = core.process(&call_response).await;
        assert_eq!(call_results[0].as_ref().unwrap().key, "call");
        core.process(&telemetry_end).await;

        assert_eq!(
            timeout(Duration::from_millis(20), response_rx)
                .await
                .expect("call response timeout")
                .unwrap(),
            Ok(vec![0xCC])
        );
        assert_eq!(*telemetry_received.lock().unwrap(), vec![0x10, 0x11]);
    }

    #[tokio::test]
    async fn telemetry_does_not_drop_an_interleaved_secure_return() {
        let core = Arc::new(RpcCore::new(RpcConfig {
            timeout_ms: 100,
            retry_count: 0,
            retry_delay_ms: 0,
            ..RpcConfig::default()
        }));
        let telemetry_received = Arc::new(StdMutex::new(Vec::new()));
        let telemetry_for_handler = Arc::clone(&telemetry_received);
        core.register(
            "telemetry",
            Arc::new(move |data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                *telemetry_for_handler.lock().unwrap() = data.to_vec();
            }),
        )
        .await
        .unwrap();

        let invoke_idx = 7;
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();
        core.dynamic_nodes.lock().await.insert(
            (String::from("secure"), invoke_idx),
            PendingCall::Secure { tx: response_tx },
        );

        let mut builder = FrameBuilder::new();
        let telemetry_start = builder.build_frame("telemetry", &[0x20], false, false, 0, 0);
        let secure_return = secure_control_frame_with_data(
            "secure",
            invoke_idx,
            2,
            None,
            &[0x59, 0x2A],
        );
        let telemetry_end = builder.build_frame("telemetry", &[0x21], false, true, 0, 0);

        core.process(&telemetry_start).await;
        let secure_results = core.process(&secure_return).await;
        assert_eq!(secure_results[0].as_ref().unwrap().key, "secure");
        core.process(&telemetry_end).await;

        assert_eq!(
            timeout(Duration::from_millis(20), response_rx.recv())
                .await
                .expect("secure response timeout"),
            Some(SecureResponse::Return {
                invoke_idx,
                data: vec![0x59, 0x2A],
            })
        );
        assert_eq!(*telemetry_received.lock().unwrap(), vec![0x20, 0x21]);
    }

    #[tokio::test]
    async fn test_process_frame() {
        let core = RpcCore::default();
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();

        let handler = Arc::new(
            move |_data: &[u8], _size: usize, _ctx: &mut InvokeContext| {
                count_clone.fetch_add(1, Ordering::SeqCst);
            },
        );

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

    #[test]
    fn test_format_hex_preview_truncates() {
        let data = [0xAA, 0x11, 0x22, 0x33];
        assert_eq!(format_hex_preview(&data, 3), "AA 11 22 ...(+1 bytes)");
    }
}
