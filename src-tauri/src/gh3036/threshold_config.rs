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
        }
    }

    pub fn evaluate(&self, value: u16, threshold: &ThresholdConfig) -> bool {
        match self {
            Self::Lt => threshold.value.map_or(false, |t| value < t),
            Self::Le => threshold.value.map_or(false, |t| value <= t),
            Self::Gt => threshold.value.map_or(false, |t| value > t),
            Self::Ge => threshold.value.map_or(false, |t| value >= t),
            Self::Eq => threshold.value.map_or(false, |t| value == t),
            Self::Ne => threshold.value.map_or(false, |t| value != t),
            Self::Range => threshold
                .range
                .map_or(false, |r| value >= r[0] && value <= r[1]),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThresholdConfig {
    pub operator: ThresholdOperator,
    pub value: Option<u16>,
    pub range: Option<[u16; 2]>,
    pub description: Option<String>,
}

impl ThresholdConfig {
    pub fn to_display_text(&self, unit: &str) -> String {
        match self.operator {
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
    pub global_threshold: Option<ThresholdConfig>,
    pub channel_rules: Option<Vec<ChannelRule>>,
}

fn default_enabled() -> bool {
    true
}

impl TestItemConfig {
    pub fn validate(&self, test_name: &str) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    pub value: u16,
    pub pass: bool,
    pub threshold_display: String,
    pub operator: String,
    pub threshold_value: Option<u16>,
    pub threshold_range: Option<[u16; 2]>,
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

    pub fn evaluate_channels(&mut self, values: &[u16], config: Option<&TestItemConfig>) {
        if !self.enabled {
            self.message = "Test disabled".to_string();
            return;
        }

        let unit = self.unit.as_deref().unwrap_or("");
        let mut all_pass = true;
        let mut failed_channels = Vec::new();

        for (idx, &value) in values.iter().enumerate() {
            let threshold = config
                .as_ref()
                .and_then(|c| c.find_threshold_for_channel(idx));

            let (pass, threshold_display, operator, threshold_value, threshold_range, description) =
                match threshold {
                    Some(t) => {
                        let p = t.operator.evaluate(value, &t);
                        let display = t.to_display_text(unit);
                        let op = t.operator.display_symbol().to_string();
                        (p, display, op, t.value, t.range, t.description.clone())
                    }
                    None => (
                        true,
                        "No threshold configured".to_string(),
                        String::new(),
                        None,
                        None,
                        None,
                    ),
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
            self.message = format!("All {} channels passed", values.len());
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStatus {
    pub enabled: bool,
    pub has_global_threshold: bool,
    pub channel_rules_count: usize,
}

impl Default for TestStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            has_global_threshold: false,
            channel_rules_count: 0,
        }
    }
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
    base_noise: &[u16],
    ppg_noise: &[u16],
    lpctr: &[u16],
    lplctr: &[u16],
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

pub const ERROR_CODE_CHIP_INIT: &str = "0x1001";
pub const ERROR_CODE_UUID: &str = "0x2001";
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

    let uuid_valid = !uuid.is_empty() && !uuid.iter().all(|&b| b == 0);
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
