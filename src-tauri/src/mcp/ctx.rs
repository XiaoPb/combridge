//! MCP 运行上下文：持有各业务 Manager 的句柄。
//!
//! 工具实现通过 ctx 访问硬件与状态，**直接调 Manager，不走 `#[tauri::command]` 层**
//! （command 层的 `State<'_, T>` 无法在 MCP handler 内构造，且 DTO 命名面向前端）。

use std::sync::Arc;

use serde_json::Value;
use tracing::warn;

use crate::device::{BleManagerRef, SerialManagerRef};
use crate::service::EventBus;
use crate::state::AppStateRef;

pub struct McpContext {
    /// ComBridge 应用版本
    pub app_version: String,
    /// 安全档位："dev" | "standard"
    pub tier: String,
    /// 服务启动时间（毫秒时间戳）
    pub started_at: u64,
    pub serial: SerialManagerRef,
    pub ble: BleManagerRef,
    pub app_state: AppStateRef,
    pub event_bus: Arc<EventBus>,
}

impl McpContext {
    /// 从 Tauri AppHandle 提取各 Manager。必须在 `setup` 之后调用。
    pub fn from_app(app: &tauri::AppHandle, tier: String) -> Self {
        use tauri::Manager;

        let serial = app.state::<SerialManagerRef>().inner().clone();
        let ble = app.state::<BleManagerRef>().inner().clone();
        let app_state = app.state::<AppStateRef>().inner().clone();
        let event_bus = app.state::<Arc<EventBus>>().inner().clone();

        Self {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            tier,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            serial,
            ble,
            app_state,
            event_bus,
        }
    }

    /// 当前已连接设备快照（序列化为 JSON，避免依赖内部 Device 类型字段）。
    pub async fn connected_devices(&self) -> Vec<Value> {
        let state = self.app_state.read().await;
        state
            .get_connected_devices()
            .into_iter()
            .filter_map(|d| match serde_json::to_value(d) {
                Ok(v) => Some(v),
                Err(e) => {
                    warn!("[MCP] 设备序列化失败: {}", e);
                    None
                }
            })
            .collect()
    }
}
