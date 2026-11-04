//! OTA 确认令牌（L3 硬门，docs/mcp-server-plan.md §6.4）。
//!
//! `combridge_ota(action=start)` 不带 `confirm` 时**只做前置检查**并签发令牌；
//! 只有把令牌原样带回才会真正烧录。规则写在代码里，不是提示词层面的约定：
//!
//! - 一次性：`consume` 成功即失效
//! - 5 分钟过期
//! - 绑定目标设备、固件路径与镜像 sha256，任何一项不匹配都视为无效令牌
//!
//! 这样即使模型"想当然"地直接触发烧录，也拿不到执行路径。

use parking_lot::Mutex;
use std::collections::HashMap;

/// 令牌有效期（毫秒）
pub const CONFIRM_TTL_MS: u64 = 5 * 60 * 1000;

/// 同时未消费的令牌上限，超出时淘汰最早过期的
const MAX_PENDING: usize = 8;

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 一次待确认的 OTA 计划。
#[derive(Debug, Clone)]
pub struct PendingOta {
    pub token: String,
    pub protocol: Option<String>,
    pub address: String,
    pub firmware_path: String,
    pub image_sha256: String,
    pub expires_at: u64,
}

impl PendingOta {
    /// 剩余有效秒数（向上取整）
    pub fn remaining_secs(&self) -> u64 {
        let now = now_millis();
        if self.expires_at <= now {
            0
        } else {
            (self.expires_at - now).div_ceil(1000)
        }
    }
}

/// 令牌校验失败的原因（都已映射为稳定的错误码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmError {
    /// 令牌不存在（拼错、或已被消费）
    NotFound,
    /// 超过 5 分钟
    Expired,
    /// 与签发的计划不一致（换了设备 / 包 / 协议）
    Mismatch(&'static str),
}

impl ConfirmError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "CONFIRM_INVALID",
            Self::Expired => "CONFIRM_EXPIRED",
            Self::Mismatch(_) => "CONFIRM_MISMATCH",
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Self::NotFound => "确认令牌不存在或已被使用（令牌一次性有效）".to_string(),
            Self::Expired => "确认令牌已过期（有效期 5 分钟）".to_string(),
            Self::Mismatch(field) => format!("确认令牌与本次计划不一致：{field} 已变更"),
        }
    }
}

/// 令牌仓库（单进程内存，随进程退出清空）。
pub struct OtaConfirmStore {
    inner: Mutex<HashMap<String, PendingOta>>,
}

impl OtaConfirmStore {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(HashMap::new()),
        }
    }

    /// 签发新令牌。每次 dry-run 都签发新令牌，旧令牌保留到自己过期
    /// （便于用户先看了计划、过一会儿再确认）。
    pub fn issue(
        &self,
        protocol: Option<String>,
        address: &str,
        firmware_path: &str,
        image_sha256: &str,
    ) -> PendingOta {
        let raw = uuid::Uuid::new_v4().simple().to_string();
        let token = format!("ota_{}", &raw[..8]);
        let pending = PendingOta {
            token: token.clone(),
            protocol,
            address: address.to_string(),
            firmware_path: firmware_path.to_string(),
            image_sha256: image_sha256.to_string(),
            expires_at: now_millis() + CONFIRM_TTL_MS,
        };

        let mut map = self.inner.lock();
        map.retain(|_, p| p.expires_at > now_millis());
        if map.len() >= MAX_PENDING {
            if let Some(oldest) = map
                .iter()
                .min_by_key(|(_, p)| p.expires_at)
                .map(|(k, _)| k.clone())
            {
                map.remove(&oldest);
            }
        }
        map.insert(token, pending.clone());
        pending
    }

    /// 校验并消费令牌。成功返回原计划，失败时令牌**不被消费**（除 Expired 会被清理）。
    pub fn consume(
        &self,
        token: &str,
        protocol: Option<&str>,
        address: &str,
        firmware_path: &str,
        image_sha256: &str,
    ) -> Result<PendingOta, ConfirmError> {
        let mut map = self.inner.lock();
        let Some(pending) = map.get(token).cloned() else {
            return Err(ConfirmError::NotFound);
        };
        if pending.expires_at <= now_millis() {
            map.remove(token);
            return Err(ConfirmError::Expired);
        }
        if pending.address != address {
            return Err(ConfirmError::Mismatch("目标设备"));
        }
        if pending.firmware_path != firmware_path {
            return Err(ConfirmError::Mismatch("固件包路径"));
        }
        if pending.image_sha256 != image_sha256 {
            return Err(ConfirmError::Mismatch("固件内容（sha256）"));
        }
        if pending.protocol.as_deref() != protocol {
            return Err(ConfirmError::Mismatch("协议"));
        }
        map.remove(token);
        Ok(pending)
    }

    /// 清理过期令牌（低频调用即可）。
    pub fn prune(&self) {
        let now = now_millis();
        self.inner.lock().retain(|_, p| p.expires_at > now);
    }
}

impl Default for OtaConfirmStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> OtaConfirmStore {
        OtaConfirmStore::new()
    }

    #[test]
    fn issue_then_consume_succeeds_once() {
        let s = store();
        let p = s.issue(Some("goodix".into()), "AA:BB", "D:/fw.zip", "sha");
        assert!(p.token.starts_with("ota_"));
        assert!(s
            .consume(&p.token, Some("goodix"), "AA:BB", "D:/fw.zip", "sha")
            .is_ok());
        // 一次性：第二次必须失败
        assert_eq!(
            s.consume(&p.token, Some("goodix"), "AA:BB", "D:/fw.zip", "sha")
                .unwrap_err(),
            ConfirmError::NotFound
        );
    }

    #[test]
    fn unknown_token_rejected() {
        let s = store();
        assert_eq!(
            s.consume("ota_deadbeef", None, "AA:BB", "D:/fw.zip", "sha")
                .unwrap_err(),
            ConfirmError::NotFound
        );
    }

    #[test]
    fn mismatch_does_not_consume() {
        let s = store();
        let p = s.issue(None, "AA:BB", "D:/fw.zip", "sha");
        assert!(matches!(
            s.consume(&p.token, None, "11:22", "D:/fw.zip", "sha"),
            Err(ConfirmError::Mismatch("目标设备"))
        ));
        // 换了地址被拒后，原令牌仍可用于正确参数
        assert!(s.consume(&p.token, None, "AA:BB", "D:/fw.zip", "sha").is_ok());
    }

    #[test]
    fn expired_token_rejected() {
        let s = store();
        let p = s.issue(None, "AA:BB", "D:/fw.zip", "sha");
        {
            let mut map = s.inner.lock();
            map.get_mut(&p.token).unwrap().expires_at = now_millis() - 1;
        }
        assert_eq!(
            s.consume(&p.token, None, "AA:BB", "D:/fw.zip", "sha")
                .unwrap_err(),
            ConfirmError::Expired
        );
        // 过期令牌已被清理
        assert!(s.inner.lock().get(&p.token).is_none());
    }

    #[test]
    fn concurrent_tokens_are_independent() {
        let s = store();
        let a = s.issue(None, "AA", "D:/a.zip", "sha-a");
        let b = s.issue(None, "BB", "D:/b.zip", "sha-b");
        assert!(s.consume(&b.token, None, "BB", "D:/b.zip", "sha-b").is_ok());
        assert!(s.consume(&a.token, None, "AA", "D:/a.zip", "sha-a").is_ok());
    }

    #[test]
    fn pending_is_capped() {
        let s = store();
        for _ in 0..(MAX_PENDING + 4) {
            s.issue(None, "AA", "D:/fw.zip", "sha");
        }
        assert!(s.inner.lock().len() <= MAX_PENDING);
    }

    #[test]
    fn remaining_secs_counts_down() {
        let s = store();
        let p = s.issue(None, "AA", "D:/fw.zip", "sha");
        let r = p.remaining_secs();
        assert!(r > 0 && r <= 300);
    }
}
