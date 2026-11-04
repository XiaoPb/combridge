//! 限流与熔断（docs/mcp-server-plan.md §8.5）。
//!
//! 三件事：
//! 1. **QPS 限流**：令牌桶。研发档 20 QPS、标准档 5 QPS（计划里是"每会话/全局"两档，
//!    但 rmcp 的 `RequestContext` 不暴露 session id，本机单用户场景下按全局收紧更保守也更简单）。
//! 2. **并发上限**：`combridge_serial_expect` 最多 8 个同时等待 —— 它是唯一会长时间挂起的工具，
//!    不设上限时一个跑飞的模型能把 tokio 任务堆成山。
//! 3. **端口熔断**：同一端口连续 5 次硬件错误 → 暂停 30s。避免"坏线缆 + 重试循环"把日志刷爆。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use parking_lot::Mutex;

/// 连续失败多少次触发熔断
const BREAKER_THRESHOLD: u32 = 5;
/// 熔断持续时长
const BREAKER_COOLDOWN: Duration = Duration::from_secs(30);

/// 单个工具的并发上限（未列出的工具不限制）
fn concurrency_cap(tool: &str) -> Option<usize> {
    match tool {
        "combridge_serial_expect" => Some(8),
        _ => None,
    }
}

/// 被限流拒绝的原因。
#[derive(Debug, Clone, Copy)]
pub enum Reject {
    /// QPS 超限，建议等待后再试
    Qps { retry_after_ms: u64 },
    /// 该工具并发已满
    Busy { limit: usize },
}

/// 令牌桶 + 并发计数。
pub struct RateLimiter {
    /// f64 的 bit 表示（原子量里没有 f64，用 to_bits/from_bits 包装）
    per_sec: AtomicU64,
    state: Mutex<Bucket>,
    inflight: Mutex<HashMap<String, usize>>,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

fn qps_for(tier: &str) -> f64 {
    if tier == "dev" {
        20.0
    } else {
        5.0
    }
}

impl RateLimiter {
    /// 按档位构造：研发档 20 QPS，标准档 5 QPS。
    pub fn for_tier(tier: &str) -> Self {
        let qps = qps_for(tier);
        Self {
            per_sec: AtomicU64::new(qps.to_bits()),
            state: Mutex::new(Bucket {
                tokens: qps, // 允许一次性打完一秒的额度，避免刚启动就被判超限
                last: Instant::now(),
            }),
            inflight: Mutex::new(HashMap::new()),
        }
    }

    fn per_sec(&self) -> f64 {
        f64::from_bits(self.per_sec.load(Ordering::Relaxed))
    }

    /// 切换档位时改速率，不重建限流器（重建会丢失在飞计数与现有令牌）。
    pub fn retune(&self, tier: &str) {
        self.per_sec
            .store(qps_for(tier).to_bits(), Ordering::Relaxed);
    }

    /// 取一个令牌；不足时返回需要等待的毫秒数。
    pub fn try_acquire(&self) -> Result<(), Reject> {
        let per_sec = self.per_sec();
        let mut bucket = self.state.lock();
        let now = Instant::now();
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.last = now;
        bucket.tokens = (bucket.tokens + elapsed * per_sec).min(per_sec);

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            let missing = 1.0 - bucket.tokens;
            let wait_ms = (missing / per_sec * 1000.0).ceil() as u64;
            Err(Reject::Qps {
                retry_after_ms: wait_ms.max(1),
            })
        }
    }

    /// 进入工具的并发区；`Drop` 时自动退出。
    pub fn enter(&self, tool: &str) -> Result<InFlightGuard<'_>, Reject> {
        let Some(cap) = concurrency_cap(tool) else {
            return Ok(InFlightGuard {
                limiter: self,
                tool: None,
            });
        };
        let mut map = self.inflight.lock();
        let slot = map.entry(tool.to_string()).or_insert(0);
        if *slot >= cap {
            return Err(Reject::Busy { limit: cap });
        }
        *slot += 1;
        Ok(InFlightGuard {
            limiter: self,
            tool: Some(tool.to_string()),
        })
    }

    /// 限流 + 并发一起判：任一不过就拒绝。
    pub fn admit(&self, tool: &str) -> Result<InFlightGuard<'_>, Reject> {
        self.try_acquire()?;
        self.enter(tool)
    }

    fn leave(&self, tool: &str) {
        let mut map = self.inflight.lock();
        if let Some(slot) = map.get_mut(tool) {
            *slot = slot.saturating_sub(1);
            if *slot == 0 {
                map.remove(tool);
            }
        }
    }
}

/// 并发占用凭证，Drop 时自动释放。
pub struct InFlightGuard<'a> {
    limiter: &'a RateLimiter,
    tool: Option<String>,
}

impl Drop for InFlightGuard<'_> {
    fn drop(&mut self) {
        if let Some(tool) = self.tool.take() {
            self.limiter.leave(&tool);
        }
    }
}

/// 单端口的连续硬件错误计数与熔断窗口。
pub struct PortBreaker {
    ports: Mutex<HashMap<String, PortState>>,
}

struct PortState {
    failures: u32,
    tripped_until: Option<Instant>,
}

impl PortBreaker {
    pub fn new() -> Self {
        Self {
            ports: Mutex::new(HashMap::new()),
        }
    }

    /// 端口是否处于熔断窗口内；返回剩余秒数。
    pub fn suspended_for(&self, port: &str) -> Option<u64> {
        let mut map = self.ports.lock();
        let state = map.get_mut(port)?;
        match state.tripped_until {
            Some(until) if until > Instant::now() => {
                Some(until.duration_since(Instant::now()).as_secs() + 1)
            }
            Some(_) => {
                // 窗口已过：清零，给端口一次重新证明自己的机会
                state.tripped_until = None;
                state.failures = 0;
                None
            }
            None => None,
        }
    }

    /// 记一次硬件错误；返回本次是否触发熔断。
    pub fn record_failure(&self, port: &str) -> bool {
        let mut map = self.ports.lock();
        let state = map.entry(port.to_string()).or_insert(PortState {
            failures: 0,
            tripped_until: None,
        });
        state.failures += 1;
        if state.failures >= BREAKER_THRESHOLD {
            state.failures = 0;
            state.tripped_until = Some(Instant::now() + BREAKER_COOLDOWN);
            true
        } else {
            false
        }
    }

    /// 一次成功调用清零计数。
    pub fn record_success(&self, port: &str) {
        let mut map = self.ports.lock();
        if let Some(state) = map.get_mut(port) {
            state.failures = 0;
        }
    }
}

impl Default for PortBreaker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucket_allows_burst_then_throttles() {
        let limiter = RateLimiter::for_tier("standard"); // 5 QPS
        for _ in 0..5 {
            assert!(limiter.try_acquire().is_ok());
        }
        match limiter.try_acquire() {
            Err(Reject::Qps { retry_after_ms }) => assert!(retry_after_ms >= 1),
            other => panic!("应被限流，实际: {other:?}"),
        }
    }

    #[test]
    fn bucket_refills_over_time() {
        let limiter = RateLimiter::for_tier("standard");
        while limiter.try_acquire().is_ok() {}
        std::thread::sleep(Duration::from_millis(450)); // 5 QPS → 约 2 个令牌
        assert!(limiter.try_acquire().is_ok());
    }

    #[test]
    fn expect_concurrency_capped() {
        let limiter = RateLimiter::for_tier("dev");
        let mut guards = Vec::new();
        for _ in 0..8 {
            guards.push(limiter.enter("combridge_serial_expect").unwrap());
        }
        match limiter.enter("combridge_serial_expect") {
            Err(Reject::Busy { limit }) => assert_eq!(limit, 8),
            other => panic!("应返回 Busy，实际: {:?}", other.map(|_| "ok")),
        }
        // 释放一个后可以再进
        guards.pop();
        assert!(limiter.enter("combridge_serial_expect").is_ok());
    }

    #[test]
    fn unaffected_tools_are_not_capped() {
        let limiter = RateLimiter::for_tier("dev");
        let guards: Vec<_> = (0..100)
            .map(|_| limiter.enter("combridge_ports").unwrap())
            .collect();
        assert_eq!(guards.len(), 100);
    }

    #[test]
    fn breaker_trips_after_threshold() {
        let b = PortBreaker::new();
        for i in 1..BREAKER_THRESHOLD {
            assert!(!b.record_failure("COM3"), "第 {i} 次不该熔断");
            assert!(b.suspended_for("COM3").is_none());
        }
        assert!(b.record_failure("COM3"), "第 5 次应熔断");
        assert!(b.suspended_for("COM3").is_some());
    }

    #[test]
    fn success_resets_failures() {
        let b = PortBreaker::new();
        for _ in 0..4 {
            b.record_failure("COM3");
        }
        b.record_success("COM3");
        for _ in 0..4 {
            assert!(!b.record_failure("COM3"));
        }
    }

    #[test]
    fn breaker_is_per_port() {
        let b = PortBreaker::new();
        for _ in 0..BREAKER_THRESHOLD {
            b.record_failure("COM3");
        }
        assert!(b.suspended_for("COM3").is_some());
        assert!(b.suspended_for("COM4").is_none());
    }
}
