//! MCP Server 模块（P0 骨架）
//!
//! 通过 Streamable HTTP 对外释放 ComBridge 能力，供 AI 客户端调用。
//! 设计文档见 `docs/mcp-server-plan.md`。
//!
//! P0 范围：
//! - 仅绑定 127.0.0.1，默认端口 47821（被占用时回退随机端口）
//! - Bearer Token 认证，每次启动轮换
//! - endpoint 发现文件 `%LOCALAPPDATA%\combridge\mcp-endpoint.json`
//! - `combridge_about` 单工具打通握手
//!
//! 默认关闭，需在 `%LOCALAPPDATA%\combridge\mcp.json` 中显式启用。

pub mod auth;
pub mod ble_hub;
pub mod ctx;
pub mod lease;
pub mod serial_hub;
pub mod server;

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{error, info, warn};

/// 默认监听端口（仅 loopback）
pub const DEFAULT_PORT: u16 = 47821;

/// MCP 配置，落盘于 `%LOCALAPPDATA%\combridge\mcp.json`。
/// 文件不存在时使用默认值（关闭状态）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct McpSettings {
    /// 是否启用 MCP Server（默认关闭）
    pub enabled: bool,
    /// 监听端口
    pub port: u16,
    /// 安全档位："dev"（研发档）| "standard"（标准档）
    pub tier: String,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
            tier: "standard".to_string(),
        }
    }
}

impl McpSettings {
    fn config_dir() -> Option<std::path::PathBuf> {
        dirs::data_local_dir().map(|d| d.join("combridge"))
    }

    pub fn config_path() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|d| d.join("mcp.json"))
    }

    fn endpoint_path() -> Option<std::path::PathBuf> {
        Self::config_dir().map(|d| d.join("mcp-endpoint.json"))
    }

    /// 读取配置；文件缺失或解析失败时返回默认值（关闭）。
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => settings,
                Err(e) => {
                    warn!("MCP 配置解析失败，使用默认值（关闭）: {} ({})", path.display(), e);
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

/// 启动成功后的信息
pub struct McpStartInfo {
    pub port: u16,
    pub token: String,
    pub tier: String,
}

/// 当前 Unix 时间戳（毫秒）
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 生成 64 位十六进制字符的随机 token（两次 UUID v4 拼接 = 32 字节熵）
fn generate_token() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

/// 绑定 loopback 监听器；指定端口被占用时回退随机端口。
async fn bind_listener(preferred_port: u16) -> Result<tokio::net::TcpListener, String> {
    let addr = format!("127.0.0.1:{preferred_port}");
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => Ok(listener),
        Err(e) => {
            warn!("端口 {preferred_port} 绑定失败（{e}），回退随机端口");
            tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .map_err(|e| format!("绑定随机端口也失败: {e}"))
        }
    }
}

/// 写入 endpoint 发现文件，供客户端 / 设置页读取。
fn write_endpoint_file(
    port: u16,
    token: &str,
    tier: &str,
) -> Result<std::path::PathBuf, String> {
    let Some(path) = McpSettings::endpoint_path() else {
        return Err("无法确定数据目录".to_string());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let endpoint = serde_json::json!({
        "url": format!("http://127.0.0.1:{port}/mcp"),
        "token": token,
        "pid": std::process::id(),
        "app_version": env!("CARGO_PKG_VERSION"),
        "tier": tier,
        "started_at": now_millis(),
    });
    let content = serde_json::to_string_pretty(&endpoint)
        .map_err(|e| format!("endpoint 序列化失败: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("写入 endpoint 失败: {e}"))?;
    Ok(path)
}

/// 启动 MCP Server（应在 tokio 运行时内调用；`app` 用于提取各业务 Manager）。
///
/// 失败仅影响 MCP 功能本身，不影响 App 主流程 —— 调用方需吞掉错误只记日志。
pub async fn start(app: tauri::AppHandle, settings: McpSettings) -> Result<McpStartInfo, String> {
    if !settings.enabled {
        return Err("MCP 未启用".to_string());
    }
    if settings.tier != "dev" && settings.tier != "standard" {
        return Err(format!("未知的档位: {}", settings.tier));
    }

    let token = generate_token();
    let tier = settings.tier.clone();
    let ctx = Arc::new(ctx::McpContext::from_app(&app, tier.clone()));
    // 订阅 EventBus 广播：串口数据 → hub（游标/等待），端口断开 → 清理
    ctx.serial_hub.spawn_subscription(Arc::clone(&ctx.event_bus));
    // BLE 通知数据 → hub（按 地址+特征 分桶）
    ctx.ble_hub.spawn_subscription(Arc::clone(&ctx.event_bus));
    let mcp_server = server::ComBridgeMcp::new(ctx);

    let service =
        rmcp::transport::streamable_http_server::StreamableHttpService::<
            server::ComBridgeMcp,
            rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
        >::new(
            move || Ok(mcp_server.clone()),
            // LocalSessionManager 默认实现即可；协议 2026-07-28 起为无状态模式
            Default::default(),
            // 默认 allowed_hosts 仅允许 loopback，防 DNS rebinding
            Default::default(),
        );

    let router = axum::Router::new()
        .nest_service("/mcp", service)
        .layer(axum::middleware::from_fn_with_state(
            token.clone(),
            auth::bearer_auth,
        ));

    let listener = bind_listener(settings.port).await?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("获取监听地址失败: {e}"))?
        .port();

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            error!("[MCP] HTTP 服务异常退出: {}", e);
        }
    });

    let endpoint_path = write_endpoint_file(port, &token, &tier)?;

    info!(
        "[MCP] 已启动: http://127.0.0.1:{}/mcp (tier={}, endpoint 文件: {})",
        port,
        tier,
        endpoint_path.display()
    );

    Ok(McpStartInfo {
        port,
        token,
        tier,
    })
}
