pub mod client;
pub mod connection_pool;
pub mod message_handler;
pub mod reconnection;

pub use client::{WebSocketClient, WebSocketConfig};
pub use connection_pool::{ConnectionPool, ConnectionStatus};
pub use message_handler::{MessageHandler, RequestMessage, ResponseMessage};
pub use reconnection::ReconnectionStrategy;

use std::sync::Arc;

pub type WebSocketClientRef = Arc<WebSocketClient>;
pub type ConnectionPoolRef = Arc<ConnectionPool>;
