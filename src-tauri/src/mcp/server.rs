//! MCP Server 定义与工具注册。
//!
//! P0 仅注册 `combridge_about`。后续阶段在此追加各域工具
//! （serial / ble / devices / gh3036 / waveform / protocol / ota），
//! 结构与命名约定见 `docs/mcp-server-plan.md` §5、§12.3。

use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::handler::server::wrapper::Json;
use rmcp::model::{Implementation, ServerCapabilities, ServerConfig};
use rmcp::{schemars, tool, tool_handler, tool_router};
use serde::Serialize;
use std::sync::Arc;

use super::ctx::McpContext;

/// `combridge_about` 的返回结构。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AboutResponse {
    /// ComBridge 应用版本
    pub app_version: String,
    /// 安全档位："dev"（研发档）| "standard"（标准档）
    pub tier: String,
    /// 服务启动时间（毫秒时间戳）
    pub started_at: u64,
    /// 当前已连接设备快照
    pub connected_devices: Vec<serde_json::Value>,
    /// 后续阶段将注册的工具域（P0 仅预告，调用无效）
    pub planned_tool_domains: Vec<String>,
}

#[derive(Clone)]
pub struct ComBridgeMcp {
    ctx: Arc<McpContext>,
}

impl ComBridgeMcp {
    pub fn new(ctx: Arc<McpContext>) -> Self {
        Self { ctx }
    }
}

#[tool_router]
impl ComBridgeMcp {
    #[tool(
        description = "获取 ComBridge 服务概况：应用版本、安全档位、已连接设备快照与可用工具域。\
首次连接时调用一次即可；设备状态变化后可重新调用获取最新快照。\
返回 JSON：{ app_version, tier, started_at, connected_devices[], planned_tool_domains[] }。",
        annotations(read_only_hint = true)
    )]
    async fn about(&self) -> Result<Json<AboutResponse>, McpError> {
        let connected_devices = self.ctx.connected_devices().await;
        Ok(Json(AboutResponse {
            app_version: self.ctx.app_version.clone(),
            tier: self.ctx.tier.clone(),
            started_at: self.ctx.started_at,
            connected_devices,
            planned_tool_domains: vec![
                "serial",
                "ble",
                "devices",
                "gh3036",
                "waveform",
                "protocol",
                "ota",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        }))
    }
}

#[tool_handler]
impl ServerHandler for ComBridgeMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(
                "ComBridge 串口/BLE/GH3036 调试工具的 MCP 服务。\
当前为 P0 骨架，仅提供 combridge_about；串口、BLE、GH3036 产测、OTA 等域级工具将按计划分阶段注册。",
            )
            .with_server_info(
                Implementation::new("combridge", env!("CARGO_PKG_VERSION"))
                    .with_title("ComBridge")
                    .with_description("ComBridge cross-platform serial/BLE debugging tool"),
            )
    }
}
