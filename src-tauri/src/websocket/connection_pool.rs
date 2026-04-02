use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::error::{ComBridgeError, Result};

use super::client::{WebSocketClient, WebSocketConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Disconnected,
    Connected,
    Connecting,
    Reconnecting,
}

impl From<super::client::ConnectionState> for ConnectionStatus {
    fn from(state: super::client::ConnectionState) -> Self {
        match state {
            super::client::ConnectionState::Disconnected => ConnectionStatus::Disconnected,
            super::client::ConnectionState::Connected => ConnectionStatus::Connected,
            super::client::ConnectionState::Connecting => ConnectionStatus::Connecting,
            super::client::ConnectionState::Reconnecting => ConnectionStatus::Reconnecting,
        }
    }
}

pub struct ConnectionPool {
    connections: RwLock<HashMap<String, Arc<WebSocketClient>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_connection(
        &self,
        id: &str,
        config: WebSocketConfig,
    ) -> Result<Arc<WebSocketClient>> {
        let mut connections = self.connections.write().await;

        if connections.contains_key(id) {
            return Err(ComBridgeError::websocket(format!(
                "连接已存在: {}",
                id
            )));
        }

        let client = Arc::new(WebSocketClient::new(config));
        connections.insert(id.to_string(), client.clone());

        info!("创建连接: {}", id);
        Ok(client)
    }

    pub async fn get_connection(&self, id: &str) -> Option<Arc<WebSocketClient>> {
        let connections = self.connections.read().await;
        connections.get(id).cloned()
    }

    pub async fn remove_connection(&self, id: &str) -> Result<()> {
        let mut connections = self.connections.write().await;

        if let Some(client) = connections.remove(id) {
            client.disconnect().await?;
            info!("移除连接: {}", id);
        }

        Ok(())
    }

    pub async fn get_all_connections(&self) -> Vec<String> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }

    pub async fn get_connection_status(&self, id: &str) -> Option<ConnectionStatus> {
        let connections = self.connections.read().await;
        if let Some(client) = connections.get(id) {
            let state = client.get_state().await;
            Some(ConnectionStatus::from(state))
        } else {
            None
        }
    }

    pub async fn get_all_status(&self) -> HashMap<String, ConnectionStatus> {
        let connections = self.connections.read().await;
        let mut status = HashMap::new();

        for (id, client) in connections.iter() {
            let state = client.get_state().await;
            status.insert(id.clone(), ConnectionStatus::from(state));
        }

        status
    }

    pub async fn disconnect_all(&self) -> Result<()> {
        let connections = self.connections.read().await;

        for (id, client) in connections.iter() {
            if let Err(e) = client.disconnect().await {
                error!("断开连接失败 {}: {}", id, e);
            }
        }

        info!("断开所有连接");
        Ok(())
    }

    pub async fn clear(&self) -> Result<()> {
        self.disconnect_all().await?;

        let mut connections = self.connections.write().await;
        connections.clear();

        info!("清空连接池");
        Ok(())
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
