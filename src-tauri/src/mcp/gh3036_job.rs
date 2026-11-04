//! GH3036 产测的 MCP 侧视图：job 标识与结果汇总。
//!
//! 产测在 `FactoryTestManager` 里是「后台线程 + 状态机 + 事件」的异步模型，
//! 而 AI 需要的是「一次调用拿到结论」。本模块负责把两者对齐：
//!
//! - `FactoryJob` 记录一次 `action=run` 的上下文（job_id、起始时间），
//!   以便超时降级后模型能用 `action=status` 续等同一个 job；
//! - `build_summary` 把 `FactoryTestResult` + `FactoryEvaluationResult`
//!   压成扁平结论（pass / failed_item / items / csv_path），不暴露中间进度。

use serde::Serialize;

use crate::gh3036::types::FactoryComputeMode;
use crate::gh3036::{
    FactoryEvaluationResult, FactoryTestResult, FactoryTestStatus, FactoryTestStep,
};

/// 一次产测调用的上下文。
#[derive(Debug, Clone)]
pub struct FactoryJob {
    /// 形如 `ft_9f2c8a1d`
    pub job_id: String,
    /// 起始时间（毫秒时间戳）
    pub started_at: u64,
}

impl FactoryJob {
    pub fn new() -> Self {
        let raw = uuid::Uuid::new_v4().simple().to_string();
        Self {
            job_id: format!("ft_{}", &raw[..8]),
            started_at: now_millis(),
        }
    }
}

impl Default for FactoryJob {
    fn default() -> Self {
        Self::new()
    }
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 单个测试项（如 `HR_ACC`）的结论。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FactoryItemView {
    /// 测试项名称
    pub name: String,
    /// 该测试项是否启用卡控（未启用的项不参与判定的项）
    pub enabled: bool,
    /// 是否通过
    pub pass: bool,
    /// 判定说明，未通过时通常会写明超限原因
    pub message: String,
    /// 各通道的实测值与阈值
    pub channels: Vec<FactoryChannelView>,
}

/// 单个通道的实测值与阈值。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FactoryChannelView {
    /// 通道序号（从 0 开始）
    pub index: usize,
    /// 实测值（计算值优先，缺失时回退设备上报值）
    pub value: Option<f64>,
    /// 是否通过
    pub pass: bool,
    /// 阈值的人类可读表示，如 `<2`、`[1, 3]`
    pub threshold: String,
}

/// 产测结论汇总，即 `combridge_gh3036_factory_test` 的返回体。
#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct FactorySummary {
    /// 本次调用的 job 标识，超时降级后用它续等
    pub job_id: String,
    /// idle | running | waiting_environment_switch | completed | failed | stopped
    pub status: String,
    /// 当前/终止步骤，如 `lpctr`
    pub step: String,
    /// 总体判定；未完成时为 None
    pub pass: Option<bool>,
    /// 耗时（秒）；未完成时为 None
    pub duration_s: Option<u64>,
    /// 第一个未通过的测试项名称（便于模型一句话复述）
    pub failed_item: Option<String>,
    /// 各测试项结论
    pub items: Vec<FactoryItemView>,
    /// 项目名（来自卡控配置）
    pub project: Option<String>,
    /// 芯片信息
    pub device_info: Option<String>,
    /// UUID（十六进制）
    pub uuid: Option<String>,
    /// 计算方式：`mcu` | `app`
    pub compute_mode: Option<String>,
    /// 错误码串（逗号分隔）
    pub error_code: Option<String>,
    /// 产测结果 CSV 的落盘路径
    pub csv_path: Option<String>,
    /// GH3036 租约 ID（后续 continue / stop / rpc 需携带）
    pub lease_id: Option<String>,
    /// 面向模型的下一步提示（例如环境切换后如何继续）
    pub hint: Option<String>,
}

/// 状态枚举 → 稳定的 snake_case 字符串（与前端枚举的序列化保持一致）。
pub fn status_str(status: FactoryTestStatus) -> String {
    match status {
        FactoryTestStatus::Idle => "idle",
        FactoryTestStatus::Running => "running",
        FactoryTestStatus::WaitingForEnvironmentSwitch => "waiting_environment_switch",
        FactoryTestStatus::Completed => "completed",
        FactoryTestStatus::Failed => "failed",
        FactoryTestStatus::Stopped => "stopped",
    }
    .to_string()
}

/// 步骤枚举 → 稳定的 snake_case 字符串。
pub fn step_str(step: FactoryTestStep) -> String {
    match step {
        FactoryTestStep::Idle => "idle",
        FactoryTestStep::Prepare => "prepare",
        FactoryTestStep::ChipInit => "chip_init",
        FactoryTestStep::Uuid => "uuid",
        FactoryTestStep::BaseNoise => "base_noise",
        FactoryTestStep::PpgNoise => "ppg_noise",
        FactoryTestStep::Lpctr => "lpctr",
        FactoryTestStep::EnvironmentSwitch => "environment_switch",
        FactoryTestStep::Lplctr => "lplctr",
        FactoryTestStep::Cleanup => "cleanup",
        FactoryTestStep::Completed => "completed",
    }
    .to_string()
}

/// 汇总一次产测结果。
///
/// 优先用卡控评估结果（`FactoryEvaluationResult`）展开各项结论；没有评估结果时
/// 退回原始测量项，此时各项 `pass` 取自 `overall_result`，阈值信息为空。
pub fn build_summary(
    job: &FactoryJob,
    status: FactoryTestStatus,
    step: FactoryTestStep,
    result: Option<&FactoryTestResult>,
    evaluation: Option<&FactoryEvaluationResult>,
    csv_path: Option<String>,
    finished_at: Option<u64>,
) -> FactorySummary {
    let mut items = Vec::new();
    let mut failed_item = None;

    if let Some(eval) = evaluation {
        for t in &eval.test_results {
            if t.enabled && !t.pass && failed_item.is_none() {
                failed_item = Some(t.test_name.clone());
            }
            items.push(FactoryItemView {
                name: t.test_name.clone(),
                enabled: t.enabled,
                pass: t.pass,
                message: t.message.clone(),
                channels: t
                    .channel_results
                    .iter()
                    .map(|c| FactoryChannelView {
                        index: c.channel_index,
                        value: c.value,
                        pass: c.pass,
                        threshold: c.threshold_display.clone(),
                    })
                    .collect(),
            });
        }
    } else if let Some(r) = result {
        let overall_pass = r.overall_result.eq_ignore_ascii_case("PASS");
        for (name, measurements) in [
            ("base_noise", &r.base_noise),
            ("ppg_noise", &r.ppg_noise),
            ("lpctr", &r.lpctr),
            ("lplctr", &r.lplctr),
        ] {
            items.push(FactoryItemView {
                name: name.to_string(),
                enabled: true,
                pass: overall_pass,
                message: if overall_pass {
                    String::new()
                } else {
                    "未通过（无卡控明细，仅总体判定）".to_string()
                },
                channels: measurements
                    .iter()
                    .enumerate()
                    .map(|(i, m)| FactoryChannelView {
                        index: i,
                        value: m
                            .computed_value
                            .or_else(|| m.device_value.map(|v| v as f64)),
                        pass: overall_pass,
                        threshold: String::new(),
                    })
                    .collect(),
            });
        }
    }

    let pass = match status {
        FactoryTestStatus::Completed => Some(evaluation.map(|e| e.overall_pass).unwrap_or_else(
            || {
                result
                    .map(|r| r.overall_result.eq_ignore_ascii_case("PASS"))
                    .unwrap_or(false)
            },
        )),
        FactoryTestStatus::Failed => Some(false),
        _ => None,
    };

    let duration_s = finished_at
        .map(|end| end.saturating_sub(job.started_at) / 1000)
        .or_else(|| {
            if pass.is_some() {
                Some(now_millis().saturating_sub(job.started_at) / 1000)
            } else {
                None
            }
        });

    FactorySummary {
        job_id: job.job_id.clone(),
        status: status_str(status),
        step: step_str(step),
        pass,
        duration_s,
        failed_item,
        items,
        project: result.map(|r| r.project_name.clone()).filter(|s| !s.is_empty()),
        device_info: result
            .map(|r| r.device_info.clone())
            .filter(|s| !s.is_empty()),
        uuid: result.map(|r| hex_encode(&r.uuid)).filter(|s| !s.is_empty()),
        compute_mode: result.map(|r| match r.compute_mode {
            FactoryComputeMode::Mcu => "mcu".to_string(),
            FactoryComputeMode::App => "app".to_string(),
        }),
        error_code: result
            .map(|r| r.error_code.clone())
            .filter(|s| !s.is_empty()),
        csv_path,
        lease_id: None,
        hint: None,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(s, "{b:02X}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gh3036::{ChannelEvaluationResult, TestEvaluationResult};

    fn job() -> FactoryJob {
        FactoryJob {
            job_id: "ft_test01".to_string(),
            started_at: 1_000,
        }
    }

    #[test]
    fn test_status_and_step_strings() {
        assert_eq!(status_str(FactoryTestStatus::Completed), "completed");
        assert_eq!(
            status_str(FactoryTestStatus::WaitingForEnvironmentSwitch),
            "waiting_environment_switch"
        );
        assert_eq!(step_str(FactoryTestStep::EnvironmentSwitch), "environment_switch");
    }

    #[test]
    fn test_summary_from_evaluation_marks_failed_item() {
        let mut eval = FactoryEvaluationResult::new("GH3036");
        eval.overall_pass = false;
        let mut bad = TestEvaluationResult::new("SPO2_ACC", None);
        bad.enabled = true;
        bad.pass = false;
        bad.message = "3.1 超出阈值".to_string();
        bad.channel_results.push(ChannelEvaluationResult {
            channel_index: 0,
            value: Some(3.1),
            pass: false,
            threshold_display: "<2".to_string(),
            operator: "<".to_string(),
            threshold_value: Some(2.0),
            threshold_range: None,
            description: None,
        });
        eval.add_test_result(bad);
        let mut ok = TestEvaluationResult::new("HR_ACC", None);
        ok.enabled = true;
        eval.add_test_result(ok);

        let summary = build_summary(
            &job(),
            FactoryTestStatus::Completed,
            FactoryTestStep::Completed,
            None,
            Some(&eval),
            Some("D:/out/a.csv".to_string()),
            Some(43_000),
        );

        assert_eq!(summary.status, "completed");
        assert_eq!(summary.pass, Some(false));
        assert_eq!(summary.failed_item.as_deref(), Some("SPO2_ACC"));
        assert_eq!(summary.items.len(), 2);
        assert_eq!(summary.items[0].channels[0].threshold, "<2");
        assert_eq!(summary.duration_s, Some(42));
        assert_eq!(summary.csv_path.as_deref(), Some("D:/out/a.csv"));
    }

    #[test]
    fn test_summary_running_has_no_verdict() {
        let summary = build_summary(
            &job(),
            FactoryTestStatus::Running,
            FactoryTestStep::Lpctr,
            None,
            None,
            None,
            None,
        );
        assert_eq!(summary.status, "running");
        assert_eq!(summary.pass, None);
        assert_eq!(summary.duration_s, None);
        assert!(summary.items.is_empty());
    }

    #[test]
    fn test_hex_encode_upper() {
        assert_eq!(hex_encode(&[0x01, 0xab, 0xff]), "01ABFF");
    }
}
