use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info};

use crate::error::{ComBridgeError, Result};

use super::message_handler::MessageHandler;
use super::reconnection::ReconnectionStrategy;

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub url: String,
    pub reconnect: bool,
    pub reconnect_interval_ms: u64,
    pub max_reconnect_attempts: u32,
    pub heartbeat_interval_ms: u64,
    pub connection_timeout_ms: u64,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            reconnect: true,
            reconnect_interval_ms: 5000,
            max_reconnect_attempts: 10,
            heartbeat_interval_ms: 30000,
            connection_timeout_ms: 10000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting,
}

pub struct WebSocketClient {
    config: WebSocketConfig,
    state: Arc<RwLock<ConnectionState>>,
    sender: mpsc::UnboundedSender<String>,
    message_handler: Arc<MessageHandler>,
    reconnection_strategy: ReconnectionStrategy,
}

impl WebSocketClient {
    pub fn new(config: WebSocketConfig) -> Self {
        let (sender, mut receiver) = mpsc::unbounded_channel::<String>();
        let state = Arc::new(RwLock::new(ConnectionState::Disconnected));
        let message_handler = Arc::new(MessageHandler::new());
        let reconnection_strategy = ReconnectionStrategy::new(
            config.reconnect_interval_ms,
            config.max_reconnect_attempts,
        );

        let client = Self {
            config,
            state,
            sender,
            message_handler,
            reconnection_strategy,
        };

        let state_clone = client.state.clone();

        tokio::spawn(async move {
            loop {
                if let Some(_msg) = receiver.recv().await {
                    if let Err(e) = Self::handle_outgoing_message(&state_clone).await {
                        error!("发送消息失败: {}", e);
                    }
                }
            }
        });

        client
    }

    pub async fn connect(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if *state == ConnectionState::Connected {
            return Ok(());
        }

        *state = ConnectionState::Connecting;
        drop(state);

        self.do_connect().await
    }

    async fn do_connect(&self) -> Result<()> {
        let url = self.config.url.clone();
        let timeout_duration = Duration::from_millis(self.config.connection_timeout_ms);

        info!("正在连接 WebSocket: {}", url);

        let connect_future = connect_async(&url);
        let result = tokio::time::timeout(timeout_duration, connect_future).await;

        match result {
            Ok(Ok((ws_stream, _))) => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Connected;
                info!("WebSocket 连接成功: {}", url);

                let (_ws_sender, mut ws_receiver) = ws_stream.split();

                let state_clone = self.state.clone();
                let handler_clone = self.message_handler.clone();
                let config_clone = self.config.clone();
                let reconnection_clone = self.reconnection_strategy.clone();

                tokio::spawn(async move {
                    while let Some(msg_result) = ws_receiver.next().await {
                        match msg_result {
                            Ok(WsMessage::Text(text)) => {
                                debug!("收到消息: {}", text);
                                if let Err(e) = handler_clone.handle_message(&text).await {
                                    error!("处理消息失败: {}", e);
                                }
                            }
                            Ok(WsMessage::Ping(_data)) => {
                                debug!("收到 Ping 消息");
                            }
                            Ok(WsMessage::Pong(_)) => {
                                debug!("收到 Pong 消息");
                            }
                            Ok(WsMessage::Close(_)) => {
                                info!("收到关闭消息");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket 错误: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }

                    let mut state = state_clone.write().await;
                    *state = ConnectionState::Disconnected;

                    if config_clone.reconnect {
                        drop(state);
                        if let Err(e) = Self::reconnect(&state_clone, &config_clone, &reconnection_clone)
                            .await
                        {
                            error!("重连失败: {}", e);
                        }
                    }
                });

                Ok(())
            }
            Ok(Err(e)) => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Disconnected;
                error!("WebSocket 连接失败: {}", e);
                Err(ComBridgeError::websocket(format!("连接失败: {}", e)))
            }
            Err(_) => {
                let mut state = self.state.write().await;
                *state = ConnectionState::Disconnected;
                error!("WebSocket 连接超时");
                Err(ComBridgeError::websocket("连接超时"))
            }
        }
    }

    async fn reconnect(
        state: &Arc<RwLock<ConnectionState>>,
        config: &WebSocketConfig,
        strategy: &ReconnectionStrategy,
    ) -> Result<()> {
        let mut attempts = 0;

        loop {
            attempts += 1;
            let delay = strategy.get_delay(attempts);

            info!("将在 {}ms 后尝试第 {} 次重连", delay, attempts);
            tokio::time::sleep(Duration::from_millis(delay)).await;

            let mut state_guard = state.write().await;
            *state_guard = ConnectionState::Reconnecting;
            drop(state_guard);

            let connect_future = connect_async(&config.url);
            let timeout_duration = Duration::from_millis(config.connection_timeout_ms);

            match tokio::time::timeout(timeout_duration, connect_future).await {
                Ok(Ok((_ws_stream, _))) => {
                    let mut state_guard = state.write().await;
                    *state_guard = ConnectionState::Connected;
                    info!("重连成功");
                    return Ok(());
                }
                _ => {
                    if attempts >= strategy.max_attempts {
                        error!("达到最大重连次数，停止重连");
                        let mut state_guard = state.write().await;
                        *state_guard = ConnectionState::Disconnected;
                        return Err(ComBridgeError::websocket("达到最大重连次数"));
                    }
                }
            }
        }
    }

    async fn handle_outgoing_message(state: &Arc<RwLock<ConnectionState>>) -> Result<()> {
        let current_state = *state.read().await;
        if current_state != ConnectionState::Connected {
            return Err(ComBridgeError::websocket("未连接"));
        }

        Ok(())
    }

    pub async fn disconnect(&self) -> Result<()> {
        let mut state = self.state.write().await;
        *state = ConnectionState::Disconnected;
        info!("WebSocket 已断开");
        Ok(())
    }

    pub async fn send_message(&self, message: &str) -> Result<()> {
        let state = self.state.read().await;
        if *state != ConnectionState::Connected {
            return Err(ComBridgeError::websocket("未连接"));
        }

        self.sender
            .send(message.to_string())
            .map_err(|e| ComBridgeError::websocket(format!("发送失败: {}", e)))?;

        Ok(())
    }

    pub async fn get_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    pub fn get_message_handler(&self) -> Arc<MessageHandler> {
        self.message_handler.clone()
    }
}
