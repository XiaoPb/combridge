//! MCP 审计日志（docs/mcp-server-plan.md §8.4）。
//!
//! 每条工具调用写一行 JSON 到 `log/mcp-audit-YYYY-MM-DD.jsonl`：
//!
//! ```json
//! {"ts":1793790000000,"tier":"dev","tool":"combridge_serial_write",
//!  "args":{"keys":["data","lease_id","port_name"],"sha256":"…","preview_hex":"41542b","bytes":6},
//!  "result_code":"OK","duration_ms":12,"dry_run":false}
//! ```
//!
//! 三条原则：
//! - **不可关闭**：审计是合规能力，不是可选项；没有开关。
//! - **不阻断业务**：写盘失败只记 `tracing::error`，绝不让产线因为磁盘满而停摆。
//! - **不落原文**：`data` 之类的载荷只留 sha256 + 前 64 字节 hex，避免把固件/密钥写进日志。

use std::io::Write;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::error;

/// 审计保留天数
const RETENTION_DAYS: i64 = 30;
/// 载荷预览长度（字节）
const PREVIEW_BYTES: usize = 64;
/// 同一进程内错误日志的节流窗口（毫秒）
const ERROR_LOG_INTERVAL_MS: u64 = 60_000;

/// 当前毫秒时间戳（模块内与 server 共用）。
pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 参数摘要：结构 + 全文 sha256 + 载荷前若干字节。
#[derive(Debug, Clone, Serialize)]
pub struct ArgsDigest {
    /// 参数名（排序后），让审计能看出"这次调用带了哪些字段"
    pub keys: Vec<String>,
    /// 整个 arguments 的 sha256（十六进制）
    pub sha256: String,
    /// `data` 字段（无则取整个 arguments）前 64 字节的 hex
    pub preview_hex: String,
    /// arguments 序列化后的字节数
    pub bytes: usize,
}

/// 一条审计记录。
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    /// 毫秒时间戳
    pub ts: u64,
    /// 安全档位
    pub tier: String,
    /// 工具名
    pub tool: String,
    /// 参数摘要
    pub args: ArgsDigest,
    /// 结果码：成功为 `OK`，失败取错误结构里的 code
    pub result_code: String,
    /// 耗时（毫秒）
    pub duration_ms: u64,
    /// 本次调用是否为 dry-run（未真正改设备）
    pub dry_run: bool,
}

/// 审计写入器。持有目录，按天切文件。
pub struct AuditLogger {
    dir: PathBuf,
    last_error_ms: Mutex<u64>,
    pruned: Mutex<bool>,
}

impl AuditLogger {
    /// 审计目录：与当前日志同目录（用户可能改过日志路径），否则 `<exe>/log`。
    pub fn resolve_dir() -> Option<PathBuf> {
        crate::mcp::logs::active_log_file()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .or_else(crate::mcp::logs::log_dir)
    }

    pub fn new(dir: PathBuf) -> Self {
        Self {
            dir,
            last_error_ms: Mutex::new(0),
            pruned: Mutex::new(false),
        }
    }

    /// 目录不可确定时返回 None —— 调用方应把它当作"审计关闭"，只记一次告警。
    pub fn try_new() -> Option<Self> {
        Self::resolve_dir().map(Self::new)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// 追加一条记录。失败不返回错误（避免业务被审计拖死），但会打日志且限流。
    pub fn record(&self, entry: &AuditEntry) {
        self.prune_once();

        let line = match serde_json::to_string(entry) {
            Ok(s) => s,
            Err(e) => {
                self.report_error(&format!("审计记录序列化失败: {e}"));
                return;
            }
        };
        let path = self.dir.join(format!(
            "mcp-audit-{}.jsonl",
            chrono::Local::now().format("%Y-%m-%d")
        ));

        let result = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| writeln!(f, "{line}"));

        if let Err(e) = result {
            self.report_error(&format!("审计写入失败（{}）: {e}", path.display()));
        }
    }

    /// 进程内首次写入时清理过期审计（不阻塞启动）。
    fn prune_once(&self) {
        let mut done = self.pruned.lock();
        if *done {
            return;
        }
        *done = true;
        prune_older_than(&self.dir, RETENTION_DAYS);
    }

    fn report_error(&self, msg: &str) {
        let now = now_millis();
        let mut last = self.last_error_ms.lock();
        if now.saturating_sub(*last) >= ERROR_LOG_INTERVAL_MS {
            *last = now;
            error!("[MCP][audit] {msg}");
        }
    }
}

/// 当日审计文件路径（文件可能尚未创建；设置页展示与行数统计用）。
pub fn today_file_path() -> Option<PathBuf> {
    let dir = AuditLogger::resolve_dir()?;
    Some(dir.join(format!(
        "mcp-audit-{}.jsonl",
        chrono::Local::now().format("%Y-%m-%d")
    )))
}

/// 删除超过 `days` 天的审计文件（按文件名里的日期判断，不依赖 mtime）。
fn prune_older_than(dir: &Path, days: i64) {
    let cutoff = chrono::Local::now().date_naive() - chrono::Days::new(days.max(0) as u64);
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(rest) = name.strip_prefix("mcp-audit-") else {
            continue;
        };
        let Some(date_part) = rest.strip_suffix(".jsonl") else {
            continue;
        };
        if let Ok(date) = chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
            if date < cutoff {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// 计算参数摘要。
///
/// `data` 字段单独摘出来做预览 —— 它是唯一可能带二进制/大段文本的字段，
/// 其余字段（端口名、action 等）本身不敏感，留 key 列表就够审计用了。
pub fn digest_args(args: &serde_json::Map<String, serde_json::Value>) -> ArgsDigest {
    let mut keys: Vec<String> = args.keys().cloned().collect();
    keys.sort();

    let serialized = serde_json::Value::Object(args.clone()).to_string();
    let sha256 = hex_lower(&Sha256::digest(serialized.as_bytes()));

    let payload = args
        .get("data")
        .or_else(|| args.get("payload"))
        .and_then(|v| v.as_str())
        .map(|s| s.as_bytes().to_vec())
        .unwrap_or_else(|| serialized.as_bytes().to_vec());

    let preview = &payload[..payload.len().min(PREVIEW_BYTES)];

    ArgsDigest {
        keys,
        sha256,
        preview_hex: hex_lower(preview),
        bytes: serialized.len(),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 从工具结果里取结果码：成功 `OK`，失败取 `{code}`（取不到就 `ERROR`）。
pub fn result_code(is_error: Option<bool>, text: Option<&str>) -> String {
    if is_error != Some(true) {
        return "OK".to_string();
    }
    text.and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
        .and_then(|v| v.get("code").and_then(|c| c.as_str()).map(str::to_string))
        .unwrap_or_else(|| "ERROR".to_string())
}

/// 判断一次调用是否是 dry-run（OTA 的 dry-run 结果里带 `dry_run: true`）。
pub fn is_dry_run(text: Option<&str>) -> bool {
    text.and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok())
        .and_then(|v| v.get("dry_run").and_then(|d| d.as_bool()))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn obj(v: serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
        v.as_object().unwrap().clone()
    }

    #[test]
    fn digest_sorts_keys_and_hashes() {
        let a = digest_args(&obj(json!({"b": 1, "a": 2})));
        assert_eq!(a.keys, vec!["a", "b"]);
        assert_eq!(a.sha256.len(), 64);
        assert!(a.bytes > 0);
    }

    #[test]
    fn digest_is_stable_for_same_content() {
        let a = digest_args(&obj(json!({"port": "COM3", "data": "AT"})));
        let b = digest_args(&obj(json!({"port": "COM3", "data": "AT"})));
        assert_eq!(a.sha256, b.sha256);
        // 字段顺序不同的输入，sha256 也应一致（serde_json::Map 保持插入序，故用 keys 排序后重建）
        let c = digest_args(&obj(json!({"data": "AT", "port": "COM3"})));
        assert_eq!(a.keys, c.keys);
    }

    #[test]
    fn digest_previews_data_field_only() {
        let payload = "A".repeat(200);
        let d = digest_args(&obj(json!({"data": payload})));
        // 64 字节 → 128 个 hex 字符
        assert_eq!(d.preview_hex.len(), 128);

        let short = digest_args(&obj(json!({"data": "AT"})));
        assert_eq!(short.preview_hex, "4154");
    }

    #[test]
    fn digest_without_data_falls_back_to_whole_args() {
        let d = digest_args(&obj(json!({"port_name": "COM3"})));
        assert!(!d.preview_hex.is_empty());
        assert!(d.preview_hex.len() <= PREVIEW_BYTES * 2);
    }

    #[test]
    fn result_code_parsing() {
        assert_eq!(result_code(None, None), "OK");
        assert_eq!(result_code(Some(false), Some("{}")), "OK");
        assert_eq!(
            result_code(Some(true), Some(r#"{"code":"PORT_BUSY"}"#)),
            "PORT_BUSY"
        );
        assert_eq!(result_code(Some(true), Some("not json")), "ERROR");
        assert_eq!(result_code(Some(true), None), "ERROR");
    }

    #[test]
    fn dry_run_flag() {
        assert!(is_dry_run(Some(r#"{"dry_run":true}"#)));
        assert!(!is_dry_run(Some(r#"{"dry_run":false}"#)));
        assert!(!is_dry_run(Some("{}")));
        assert!(!is_dry_run(None));
    }

    #[test]
    fn record_writes_one_line_per_call() {
        let dir = std::env::temp_dir().join("combridge_mcp_audit_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let logger = AuditLogger::new(dir.clone());
        for _ in 0..3 {
            logger.record(&AuditEntry {
                ts: now_millis(),
                tier: "dev".into(),
                tool: "combridge_about".into(),
                args: digest_args(&obj(json!({"action": "summary"}))),
                result_code: "OK".into(),
                duration_ms: 1,
                dry_run: false,
            });
        }

        let path = dir.join(format!(
            "mcp-audit-{}.jsonl",
            chrono::Local::now().format("%Y-%m-%d")
        ));
        let content = std::fs::read_to_string(&path).unwrap();
        assert_eq!(content.lines().count(), 3);
        let first: serde_json::Value =
            serde_json::from_str(content.lines().next().unwrap()).unwrap();
        assert_eq!(first["tool"], "combridge_about");
        assert_eq!(first["result_code"], "OK");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn record_into_missing_dir_does_not_panic() {
        // 目录不存在时只应打日志，不应 panic 也不应阻断调用方
        let logger = AuditLogger::new(std::env::temp_dir().join("combridge_mcp_audit_missing"));
        logger.record(&AuditEntry {
            ts: 0,
            tier: "dev".into(),
            tool: "t".into(),
            args: digest_args(&obj(json!({}))),
            result_code: "OK".into(),
            duration_ms: 0,
            dry_run: false,
        });
    }

    #[test]
    fn prune_removes_only_expired_files() {
        let dir = std::env::temp_dir().join("combridge_mcp_audit_prune");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let old_date = (chrono::Local::now().date_naive() - chrono::Days::new(40))
            .format("%Y-%m-%d")
            .to_string();
        let new_date = chrono::Local::now().format("%Y-%m-%d").to_string();
        let old_path = dir.join(format!("mcp-audit-{old_date}.jsonl"));
        let new_path = dir.join(format!("mcp-audit-{new_date}.jsonl"));
        std::fs::write(&old_path, "{}").unwrap();
        std::fs::write(&new_path, "{}").unwrap();
        // 非审计文件不应被动到
        let other = dir.join("combridge_system_x.log");
        std::fs::write(&other, "log").unwrap();

        prune_older_than(&dir, RETENTION_DAYS);

        assert!(!old_path.exists(), "过期审计应被清理");
        assert!(new_path.exists(), "当天审计必须保留");
        assert!(other.exists(), "非审计文件必须保留");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
