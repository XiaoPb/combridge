use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use crate::error::{ComBridgeError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMessage {
    pub id: Option<String>,
    pub method: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub id: Option<String>,
    pub result: Option<serde_json::Value>,
    pub error: Option<ErrorInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: i32,
    pub message: String,
}

pub type MessageCallback = Arc<dyn Fn(&str, serde_json::Value) + Send + Sync>;

pub struct MessageHandler {
    callbacks: RwLock<HashMap<String, MessageCallback>>,
    pending_requests: RwLock<HashMap<String, tokio::sync::oneshot::Sender<ResponseMessage>>>,
}

impl MessageHandler {
    pub fn new() -> Self {
        Self {
            callbacks: RwLock::new(HashMap::new()),
            pending_requests: RwLock::new(HashMap::new()),
        }
    }

    pub async fn handle_message(&self, message: &str) -> Result<()> {
        debug!("处理消息: {}", message);

        let response: ResponseMessage = serde_json::from_str(message)?;

        if let Some(id) = &response.id.clone() {
            let mut pending = self.pending_requests.write().await;
            if let Some(sender) = pending.remove(id) {
                if sender.send(response).is_err() {
                    error!("发送响应到等待队列失败: {}", id);
                }
                return Ok(());
            }
        }

        let request: std::result::Result<RequestMessage, _> = serde_json::from_str(message);
        if let Ok(req) = request {
            self.handle_request(req).await?;
        }

        Ok(())
    }

    async fn handle_request(&self, request: RequestMessage) -> Result<()> {
        info!("处理请求: method={}, id={:?}", request.method, request.id);

        let callbacks = self.callbacks.read().await;
        if let Some(callback) = callbacks.get(&request.method) {
            callback(&request.method, request.params);
        } else {
            debug!("未找到回调: {}", request.method);
        }

        Ok(())
    }

    pub async fn register_callback<F>(&self, method: &str, callback: F)
    where
        F: Fn(&str, serde_json::Value) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.write().await;
        callbacks.insert(method.to_string(), Arc::new(callback));
        info!("注册回调: {}", method);
    }

    pub async fn unregister_callback(&self, method: &str) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.remove(method);
        info!("注销回调: {}", method);
    }

    pub async fn create_request(&self, method: &str, params: serde_json::Value) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();

        let request = RequestMessage {
            id: Some(id.clone()),
            method: method.to_string(),
            params,
        };

        serde_json::to_string(&request).map_err(ComBridgeError::from)
    }

    pub async fn wait_for_response(
        &self,
        id: &str,
        timeout_ms: u64,
    ) -> Result<ResponseMessage> {
        let (sender, receiver) = tokio::sync::oneshot::channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id.to_string(), sender);
        }

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            receiver,
        )
        .await;

        match result {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(ComBridgeError::websocket("响应通道已关闭")),
            Err(_) => {
                let mut pending = self.pending_requests.write().await;
                pending.remove(id);
                Err(ComBridgeError::websocket("等待响应超时"))
            }
        }
    }
}

impl Default for MessageHandler {
    fn default() -> Self {
        Self::new()
    }
}
