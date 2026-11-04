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

pub mod audit;
pub mod auth;
pub mod ble_hub;
pub mod ctx;
pub mod elicit;
pub mod gh3036_job;
pub mod lease;
pub mod logs;
pub mod ota_confirm;
pub mod ratelimit;
pub mod serial_hub;
pub mod server;

use std::sync::{Arc, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
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

    /// 数据目录（`%LOCALAPPDATA%\combridge`），设置页展示用。
    pub fn data_dir() -> Option<std::path::PathBuf> {
        Self::config_dir()
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

    /// 落盘配置。设置页改动后立刻写，保证下次启动与当前行为一致。
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path().ok_or("无法确定数据目录")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("配置序列化失败: {e}"))?;
        std::fs::write(&path, content).map_err(|e| format!("写入配置失败: {e}"))
    }

    /// 档位取值校验（设置页传入的字符串不可信）。
    pub fn normalized_tier(tier: &str) -> Option<&'static str> {
        match tier {
            "dev" => Some("dev"),
            "standard" => Some("standard"),
            _ => None,
        }
    }
}

/// 启动成功后的信息
#[derive(Debug, Clone)]
pub struct McpStartInfo {
    pub port: u16,
    pub token: String,
    pub tier: String,
    /// 请求端口与实际端口不一致（端口被占用后回退）
    pub port_fallback: bool,
}

/// 进程内唯一的运行实例。
///
/// 保存 shutdown 句柄与后台任务，是为了让设置页的开关能**当场生效** ——
/// 没有它就只能写配置、提示"请重启"，那对一个要被终端客户操作的开关太糙了。
struct RunningServer {
    port: u16,
    token: String,
    started_at: u64,
    port_fallback: bool,
    ctx: Arc<ctx::McpContext>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    /// 服务主任务（优雅关闭超时后强杀）
    serve_task: Option<tokio::task::JoinHandle<()>>,
    /// EventBus 订阅任务，必须 abort，否则每重启一次就多一个消费者
    subscriptions: Vec<tokio::task::JoinHandle<()>>,
}

static RUNTIME: OnceLock<Mutex<Option<RunningServer>>> = OnceLock::new();

fn runtime() -> &'static Mutex<Option<RunningServer>> {
    RUNTIME.get_or_init(|| Mutex::new(None))
}

/// 档位共享 cell：重启后仍复用同一份，切档位时才不用重建服务。
static TIER: OnceLock<Arc<std::sync::RwLock<String>>> = OnceLock::new();

fn tier_cell() -> &'static Arc<std::sync::RwLock<String>> {
    TIER.get_or_init(|| Arc::new(std::sync::RwLock::new(McpSettings::default().tier)))
}

/// 当前档位。
pub fn current_tier() -> String {
    tier_cell().read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// 热切换档位：改共享 cell + 给限流器换速率，**不重启服务、不换 token**。
///
/// 这是设置页最常被点的开关（研发档 ↔ 标准档），走重启会让所有已配置的客户端失效。
pub async fn switch_tier(tier: &str) {
    if McpSettings::normalized_tier(tier).is_none() {
        return;
    }
    {
        let mut guard = tier_cell().write().unwrap_or_else(|e| e.into_inner());
        if *guard == tier {
            return;
        }
        *guard = tier.to_string();
    }
    let guard = runtime().lock().await;
    if let Some(server) = guard.as_ref() {
        server.ctx.limiter.retune(tier);
    }
    info!("[MCP] 档位切换为 {tier}（未重启服务）");
}

/// 运行期快照（供设置页与命令层读取）。
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    pub port: u16,
    pub token: String,
    pub tier: String,
    pub started_at: u64,
    pub port_fallback: bool,
}

/// 当前是否在运行（含实际端口 / token / 档位）。
pub async fn runtime_info() -> Option<RuntimeInfo> {
    let guard = runtime().lock().await;
    guard.as_ref().map(|s| RuntimeInfo {
        port: s.port,
        token: s.token.clone(),
        tier: current_tier(),
        started_at: s.started_at,
        port_fallback: s.port_fallback,
    })
}

/// 已注册的工具数量（不需要构造 `McpContext`，直接问路由表）。
pub fn tool_count() -> usize {
    server::ComBridgeMcp::tool_router().list_all().len()
}

/// 停止服务。返回是否真的停了一个正在跑的服务。
pub async fn stop() -> bool {
    let taken = {
        let mut guard = runtime().lock().await;
        guard.take()
    };
    let Some(server) = taken else {
        return false;
    };
    shutdown(server).await;
    remove_endpoint_file();
    info!("[MCP] 已停止");
    true
}

/// 关闭一个运行实例：先优雅停服，超时再硬杀；订阅任务直接 abort。
async fn shutdown(server: RunningServer) {
    if let Some(tx) = server.shutdown {
        let _ = tx.send(());
    }
    for task in server.subscriptions {
        task.abort();
    }
    if let Some(task) = server.serve_task {
        // SSE 长连接会让优雅关闭一直等下去，1.5s 后直接 abort
        let abort = task.abort_handle();
        if tokio::time::timeout(std::time::Duration::from_millis(1500), task)
            .await
            .is_err()
        {
            abort.abort();
        }
    }
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

/// 停服后删除 endpoint 文件 —— 留着等于对外发一个已经失效的 token。
fn remove_endpoint_file() {
    if let Some(path) = McpSettings::endpoint_path() {
        let _ = std::fs::remove_file(path);
    }
}

/// 启动时清理上一次运行残留的 endpoint 文件（本次未启用/启动失败时调用）。
pub fn clear_stale_endpoint() {
    remove_endpoint_file();
}

/// 启动 MCP Server（应在 tokio 运行时内调用；`app` 用于提取各业务 Manager）。
///
/// 已在运行时先停旧的再起新的（设置页改端口 / 档位走这条路，无需重启应用）。
/// 失败仅影响 MCP 功能本身，不影响 App 主流程 —— 调用方需吞掉错误只记日志。
pub async fn start(app: tauri::AppHandle, settings: McpSettings) -> Result<McpStartInfo, String> {
    if !settings.enabled {
        return Err("MCP 未启用".to_string());
    }
    let Some(tier) = McpSettings::normalized_tier(&settings.tier) else {
        return Err(format!("未知的档位: {}", settings.tier));
    };
    let tier = tier.to_string();

    // 整个启停串行化：两处同时点开关时不会出现两个监听器
    let mut guard = runtime().lock().await;
    if let Some(prev) = guard.take() {
        info!("[MCP] 配置变更，重启服务");
        shutdown(prev).await;
    }

    let token = generate_token();
    {
        let mut guard = tier_cell().write().unwrap_or_else(|e| e.into_inner());
        *guard = tier.clone();
    }
    let ctx = Arc::new(ctx::McpContext::from_app(&app, Arc::clone(tier_cell())));
    // 订阅 EventBus 广播：串口数据 → hub（游标/等待），端口断开 → 清理
    let serial_sub = ctx.serial_hub.spawn_subscription(Arc::clone(&ctx.event_bus));
    // BLE 通知数据 → hub（按 地址+特征 分桶）
    let ble_sub = ctx.ble_hub.spawn_subscription(Arc::clone(&ctx.event_bus));
    let mcp_server = server::ComBridgeMcp::new(Arc::clone(&ctx));

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
    let port_fallback = port != settings.port;

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let serve_task = tokio::spawn(async move {
        let serve = axum::serve(listener, router).with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        });
        if let Err(e) = serve.await {
            error!("[MCP] HTTP 服务异常退出: {}", e);
        }
    });

    let endpoint_path = write_endpoint_file(port, &token, &tier)?;

    *guard = Some(RunningServer {
        port,
        token: token.clone(),
        started_at: now_millis(),
        port_fallback,
        ctx,
        shutdown: Some(shutdown_tx),
        serve_task: Some(serve_task),
        subscriptions: vec![serial_sub, ble_sub],
    });

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
        port_fallback,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_normalization_is_strict() {
        assert_eq!(McpSettings::normalized_tier("dev"), Some("dev"));
        assert_eq!(McpSettings::normalized_tier("standard"), Some("standard"));
        // 大小写、空串、未知值一律拒绝：档位是安全边界，不做"宽容解析"
        assert_eq!(McpSettings::normalized_tier("DEV"), None);
        assert_eq!(McpSettings::normalized_tier(""), None);
        assert_eq!(McpSettings::normalized_tier("prod"), None);
    }

    #[test]
    fn test_settings_default_is_off() {
        let settings = McpSettings::default();
        assert!(!settings.enabled, "默认必须是关闭的");
        assert_eq!(settings.port, DEFAULT_PORT);
        assert_eq!(settings.tier, "standard");
    }

    #[tokio::test]
    async fn test_switch_tier_updates_shared_cell() {
        switch_tier("dev").await;
        assert_eq!(current_tier(), "dev");
        switch_tier("standard").await;
        assert_eq!(current_tier(), "standard");
        // 非法档位不得写进 cell
        switch_tier("prod").await;
        assert_eq!(current_tier(), "standard");
    }
}
