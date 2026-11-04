use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThresholdOperator {
    #[default]
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    Range,
    /// 仅要求读到有效值，不比对数值大小。
    /// 用于量纲尚未标定或测试环境受限的测试项（如 MCU 模式下的 LPCTR），
    /// 待标定完成后应改回 `range` 恢复严格卡控。
    Present,
}

impl ThresholdOperator {
    pub fn display_symbol(&self) -> &'static str {
        match self {
            Self::Lt => "<",
            Self::Le => "≤",
            Self::Gt => ">",
            Self::Ge => "≥",
            Self::Eq => "=",
            Self::Ne => "≠",
            Self::Range => "∈",
            Self::Present => "present",
        }
    }

    pub fn evaluate(&self, value: f64, threshold: &ThresholdConfig) -> bool {
        match self {
            Self::Lt => threshold.value.is_some_and(|t| value < t),
            Self::Le => threshold.value.is_some_and(|t| value <= t),
            Self::Gt => threshold.value.is_some_and(|t| value > t),
            Self::Ge => threshold.value.is_some_and(|t| value >= t),
            Self::Eq => threshold.value == Some(value),
            Self::Ne => threshold.value.is_some_and(|t| value != t),
            Self::Range => threshold
                .range
                .is_some_and(|r| value >= r[0] && value <= r[1]),
            // 调用方用 `value.is_some_and(...)` 兜底，缺测仍判失败，
            // 这里只需放行任何已读到的数值。
            Self::Present => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThresholdConfig {
    pub operator: ThresholdOperator,
    pub value: Option<f64>,
    pub range: Option<[f64; 2]>,
    pub description: Option<String>,
}

impl ThresholdConfig {
    pub fn to_display_text(&self, unit: &str) -> String {
        match self.operator {
            ThresholdOperator::Present => "requires a reading".to_string(),
            ThresholdOperator::Range => {
                if let Some(range) = self.range {
                    format!("{} ≤ value ≤ {} {}", range[0], range[1], unit)
                } else {
                    String::new()
                }
            }
            _ => {
                if let Some(v) = self.value {
                    format!("value {} {} {}", self.operator.display_symbol(), v, unit)
                } else {
                    String::new()
                }
            }
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        match self.operator {
            // Present 不使用 value/range，无需额外字段。
            ThresholdOperator::Present => {}
            ThresholdOperator::Range => {
                if self.range.is_none() {
                    return Err("Range operator requires range field".to_string());
                }
                if let Some(r) = self.range {
                    if r[0] > r[1] {
                        return Err(format!("Invalid range: min({}) > max({})", r[0], r[1]));
                    }
                }
            }
            _ => {
                if self.value.is_none() {
                    return Err(format!("{:?} operator requires value field", self.operator));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelRule {
    pub channels: Vec<usize>,
    #[serde(flatten)]
    pub threshold: ThresholdConfig,
}

impl ChannelRule {
    pub fn validate(&self) -> Result<(), String> {
        if self.channels.is_empty() {
            return Err("channels list cannot be empty".to_string());
        }
        self.threshold.validate()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItemConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    pub description: Option<String>,
    pub unit: Option<String>,
    pub mode: Option<u8>,
    pub channels: Option<usize>,
    pub compute: Option<ComputeConfig>,
    pub global_threshold: Option<ThresholdConfig>,
    pub channel_rules: Option<Vec<ChannelRule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComputeConfig {
    pub sample_rate_hz: Option<f64>,
    pub min_number: Option<usize>,
    pub skip_number: Option<usize>,
    pub is_continuous: Option<bool>,
    pub timeout_ms: Option<u64>,
    pub gain_k: Option<f64>,
    pub led_current_ma: Option<f64>,
}

impl ComputeConfig {
    fn validate(&self, test_name: &str) -> Result<(), String> {
        if self.min_number.is_some_and(|value| value == 0) {
            return Err(format!(
                "{} compute.min_number must be greater than 0",
                test_name
            ));
        }
        if self.timeout_ms.is_some_and(|value| value == 0) {
            return Err(format!(
                "{} compute.timeout_ms must be greater than 0",
                test_name
            ));
        }
        if self.sample_rate_hz.is_some_and(|value| value <= 1.0) {
            return Err(format!(
                "{} compute.sample_rate_hz must be greater than 1.0",
                test_name
            ));
        }
        Ok(())
    }
}

fn default_enabled() -> bool {
    true
}

impl TestItemConfig {
    pub fn validate(&self, test_name: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(mode) = self.mode {
            if !matches!(mode, 1 | 2 | 4 | 8 | 16 | 32) {
                return Err(format!(
                    "{} mode must be one of 1, 2, 4, 8, 16, 32",
                    test_name
                ));
            }
        }

        // channels 可省略：真实产测配置常由设备侧配置决定通道数，
        // 运行期会退化为「按设备返回的采样数评估」。显式写 0 仍视为填错。
        if test_name != "chip_uid" && self.channels == Some(0) {
            return Err(format!(
                "{test_name} channels cannot be 0（省略该字段表示由设备返回的通道数决定）"
            ));
        }

        if let Some(compute) = &self.compute {
            compute.validate(test_name)?;
        }

        if let Some(gt) = &self.global_threshold {
            gt.validate()
                .map_err(|e| format!("{} global_threshold: {}", test_name, e))?;
        }

        if let Some(rules) = &self.channel_rules {
            for (i, rule) in rules.iter().enumerate() {
                rule.validate()
                    .map_err(|e| format!("{} channel_rules[{}]: {}", test_name, i, e))?;
            }
        }

        Ok(())
    }

    pub fn find_threshold_for_channel(&self, channel_index: usize) -> Option<ThresholdConfig> {
        if let Some(rules) = &self.channel_rules {
            for rule in rules {
                if rule.channels.contains(&channel_index) {
                    return Some(rule.threshold.clone());
                }
            }
        }
        self.global_threshold.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FailAction {
    #[default]
    Stop,
    Continue,
}

fn default_operator() -> ThresholdOperator {
    ThresholdOperator::Lt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_operator")]
    pub default_operator: ThresholdOperator,
    #[serde(default)]
    pub fail_action: FailAction,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            default_operator: default_operator(),
            fail_action: FailAction::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestsConfig {
    pub chip_init: Option<TestItemConfig>,
    pub chip_uid: Option<TestItemConfig>,
    pub base_noise: Option<TestItemConfig>,
    pub ppg_noise: Option<TestItemConfig>,
    pub lpctr: Option<TestItemConfig>,
    pub lplctr: Option<TestItemConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryThresholdConfig {
    pub project: String,
    pub version: String,
    pub description: Option<String>,
    pub chip: Option<String>,
    #[serde(default)]
    pub global: Option<GlobalConfig>,
    pub tests: TestsConfig,
}

impl FactoryThresholdConfig {
    pub fn from_yaml(yaml_str: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml_str).map_err(|e| format!("Failed to parse YAML: {}", e))
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
        Self::from_yaml(&content)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.project.is_empty() {
            return Err("project field is required".to_string());
        }
        if self.version.is_empty() {
            return Err("version field is required".to_string());
        }

        if let Some(config) = &self.tests.chip_init {
            config.validate("chip_init")?;
        }
        if let Some(config) = &self.tests.chip_uid {
            config.validate("chip_uid")?;
        }
        if let Some(config) = &self.tests.base_noise {
            config.validate("base_noise")?;
        }
        if let Some(config) = &self.tests.ppg_noise {
            config.validate("ppg_noise")?;
        }
        if let Some(config) = &self.tests.lpctr {
            config.validate("lpctr")?;
        }
        if let Some(config) = &self.tests.lplctr {
            config.validate("lplctr")?;
        }

        Ok(())
    }

    pub fn find_config_file(config_dir: &Path) -> Option<std::path::PathBuf> {
        if !config_dir.exists() {
            return None;
        }

        let entries = std::fs::read_dir(config_dir).ok()?;
        let mut matches: Vec<std::path::PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.starts_with("factory_config_") && file_name_str.ends_with(".yaml")
                {
                    matches.push(path);
                }
            }
        }

        if matches.len() == 1 {
            Some(matches.into_iter().next().unwrap())
        } else {
            None
        }
    }

    pub fn find_all_config_files(config_dir: &Path) -> Vec<std::path::PathBuf> {
        if !config_dir.exists() {
            return Vec::new();
        }

        let entries = match std::fs::read_dir(config_dir) {
            Ok(e) => e,
            Err(_) => return Vec::new(),
        };

        let mut matches: Vec<std::path::PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if file_name_str.starts_with("factory_config_") && file_name_str.ends_with(".yaml")
                {
                    matches.push(path);
                }
            }
        }

        matches
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelEvaluationResult {
    pub channel_index: usize,
    pub value: Option<f64>,
    pub pass: bool,
    pub threshold_display: String,
    pub operator: String,
    pub threshold_value: Option<f64>,
    pub threshold_range: Option<[f64; 2]>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestEvaluationResult {
    pub test_name: String,
    pub enabled: bool,
    pub pass: bool,
    pub channel_results: Vec<ChannelEvaluationResult>,
    pub message: String,
    pub description: Option<String>,
    pub unit: Option<String>,
}

impl TestEvaluationResult {
    pub fn new(test_name: &str, config: Option<&TestItemConfig>) -> Self {
        match config {
            Some(cfg) => Self {
                test_name: test_name.to_string(),
                enabled: cfg.enabled,
                pass: true,
                channel_results: Vec::new(),
                message: String::new(),
                description: cfg.description.clone(),
                unit: cfg.unit.clone(),
            },
            None => Self {
                test_name: test_name.to_string(),
                enabled: false,
                pass: true,
                channel_results: Vec::new(),
                message: "Test not configured".to_string(),
                description: None,
                unit: None,
            },
        }
    }

    pub fn evaluate_channels(&mut self, values: &[Option<f64>], config: Option<&TestItemConfig>) {
        if !self.enabled {
            self.message = "Test disabled".to_string();
            return;
        }

        let unit = self.unit.as_deref().unwrap_or("");
        // 没有任何实测值绝不能判合格：channels 省略时通道数由实测长度决定，
        // 空数组曾导致「0 通道全部通过」的假合格。
        if values.is_empty() {
            self.pass = false;
            self.message = "No measurements".to_string();
            return;
        }

        let mut all_pass = true;
        let mut failed_channels = Vec::new();
        // 只要存在非 present 的判定，就说明该项仍在做数值卡控。
        let mut presence_only = true;

        let channel_count = config
            .and_then(|item| item.channels)
            .unwrap_or(values.len())
            .max(values.len());

        for idx in 0..channel_count {
            let value = values.get(idx).copied().flatten();
            let threshold = config
                .as_ref()
                .and_then(|c| c.find_threshold_for_channel(idx));

            let (pass, threshold_display, operator, threshold_value, threshold_range, description) =
                match threshold {
                    Some(t) => {
                        let p =
                            value.is_some_and(|measurement| t.operator.evaluate(measurement, &t));
                        let display = t.to_display_text(unit);
                        let op = t.operator.display_symbol().to_string();
                        presence_only &= t.operator == ThresholdOperator::Present;
                        (p, display, op, t.value, t.range, t.description.clone())
                    }
                    None => {
                        presence_only = false;
                        (
                            value.is_some(),
                            "No threshold configured".to_string(),
                            String::new(),
                            None,
                            None,
                            None,
                        )
                    }
                };

            if !pass {
                all_pass = false;
                failed_channels.push(idx);
            }

            self.channel_results.push(ChannelEvaluationResult {
                channel_index: idx,
                value,
                pass,
                threshold_display,
                operator,
                threshold_value,
                threshold_range,
                description,
            });
        }

        self.pass = all_pass;
        if all_pass {
            self.message = if presence_only {
                format!(
                    "All {} channels reported a reading (presence-only, no numeric limit)",
                    values.len()
                )
            } else {
                format!("All {} channels passed", values.len())
            };
        } else {
            self.message = format!("Failed channels: {:?}", failed_channels);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryEvaluationResult {
    pub overall_pass: bool,
    pub project: String,
    pub test_results: Vec<TestEvaluationResult>,
    pub timestamp: u64,
}

impl FactoryEvaluationResult {
    pub fn new(project: &str) -> Self {
        Self {
            overall_pass: true,
            project: project.to_string(),
            test_results: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }

    pub fn add_test_result(&mut self, result: TestEvaluationResult) {
        if !result.pass && result.enabled {
            self.overall_pass = false;
        }
        self.test_results.push(result);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestStatus {
    pub enabled: bool,
    pub has_global_threshold: bool,
    pub channel_rules_count: usize,
}

impl From<Option<&TestItemConfig>> for TestStatus {
    fn from(config: Option<&TestItemConfig>) -> Self {
        match config {
            Some(cfg) => Self {
                enabled: cfg.enabled,
                has_global_threshold: cfg.global_threshold.is_some(),
                channel_rules_count: cfg.channel_rules.as_ref().map_or(0, |r| r.len()),
            },
            None => Self::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TestsStatus {
    pub base_noise: TestStatus,
    pub ppg_noise: TestStatus,
    pub lpctr: TestStatus,
    pub lplctr: TestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfigValidation {
    pub is_valid: bool,
    pub file_path: Option<String>,
    pub project: Option<String>,
    pub version: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub tests_status: TestsStatus,
}

impl ThresholdConfigValidation {
    pub fn from_config(config: &FactoryThresholdConfig, file_path: Option<&Path>) -> Self {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        if let Err(e) = config.validate() {
            errors.push(e);
        }

        if config.tests.base_noise.is_none() {
            warnings.push("base_noise test not configured".to_string());
        }
        if config.tests.ppg_noise.is_none() {
            warnings.push("ppg_noise test not configured".to_string());
        }
        if config.tests.lpctr.is_none() {
            warnings.push("lpctr test not configured".to_string());
        }
        if config.tests.lplctr.is_none() {
            warnings.push("lplctr test not configured".to_string());
        }

        Self {
            is_valid: errors.is_empty(),
            file_path: file_path.map(|p| p.to_string_lossy().to_string()),
            project: Some(config.project.clone()),
            version: Some(config.version.clone()),
            errors,
            warnings,
            tests_status: TestsStatus {
                base_noise: TestStatus::from(config.tests.base_noise.as_ref()),
                ppg_noise: TestStatus::from(config.tests.ppg_noise.as_ref()),
                lpctr: TestStatus::from(config.tests.lpctr.as_ref()),
                lplctr: TestStatus::from(config.tests.lplctr.as_ref()),
            },
        }
    }

    pub fn from_error(error: String, file_path: Option<&Path>) -> Self {
        Self {
            is_valid: false,
            file_path: file_path.map(|p| p.to_string_lossy().to_string()),
            project: None,
            version: None,
            errors: vec![error],
            warnings: Vec::new(),
            tests_status: TestsStatus::default(),
        }
    }
}

pub fn validate_threshold_config_file(config_dir: &Path) -> ThresholdConfigValidation {
    match FactoryThresholdConfig::find_config_file(config_dir) {
        Some(path) => {
            info!("[ThresholdConfig] Found config file: {:?}", path);
            match FactoryThresholdConfig::from_file(&path) {
                Ok(config) => {
                    info!(
                        "[ThresholdConfig] Loaded config for project: {}",
                        config.project
                    );
                    ThresholdConfigValidation::from_config(&config, Some(&path))
                }
                Err(e) => {
                    error!("[ThresholdConfig] Failed to load config: {}", e);
                    ThresholdConfigValidation::from_error(e, Some(&path))
                }
            }
        }
        None => {
            let all_configs = FactoryThresholdConfig::find_all_config_files(config_dir);
            if all_configs.is_empty() {
                let msg = format!("No factory_config_*.yaml found in {:?}", config_dir);
                warn!("[ThresholdConfig] {}", msg);
                ThresholdConfigValidation::from_error(msg, None)
            } else {
                let msg = format!(
                    "Found {} factory_config_*.yaml files, expected exactly 1: {:?}",
                    all_configs.len(),
                    all_configs
                );
                warn!("[ThresholdConfig] {}", msg);
                ThresholdConfigValidation::from_error(msg, None)
            }
        }
    }
}

pub fn evaluate_test_data(
    config: &FactoryThresholdConfig,
    base_noise: &[Option<f64>],
    ppg_noise: &[Option<f64>],
    lpctr: &[Option<f64>],
    lplctr: &[Option<f64>],
) -> FactoryEvaluationResult {
    let mut result = FactoryEvaluationResult::new(&config.project);

    let mut base_noise_result =
        TestEvaluationResult::new("base_noise", config.tests.base_noise.as_ref());
    base_noise_result.evaluate_channels(base_noise, config.tests.base_noise.as_ref());
    result.add_test_result(base_noise_result);

    let mut ppg_noise_result =
        TestEvaluationResult::new("ppg_noise", config.tests.ppg_noise.as_ref());
    ppg_noise_result.evaluate_channels(ppg_noise, config.tests.ppg_noise.as_ref());
    result.add_test_result(ppg_noise_result);

    let mut lpctr_result = TestEvaluationResult::new("lpctr", config.tests.lpctr.as_ref());
    lpctr_result.evaluate_channels(lpctr, config.tests.lpctr.as_ref());
    result.add_test_result(lpctr_result);

    let mut lplctr_result = TestEvaluationResult::new("lplctr", config.tests.lplctr.as_ref());
    lplctr_result.evaluate_channels(lplctr, config.tests.lplctr.as_ref());
    result.add_test_result(lplctr_result);

    result
}

pub fn evaluate_measurements(
    test_name: &str,
    config: Option<&TestItemConfig>,
    values: &[Option<f64>],
) -> TestEvaluationResult {
    let mut result = TestEvaluationResult::new(test_name, config);
    result.evaluate_channels(values, config);
    result
}

pub fn evaluate_test_item(
    test_name: &str,
    config: Option<&TestItemConfig>,
    values: &[u16],
) -> TestEvaluationResult {
    let measurements: Vec<Option<f64>> = values.iter().copied().map(f64::from).map(Some).collect();
    evaluate_measurements(test_name, config, &measurements)
}

pub const ERROR_CODE_CHIP_INIT: &str = "0x1001";
pub const ERROR_CODE_UUID: &str = "0x2001";
/// 历史错误码：旧逻辑把 UUID 拆成前后两半分别判定，后半段全零时报此码。
/// 现行口径为「长度非 0 且值不全为 0」，不会再产生该码；
/// 常量保留，避免已落盘的历史 CSV / 外部解析失效。
pub const ERROR_CODE_UUID_CHANNEL_1: &str = "0x2002";
pub const ERROR_CODE_BASE_NOISE_BASE: u32 = 0x3000;
pub const ERROR_CODE_PPG_NOISE_BASE: u32 = 0x4000;
pub const ERROR_CODE_LPCTR_BASE: u32 = 0x5000;
pub const ERROR_CODE_LPLCTR_BASE: u32 = 0x6000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactoryErrorResult {
    pub error_codes: Vec<String>,
    pub has_error: bool,
}

pub fn generate_error_codes(
    chip_init_status: u16,
    uuid: &[u8],
    evaluation_result: &FactoryEvaluationResult,
) -> FactoryErrorResult {
    let mut error_codes = Vec::new();
    let mut has_error = false;

    if chip_init_status != 1 {
        error_codes.push(ERROR_CODE_CHIP_INIT.to_string());
        has_error = true;
    }

    // 与 UUID 步骤同口径：长度非 0 且值不全为 0 即视为读到有效 UUID。
    // 固件（含桩实现）只填充自身长度，后半段补零属正常，
    // 按两半分别判定会把合格片误报成 0x2002。
    let uuid_valid = !uuid.is_empty() && uuid.iter().any(|&byte| byte != 0);
    if !uuid_valid {
        error_codes.push(ERROR_CODE_UUID.to_string());
        has_error = true;
    }

    for test_result in &evaluation_result.test_results {
        if !test_result.pass && test_result.enabled {
            let base_code = match test_result.test_name.as_str() {
                "base_noise" => ERROR_CODE_BASE_NOISE_BASE,
                "ppg_noise" => ERROR_CODE_PPG_NOISE_BASE,
                "lpctr" => ERROR_CODE_LPCTR_BASE,
                "lplctr" => ERROR_CODE_LPLCTR_BASE,
                _ => continue,
            };

            for channel_result in &test_result.channel_results {
                if !channel_result.pass {
                    let error_code = format!(
                        "0x{:04X}",
                        base_code + channel_result.channel_index as u32 + 1
                    );
                    error_codes.push(error_code);
                }
            }
            has_error = true;
        }
    }

    FactoryErrorResult {
        error_codes,
        has_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config_with_threshold(enabled: bool, maximum: u16) -> TestItemConfig {
        TestItemConfig {
            enabled,
            description: Some("测试项".to_string()),
            unit: Some("LSB".to_string()),
            mode: Some(4),
            channels: None,
            compute: None,
            global_threshold: Some(ThresholdConfig {
                operator: ThresholdOperator::Lt,
                value: Some(f64::from(maximum)),
                range: None,
                description: None,
            }),
            channel_rules: None,
        }
    }

    fn test_config(channels: usize) -> TestItemConfig {
        TestItemConfig {
            enabled: true,
            description: None,
            unit: Some("nA/mA".to_string()),
            mode: Some(16),
            channels: Some(channels),
            compute: None,
            global_threshold: Some(ThresholdConfig {
                operator: ThresholdOperator::Lt,
                value: Some(100.0),
                range: None,
                description: None,
            }),
            channel_rules: None,
        }
    }

    #[test]
    fn evaluate_test_item_passes_enabled_values() {
        let config = test_config_with_threshold(true, 100);

        let result = evaluate_test_item("base_noise", Some(&config), &[10, 99]);

        assert!(result.enabled);
        assert!(result.pass);
        assert_eq!(result.channel_results.len(), 2);
    }

    #[test]
    fn evaluate_test_item_reports_failed_channels() {
        let config = test_config_with_threshold(true, 100);

        let result = evaluate_test_item("base_noise", Some(&config), &[10, 100, 120]);

        assert!(!result.pass);
        assert!(result.channel_results[0].pass);
        assert!(!result.channel_results[1].pass);
        assert!(!result.channel_results[2].pass);
    }

    #[test]
    fn evaluate_test_item_marks_disabled_without_channels() {
        let config = test_config_with_threshold(false, 100);

        let result = evaluate_test_item("base_noise", Some(&config), &[120]);

        assert!(!result.enabled);
        assert!(result.pass);
        assert!(result.channel_results.is_empty());
        assert_eq!(result.message, "Test disabled");
    }

    #[test]
    fn app_compute_config_requires_channels_and_gain_for_raw_ctr_fallback() {
        let config = FactoryThresholdConfig::from_yaml(
            r#"
project: GH3036
version: "2"
chip: gh3036
tests:
  chip_init:
    enabled: true
    mode: 1
    channels: 1
    unit: status
    global_threshold: { operator: eq, value: 1 }
  lpctr:
    enabled: true
    mode: 16
    channels: 2
    compute: { gain_k: 100.0, led_current_ma: 20.0 }
    global_threshold: { operator: ge, value: 0.1 }
"#,
        )
        .unwrap();
        assert!(config.validate().is_ok());
    }

    /// 真实产测配置（Goodix l-evk-t2-gh3038-q）不写 channels：通道数由设备侧配置决定，
    /// 校验不能因此拒掉整份配置。
    #[test]
    fn config_without_channels_is_valid() {
        let config = FactoryThresholdConfig::from_yaml(
            r#"
project: GH3036
version: "1.0"
description: GH3036
global: { default_operator: lt, fail_action: stop }
tests:
  base_noise:
    enabled: true
    unit: uV
    global_threshold: { operator: lt, value: 95 }
  ppg_noise:
    enabled: true
    unit: uV
    global_threshold: { operator: lt, value: 280 }
  lpctr:
    enabled: true
    unit: nA/mA
    global_threshold: { operator: range, range: [100, 3000] }
  lplctr:
    enabled: true
    unit: nA/mA
    global_threshold: { operator: range, range: [0, 6] }
"#,
        )
        .unwrap();

        assert!(config.validate().is_ok());
    }

    /// 省略 channels 表示「按设备返回的通道数」，但显式写 0 仍是填错。
    #[test]
    fn explicit_zero_channels_is_rejected() {
        let mut config = test_config(8);
        config.channels = Some(0);
        let err = config.validate("base_noise").unwrap_err();
        assert!(err.contains("cannot be 0"), "unexpected: {err}");
    }

    /// channels 省略时按实测通道数评估，不因缺少该字段漏判。
    #[test]
    fn evaluation_without_channels_uses_measured_length() {
        let mut config = test_config_with_threshold(true, 100);
        config.channels = None;

        let result = evaluate_test_item("base_noise", Some(&config), &[10, 999]);

        assert_eq!(result.channel_results.len(), 2);
        assert!(!result.pass, "超阈值通道必须判失败");
        assert!(result.channel_results[0].pass);
        assert!(!result.channel_results[1].pass);
    }

    /// present 算子只要求读到值：任何数值都放行，缺测仍判失败。
    #[test]
    fn present_operator_passes_any_reading_but_not_absence() {
        let mut config = test_config_with_threshold(true, 100);
        config.unit = Some("nA/mA".to_string());
        config.global_threshold = Some(ThresholdConfig {
            operator: ThresholdOperator::Present,
            value: None,
            range: None,
            description: Some("测试环境受限，仅校验有读数".to_string()),
        });

        // 实测 0~5 远低于原 range 下限，present 下应判合格
        let result = evaluate_test_item("lpctr", Some(&config), &[0, 1, 2, 5, 0, 0, 0, 0]);
        assert!(result.pass);
        assert_eq!(result.channel_results.len(), 8);
        assert!(result.message.contains("presence-only"));

        // 一个通道都没读到 → 不合格
        let empty = evaluate_test_item("lpctr", Some(&config), &[]);
        assert!(!empty.pass);
        assert_eq!(empty.message, "No measurements");

        // present 不需要 value/range 字段
        assert!(config.global_threshold.as_ref().unwrap().validate().is_ok());
    }

    /// 一条实测值都没有时必须判失败——这是产测里最危险的一种假合格。
    #[test]
    fn empty_measurements_must_fail() {
        let mut config = test_config_with_threshold(true, 100);
        config.channels = None;

        let result = evaluate_test_item("base_noise", Some(&config), &[]);

        assert!(!result.pass, "空测量不能判合格");
        assert!(result.channel_results.is_empty());
        assert_eq!(result.message, "No measurements");
    }

    #[test]
    fn missing_measurement_is_a_failed_configured_channel() {
        let result = evaluate_measurements("lpctr", Some(&test_config(2)), &[None, Some(2.5)]);
        assert!(!result.pass);
        assert!(!result.channel_results[0].pass);
        assert_eq!(result.channel_results[0].value, None);
    }

    #[test]
    fn uuid_error_codes_flag_missing_reading_only() {
        let evaluation_result = FactoryEvaluationResult::new("GH3036");

        // 全 0：没读到 UUID
        let all_zero = generate_error_codes(1, &[0; 32], &evaluation_result);
        assert!(all_zero.error_codes.contains(&ERROR_CODE_UUID.to_string()));
        assert!(!all_zero
            .error_codes
            .contains(&ERROR_CODE_UUID_CHANNEL_1.to_string()));

        // 长度 0：同样视为没读到
        let empty = generate_error_codes(1, &[], &evaluation_result);
        assert!(empty.error_codes.contains(&ERROR_CODE_UUID.to_string()));
    }

    /// 固件桩只填自身长度（0xAA），后半段补零属正常，不得误报。
    #[test]
    fn uuid_error_codes_accept_partial_readings() {
        let evaluation_result = FactoryEvaluationResult::new("GH3036");

        let mut stub = vec![0; 32];
        stub[..16].fill(0xAA);
        let result = generate_error_codes(1, &stub, &evaluation_result);
        assert!(!result.error_codes.contains(&ERROR_CODE_UUID.to_string()));
        assert!(!result
            .error_codes
            .contains(&ERROR_CODE_UUID_CHANNEL_1.to_string()));
        assert!(
            !result.has_error,
            "chip_init 正常且 UUID 有效时不应有错误码"
        );

        // 任意一个非零字节即可
        let mut single = vec![0; 32];
        single[31] = 7;
        assert!(!generate_error_codes(1, &single, &evaluation_result).has_error);
    }
}
