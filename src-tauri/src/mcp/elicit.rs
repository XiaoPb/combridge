//! 危险操作的客户端确认（docs/mcp-server-plan.md §8.3）。
//!
//! 标准档（`standard`）下，写设备与产测/烧录这类操作必须由**用户点确认**，
//! 而不是靠模型自律。研发档（`dev`）跳过弹窗（内部联调时每次弹框会把人逼疯），
//! 但仍然记审计。
//!
//! 确认走两条通道，优先级从高到低：
//!
//! 1. **MCP elicitation** —— 客户端在 `initialize` 里声明了 `elicitation` 能力时，
//!    弹窗由客户端渲染，体验最自然。
//! 2. **本机窗口**（[`super::confirm`]）—— 客户端没声明能力时（WorkBuddy Connector
//!    5.9.3 等桌面宿主就是这种），改由 ComBridge 自己的窗口弹确认框。协议外通道，
//!    但不依赖客户端配合，实测可用。
//!
//! 三种情况仍然**明确拒绝**，不能"问不到就当同意"：
//! - 用户点了拒绝 / 取消
//! - 两条通道都不可用（无 GUI、事件发不出去、服务正在停）
//! - 等满时限没人应答
//!
//! 全部映射为 `CONFIRM_REQUIRED`，`retryable: false` —— 重试只会再弹一次，没有意义。

use std::time::Duration;

use rmcp::service::ElicitationError;
use rmcp::Peer;
use rmcp::RoleServer;
use serde::{Deserialize, Serialize};

use super::confirm::{ConfirmBroker, Outcome, Request};

/// 等用户点按钮的上限。超时按"没确认"处理。
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(120);

/// 确认表单。字段越少越好：客户端只会渲染一个布尔勾选框。
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ConfirmForm {
    /// 勾选表示确认执行；不勾选或关闭弹窗都视为拒绝
    pub confirm: bool,
}

rmcp::elicit_safe!(ConfirmForm);

/// 确认结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// 用户在客户端 elicitation 弹窗里点了确认
    Accepted,
    /// 用户在本机窗口里点了确认
    AcceptedLocally,
    /// 该操作在本轮会话里已被放行过（用户勾了"记住"），这次不再打扰
    Granted,
    /// 用户拒绝 / 取消 / 超时
    Declined(String),
    /// 没法问：客户端不支持 elicitation，本机窗口也不可用
    Unsupported(String),
}

impl Decision {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted | Self::AcceptedLocally | Self::Granted)
    }

    /// 给模型看的拒绝原因。
    pub fn reason(&self) -> String {
        match self {
            Self::Accepted | Self::AcceptedLocally | Self::Granted => "已确认".to_string(),
            Self::Declined(why) => format!("用户未确认：{why}"),
            Self::Unsupported(why) => format!("无法向用户确认：{why}"),
        }
    }

    /// 确认通道名字（写日志 / 排障用）。
    pub fn channel(&self) -> &'static str {
        match self {
            Self::Accepted | Self::Declined(_) | Self::Unsupported(_) => "mcp-elicitation",
            Self::AcceptedLocally => "local-window",
            Self::Granted => "session-grant",
        }
    }
}

/// 客户端有没有声明 elicitation 能力。
///
/// 先探测再发请求：省掉一次注定失败的往返，日志里也能直接看出是"客户端不支持"
/// 还是"用户拒绝了"。
fn client_supports_elicitation(peer: &Peer<RoleServer>) -> bool {
    peer.peer_info()
        .map(|info| info.capabilities.elicitation.is_some())
        .unwrap_or(false)
}

/// 向用户要一次确认。
///
/// `req.detail` 会被原样展示，因此要写清"改什么、改哪个设备、改多少"，
/// 而不是写"确认执行吗"。
pub async fn ask(peer: &Peer<RoleServer>, broker: &ConfirmBroker, req: Request<'_>) -> Decision {
    // 用户之前勾过"本次会话内放行"，同工具同目标不再打扰
    if broker.is_granted(req.tool, req.scope) {
        tracing::debug!(
            "[MCP] {} @ {} 命中会话内授权，跳过确认",
            req.tool,
            req.scope_label
        );
        return Decision::Granted;
    }

    let note = if client_supports_elicitation(peer) {
        let message = format!("【{}】{}\n确认执行？", req.risk, req.detail);
        match elicit_once(peer, message).await {
            // 用户明确拒绝：尊重这一票，不再拿本机窗口追问一遍
            Decision::Declined(why) => return Decision::Declined(why),
            Decision::Accepted => return Decision::Accepted,
            Decision::Unsupported(why) => why,
            // elicit_once 不会返回这些分支
            Decision::AcceptedLocally | Decision::Granted => return Decision::Accepted,
        }
    } else {
        "客户端未声明 elicitation 能力".to_string()
    };
    tracing::info!(
        "[MCP] {note}，改走本机窗口确认（tool={}, scope={}）",
        req.tool,
        req.scope_label
    );

    match broker.request(req, CONFIRM_TIMEOUT).await {
        Outcome::Accepted { remembered } => {
            if remembered {
                tracing::info!("[MCP] 本次确认同时生成会话内授权");
            }
            Decision::AcceptedLocally
        }
        Outcome::Declined => Decision::Declined("用户在 ComBridge 窗口中取消".to_string()),
        Outcome::Timeout => Decision::Declined(format!(
            "等待本机确认超时（{}s 内未在 ComBridge 窗口中点确认）",
            CONFIRM_TIMEOUT.as_secs()
        )),
        Outcome::NoChannel => Decision::Unsupported(format!(
            "{note}，且本机 ComBridge 窗口不可用（应用未运行或窗口已关闭）"
        )),
    }
}

/// 单次 elicitation 调用。
async fn elicit_once(peer: &Peer<RoleServer>, message: String) -> Decision {
    match peer
        .elicit_with_timeout::<ConfirmForm>(message, Some(CONFIRM_TIMEOUT))
        .await
    {
        Ok(Some(form)) if form.confirm => Decision::Accepted,
        Ok(Some(_)) => Decision::Declined("用户未勾选确认项".to_string()),
        Ok(None) => Decision::Declined("用户未提交表单".to_string()),
        Err(ElicitationError::UserDeclined) => Decision::Declined("用户拒绝".to_string()),
        Err(ElicitationError::UserCancelled) => Decision::Declined("用户取消".to_string()),
        Err(ElicitationError::NoContent) => Decision::Declined("用户未提交内容".to_string()),
        Err(ElicitationError::CapabilityNotSupported) => {
            Decision::Unsupported("客户端未声明 elicitation 能力".to_string())
        }
        Err(ElicitationError::ParseError { error, .. }) => {
            Decision::Declined(format!("确认结果无法解析: {error}"))
        }
        Err(e) => Decision::Unsupported(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_flag() {
        assert!(Decision::Accepted.is_accepted());
        assert!(Decision::AcceptedLocally.is_accepted());
        assert!(Decision::Granted.is_accepted());
        assert!(!Decision::Declined("x".into()).is_accepted());
        assert!(!Decision::Unsupported("x".into()).is_accepted());
    }

    #[test]
    fn reasons_are_model_readable() {
        assert_eq!(
            Decision::Declined("用户拒绝".into()).reason(),
            "用户未确认：用户拒绝"
        );
        assert_eq!(
            Decision::Unsupported("客户端未声明 elicitation 能力".into()).reason(),
            "无法向用户确认：客户端未声明 elicitation 能力"
        );
        // 已确认的两种通道都返回同一句话，模型侧不区分
        assert_eq!(Decision::AcceptedLocally.reason(), "已确认");
    }

    #[test]
    fn channels_are_distinguishable_for_audit() {
        assert_eq!(Decision::Accepted.channel(), "mcp-elicitation");
        assert_eq!(Decision::AcceptedLocally.channel(), "local-window");
        assert_eq!(Decision::Granted.channel(), "session-grant");
    }

    /// 确认表单必须是 object schema，否则 elicitation 直接失败。
    #[test]
    fn confirm_form_schema_is_object() {
        let schema = serde_json::to_value(schemars::schema_for!(ConfirmForm)).unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["confirm"].is_object());
    }

    /// 本机通道不可用时必须是"明确拒绝"，不能放行。
    #[tokio::test]
    async fn no_channel_is_a_refusal() {
        let broker = ConfirmBroker::headless();
        let outcome = broker
            .request(
                Request {
                    tool: "t",
                    risk: "r",
                    scope: "s",
                    scope_label: "s",
                    detail: "d",
                    grantable: true,
                },
                Duration::from_millis(10),
            )
            .await;
        assert_eq!(outcome, Outcome::NoChannel);
    }
}
