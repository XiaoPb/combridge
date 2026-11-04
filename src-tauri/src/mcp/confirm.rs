//! 本机窗口确认通道 —— elicitation 不可用时的兜底（docs/mcp-server-plan.md §8.3）。
//!
//! 背景：MCP 规范里的人机确认靠 `elicitation/create`，前提是**客户端在 initialize
//! 里声明了 elicitation 能力**。主流的桌面宿主（含 WorkBuddy Connector 5.9.3）
//! 都没有声明，服务端只能拿到 `CapabilityNotSupported`，于是标准档的 L3 工具
//! 在这类客户端上等于恒拒绝 —— 安全上没错，但"人工确认"这条产品设计落不了地。
//!
//! 折中办法：ComBridge 自己就是本机 GUI 进程，确认框可以由**本机窗口**弹出，
//! 不依赖 MCP 协议。链路是：
//!
//! ```text
//! MCP handler ──emit(mcp-confirm-request)──▶ 前端 Modal
//!      ▲                                        │
//!      └───── oneshot ◀── resolve_mcp_confirm ───┘
//! ```
//!
//! **授权粒度**：每次都弹窗等于把人逼疯（产测一次要点三下、连写串口要点五下），
//! 所以在弹窗里允许用户勾"本次会话内允许对该目标执行该操作"。授权表按
//! `(tool, scope)` 记账、30 分钟过期、服务重启即清空 —— 仍然是用户主动授予，
//! 只是把"信一次"的范围限定在一台设备、一类操作、一个时间窗内。
//!
//! 仍然 fail-closed：窗口不可用、无人应答、超时，一律按拒绝处理。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use parking_lot::Mutex;
use serde::Serialize;
use tokio::sync::oneshot;
use tracing::{info, warn};

/// 请求确认（后端 → 前端）
pub const EVENT_REQUEST: &str = "mcp-confirm-request";
/// 请求结束（已应答 / 超时 / 通道关闭），前端据此收起弹窗
pub const EVENT_SETTLED: &str = "mcp-confirm-settled";

/// 一次授权的有效期。
///
/// 不设成"永久"：用户勾选时的心理预期是"这轮调试别烦我"，不是"以后都别问我"。
pub const GRANT_TTL: Duration = Duration::from_secs(30 * 60);

/// 事件发送器。抽成闭包是为了让单测能注入假通道 —— 真实实现依赖 `AppHandle`，
/// 在单测里造不出来。
type EventSink = Arc<dyn Fn(&str, serde_json::Value) -> bool + Send + Sync>;

fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 一次确认请求的入参。
///
/// `scope` 是**机器可读**的授权范围（端口名 / 蓝牙地址 / 产测 owner），
/// `scope_label` 是同一个东西的人读形式（弹窗里显示"COM3"而不是内部 id）。
pub struct Request<'a> {
    pub tool: &'a str,
    pub risk: &'a str,
    pub scope: &'a str,
    pub scope_label: &'a str,
    pub detail: &'a str,
    /// 是否允许用户勾选"本次会话内放行"。OTA 这类一次性高危操作应为 false。
    pub grantable: bool,
}

/// 一条等待应答的确认请求。
struct Pending {
    tool: String,
    risk: String,
    scope_label: String,
    detail: String,
    grantable: bool,
    created_at: u64,
    deadline_at: u64,
    tx: oneshot::Sender<Reply>,
}

/// 前端回传的应答。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub accepted: bool,
    /// 用户勾了"本次会话内允许"。只在 `grantable` 时生效。
    pub remember: bool,
}

/// 待确认请求的快照（前端渲染用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmRequestView {
    pub id: String,
    /// 发起的工具名，便于用户判断是谁在要权限
    pub tool: String,
    /// 风险标签，如"串口写入"
    pub risk: String,
    /// 授权范围（人读）
    pub scope_label: String,
    /// 具体内容："向 COM3 写入 12 字节：…"
    pub detail: String,
    /// 是否展示"本次会话内放行"勾选框
    pub grantable: bool,
    /// 勾选后的有效期（秒），用于文案
    pub grant_ttl_secs: u64,
    pub created_at: u64,
    /// 截止时间（毫秒时间戳），前端据此倒计时
    pub deadline_at: u64,
    /// 剩余毫秒（列表补拉时用，≤0 表示已过期）
    pub remaining_ms: u64,
}

/// 确认结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// 用户点了确认；`remembered` 表示这次授权已被记住（后续同 tool+scope 免确认）
    Accepted {
        remembered: bool,
    },
    Declined,
    /// 等满时限也没人点
    Timeout,
    /// 根本没有可用通道（本机窗口不可用 / 事件发不出去 / 服务正在停）
    NoChannel,
}

/// 一条已生效的临时授权（设置页展示 / 撤销用）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantView {
    pub tool: String,
    pub scope: String,
    pub scope_label: String,
    pub risk: String,
    pub granted_at: u64,
    pub expires_at: u64,
    pub remaining_secs: u64,
}

/// 授权键：同工具 + 同目标才算同一次授权。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct GrantKey {
    tool: String,
    scope: String,
}

struct Grant {
    scope_label: String,
    risk: String,
    granted_at: u64,
    expires_at: u64,
}

/// 待确认请求表 + 临时授权表。
pub struct ConfirmBroker {
    emit: Option<EventSink>,
    pending: Mutex<HashMap<String, Pending>>,
    grants: Mutex<HashMap<GrantKey, Grant>>,
    seq: AtomicU64,
}

impl ConfirmBroker {
    /// 绑定 Tauri 应用：确认框走主窗口。
    pub fn new(app: tauri::AppHandle) -> Self {
        use tauri::Emitter;

        let emit: EventSink = Arc::new(move |event: &str, payload: serde_json::Value| -> bool {
            // 先把窗口弄到眼前，否则用户根本不知道有人在等确认
            surface_main_window(&app);
            match app.emit(event, payload) {
                Ok(()) => true,
                Err(e) => {
                    warn!("[MCP] 确认事件 {event} 发送失败: {e}");
                    false
                }
            }
        });
        Self {
            emit: Some(emit),
            pending: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
        }
    }

    /// 无 GUI 通道（headless / 单测）：所有请求立即返回 `NoChannel`。
    pub fn headless() -> Self {
        Self {
            emit: None,
            pending: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
        }
    }

    /// 注入自定义发送器（单测用）。
    #[cfg(test)]
    pub(crate) fn with_emitter(emit: EventSink) -> Self {
        Self {
            emit: Some(emit),
            pending: Mutex::new(HashMap::new()),
            grants: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
        }
    }

    /// 是否有可用的确认通道。
    pub fn has_channel(&self) -> bool {
        self.emit.is_some()
    }

    /// 该操作是否已被用户放行（未过期）。
    pub fn is_granted(&self, tool: &str, scope: &str) -> bool {
        let key = GrantKey {
            tool: tool.to_string(),
            scope: scope.to_string(),
        };
        let now = now_millis();
        let mut grants = self.grants.lock();
        match grants.get(&key) {
            Some(g) if g.expires_at > now => true,
            Some(_) => {
                grants.remove(&key);
                false
            }
            None => false,
        }
    }

    /// 当前所有有效授权。
    pub fn grants(&self) -> Vec<GrantView> {
        let now = now_millis();
        let mut grants = self.grants.lock();
        grants.retain(|_, g| g.expires_at > now);
        let mut views: Vec<_> = grants
            .iter()
            .map(|(k, g)| GrantView {
                tool: k.tool.clone(),
                scope: k.scope.clone(),
                scope_label: g.scope_label.clone(),
                risk: g.risk.clone(),
                granted_at: g.granted_at,
                expires_at: g.expires_at,
                remaining_secs: (g.expires_at - now).div_ceil(1000),
            })
            .collect();
        views.sort_by(|a, b| a.granted_at.cmp(&b.granted_at));
        views
    }

    /// 撤销全部临时授权。
    pub fn revoke_grants(&self) -> usize {
        let mut grants = self.grants.lock();
        let n = grants.len();
        grants.clear();
        n
    }

    /// 发起一次确认并等待应答。`timeout` 内没人点即按 `Timeout` 返回。
    pub async fn request(&self, req: Request<'_>, timeout: Duration) -> Outcome {
        let Some(emit) = self.emit.clone() else {
            return Outcome::NoChannel;
        };

        let id = format!("cf_{}", self.seq.fetch_add(1, Ordering::Relaxed) + 1);
        let created_at = now_millis();
        let deadline_at = created_at + timeout.as_millis() as u64;
        let (tx, rx) = oneshot::channel();

        // 先登记再发事件：前端收到事件后可以立刻应答，不能出现"未知 id"
        self.pending.lock().insert(
            id.clone(),
            Pending {
                tool: req.tool.to_string(),
                risk: req.risk.to_string(),
                scope_label: req.scope_label.to_string(),
                detail: req.detail.to_string(),
                grantable: req.grantable,
                created_at,
                deadline_at,
                tx,
            },
        );

        let payload = serde_json::to_value(ConfirmRequestView {
            id: id.clone(),
            tool: req.tool.to_string(),
            risk: req.risk.to_string(),
            scope_label: req.scope_label.to_string(),
            detail: req.detail.to_string(),
            grantable: req.grantable,
            grant_ttl_secs: GRANT_TTL.as_secs(),
            created_at,
            deadline_at,
            remaining_ms: timeout.as_millis() as u64,
        })
        .unwrap_or_else(|_| serde_json::json!({ "id": id }));

        if !emit(EVENT_REQUEST, payload) {
            self.pending.lock().remove(&id);
            return Outcome::NoChannel;
        }

        info!(
            "[MCP] 已发出本机确认请求 {id}（{} / {}）",
            req.risk, req.scope_label
        );
        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(reply)) if reply.accepted => {
                if reply.remember && req.grantable {
                    self.remember(req.tool, req.scope, req.scope_label, req.risk);
                    Outcome::Accepted { remembered: true }
                } else {
                    Outcome::Accepted { remembered: false }
                }
            }
            Ok(Ok(_)) => Outcome::Declined,
            // 发送端被丢弃：服务在等的时候被停了
            Ok(Err(_)) => Outcome::NoChannel,
            Err(_) => {
                self.pending.lock().remove(&id);
                self.settle(EVENT_SETTLED, &id, "timeout");
                warn!("[MCP] 本机确认请求 {id} 超时未应答，按拒绝处理");
                Outcome::Timeout
            }
        }
    }

    /// 前端应答。返回 `false` 表示该 id 已失效（超时后点了确认、或重复点击）。
    pub fn resolve(&self, id: &str, reply: Reply) -> bool {
        let Some(pending) = self.pending.lock().remove(id) else {
            warn!("[MCP] 确认应答 {id} 未找到对应请求（可能已超时）");
            return false;
        };
        // 不 grantable 的操作不接受"记住"（前端被绕过时的兜底）
        let reply = Reply {
            accepted: reply.accepted,
            remember: reply.remember && pending.grantable,
        };
        let _ = pending.tx.send(reply);
        let decision = if reply.accepted {
            "accepted"
        } else {
            "declined"
        };
        self.settle(EVENT_SETTLED, id, decision);
        info!(
            "[MCP] 本机确认 {id}：{}{}",
            if reply.accepted {
                "已确认"
            } else {
                "已取消"
            },
            if reply.remember {
                "（本次会话内放行）"
            } else {
                ""
            }
        );
        true
    }

    /// 未被应答的请求（前端启动时补拉，防事件丢失）。
    pub fn list(&self) -> Vec<ConfirmRequestView> {
        let now = now_millis();
        let mut views: Vec<_> = self
            .pending
            .lock()
            .iter()
            .filter(|(_, p)| p.deadline_at > now)
            .map(|(id, p)| ConfirmRequestView {
                id: id.clone(),
                tool: p.tool.clone(),
                risk: p.risk.clone(),
                scope_label: p.scope_label.clone(),
                detail: p.detail.clone(),
                grantable: p.grantable,
                grant_ttl_secs: GRANT_TTL.as_secs(),
                created_at: p.created_at,
                deadline_at: p.deadline_at,
                remaining_ms: p.deadline_at - now,
            })
            .collect();
        views.sort_by_key(|v| v.created_at);
        views
    }

    /// 丢弃全部待确认请求（停服时调用）。等待侧会收到通道关闭，按拒绝处理。
    pub fn abandon_all(&self) {
        let drained: Vec<_> = self.pending.lock().drain().map(|(_, p)| p).collect();
        if drained.is_empty() {
            return;
        }
        warn!("[MCP] 停服，丢弃 {} 条待确认请求", drained.len());
        for p in drained {
            drop(p.tx);
        }
    }

    fn remember(&self, tool: &str, scope: &str, scope_label: &str, risk: &str) {
        let now = now_millis();
        self.grants.lock().insert(
            GrantKey {
                tool: tool.to_string(),
                scope: scope.to_string(),
            },
            Grant {
                scope_label: scope_label.to_string(),
                risk: risk.to_string(),
                granted_at: now,
                expires_at: now + GRANT_TTL.as_millis() as u64,
            },
        );
        info!(
            "[MCP] 已记住授权：{risk} @ {scope_label}（{} 分钟后过期）",
            GRANT_TTL.as_secs() / 60
        );
    }

    fn settle(&self, event: &str, id: &str, decision: &str) {
        if let Some(emit) = self.emit.as_ref() {
            emit(event, serde_json::json!({ "id": id, "decision": decision }));
        }
    }
}

/// 把主窗口提到前台。失败不阻断（用户可能把窗口关了，那就按超时拒绝）。
fn surface_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;

    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    if window.is_minimized().unwrap_or(false) {
        let _ = window.unminimize();
    }
    if !window.is_visible().unwrap_or(true) {
        let _ = window.show();
    }
    let _ = window.set_focus();
}

#[cfg(test)]
mod tests {
    use super::*;

    type Sink = Arc<Mutex<Vec<(String, serde_json::Value)>>>;

    fn capture() -> (Sink, EventSink) {
        let sink: Sink = Arc::new(Mutex::new(Vec::new()));
        let s = Arc::clone(&sink);
        let emit: EventSink = Arc::new(move |event: &str, payload: serde_json::Value| {
            s.lock().push((event.to_string(), payload));
            true
        });
        (sink, emit)
    }

    fn req<'a>(tool: &'a str, scope: &'a str, grantable: bool) -> Request<'a> {
        Request {
            tool,
            risk: "串口写入",
            scope,
            scope_label: scope,
            detail: "向 COM3 写入 2 字节",
            grantable,
        }
    }

    /// 等前端拿到事件（等价于"弹窗已经渲染出来"）。
    async fn wait_pending(broker: &ConfirmBroker) -> String {
        for _ in 0..200 {
            if let Some(v) = broker.list().first() {
                return v.id.clone();
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("确认请求未出现在待确认列表里");
    }

    #[tokio::test]
    async fn headless_has_no_channel() {
        let broker = ConfirmBroker::headless();
        assert!(!broker.has_channel());
        assert_eq!(
            broker
                .request(req("t", "s", true), Duration::from_millis(10))
                .await,
            Outcome::NoChannel
        );
        assert!(broker.list().is_empty());
    }

    #[tokio::test]
    async fn accept_flows_through() {
        let (sink, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(async move {
                b.request(
                    req("combridge_serial_write", "COM3", true),
                    Duration::from_secs(2),
                )
                .await
            })
        };
        let id = wait_pending(&broker).await;
        assert!(broker.resolve(
            &id,
            Reply {
                accepted: true,
                remember: false
            }
        ));
        assert_eq!(wait.await.unwrap(), Outcome::Accepted { remembered: false });
        assert!(broker.list().is_empty());
        // 没勾"记住"，不应留下授权
        assert!(!broker.is_granted("combridge_serial_write", "COM3"));
        assert!(broker.grants().is_empty());

        let events = sink.lock();
        assert_eq!(events[0].0, EVENT_REQUEST);
        assert_eq!(events.last().unwrap().1["decision"], "accepted");
    }

    #[tokio::test]
    async fn decline_returns_declined_without_grant() {
        let (_, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(
                async move { b.request(req("t", "s", true), Duration::from_secs(2)).await },
            )
        };
        let id = wait_pending(&broker).await;
        // 取消时即便前端错传了 remember=true，也不能变成授权
        assert!(broker.resolve(
            &id,
            Reply {
                accepted: false,
                remember: true
            }
        ));
        assert_eq!(wait.await.unwrap(), Outcome::Declined);
        assert!(!broker.is_granted("t", "s"));
    }

    #[tokio::test]
    async fn remember_creates_scoped_grant() {
        let (_, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(async move {
                b.request(
                    req("combridge_serial_write", "COM3", true),
                    Duration::from_secs(2),
                )
                .await
            })
        };
        let id = wait_pending(&broker).await;
        assert!(broker.resolve(
            &id,
            Reply {
                accepted: true,
                remember: true
            }
        ));
        assert_eq!(wait.await.unwrap(), Outcome::Accepted { remembered: true });

        // 只对这一台设备有效
        assert!(broker.is_granted("combridge_serial_write", "COM3"));
        assert!(!broker.is_granted("combridge_serial_write", "COM4"));
        assert!(!broker.is_granted("combridge_ble_gatt", "COM3"));
        let grants = broker.grants();
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].scope_label, "COM3");
    }

    #[tokio::test]
    async fn non_grantable_ignores_remember() {
        let (_, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(async move {
                b.request(req("combridge_ota", "AA:BB", false), Duration::from_secs(2))
                    .await
            })
        };
        let id = wait_pending(&broker).await;
        assert!(broker.resolve(
            &id,
            Reply {
                accepted: true,
                remember: true
            }
        ));
        // OTA 每次都要确认，不接受"记住"
        assert_eq!(wait.await.unwrap(), Outcome::Accepted { remembered: false });
        assert!(!broker.is_granted("combridge_ota", "AA:BB"));
    }

    #[tokio::test]
    async fn expired_grant_is_ignored() {
        let broker = ConfirmBroker::with_emitter(Arc::new(|_: &str, _: serde_json::Value| true));
        broker.remember("t", "s", "s", "r");
        assert!(broker.is_granted("t", "s"));
        {
            let mut grants = broker.grants.lock();
            let key = GrantKey {
                tool: "t".into(),
                scope: "s".into(),
            };
            grants.get_mut(&key).unwrap().expires_at = now_millis() - 1;
        }
        assert!(!broker.is_granted("t", "s"));
        assert!(broker.grants().is_empty());
    }

    #[tokio::test]
    async fn revoke_clears_all() {
        let broker = ConfirmBroker::with_emitter(Arc::new(|_: &str, _: serde_json::Value| true));
        broker.remember("t", "s1", "s1", "r");
        broker.remember("t", "s2", "s2", "r");
        assert_eq!(broker.grants().len(), 2);
        assert_eq!(broker.revoke_grants(), 2);
        assert!(!broker.is_granted("t", "s1"));
    }

    #[tokio::test]
    async fn timeout_is_not_acceptance() {
        let (_, emit) = capture();
        let broker = ConfirmBroker::with_emitter(emit);
        let outcome = broker
            .request(req("t", "s", true), Duration::from_millis(30))
            .await;
        assert_eq!(outcome, Outcome::Timeout);
        assert!(broker.list().is_empty(), "超时后请求必须被清掉");
    }

    #[tokio::test]
    async fn unknown_or_duplicate_resolve_is_rejected() {
        let (_, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        assert!(!broker.resolve(
            "cf_nope",
            Reply {
                accepted: true,
                remember: false
            }
        ));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(
                async move { b.request(req("t", "s", true), Duration::from_secs(2)).await },
            )
        };
        let id = wait_pending(&broker).await;
        assert!(broker.resolve(
            &id,
            Reply {
                accepted: true,
                remember: false
            }
        ));
        // 幂等性：同一个 id 不能再被应答一次
        assert!(!broker.resolve(
            &id,
            Reply {
                accepted: true,
                remember: false
            }
        ));
        assert_eq!(wait.await.unwrap(), Outcome::Accepted { remembered: false });
    }

    #[tokio::test]
    async fn emit_failure_means_no_channel() {
        let broker = ConfirmBroker::with_emitter(Arc::new(|_: &str, _: serde_json::Value| false));
        // 发送器在，但事件发不出去（窗口已销毁等）：仍然算"没有通道"
        assert!(broker.has_channel());
        assert_eq!(
            broker
                .request(req("t", "s", true), Duration::from_secs(1))
                .await,
            Outcome::NoChannel
        );
        assert!(broker.list().is_empty());
    }

    #[tokio::test]
    async fn abandon_all_rejects_waiters() {
        let (_, emit) = capture();
        let broker = Arc::new(ConfirmBroker::with_emitter(emit));
        let wait = {
            let b = Arc::clone(&broker);
            tokio::spawn(async move {
                b.request(req("t", "s", true), Duration::from_secs(30))
                    .await
            })
        };
        wait_pending(&broker).await;
        broker.abandon_all();
        // 停服 → 等待侧拿不到"确认"，必须是 NoChannel 而不是 Accepted
        assert_eq!(wait.await.unwrap(), Outcome::NoChannel);
        assert!(broker.list().is_empty());
    }
}
