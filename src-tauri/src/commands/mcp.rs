//! MCP 集成面板的后端命令（设置页「集成」一节）。
//!
//! 设计要点：
//! - **开关即时生效**：启用/停用走 `mcp::start` / `mcp::stop`，不需要重启应用。
//! - **状态单一来源**：运行期状态问 `mcp::runtime_info()`（进程内），配置状态读 `mcp.json`。
//!   两者分开返回，UI 才能表达"配置已启用但启动失败"这种真实情况。
//! - **token 只在本地 GUI 出现**：这里返回完整 token 供"复制客户端配置"使用，
//!   UI 默认打码；MCP 侧的 token 只经 endpoint 文件与 Bearer 头流通。

use serde::Serialize;

use crate::error::ComBridgeError;
use crate::mcp::{self, McpSettings};

/// 设置页「集成」的完整状态。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpStatusView {
    /// 配置文件里的启用意图（用户勾选的）
    pub enabled: bool,
    /// 实际是否在监听（启动失败时为 false，与 enabled 不一致）
    pub running: bool,
    /// 生效档位（运行中取运行期的，否则取配置的）
    pub tier: String,
    /// 配置档位（可能还没生效）
    pub config_tier: String,
    /// 配置端口
    pub config_port: u16,
    /// 实际监听端口（未运行时为 0）
    pub port: u16,
    /// 客户端应连接的 URL（未运行时按配置端口拼，仅作展示）
    pub url: String,
    /// Bearer token（未运行时为空串）
    pub token: String,
    /// 服务启动时间（毫秒时间戳，未运行 0）
    pub started_at: u64,
    /// 配置端口被占用、实际换了端口
    pub port_fallback: bool,
    /// 已注册工具数
    pub tool_count: usize,
    /// 配置文件路径
    pub config_path: String,
    /// 数据目录
    pub data_dir: String,
    /// 审计目录（与日志同目录；打不开文件时用它兜底）
    pub audit_dir: Option<String>,
    /// 当日审计文件（不存在为 null）
    pub audit_file: Option<String>,
    /// 当日审计记录条数
    pub audit_records: u64,
    /// 是否在启动时自动接入本地 MCP 客户端
    pub auto_wire: bool,
    /// 客户端配置文件路径（未检测到客户端为空串）
    pub client_config_path: String,
    /// 自动接入状态：`wired` | `missing` | `stale` | `noClient` | `failed`
    pub client_wire_state: String,
    /// 该状态的原因说明（如"地址不一致：…"）
    pub client_wire_detail: Option<String>,
    /// 上次启动失败原因（命令层临时状态，见 `LAST_ERROR`）
    pub last_error: Option<String>,
}

/// 最近一次启动失败原因。进程内保留一份，避免"开关打开了但服务没起来"时用户无从下手。
static LAST_ERROR: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

fn set_last_error(err: Option<String>) {
    *LAST_ERROR.lock().unwrap_or_else(|e| e.into_inner()) = err;
}

fn last_error() -> Option<String> {
    LAST_ERROR
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

/// 汇总当前状态。
async fn collect_status() -> McpStatusView {
    let settings = McpSettings::load();
    let runtime = mcp::runtime_info().await;
    let audit_dir = mcp::audit::AuditLogger::resolve_dir();
    let audit_file = mcp::audit::today_file_path();
    let audit_records = audit_file
        .as_ref()
        .and_then(|p| count_lines(p).ok())
        .unwrap_or(0);
    let wire = compose_wire(&settings, runtime.as_ref());

    compose_status(
        &settings,
        runtime.as_ref(),
        &wire,
        audit_dir.as_deref(),
        audit_file.as_deref(),
        audit_records,
        mcp::tool_count(),
        last_error(),
    )
}

/// 检查自动接入状态。运行中按实际 url+token 比对，未运行则一律视为残留。
fn compose_wire(settings: &McpSettings, runtime: Option<&mcp::RuntimeInfo>) -> ClientWireView {
    let report = match runtime {
        Some(info) => mcp::client_wire::inspect(Some((
            &mcp::endpoint_url(info.port),
            info.token.as_str(),
        ))),
        None => mcp::client_wire::inspect(None),
    };
    ClientWireView {
        auto_wire: settings.auto_wire,
        config_path: report.config_path.map(|p| p.display().to_string()),
        state: report.state.as_str().to_string(),
        detail: report.detail,
    }
}

/// 自动接入状态（拼进视图的中间结构，避免给 `compose_status` 再加四个参数）。
struct ClientWireView {
    auto_wire: bool,
    config_path: Option<String>,
    state: String,
    detail: Option<String>,
}

/// 纯函数形式的视图组装：把 IO 与拼接分开，才能覆盖"未运行 / 运行中 / 端口回退"三种组合。
#[allow(clippy::too_many_arguments)]
fn compose_status(
    settings: &McpSettings,
    runtime: Option<&mcp::RuntimeInfo>,
    wire: &ClientWireView,
    audit_dir: Option<&std::path::Path>,
    audit_file: Option<&std::path::Path>,
    audit_records: u64,
    tool_count: usize,
    last_error: Option<String>,
) -> McpStatusView {
    let running = runtime.is_some();
    // 未运行时 port=0，界面据此判断"没有实际监听"，不要拿配置端口冒充
    let port = runtime.map(|r| r.port).unwrap_or(0);
    let url = mcp::endpoint_url(if running { port } else { settings.port });

    McpStatusView {
        enabled: settings.enabled,
        running,
        tier: runtime
            .map(|r| r.tier.clone())
            .unwrap_or_else(|| settings.tier.clone()),
        config_tier: settings.tier.clone(),
        config_port: settings.port,
        port,
        url,
        token: runtime.map(|r| r.token.clone()).unwrap_or_default(),
        started_at: runtime.map(|r| r.started_at).unwrap_or(0),
        port_fallback: runtime.is_some_and(|r| r.port_fallback),
        tool_count,
        config_path: McpSettings::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        data_dir: McpSettings::data_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        audit_dir: audit_dir.map(|p| p.display().to_string()),
        audit_file: audit_file.map(|p| p.display().to_string()),
        audit_records,
        auto_wire: wire.auto_wire,
        client_config_path: wire.config_path.clone().unwrap_or_default(),
        client_wire_state: wire.state.clone(),
        client_wire_detail: wire.detail.clone(),
        last_error,
    }
}

fn count_lines(path: &std::path::Path) -> std::io::Result<u64> {
    use std::io::BufRead;
    let file = std::fs::File::open(path)?;
    Ok(std::io::BufReader::new(file).lines().count() as u64)
}

/// 读取 MCP 集成状态。
#[tauri::command]
pub async fn get_mcp_status() -> Result<McpStatusView, ComBridgeError> {
    Ok(collect_status().await)
}

/// 应用 MCP 配置（启用开关 / 端口 / 档位），并让它当场生效。
///
/// 顺序固定为 **先落盘再启停**：即使启动失败，用户的选择也已保存，
/// 下次启动仍会尝试，不会出现"界面显示已开启、配置文件却是关闭"的错位。
#[tauri::command]
pub async fn apply_mcp_settings(
    app: tauri::AppHandle,
    enabled: bool,
    port: u16,
    tier: String,
) -> Result<McpStatusView, ComBridgeError> {
    if !(1024..=65535).contains(&port) {
        return Err(ComBridgeError::ConfigError(format!(
            "端口 {port} 不合法，取值范围 1024~65535"
        )));
    }
    let Some(tier) = McpSettings::normalized_tier(&tier) else {
        return Err(ComBridgeError::ConfigError(format!("未知的档位: {tier}")));
    };

    // 保留已持久化的 token：改端口/档位不应顺带作废所有客户端的 Bearer 配置
    let current = McpSettings::load();
    let settings = McpSettings {
        enabled,
        port,
        tier: tier.to_string(),
        token: current.token,
        auto_wire: current.auto_wire,
    };
    settings.save().map_err(ComBridgeError::ConfigError)?;

    if enabled {
        // 只改了档位（端口未变）且服务正在跑 → 热切换，不重启。
        // 重启会换 token，等于逼用户把所有客户端配置重贴一遍。
        if let Some(info) = mcp::runtime_info().await {
            if info.port == port {
                mcp::switch_tier(tier).await;
                set_last_error(None);
                return Ok(collect_status().await);
            }
        }
        match mcp::start(app, settings).await {
            Ok(_) => set_last_error(None),
            Err(e) => {
                // 落盘已成功，这里只把启动失败原因带回界面
                set_last_error(Some(e));
            }
        }
    } else {
        mcp::stop().await;
        set_last_error(None);
    }

    Ok(collect_status().await)
}

/// 轮换 token（强制重启服务）。
///
/// 场景：token 被贴进过聊天记录 / 工单。清掉落盘值后由 `start` 重新生成，
/// 旧客户端携带的 Bearer 随即失效，需要重新取一次 token。
#[tauri::command]
pub async fn rotate_mcp_token(app: tauri::AppHandle) -> Result<McpStatusView, ComBridgeError> {
    let mut settings = McpSettings::load();
    if !settings.enabled {
        return Err(ComBridgeError::ConfigError(
            "MCP 当前未启用，无需轮换 token".to_string(),
        ));
    }
    settings.token = None;
    match mcp::start(app, settings).await {
        Ok(_) => set_last_error(None),
        Err(e) => set_last_error(Some(e)),
    }
    Ok(collect_status().await)
}

/// 立即把当前运行地址写进本地 MCP 客户端配置。
///
/// 兜底场景：客户端升级把配置重置了、用户手工删掉了那一条、或 `auto_wire` 关闭后
/// 想临时写一次。
#[tauri::command]
pub async fn wire_mcp_client() -> Result<McpStatusView, ComBridgeError> {
    let Some(info) = mcp::runtime_info().await else {
        return Err(ComBridgeError::ConfigError(
            "MCP 未运行，先启用服务".to_string(),
        ));
    };
    match mcp::client_wire::wire(&mcp::endpoint_url(info.port), &info.token) {
        Ok(Some(path)) => {
            set_last_error(None);
            tracing::info!("[MCP] 手动接入客户端: {}", path.display());
        }
        Ok(None) => set_last_error(Some(
            "未检测到本机 MCP 客户端配置目录（默认 ~/.workbuddy）".to_string(),
        )),
        Err(e) => set_last_error(Some(format!("写客户端配置失败: {e}"))),
    }
    Ok(collect_status().await)
}

/// 开关「启动时自动接入客户端」。
///
/// 关闭时**不删**已写入的配置：那是用户此刻正在用的东西，删掉等于顺手把它的
/// 客户端弄坏。要清理请直接停用服务（停服会撤掉指向死端口的条目）。
#[tauri::command]
pub async fn set_mcp_auto_wire(enabled: bool) -> Result<McpStatusView, ComBridgeError> {
    let mut settings = McpSettings::load();
    settings.auto_wire = enabled;
    settings.save().map_err(ComBridgeError::ConfigError)?;
    if enabled {
        if let Some(info) = mcp::runtime_info().await {
            match mcp::client_wire::wire(&mcp::endpoint_url(info.port), &info.token) {
                Ok(_) => set_last_error(None),
                Err(e) => set_last_error(Some(format!("写客户端配置失败: {e}"))),
            }
        }
    }
    Ok(collect_status().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::RuntimeInfo;

    fn settings(enabled: bool, port: u16, tier: &str) -> McpSettings {
        McpSettings {
            enabled,
            port,
            tier: tier.to_string(),
            token: None,
            auto_wire: true,
        }
    }

    fn wire(state: &str) -> ClientWireView {
        ClientWireView {
            auto_wire: true,
            config_path: Some("C:/Users/me/.workbuddy/mcp.json".to_string()),
            state: state.to_string(),
            detail: None,
        }
    }

    fn runtime(port: u16, fallback: bool) -> RuntimeInfo {
        RuntimeInfo {
            port,
            token: "tok".to_string(),
            tier: "dev".to_string(),
            started_at: 1_700_000_000_000,
            port_fallback: fallback,
        }
    }

    #[test]
    fn stopped_view_has_no_token_and_no_port() {
        let view = compose_status(&settings(false, 47821, "standard"), None, &wire("noClient"), None, None, 0, 16, None);
        assert!(!view.running);
        assert!(!view.enabled);
        // 未运行时不能拿配置端口冒充实际端口：UI 靠 port==0 区分
        assert_eq!(view.port, 0);
        assert_eq!(view.config_port, 47821);
        assert!(view.token.is_empty());
        // 但仍给出"将会是"的地址，方便用户提前配客户端
        assert_eq!(view.url, "http://127.0.0.1:47821/mcp");
        assert_eq!(view.tier, "standard");
    }

    #[test]
    fn running_view_reports_actual_port() {
        let rt = runtime(47821, false);
        let view = compose_status(&settings(true, 47821, "dev"), Some(&rt), &wire("wired"), None, None, 3, 16, None);
        assert!(view.running);
        assert_eq!(view.port, 47821);
        assert_eq!(view.url, "http://127.0.0.1:47821/mcp");
        assert_eq!(view.token, "tok");
        assert_eq!(view.audit_records, 3);
    }

    #[test]
    fn port_fallback_is_surfaced() {
        // 配置 47821，实际落在 51xxx：URL 必须跟着实际端口走，否则用户配了也连不上
        let rt = runtime(51234, true);
        let view = compose_status(&settings(true, 47821, "dev"), Some(&rt), &wire("wired"), None, None, 0, 16, None);
        assert!(view.port_fallback);
        assert_eq!(view.url, "http://127.0.0.1:51234/mcp");
        assert_eq!(view.config_port, 47821);
    }

    #[test]
    fn last_error_reaches_the_view() {
        let view = compose_status(
            &settings(true, 47821, "dev"),
            None,
            &wire("stale"),
            None,
            None,
            0,
            16,
            Some("端口绑定失败".to_string()),
        );
        // enabled=true 但 running=false 且带原因 —— 这正是"开关开了却没起来"的界面表达
        assert!(view.enabled);
        assert!(!view.running);
        assert_eq!(view.last_error.as_deref(), Some("端口绑定失败"));
    }

    #[test]
    fn wire_state_reaches_the_view() {
        let rt = runtime(47821, false);
        let mut wiring = wire("stale");
        wiring.detail = Some("token 与当前运行的不一致".to_string());
        let view = compose_status(
            &settings(true, 47821, "dev"),
            Some(&rt),
            &wiring,
            None,
            None,
            0,
            16,
            None,
        );
        // 界面要靠这三个字段回答"我要不要再手动配一遍客户端"
        assert!(view.auto_wire);
        assert_eq!(view.client_wire_state, "stale");
        assert_eq!(
            view.client_wire_detail.as_deref(),
            Some("token 与当前运行的不一致")
        );
        assert!(view.client_config_path.ends_with("mcp.json"));
    }
}
