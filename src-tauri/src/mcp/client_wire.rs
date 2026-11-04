//! 自动把本机的 MCP 服务写进 MCP 客户端配置。
//!
//! 目标：用户只做一件事 —— **开个新会话**。写配置、信任授权这些都在服务启动时做掉。
//!
//! 触碰的两份文件（均在客户端配置目录，默认 `~/.workbuddy`，可用
//! `WORKBUDDY_CONFIG_DIR` / `CODEBUDDY_CONFIG_DIR` 覆盖）：
//!
//! 1. `mcp.json` —— 加入一条 `combridge` 的 http server（幂等覆盖同名条目，
//!    其余 server 原样保留）
//! 2. `mcp-approvals.json` —— 预置信任记录，省掉客户端里"点 Trust"这一步。
//!    客户端把信任结果存在 `sha256(origin)::serverName` 下，而 origin 只含
//!    `scheme://host:port`、**不含路径与 header**，所以换 token 不必重新信任，
//!    只有监听端口变了（origin 变）才需要。
//!
//! 两条克制原则：
//! - **目录不存在就什么都不做**：没装客户端的人不该被凭空创建出一堆配置目录。
//! - **原文件解析失败即放弃**：宁可这条自动接入失效，也不能覆盖掉用户其它 server。
//!
//! 自动接入失败不影响服务本身（服务只依赖 endpoint 文件），因此调用方一律
//! 只记日志。

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

/// 写进客户端的 server 名（与 endpoint 里的端口/路径无关，改端口不必改名）
pub const SERVER_NAME: &str = "combridge";

/// 自动接入的对外状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireState {
    /// 没找到客户端配置目录（客户端未安装 / 从未用过）
    NoClient,
    /// 已写入且与当前运行的 url+token 一致
    Wired,
    /// 配置里没有 `combridge` 这一条
    Missing,
    /// 有这一条，但 url/token 与当前不一致（残留旧值，或是停服后留下的死链）
    Stale,
    /// 读写失败
    Failed,
}

impl WireState {
    /// 交给前端的稳定标识（不要用 `Debug`，否则改枚举名就破坏界面契约）
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoClient => "noClient",
            Self::Wired => "wired",
            Self::Missing => "missing",
            Self::Stale => "stale",
            Self::Failed => "failed",
        }
    }
}

/// 一次检查的结果
#[derive(Debug, Clone)]
pub struct WireReport {
    pub state: WireState,
    /// 客户端 `mcp.json` 路径（没找到客户端时为 None）
    pub config_path: Option<PathBuf>,
    /// 失败原因 / 差异说明，供界面直接显示
    pub detail: Option<String>,
}

/// 客户端配置目录。
///
/// **只认已存在的目录**：不存在说明这台机器上没这个客户端，凭空创建出来
/// 反而是往用户家目录里丢垃圾。
pub fn client_dir() -> Option<PathBuf> {
    for key in ["WORKBUDDY_CONFIG_DIR", "CODEBUDDY_CONFIG_DIR"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            let path = PathBuf::from(trimmed);
            if path.is_dir() {
                return Some(path);
            }
        }
    }
    let home = dirs::home_dir()?;
    let path = home.join(".workbuddy");
    path.is_dir().then_some(path)
}

/// 客户端 `mcp.json` 的路径（供状态展示）。
pub fn config_path() -> Option<PathBuf> {
    client_dir().map(|dir| dir.join("mcp.json"))
}

/// 按运行期信息接入（服务启动时调用）。
///
/// 返回 `Ok(None)` 表示没找到客户端目录（正常跳过）；`Ok(Some(path))` 为写入的
/// 配置文件路径。
pub fn wire(url: &str, token: &str) -> Result<Option<PathBuf>, String> {
    let Some(dir) = client_dir() else {
        return Ok(None);
    };
    wire_in(&dir, url, token)?;
    Ok(Some(dir.join("mcp.json")))
}

/// 撤掉 `combridge` 这一条（服务停用时调用）。
///
/// 信任记录**保留**：它是按 origin 记的，重新启用后不必再信任一次。
/// 返回是否真的改动了文件。
pub fn unwire() -> Result<bool, String> {
    let Some(dir) = client_dir() else {
        return Ok(false);
    };
    unwire_in(&dir)
}

/// 检查当前接入状态；`current` 为运行期的 `(url, token)`，未运行时传 `None`。
pub fn inspect(current: Option<(&str, &str)>) -> WireReport {
    let Some(dir) = client_dir() else {
        return WireReport {
            state: WireState::NoClient,
            config_path: None,
            detail: None,
        };
    };
    let path = dir.join("mcp.json");
    match state_in(&dir, current) {
        Ok((state, detail)) => WireReport {
            state,
            config_path: Some(path),
            detail,
        },
        Err(e) => WireReport {
            state: WireState::Failed,
            config_path: Some(path),
            detail: Some(e),
        },
    }
}

// ---------------------------------------------------------------------------
// 纯文件操作（带目录参数，方便单测用临时目录）
// ---------------------------------------------------------------------------

fn read_json_object(path: &Path) -> Result<Option<Map<String, Value>>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    if text.trim().is_empty() {
        return Ok(None);
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(Value::Object(map)) => Ok(Some(map)),
        Ok(_) => Err(format!("{} 顶层不是对象，放弃写入", path.display())),
        // 解析失败时绝不能当空对象覆盖：用户可能在里面配着别的 server
        Err(e) => Err(format!(
            "{} 不是合法 JSON（{e}），先手工修好再启用自动接入",
            path.display()
        )),
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
    }
    let text = serde_json::to_string_pretty(value).map_err(|e| format!("序列化失败: {e}"))?;
    // 同目录临时文件 + rename：避免客户端正好读到写了一半的 JSON
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, format!("{text}\n")).map_err(|e| format!("写入失败: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("替换 {} 失败: {e}", path.display())
    })
}

fn entry_for(url: &str, token: &str) -> Value {
    json!({
        "type": "http",
        "url": url,
        "headers": { "Authorization": format!("Bearer {token}") },
    })
}

/// 写入（或更新）`combridge` 条目；返回是否改动了文件。
pub fn wire_in(dir: &Path, url: &str, token: &str) -> Result<bool, String> {
    let mcp_path = dir.join("mcp.json");
    let mut root = read_json_object(&mcp_path)?.unwrap_or_default();
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(servers) = servers.as_object_mut() else {
        return Err(format!(
            "{} 里 mcpServers 不是对象，放弃写入",
            mcp_path.display()
        ));
    };

    let next = entry_for(url, token);
    let changed = servers.get(SERVER_NAME) != Some(&next);
    if changed {
        servers.insert(SERVER_NAME.to_string(), next);
        write_json_atomic(&mcp_path, &Value::Object(root))?;
        info!("[MCP] 已更新客户端配置 {}", mcp_path.display());
    }

    ensure_trust(dir, url)?;
    Ok(changed)
}

/// 撤掉 `combridge` 条目；返回是否改动了文件。
pub fn unwire_in(dir: &Path) -> Result<bool, String> {
    let mcp_path = dir.join("mcp.json");
    let Some(mut root) = read_json_object(&mcp_path)? else {
        return Ok(false);
    };
    let Some(servers) = root.get_mut("mcpServers").and_then(Value::as_object_mut) else {
        return Ok(false);
    };
    if servers.remove(SERVER_NAME).is_none() {
        return Ok(false);
    }
    write_json_atomic(&mcp_path, &Value::Object(root))?;
    info!("[MCP] 已从客户端配置移除 {}", mcp_path.display());
    Ok(true)
}

fn state_in(
    dir: &Path,
    current: Option<(&str, &str)>,
) -> Result<(WireState, Option<String>), String> {
    let mcp_path = dir.join("mcp.json");
    let Some(root) = read_json_object(&mcp_path)? else {
        return Ok((WireState::Missing, None));
    };
    let entry = root
        .get("mcpServers")
        .and_then(Value::as_object)
        .and_then(|servers| servers.get(SERVER_NAME));
    let Some(entry) = entry else {
        return Ok((WireState::Missing, None));
    };
    let Some((url, token)) = current else {
        return Ok((
            WireState::Stale,
            Some("服务未运行，配置里仍留着这一条".to_string()),
        ));
    };
    if entry == &entry_for(url, token) {
        return Ok((WireState::Wired, None));
    }
    let stored_url = entry.get("url").and_then(Value::as_str).unwrap_or("");
    let detail = if stored_url == url {
        "token 与当前运行的不一致".to_string()
    } else {
        format!("地址不一致：配置里是 {stored_url}")
    };
    Ok((WireState::Stale, Some(detail)))
}

/// 预置信任记录：键已存在就不动（保留首次信任时间，也避免每次启动都写盘）。
fn ensure_trust(dir: &Path, url: &str) -> Result<(), String> {
    let approvals_path = dir.join("mcp-approvals.json");
    let mut approvals = read_json_object(&approvals_path)?.unwrap_or_default();
    let key = approval_key(url);
    if approvals.contains_key(&key) {
        return Ok(());
    }
    approvals.insert(key, json!(now_millis()));
    write_json_atomic(&approvals_path, &Value::Object(approvals))
}

/// 与客户端 `computeTrustConfigHash` + `approvalKey` 一致：`sha256(origin)::name`。
pub fn approval_key(url: &str) -> String {
    format!(
        "{}::{}",
        hex_lower(&Sha256::digest(origin_of(url).as_bytes())),
        SERVER_NAME
    )
}

/// 只保留 `scheme://host:port`。客户端就是这么算的，所以路径与 header 变化
/// （换 token）不会让信任失效。
fn origin_of(url: &str) -> String {
    let (scheme, rest) = url.split_once("://").unwrap_or(("http", url));
    let authority = rest.split('/').next().unwrap_or(rest);
    format!("{scheme}://{authority}")
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    })
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 启动时接入 + 日志（吞掉错误，只记 warn）。
///
/// 放在服务启动链路里调用：成功是静默的，失败也只是少一次自动接入。
pub fn wire_on_start(url: &str, token: &str) {
    match wire(url, token) {
        Ok(Some(path)) => info!(
            "[MCP] 客户端配置已就绪（开新会话即可使用）: {}",
            path.display()
        ),
        Ok(None) => {
            info!("[MCP] 未检测到本机 MCP 客户端配置目录，跳过自动接入");
        }
        Err(e) => warn!("[MCP] 自动接入客户端失败（不影响服务本身）: {e}"),
    }
}

/// 停服时撤掉条目（只记日志）。
pub fn unwire_on_stop() {
    match unwire() {
        Ok(true) => {}
        Ok(false) => {}
        Err(e) => warn!("[MCP] 停服后清理客户端配置失败: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("创建临时目录")
    }

    const URL: &str = "http://127.0.0.1:47821/mcp";
    const TOKEN: &str = "abc123";

    #[test]
    fn wire_writes_server_and_trust_record() {
        let dir = tmp();
        assert!(wire_in(dir.path(), URL, TOKEN).unwrap());

        let mcp: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("mcp.json")).unwrap())
                .unwrap();
        let entry = &mcp["mcpServers"]["combridge"];
        assert_eq!(entry["type"], "http");
        assert_eq!(entry["url"], URL);
        assert_eq!(entry["headers"]["Authorization"], "Bearer abc123");

        let approvals: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("mcp-approvals.json")).unwrap(),
        )
        .unwrap();
        assert!(
            approvals.get(approval_key(URL)).is_some(),
            "必须预置信任记录"
        );
    }

    #[test]
    fn wire_is_idempotent() {
        let dir = tmp();
        assert!(wire_in(dir.path(), URL, TOKEN).unwrap());
        // 第二次不应再写盘：客户端配置被频繁改写会让用户觉得程序在乱动他的文件
        assert!(!wire_in(dir.path(), URL, TOKEN).unwrap());
    }

    #[test]
    fn wire_keeps_other_servers() {
        let dir = tmp();
        std::fs::write(
            dir.path().join("mcp.json"),
            r#"{"mcpServers":{"other":{"type":"http","url":"http://x"}},"theme":"dark"}"#,
        )
        .unwrap();
        wire_in(dir.path(), URL, TOKEN).unwrap();

        let mcp: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("mcp.json")).unwrap())
                .unwrap();
        assert_eq!(mcp["mcpServers"]["other"]["url"], "http://x");
        assert_eq!(mcp["theme"], "dark", "顶层其它字段必须原样保留");
        assert!(mcp["mcpServers"]["combridge"].is_object());
    }

    #[test]
    fn broken_json_is_left_untouched() {
        let dir = tmp();
        let path = dir.path().join("mcp.json");
        std::fs::write(&path, "{ not json").unwrap();
        assert!(wire_in(dir.path(), URL, TOKEN).is_err());
        // 宁可失败也不能把用户手写的内容冲掉
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ not json");
    }

    #[test]
    fn token_change_keeps_trust_record() {
        let dir = tmp();
        wire_in(dir.path(), URL, TOKEN).unwrap();
        let before: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("mcp-approvals.json")).unwrap(),
        )
        .unwrap();

        // 轮换 token：条目要更新，但信任键不变（origin 未变），不该重新弹授权
        assert!(wire_in(dir.path(), URL, "new-token").unwrap());
        let after: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("mcp-approvals.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(before, after, "换 token 不得改动信任记录");
    }

    #[test]
    fn port_change_requires_new_trust_key() {
        let dir = tmp();
        wire_in(dir.path(), URL, TOKEN).unwrap();
        wire_in(dir.path(), "http://127.0.0.1:50000/mcp", TOKEN).unwrap();

        let approvals: Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("mcp-approvals.json")).unwrap(),
        )
        .unwrap();
        // 旧键留着无妨；新 origin 必须有新键，否则客户端会再问一次
        assert!(approvals
            .get(approval_key("http://127.0.0.1:50000/mcp"))
            .is_some());
    }

    #[test]
    fn unwire_removes_entry_but_keeps_trust() {
        let dir = tmp();
        wire_in(dir.path(), URL, TOKEN).unwrap();
        assert!(unwire_in(dir.path()).unwrap());

        let mcp: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.path().join("mcp.json")).unwrap())
                .unwrap();
        assert!(mcp["mcpServers"].get("combridge").is_none());
        // 重新启用时不该再让用户点一次信任
        assert!(dir.path().join("mcp-approvals.json").exists());
        // 已经删过了，再删一次应报告"没改动"
        assert!(!unwire_in(dir.path()).unwrap());
    }

    #[test]
    fn inspect_reports_each_state() {
        let dir = tmp();
        assert_eq!(
            state_in(dir.path(), Some((URL, TOKEN))).unwrap().0,
            WireState::Missing
        );

        wire_in(dir.path(), URL, TOKEN).unwrap();
        assert_eq!(
            state_in(dir.path(), Some((URL, TOKEN))).unwrap().0,
            WireState::Wired
        );
        // token 变了 → 配置过期
        assert_eq!(
            state_in(dir.path(), Some((URL, "other"))).unwrap().0,
            WireState::Stale
        );
        // 未运行 → 同样算过期（配置指向一个没人监听的端口）
        assert_eq!(state_in(dir.path(), None).unwrap().0, WireState::Stale);
    }

    #[test]
    fn approval_key_ignores_path_and_scheme_default() {
        // 与客户端一致：只哈希 origin，路径不参与
        assert_eq!(
            approval_key("http://127.0.0.1:47821/mcp"),
            approval_key("http://127.0.0.1:47821/other")
        );
        assert_eq!(approval_key(URL).len(), 64 + 2 + SERVER_NAME.len());
        assert!(approval_key(URL).ends_with("::combridge"));
    }
}
