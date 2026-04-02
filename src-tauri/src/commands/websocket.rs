use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::error::{ComBridgeError, Result};
use crate::websocket::{
    ConnectionPoolRef, ConnectionStatus, WebSocketConfig,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConnectionConfig {
    pub id: String,
    pub url: String,
    pub reconnect: Option<bool>,
    pub reconnect_interval_ms: Option<u64>,
    pub max_reconnect_attempts: Option<u32>,
    pub heartbeat_interval_ms: Option<u64>,
    pub connection_timeout_ms: Option<u64>,
}

impl From<WebSocketConnectionConfig> for WebSocketConfig {
    fn from(config: WebSocketConnectionConfig) -> Self {
        WebSocketConfig {
            url: config.url,
            reconnect: config.reconnect.unwrap_or(true),
            reconnect_interval_ms: config.reconnect_interval_ms.unwrap_or(5000),
            max_reconnect_attempts: config.max_reconnect_attempts.unwrap_or(10),
            heartbeat_interval_ms: config.heartbeat_interval_ms.unwrap_or(30000),
            connection_timeout_ms: config.connection_timeout_ms.unwrap_or(10000),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSocketStatusEvent {
    pub id: String,
    pub status: ConnectionStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSocketMessageEvent {
    pub id: String,
    pub message: String,
}

#[tauri::command]
pub async fn connect_websocket(
    pool: State<'_, ConnectionPoolRef>,
    app: AppHandle,
    config: WebSocketConnectionConfig,
) -> Result<String> {
    let id = config.id.clone();
    let ws_config: WebSocketConfig = config.into();

    let client = pool.create_connection(&id, ws_config).await?;

    let app_clone = app.clone();
    let id_clone = id.clone();
    let client_clone = client.clone();

    tokio::spawn(async move {
        loop {
            let status = client_clone.get_state().await;
            let event = WebSocketStatusEvent {
                id: id_clone.clone(),
                status: ConnectionStatus::from(status),
            };

            if let Err(e) = app_clone.emit("websocket-status", &event) {
                tracing::error!("发送状态事件失败: {}", e);
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });

    client.connect().await?;

    Ok(id)
}

#[tauri::command]
pub async fn send_websocket_message(
    pool: State<'_, ConnectionPoolRef>,
    id: String,
    message: String,
) -> Result<()> {
    let client = pool
        .get_connection(&id)
        .await
        .ok_or_else(|| ComBridgeError::websocket(format!("连接不存在: {}", id)))?;

    client.send_message(&message).await
}

#[tauri::command]
pub async fn disconnect_websocket(
    pool: State<'_, ConnectionPoolRef>,
    id: String,
) -> Result<()> {
    pool.remove_connection(&id).await
}

#[tauri::command]
pub async fn get_websocket_status(
    pool: State<'_, ConnectionPoolRef>,
    id: String,
) -> Result<Option<ConnectionStatus>> {
    Ok(pool.get_connection_status(&id).await)
}

#[tauri::command]
pub async fn get_all_websocket_connections(
    pool: State<'_, ConnectionPoolRef>,
) -> Result<Vec<String>> {
    Ok(pool.get_all_connections().await)
}

#[tauri::command]
pub async fn get_all_websocket_status(
    pool: State<'_, ConnectionPoolRef>,
) -> Result<std::collections::HashMap<String, ConnectionStatus>> {
    Ok(pool.get_all_status().await)
}
