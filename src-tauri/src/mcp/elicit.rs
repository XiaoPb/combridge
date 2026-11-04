//! 危险操作的客户端确认（docs/mcp-server-plan.md §8.3）。
//!
//! 标准档（`standard`）下，写设备与产测/烧录这类操作必须由**用户在客户端弹窗里点确认**，
//! 而不是靠模型自律。研发档（`dev`）跳过弹窗（内部联调时每次弹框会把人逼疯），但仍然记审计。
//!
//! 三种失败都必须**明确拒绝**，不能"问不到就当同意"：
//! - 用户点了拒绝 / 取消
//! - 客户端没声明 elicitation 能力（多数 CLI 客户端）
//! - 协议版本 ≥ 2026-07-28（规范禁止服务端向客户端发起请求）
//!
//! 全部映射为 `CONFIRM_REQUIRED`，`retryable: false` —— 重试只会再弹一次，没有意义。

use std::time::Duration;

use rmcp::RoleServer;
use rmcp::service::ElicitationError;
use rmcp::Peer;
use serde::{Deserialize, Serialize};

/// 等用户点按钮的上限。超时按"没确认"处理。
const CONFIRM_TIMEOUT: Duration = Duration::from_secs(120);

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
    /// 用户点了确认
    Accepted,
    /// 用户拒绝 / 取消 / 超时
    Declined(String),
    /// 没法问：客户端不支持 elicitation，或协议版本不允许
    Unsupported(String),
}

impl Decision {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted)
    }

    /// 给模型看的拒绝原因。
    pub fn reason(&self) -> String {
        match self {
            Self::Accepted => "已确认".to_string(),
            Self::Declined(why) => format!("用户未确认：{why}"),
            Self::Unsupported(why) => format!("无法向用户确认：{why}"),
        }
    }
}

/// 向客户端发起一次确认请求。
///
/// `message` 会被原样展示给用户，因此要写清"改什么、改哪个设备、改多少"，
/// 而不是写"确认执行吗"。
pub async fn ask(peer: &Peer<RoleServer>, message: impl Into<String>) -> Decision {
    let message = message.into();
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
        assert!(!Decision::Declined("x".into()).is_accepted());
        assert!(!Decision::Unsupported("x".into()).is_accepted());
    }

    #[test]
    fn reasons_are_model_readable() {
        assert_eq!(Decision::Declined("用户拒绝".into()).reason(), "用户未确认：用户拒绝");
        assert_eq!(
            Decision::Unsupported("客户端未声明 elicitation 能力".into()).reason(),
            "无法向用户确认：客户端未声明 elicitation 能力"
        );
    }

    /// 确认表单必须是 object schema，否则 elicitation 直接失败。
    #[test]
    fn confirm_form_schema_is_object() {
        let schema = serde_json::to_value(schemars::schema_for!(ConfirmForm)).unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["confirm"].is_object());
    }
}
