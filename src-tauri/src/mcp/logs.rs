//! 日志尾读（`combridge_about(action="logs")` 的实现）。
//!
//! 日志文件有两个来源，优先级不同：
//! 1. `LoggerService` 当前正在写的文件 —— 用户在设置里改过日志路径时以它为准
//!    （`log_config.json` 里的 `filePath` 会覆盖默认的 `<exe>/log/...`）
//! 2. 回退：`<exe 目录>/log/` 内修改时间最新的 `.log`
//!
//! 只读文件尾部一个窗口（默认 256KB），避免把几十 MB 的日志整个读进内存。

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// 单次读取的窗口大小
const TAIL_WINDOW: u64 = 256 * 1024;

/// 日志目录（`init_logger` 的默认落点：可执行文件同级 `log/`）。
pub fn log_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("log")))
}

/// 目录内修改时间最新的 `.log` 文件。
fn newest_log_in(dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("log") {
            continue;
        }
        let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
            continue;
        };
        if best.as_ref().is_none_or(|(t, _)| modified > *t) {
            best = Some((modified, path));
        }
    }
    best.map(|(_, p)| p)
}

/// 回退路径：`<exe>/log/` 内最新的 `.log`。
pub fn latest_log_file() -> Option<PathBuf> {
    newest_log_in(&log_dir()?)
}

/// 当前真正在写的日志文件。
///
/// 先问 `LoggerService`（尊重用户改过的 `filePath`）；文件还没创建或服务未初始化时，
/// 退回到「exe 同级 log/ 目录里最新的那个」，两者都找不到才算没有日志。
pub fn active_log_file() -> Option<PathBuf> {
    let configured = crate::service::logger::LoggerService::global()
        .and_then(|s| s.log_path())
        .cloned();

    if let Some(path) = configured.as_ref() {
        if path.exists() {
            return Some(path.clone());
        }
        // 配置的路径还没落盘：优先在同目录里找（可能已被轮转）
        if let Some(found) = path.parent().and_then(newest_log_in) {
            return Some(found);
        }
    }

    latest_log_file()
}

/// 读取文件末尾 `lines` 行（返回的行数为实际可读行数）。
pub fn tail_lines(path: &Path, lines: usize) -> Result<Vec<String>, String> {
    let mut file = std::fs::File::open(path).map_err(|e| format!("打开日志失败: {e}"))?;
    let len = file
        .metadata()
        .map_err(|e| format!("读取日志元信息失败: {e}"))?
        .len();

    let window = TAIL_WINDOW.min(len);
    let start = len - window;
    if start > 0 {
        file.seek(SeekFrom::Start(start))
            .map_err(|e| format!("定位日志失败: {e}"))?;
    }

    let mut buf = Vec::with_capacity(window as usize);
    file.take(window)
        .read_to_end(&mut buf)
        .map_err(|e| format!("读取日志失败: {e}"))?;

    let text = String::from_utf8_lossy(&buf);
    let mut iter: Vec<&str> = text.lines().collect();
    // 从窗口中间开始，首行大概率是被截断的半行，丢弃
    if start > 0 && !iter.is_empty() {
        iter.remove(0);
    }

    let take = lines.min(iter.len());
    Ok(iter[iter.len() - take..]
        .iter()
        .map(|s| s.to_string())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_returns_last_n_lines() {
        let path = std::env::temp_dir().join("combridge_mcp_log_tail_test.log");
        let content = (1..=50)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        std::fs::write(&path, content).unwrap();

        let tail = tail_lines(&path, 3).unwrap();
        assert_eq!(tail, vec!["line 48", "line 49", "line 50"]);

        // 请求行数超过文件行数时返回全部
        let all = tail_lines(&path, 10_000).unwrap();
        assert_eq!(all.len(), 50);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn tail_missing_file_errors() {
        let path = std::env::temp_dir().join("combridge_mcp_no_such_log.log");
        assert!(tail_lines(&path, 10).is_err());
    }
}
