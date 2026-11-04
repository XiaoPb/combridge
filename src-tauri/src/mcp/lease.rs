//! 设备租约管理。
//!
//! 多个 AI 客户端（Claude Code / Cursor 等）可能同时连接 MCP Server，
//! 串口等独占资源必须通过租约仲裁：同一时刻一个端口只允许一个租约持有者操作。
//!
//! 规则（对应设计文档 §7.3）：
//! - `acquire`：空闲则发新租约；被占用且 `force=false` 返回 `PORT_BUSY`；
//!   `force=true` 踢出现有租约（审计记录）。
//! - 租约带 TTL，过期自动失效；每次操作前 `validate` 顺带清理过期项。
//! - `close` 工具成功后由调用方 `release`。

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::Duration;

use serde::Serialize;

/// 租约默认时长
pub const DEFAULT_TTL_SECS: u64 = 3600;
/// TTL 上限（防止永久占死设备）
pub const MAX_TTL_SECS: u64 = 24 * 3600;

#[derive(Debug, Clone, Serialize)]
pub struct Lease {
    pub lease_id: String,
    /// 端口名等资源标识
    pub resource: String,
    /// 客户端自报身份（如 "claude-code"、"cursor"）
    pub owner: String,
    /// 获取时间（毫秒时间戳）
    pub acquired_at: u64,
    /// 过期时间（毫秒时间戳）
    pub expires_at: u64,
}

#[derive(Debug)]
enum LeaseSlot {
    Active(Lease),
    /// 刚被踢出，保留短暂信息便于 PORT_BUSY 错误里给出提示
    _Tombstone,
}

/// 资源被占用错误，携带当前租约信息供模型决策（是否 force）。
#[derive(Debug, Clone, Serialize)]
pub struct BusyInfo {
    pub current_owner: String,
    pub lease_id: String,
    pub expires_at: u64,
}

#[derive(Debug)]
pub enum AcquireError {
    Busy(BusyInfo),
}

struct Inner {
    leases: HashMap<String, LeaseSlot>,
}

pub struct LeaseManager {
    inner: RwLock<Inner>,
}

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl LeaseManager {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(Inner {
                leases: HashMap::new(),
            }),
        }
    }

    /// 尝试获取资源租约。
    ///
    /// `force=true` 时踢出现有租约（跨客户端抢占）；`force=false` 且被占用
    /// 时返回 `AcquireError::Busy`。
    pub fn acquire(
        &self,
        resource: &str,
        owner: &str,
        ttl_secs: u64,
        force: bool,
    ) -> Result<Lease, AcquireError> {
        let ttl_secs = ttl_secs.clamp(1, MAX_TTL_SECS);
        let now = now_millis();
        let mut inner = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());

        // 先清理该资源上已过期的租约
        let expired_lease_id = match inner.leases.get(resource) {
            Some(LeaseSlot::Active(l)) if l.expires_at <= now => Some(l.lease_id.clone()),
            _ => None,
        };
        if let Some(id) = expired_lease_id {
            tracing::info!("[MCP-Lease] 租约已过期自动释放: {} ({})", resource, id);
            inner.leases.remove(resource);
        }

        if let Some(LeaseSlot::Active(existing)) = inner.leases.get(resource) {
            if !force {
                return Err(AcquireError::Busy(BusyInfo {
                    current_owner: existing.owner.clone(),
                    lease_id: existing.lease_id.clone(),
                    expires_at: existing.expires_at,
                }));
            }
            tracing::warn!(
                "[MCP-Lease] 强制踢出租约: {} 原 owner={} lease={}",
                resource,
                existing.owner,
                existing.lease_id
            );
        }

        let lease = Lease {
            lease_id: uuid::Uuid::new_v4().to_string(),
            resource: resource.to_string(),
            owner: owner.to_string(),
            acquired_at: now,
            expires_at: now + ttl_secs * 1000,
        };
        tracing::info!(
            "[MCP-Lease] 授予租约: {} -> {} (lease={}, ttl={}s)",
            resource,
            owner,
            lease.lease_id,
            ttl_secs
        );
        inner.leases.insert(resource.to_string(), LeaseSlot::Active(lease.clone()));
        Ok(lease)
    }

    /// 校验租约有效性（存在、未过期、匹配资源）。
    /// 过期或无效时返回 Err，并附带当前占用者信息（若有）。
    pub fn validate(&self, resource: &str, lease_id: &str) -> Result<(), String> {
        let now = now_millis();
        let mut inner = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());

        let Some(slot) = inner.leases.get(resource) else {
            return Err(format!("资源 {resource} 当前无租约（可能已被释放或从未获取）"));
        };
        let LeaseSlot::Active(lease) = slot else {
            return Err(format!("资源 {resource} 租约槽位异常"));
        };
        if lease.lease_id != lease_id {
            return Err(format!(
                "租约不匹配：资源 {} 由 {} 持有（lease={}），传入的 lease_id 无效",
                resource, lease.owner, lease.lease_id
            ));
        }
        if lease.expires_at <= now {
            inner.leases.remove(resource);
            return Err("租约已过期（TTL 用尽），请重新 serial_open 获取".to_string());
        }
        Ok(())
    }

    /// 释放租约。仅持有者本人可释放；不存在时静默成功（幂等）。
    pub fn release(&self, resource: &str, lease_id: &str) {
        let mut inner = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(LeaseSlot::Active(lease)) = inner.leases.get(resource) {
            if lease.lease_id == lease_id {
                inner.leases.remove(resource);
                tracing::info!("[MCP-Lease] 释放租约: {} (lease={})", resource, lease_id);
            }
        }
    }

    /// 查询资源当前租约（无则 None）。
    pub fn current(&self, resource: &str) -> Option<Lease> {
        let now = now_millis();
        let inner = self
            .inner
            .read()
            .unwrap_or_else(|e| e.into_inner());
        match inner.leases.get(resource) {
            Some(LeaseSlot::Active(lease)) if lease.expires_at > now => Some(lease.clone()),
            _ => None,
        }
    }

    /// 系统级强制释放（不校验持有者），如端口意外断开时调用。
    pub fn revoke(&self, resource: &str) {
        let mut inner = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());
        if inner.leases.remove(resource).is_some() {
            tracing::info!("[MCP-Lease] 系统强制释放租约: {}", resource);
        }
    }

    /// 释放全部租约（App 退出时调用）。
    pub fn release_all(&self) {
        let mut inner = self
            .inner
            .write()
            .unwrap_or_else(|e| e.into_inner());
        inner.leases.clear();
    }
}

impl Default for LeaseManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 将 TTL 秒转换为 `Duration`（供 tokio 超时等使用）。
pub fn ttl_duration(secs: u64) -> Duration {
    Duration::from_secs(secs.clamp(1, MAX_TTL_SECS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_and_validate() {
        let m = LeaseManager::new();
        let lease = m.acquire("COM3", "claude", 60, false).unwrap();
        assert!(m.validate("COM3", &lease.lease_id).is_ok());
        assert!(m.validate("COM3", "bad-id").is_err());
    }

    #[test]
    fn test_busy_and_force() {
        let m = LeaseManager::new();
        let first = m.acquire("COM3", "claude", 60, false).unwrap();

        let err = m.acquire("COM3", "cursor", 60, false);
        match err {
            Err(AcquireError::Busy(info)) => {
                assert_eq!(info.current_owner, "claude");
                assert_eq!(info.lease_id, first.lease_id);
            }
            _ => panic!("应为 Busy 错误"),
        }

        // force 抢占
        let second = m.acquire("COM3", "cursor", 60, true).unwrap();
        assert_ne!(second.lease_id, first.lease_id);
        // 原租约失效
        assert!(m.validate("COM3", &first.lease_id).is_err());
        assert!(m.validate("COM3", &second.lease_id).is_ok());
    }

    #[test]
    fn test_ttl_expiry() {
        let m = LeaseManager::new();
        let lease = m.acquire("COM3", "claude", 1, false).unwrap();
        // 手动把过期时间拨到过去
        {
            let mut inner = m.inner.write().unwrap();
            if let Some(LeaseSlot::Active(l)) = inner.leases.get_mut("COM3") {
                l.expires_at = now_millis() - 1;
            }
        }
        assert!(m.validate("COM3", &lease.lease_id).is_err());
        // 过期后可以重新获取
        assert!(m.acquire("COM3", "cursor", 60, false).is_ok());
    }

    #[test]
    fn test_release() {
        let m = LeaseManager::new();
        let lease = m.acquire("COM3", "claude", 60, false).unwrap();
        m.release("COM3", "wrong-id"); // 不应误删
        assert!(m.current("COM3").is_some());
        m.release("COM3", &lease.lease_id);
        assert!(m.current("COM3").is_none());
        m.release("COM3", &lease.lease_id); // 幂等
    }

    #[test]
    fn test_ttl_clamp() {
        let m = LeaseManager::new();
        // 超大 TTL 被 clamp 到 MAX_TTL_SECS
        let lease = m.acquire("COM3", "claude", u64::MAX, false).unwrap();
        assert!(lease.expires_at > lease.acquired_at);
        assert!(lease.expires_at - lease.acquired_at <= (MAX_TTL_SECS + 1) * 1000);
    }
}
