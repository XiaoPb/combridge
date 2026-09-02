//! GH3036 产测流程管理模块
//!
//! 本模块实现 GH3036 芯片的产测流程管理：
//! - 配置文件校验
//! - 自动化产测流程执行
//! - 测试结果保存

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use chrono::Local;
use parking_lot::Mutex;
use tokio::runtime::Runtime;
use tracing::{error, info, warn};

use super::config_loader::ConfigLoader;
use super::factory_compute::{CollectionSpec, CollectedFrames, FactoryFrameCollector};
use super::manager::Gh3036Manager;
use super::threshold_config::{
    evaluate_measurements, evaluate_test_item, generate_error_codes,
    validate_threshold_config_file, FactoryEvaluationResult, FactoryThresholdConfig, FailAction,
    TestItemConfig, ThresholdConfigValidation,
};
use super::types::{
    ChannelMeasurement, ConfigValidationResult, FactoryComputeMode, FactoryTestProgressEvent,
    FactoryTestResult, FactoryTestStatus, FactoryTestStep, FactoryTestStepResult, GhFuncFrame,
};
use crate::service::EventBus;

pub struct FactoryTestManager {
    config_dir: Mutex<PathBuf>,
    status: Arc<Mutex<FactoryTestStatus>>,
    current_step: Arc<Mutex<FactoryTestStep>>,
    result: Arc<Mutex<Option<FactoryTestResult>>>,
    running: Arc<AtomicBool>,
    event_bus: Arc<EventBus>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
    threshold_config: Mutex<Option<FactoryThresholdConfig>>,
    evaluation_result: Arc<Mutex<Option<FactoryEvaluationResult>>>,
    frame_collector: Arc<Mutex<FactoryFrameCollector>>,
}

#[cfg(test)]
mod collector_lifecycle_tests {
    use std::sync::Arc;

    use super::super::types::{GhFuncFixIdx, GhFuncFrame};
    use super::*;
    use crate::service::EventBus;

    fn test1_frame(frame_cnt: u32) -> GhFuncFrame {
        GhFuncFrame {
            frame_cnt,
            id: GhFuncFixIdx::AlgoMax,
            ch_num: 0,
            data: Vec::new(),
            ..GhFuncFrame::default()
        }
    }

    fn spo2_frame(frame_cnt: u32) -> GhFuncFrame {
        GhFuncFrame {
            frame_cnt,
            id: GhFuncFixIdx::Spo2,
            ch_num: 0,
            data: Vec::new(),
            ..GhFuncFrame::default()
        }
    }

    #[test]
    fn collector_records_only_active_test1_frames_and_clears_between_steps() {
        let manager = FactoryTestManager::new(Arc::new(EventBus::new(8)));

        manager.start_frame_collection(CollectionSpec::ctr_defaults());
        manager.record_test1_frame(&test1_frame(1));
        manager.record_test1_frame(&spo2_frame(2));

        assert_eq!(manager.finish_frame_collection().frame_cnts, vec![1]);
        assert!(manager.finish_frame_collection().frame_cnts.is_empty());
    }
}

// SAFETY: All fields use parking_lot::Mutex, Arc, or AtomicBool which are Send+Sync.
unsafe impl Send for FactoryTestManager {}
unsafe impl Sync for FactoryTestManager {}

impl FactoryTestManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let config_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .map(|exe_dir| exe_dir.join("config").join("factory"))
            .unwrap_or_else(|| PathBuf::from("config/factory"));

        Self {
            config_dir: Mutex::new(config_dir),
            status: Arc::new(Mutex::new(FactoryTestStatus::Idle)),
            current_step: Arc::new(Mutex::new(FactoryTestStep::Idle)),
            result: Arc::new(Mutex::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            event_bus,
            thread_handle: Mutex::new(None),
            threshold_config: Mutex::new(None),
            evaluation_result: Arc::new(Mutex::new(None)),
            frame_collector: Arc::new(Mutex::new(FactoryFrameCollector::default())),
        }
    }

    pub fn set_config_dir(&self, dir: PathBuf) {
        info!("[FactoryTest] 配置目录设置为: {}", dir.display());
        let mut config_dir = self.config_dir.lock();
        *config_dir = dir.clone();
        drop(config_dir);

        {
            let mut threshold_config = self.threshold_config.lock();
            *threshold_config = None;
        }

        match FactoryThresholdConfig::find_config_file(&dir) {
            Some(config_file) => match FactoryThresholdConfig::from_file(&config_file) {
                Ok(config) => {
                    info!(
                        "[FactoryTest] 加载卡控配置成功: {} (项目: {})",
                        config_file.display(),
                        config.project
                    );
                    let mut threshold_config = self.threshold_config.lock();
                    *threshold_config = Some(config);
                }
                Err(e) => {
                    error!("[FactoryTest] 加载卡控配置失败: {}", e);
                }
            },
            None => {
                let all_configs = FactoryThresholdConfig::find_all_config_files(&dir);
                if all_configs.is_empty() {
                    info!("[FactoryTest] 未找到卡控配置文件 (factory_config_*.yaml)");
                } else if all_configs.len() > 1 {
                    warn!(
                        "[FactoryTest] 找到多个卡控配置文件，请确保只有一个: {:?}",
                        all_configs
                    );
                }
            }
        }
    }

    pub fn get_threshold_config(&self) -> Option<FactoryThresholdConfig> {
        self.threshold_config.lock().clone()
    }

    pub fn get_evaluation_result(&self) -> Option<FactoryEvaluationResult> {
        self.evaluation_result.lock().clone()
    }

    pub fn start_frame_collection(&self, spec: CollectionSpec) {
        self.frame_collector.lock().start(spec);
    }

    pub fn record_test1_frame(&self, frame: &GhFuncFrame) {
        self.frame_collector.lock().push_frame(frame.clone());
    }

    pub fn finish_frame_collection(&self) -> CollectedFrames {
        self.frame_collector.lock().finish()
    }

    fn reset_frame_collection(&self) {
        let _ = self.finish_frame_collection();
    }

    pub fn validate_threshold_config(&self) -> ThresholdConfigValidation {
        let config_dir = self.config_dir.lock();
        validate_threshold_config_file(&config_dir)
    }

    pub fn get_config_dir(&self) -> PathBuf {
        self.config_dir.lock().clone()
    }

    pub fn get_status(&self) -> FactoryTestStatus {
        *self.status.lock()
    }

    pub fn get_current_step(&self) -> FactoryTestStep {
        *self.current_step.lock()
    }

    pub fn get_result(&self) -> Option<FactoryTestResult> {
        self.result.lock().clone()
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn find_config_file(dir: &Path, pattern: &str) -> Result<Option<PathBuf>, String> {
        if !dir.exists() {
            return Err(format!("配置目录不存在: {}", dir.display()));
        }

        let pattern_lower = pattern.to_lowercase();
        let entries = std::fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))?;

        let mut matches: Vec<PathBuf> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy().to_lowercase();
                if file_name_str.contains(&pattern_lower) {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy().to_lowercase();
                        if ext_str.ends_with("config") || ext_str == "ini" {
                            matches.push(path);
                        }
                    }
                }
            }
        }

        if matches.len() > 1 {
            return Err(format!("找到多个 {} 配置文件: {:?}", pattern, matches));
        }

        Ok(matches.into_iter().next())
    }

    pub fn validate_config_dir(&self) -> ConfigValidationResult {
        let config_dir = self.config_dir.lock();
        let dir = config_dir.clone();
        drop(config_dir);
        let threshold_config = self.threshold_config.lock().clone();
        let is_enabled = |item: Option<&TestItemConfig>| match item {
            Some(config) => config.enabled,
            None => true,
        };

        let mut result = ConfigValidationResult {
            base_noise_config: None,
            ppg_noise_config: None,
            lpctr_config: None,
            lplctr_config: None,
            errors: Vec::new(),
            is_valid: true,
        };

        if is_enabled(
            threshold_config
                .as_ref()
                .and_then(|config| config.tests.base_noise.as_ref()),
        ) {
            match Self::find_config_file(&dir, "base_noise") {
                Ok(Some(path)) => {
                    result.base_noise_config = Some(path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    result.errors.push("未找到 base_noise 配置文件".to_string());
                    result.is_valid = false;
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("查找 base_noise 配置失败: {}", e));
                    result.is_valid = false;
                }
            }
        }

        if is_enabled(
            threshold_config
                .as_ref()
                .and_then(|config| config.tests.ppg_noise.as_ref()),
        ) {
            match Self::find_config_file(&dir, "ppg_noise") {
                Ok(Some(path)) => {
                    result.ppg_noise_config = Some(path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    result.errors.push("未找到 ppg_noise 配置文件".to_string());
                    result.is_valid = false;
                }
                Err(e) => {
                    result
                        .errors
                        .push(format!("查找 ppg_noise 配置失败: {}", e));
                    result.is_valid = false;
                }
            }
        }

        if is_enabled(
            threshold_config
                .as_ref()
                .and_then(|config| config.tests.lpctr.as_ref()),
        ) {
            match Self::find_config_file(&dir, "lpctr") {
                Ok(Some(path)) => {
                    result.lpctr_config = Some(path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    result.errors.push("未找到 lpctr 配置文件".to_string());
                    result.is_valid = false;
                }
                Err(e) => {
                    result.errors.push(format!("查找 lpctr 配置失败: {}", e));
                    result.is_valid = false;
                }
            }
        }

        if is_enabled(
            threshold_config
                .as_ref()
                .and_then(|config| config.tests.lplctr.as_ref()),
        ) {
            match Self::find_config_file(&dir, "lplctr") {
                Ok(Some(path)) => {
                    result.lplctr_config = Some(path.to_string_lossy().to_string());
                }
                Ok(None) => {
                    result.errors.push("未找到 lplctr 配置文件".to_string());
                    result.is_valid = false;
                }
                Err(e) => {
                    result.errors.push(format!("查找 lplctr 配置失败: {}", e));
                    result.is_valid = false;
                }
            }
        }

        result
    }

    pub fn start_test(&self, gh3036_manager: Arc<Gh3036Manager>) -> Result<(), String> {
        info!(
            "[FactoryTest] start_test 被调用, running={}",
            self.running.load(Ordering::SeqCst)
        );

        if self.running.load(Ordering::SeqCst) {
            error!("[FactoryTest] 产测流程已在运行中，拒绝启动");
            return Err("产测流程已在运行中".to_string());
        }

        {
            let mut handle = self.thread_handle.lock();
            if let Some(thread) = handle.take() {
                info!("[FactoryTest] 等待之前的线程结束...");
                drop(handle);
                let _ = thread.join();
                info!("[FactoryTest] 之前的线程已结束");
            }
        }

        let threshold_validation = self.validate_threshold_config();
        if !threshold_validation.is_valid {
            error!(
                "[FactoryTest] 卡控配置校验失败: {:?}",
                threshold_validation.errors
            );
            return Err(format!(
                "卡控配置校验失败: {}",
                threshold_validation.errors.join("；")
            ));
        }
        let config_dir = self.config_dir.lock().clone();
        let config_file = FactoryThresholdConfig::find_config_file(&config_dir)
            .ok_or_else(|| "未找到唯一的卡控配置文件".to_string())?;
        let threshold_config = FactoryThresholdConfig::from_file(&config_file)?;
        threshold_config.validate()?;
        *self.threshold_config.lock() = Some(threshold_config);

        let validation = self.validate_config_dir();
        if !validation.is_valid {
            error!("[FactoryTest] 配置文件校验失败: {:?}", validation.errors);
            return Err(format!("配置文件校验失败: {:?}", validation.errors));
        }

        info!("[FactoryTest] 配置校验通过，开始启动产测流程");

        self.reset_frame_collection();
        self.running.store(true, Ordering::SeqCst);
        {
            let mut status = self.status.lock();
            *status = FactoryTestStatus::Running;
        }
        {
            let mut current_step = self.current_step.lock();
            *current_step = FactoryTestStep::Prepare;
        }
        {
            let mut result = self.result.lock();
            *result = None;
        }
        {
            let mut evaluation_result = self.evaluation_result.lock();
            *evaluation_result = None;
        }

        self.publish_progress(
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.0,
            "产测流程开始",
        );

        let running = self.running.clone();
        let event_bus = self.event_bus.clone();
        let config_dir = self.config_dir.lock().clone();
        let status_state = self.status.clone();
        let current_step_state = self.current_step.clone();
        let result_state = self.result.clone();
        let manager = gh3036_manager;
        let threshold_config = self.threshold_config.lock().clone();
        let evaluation_result_clone = self.evaluation_result.clone();
        let frame_collector = self.frame_collector.clone();

        let thread_handle = thread::spawn(move || {
            info!("[FactoryTest] 产测流程线程启动");

            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    let reason = format!("创建 Tokio runtime 失败: {}", e);
                    error!("[FactoryTest] {}", reason);
                    Self::set_state(
                        &status_state,
                        &current_step_state,
                        FactoryTestStatus::Failed,
                        FactoryTestStep::Prepare,
                    );
                    Self::publish_progress_static(
                        &event_bus,
                        FactoryTestStep::Prepare,
                        FactoryTestStatus::Failed,
                        0.0,
                        &reason,
                    );
                    running.store(false, Ordering::SeqCst);
                    return;
                }
            };

            let mut test_result = FactoryTestResult {
                chip_init_status: 0,
                uuid: Vec::new(),
                compute_mode: FactoryComputeMode::Mcu,
                base_noise: Vec::new(),
                ppg_noise: Vec::new(),
                lpctr: Vec::new(),
                lplctr: Vec::new(),
                overall_result: "PASS".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                device_info: String::new(),
                error_code: String::new(),
                project_name: String::new(),
            };

            if let Some(tx_channel) = manager.get_tx_channel() {
                test_result.device_info = tx_channel.device_id;
            }

            if let Some(ref config) = threshold_config {
                test_result.project_name = config.project.clone();
            }

            let all_steps: [(FactoryTestStep, f32, f32); 8] = [
                (FactoryTestStep::Prepare, 0.0, 0.05),
                (FactoryTestStep::ChipInit, 0.05, 0.15),
                (FactoryTestStep::Uuid, 0.15, 0.25),
                (FactoryTestStep::BaseNoise, 0.25, 0.40),
                (FactoryTestStep::PpgNoise, 0.40, 0.55),
                (FactoryTestStep::Lpctr, 0.55, 0.70),
                (FactoryTestStep::EnvironmentSwitch, 0.70, 0.75),
                (FactoryTestStep::Lplctr, 0.75, 0.90),
            ];

            let steps: Vec<_> = all_steps
                .into_iter()
                .filter(|(step, _, _)| Self::should_execute_step(&threshold_config, *step))
                .collect();
            let mut evaluation = threshold_config
                .as_ref()
                .map(Self::initial_evaluation_result);
            let fail_action = threshold_config
                .as_ref()
                .and_then(|config| config.global.as_ref())
                .map(|global| global.fail_action)
                .unwrap_or_default();
            let mut failure_reason: Option<String> = None;
            let mut failure_step: Option<FactoryTestStep> = None;

            for (step, progress_start, progress_end) in &steps {
                if !running.load(Ordering::SeqCst) {
                    info!("[FactoryTest] 产测流程被停止");
                    break;
                }

                Self::set_state(
                    &status_state,
                    &current_step_state,
                    FactoryTestStatus::Running,
                    *step,
                );
                Self::publish_progress_static(
                    &event_bus,
                    *step,
                    FactoryTestStatus::Running,
                    *progress_start,
                    &format!("执行步骤: {:?}", step),
                );

                let step_result = rt.block_on(Self::execute_step(
                    *step,
                    &config_dir,
                    &mut test_result,
                    &event_bus,
                    &manager,
                ));

                match step_result {
                    Ok(step_result_opt) => {
                        if let Some(mut step_result) = step_result_opt {
                            if let (Some(config), Some(evaluation_result)) =
                                (threshold_config.as_ref(), evaluation.as_mut())
                            {
                                if let Some((test_name, item_config, values)) =
                                    Self::evaluation_input(config, &test_result, *step)
                                {
                                    let item_result =
                                        evaluate_measurements(test_name, item_config, &values);
                                    step_result.success = item_result.pass;
                                    step_result.message = item_result.message.clone();
                                    evaluation_result.add_test_result(item_result);
                                }
                            }

                            Self::publish_step_result_static(
                                &event_bus,
                                FactoryTestStatus::Running,
                                *progress_end,
                                step_result.clone(),
                            );

                            if !step_result.success {
                                test_result.overall_result = "FAIL".to_string();
                                if Self::should_stop_after_threshold_failure(
                                    fail_action,
                                    step_result.success,
                                ) {
                                    Self::append_failure_reason(
                                        &mut failure_reason,
                                        format!(
                                            "测试项 {:?} 卡控失败: {}",
                                            step, step_result.message
                                        ),
                                    );
                                    failure_step = Some(*step);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("[FactoryTest] 步骤 {:?} 执行失败: {}", step, e);
                        test_result.overall_result = "FAIL".to_string();
                        Self::append_failure_reason(
                            &mut failure_reason,
                            format!("步骤 {:?} 执行失败: {}", step, e),
                        );
                        failure_step = Some(*step);
                        break;
                    }
                }

                Self::publish_progress_static(
                    &event_bus,
                    *step,
                    FactoryTestStatus::Running,
                    *progress_end,
                    &format!("步骤 {:?} 完成", step),
                );

                if *step == FactoryTestStep::EnvironmentSwitch {
                    Self::set_state(
                        &status_state,
                        &current_step_state,
                        FactoryTestStatus::WaitingForEnvironmentSwitch,
                        FactoryTestStep::EnvironmentSwitch,
                    );
                    Self::publish_progress_static(
                        &event_bus,
                        FactoryTestStep::EnvironmentSwitch,
                        FactoryTestStatus::WaitingForEnvironmentSwitch,
                        *progress_end,
                        "等待切换测试环境",
                    );

                    while running.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(100));
                        let current_status = *status_state.lock();
                        if current_status == FactoryTestStatus::Running {
                            break;
                        }
                    }
                }
            }

            if running.load(Ordering::SeqCst) {
                Self::set_state(
                    &status_state,
                    &current_step_state,
                    FactoryTestStatus::Running,
                    FactoryTestStep::Cleanup,
                );
                Self::publish_progress_static(
                    &event_bus,
                    FactoryTestStep::Cleanup,
                    FactoryTestStatus::Running,
                    0.90,
                    "执行清理步骤",
                );

                match rt.block_on(Self::execute_cleanup_step(&event_bus, &manager)) {
                    Ok(Some(step_result)) => Self::publish_step_result_static(
                        &event_bus,
                        FactoryTestStatus::Running,
                        0.95,
                        step_result,
                    ),
                    Ok(None) => {}
                    Err(error) => {
                        let cleanup_reason = format!("清理步骤执行失败: {}", error);
                        error!("[FactoryTest] {}", cleanup_reason);
                        test_result.overall_result = "FAIL".to_string();
                        Self::append_failure_reason(&mut failure_reason, cleanup_reason);
                        failure_step.get_or_insert(FactoryTestStep::Cleanup);
                    }
                }

                if let Some(eval_result) = evaluation.as_ref() {
                    let error_result = generate_error_codes(
                        test_result.chip_init_status,
                        &test_result.uuid,
                        eval_result,
                    );
                    test_result.error_code = error_result.error_codes.join(",");

                    if error_result.has_error {
                        test_result.overall_result = "FAIL".to_string();
                    }

                    *evaluation_result_clone.lock() = Some(eval_result.clone());
                }

                let _ = frame_collector.lock().finish();

                let project_name = if test_result.project_name.is_empty() {
                    "unknown".to_string()
                } else {
                    test_result.project_name.clone()
                };

                if let Err(e) = Self::save_result_to_csv(&test_result, &project_name) {
                    error!("[FactoryTest] 保存结果失败: {}", e);
                }

                *result_state.lock() = Some(test_result.clone());

                let (terminal_status, terminal_step, message) = match failure_reason {
                    Some(reason) => (
                        FactoryTestStatus::Failed,
                        failure_step.unwrap_or(FactoryTestStep::Cleanup),
                        reason,
                    ),
                    None => (
                        FactoryTestStatus::Completed,
                        FactoryTestStep::Completed,
                        format!("产测完成，结果: {}", test_result.overall_result),
                    ),
                };
                Self::set_state(
                    &status_state,
                    &current_step_state,
                    terminal_status,
                    terminal_step,
                );
                Self::publish_progress_static(
                    &event_bus,
                    terminal_step,
                    terminal_status,
                    1.0,
                    &message,
                );
            }

            running.store(false, Ordering::SeqCst);
            info!("[FactoryTest] 产测流程线程结束, running 已重置为 false");
        });

        {
            let mut handle = self.thread_handle.lock();
            *handle = Some(thread_handle);
        }

        Ok(())
    }

    pub fn stop_test(&self) -> Result<(), String> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("产测流程未在运行".to_string());
        }

        self.running.store(false, Ordering::SeqCst);
        {
            let mut status = self.status.lock();
            *status = FactoryTestStatus::Stopped;
        }
        {
            let mut current_step = self.current_step.lock();
            *current_step = FactoryTestStep::Idle;
        }

        self.publish_progress(
            FactoryTestStep::Idle,
            FactoryTestStatus::Stopped,
            0.0,
            "产测流程已停止",
        );

        let mut handle = self.thread_handle.lock();
        if let Some(thread) = handle.take() {
            let _ = thread.join();
        }

        self.reset_frame_collection();

        Ok(())
    }

    pub fn continue_test(&self) -> Result<(), String> {
        let current_status = *self.status.lock();
        if current_status != FactoryTestStatus::WaitingForEnvironmentSwitch {
            return Err("当前不在等待环境切换状态".to_string());
        }

        {
            let mut status = self.status.lock();
            *status = FactoryTestStatus::Running;
        }
        {
            let mut current_step = self.current_step.lock();
            *current_step = FactoryTestStep::Lplctr;
        }

        self.publish_progress(
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.75,
            "环境切换完成，继续测试",
        );

        Ok(())
    }

    async fn execute_step(
        step: FactoryTestStep,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        match step {
            FactoryTestStep::Prepare => Self::execute_prepare_step(event_bus, manager).await,
            FactoryTestStep::ChipInit => {
                Self::execute_chip_init_step(event_bus, test_result, manager).await
            }
            FactoryTestStep::Uuid => Self::execute_uuid_step(event_bus, test_result, manager).await,
            FactoryTestStep::BaseNoise => {
                Self::execute_base_noise_step(event_bus, config_dir, test_result, manager).await
            }
            FactoryTestStep::PpgNoise => {
                Self::execute_ppg_noise_step(event_bus, config_dir, test_result, manager).await
            }
            FactoryTestStep::Lpctr => {
                Self::execute_lpctr_step(event_bus, config_dir, test_result, manager).await
            }
            FactoryTestStep::EnvironmentSwitch => Ok(None),
            FactoryTestStep::Lplctr => {
                Self::execute_lplctr_step(event_bus, config_dir, test_result, manager).await
            }
            FactoryTestStep::Cleanup => Self::execute_cleanup_step(event_bus, manager).await,
            FactoryTestStep::Idle | FactoryTestStep::Completed => Ok(None),
        }
    }

    fn should_execute_step(
        threshold_config: &Option<FactoryThresholdConfig>,
        step: FactoryTestStep,
    ) -> bool {
        let Some(config) = threshold_config else {
            return true;
        };

        match step {
            FactoryTestStep::BaseNoise => config
                .tests
                .base_noise
                .as_ref()
                .is_some_and(|item| item.enabled),
            FactoryTestStep::PpgNoise => config
                .tests
                .ppg_noise
                .as_ref()
                .is_some_and(|item| item.enabled),
            FactoryTestStep::Lpctr => config.tests.lpctr.as_ref().is_some_and(|item| item.enabled),
            FactoryTestStep::EnvironmentSwitch | FactoryTestStep::Lplctr => config
                .tests
                .lplctr
                .as_ref()
                .is_some_and(|item| item.enabled),
            _ => true,
        }
    }

    fn should_stop_after_threshold_failure(fail_action: FailAction, step_passed: bool) -> bool {
        !step_passed && fail_action == FailAction::Stop
    }

    fn append_failure_reason(failure_reason: &mut Option<String>, reason: impl Into<String>) {
        let reason = reason.into();
        match failure_reason {
            Some(existing) => {
                existing.push('；');
                existing.push_str(&reason);
            }
            None => *failure_reason = Some(reason),
        }
    }

    fn initial_evaluation_result(config: &FactoryThresholdConfig) -> FactoryEvaluationResult {
        let mut result = FactoryEvaluationResult::new(&config.project);
        for (test_name, item_config) in [
            ("base_noise", config.tests.base_noise.as_ref()),
            ("ppg_noise", config.tests.ppg_noise.as_ref()),
            ("lpctr", config.tests.lpctr.as_ref()),
            ("lplctr", config.tests.lplctr.as_ref()),
        ] {
            if !item_config.is_some_and(|item| item.enabled) {
                result.add_test_result(evaluate_test_item(test_name, item_config, &[]));
            }
        }
        result
    }

    fn evaluation_input<'a>(
        config: &'a FactoryThresholdConfig,
        result: &'a FactoryTestResult,
        step: FactoryTestStep,
    ) -> Option<(&'static str, Option<&'a TestItemConfig>, Vec<Option<f64>>)> {
        match step {
            FactoryTestStep::BaseNoise => Some((
                "base_noise",
                config.tests.base_noise.as_ref(),
                result
                    .base_noise
                    .iter()
                    .map(ChannelMeasurement::evaluation_value)
                    .collect(),
            )),
            FactoryTestStep::PpgNoise => Some((
                "ppg_noise",
                config.tests.ppg_noise.as_ref(),
                result
                    .ppg_noise
                    .iter()
                    .map(ChannelMeasurement::evaluation_value)
                    .collect(),
            )),
            FactoryTestStep::Lpctr => Some((
                "lpctr",
                config.tests.lpctr.as_ref(),
                result
                    .lpctr
                    .iter()
                    .map(ChannelMeasurement::evaluation_value)
                    .collect(),
            )),
            FactoryTestStep::Lplctr => Some((
                "lplctr",
                config.tests.lplctr.as_ref(),
                result
                    .lplctr
                    .iter()
                    .map(ChannelMeasurement::evaluation_value)
                    .collect(),
            )),
            _ => None,
        }
    }

    async fn execute_prepare_step(
        event_bus: &Arc<EventBus>,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行准备步骤: 切换工作模式为 2");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.02,
            "切换工作模式为 2",
        );

        manager
            .execute_rpc("M", &["2".to_string()])
            .await
            .map_err(|e| format!("设置工作模式失败: {}", e))?;

        info!("[FactoryTest] 工作模式已切换为 2");

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.04,
            "关闭全部功能",
        );

        manager
            .execute_rpc("S", &["0x0".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("关闭功能失败: {}", e))?;

        info!("[FactoryTest] 全部功能已关闭");

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Prepare,
            success: true,
            message: "准备步骤完成".to_string(),
            data: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_chip_init_step(
        event_bus: &Arc<EventBus>,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行芯片初始化步骤: CMD_FACTORY_SET_MODE 0x01");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::ChipInit,
            FactoryTestStatus::Running,
            0.08,
            "发送芯片初始化命令 FS 0x01",
        );

        manager
            .execute_rpc("FS", &["0x01".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x01 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::ChipInit,
            FactoryTestStatus::Running,
            0.10,
            "获取芯片初始化结果 FG 0x01",
        );

        let result = manager
            .execute_rpc("FG", &["0x01".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x01 结果失败: {}", e))?;

        let status = if !result.is_empty() {
            result[0] as u16
        } else {
            0
        };

        test_result.chip_init_status = status;
        info!("[FactoryTest] 芯片初始化状态: {}", status);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::ChipInit,
            success: true,
            message: format!("芯片初始化完成, status={}", status),
            data: vec![Some(f64::from(status))],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_uuid_step(
        event_bus: &Arc<EventBus>,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 UUID 读取步骤: CMD_FACTORY_SET_MODE 0x02");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Uuid,
            FactoryTestStatus::Running,
            0.18,
            "发送 UUID 读取命令 FS 0x02",
        );

        manager
            .execute_rpc("FS", &["0x02".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x02 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Uuid,
            FactoryTestStatus::Running,
            0.20,
            "获取 UUID 结果 FG 0x02",
        );

        let result = manager
            .execute_rpc("FG", &["0x02".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x02 结果失败: {}", e))?;

        let uuid = result.to_vec();
        test_result.uuid = uuid.clone();

        let uuid_str: String = uuid
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":");
        info!("[FactoryTest] UUID: {}", uuid_str);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Uuid,
            success: true,
            message: format!("UUID: {}", uuid_str),
            data: uuid.iter().map(|&b| Some(f64::from(b))).collect(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_base_noise_step(
        event_bus: &Arc<EventBus>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行基底噪声测试: CMD_FACTORY_SET_MODE 0x04");

        let config_file = Self::find_config_file(config_dir, "base_noise")?
            .ok_or("未找到 base_noise 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.26,
            &format!("加载配置文件: {}", config_file.display()),
        );

        let config_loader = ConfigLoader::from_file(&config_file)?;
        let reg_values = config_loader.get_values();

        info!("[FactoryTest] 解析到 {} 个寄存器", reg_values.len());

        if reg_values.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.28,
            "发送产测模式命令 FS 0x04",
        );

        manager
            .execute_rpc("FS", &["0x04".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x04 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.29,
            "下载配置 D 0",
        );

        manager
            .execute_rpc("D", &["0".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.30,
            &format!("写入寄存器列表 ({} 个)", reg_values.len()),
        );

        let params = config_loader.format_for_download();
        manager
            .execute_rpc("L", &params)
            .await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.31,
            "下载配置 D 1",
        );

        manager
            .execute_rpc("D", &["1".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.32,
            "启动 TEST1 功能",
        );

        manager
            .execute_rpc("S", &["0x40".to_string(), "0".to_string()])
            .await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.35,
            "采集数据中 (3秒)",
        );

        thread::sleep(Duration::from_secs(3));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.38,
            "停止 TEST1 功能",
        );

        manager
            .execute_rpc("S", &["0x40".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.39,
            "获取底噪结果 FG 0x04",
        );

        let result = manager
            .execute_rpc("FG", &["0x04".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x04 结果失败: {}", e))?;

        let base_noise: Vec<u16> = result
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else if chunk.len() == 1 {
                    chunk[0] as u16
                } else {
                    0
                }
            })
            .collect();

        test_result.base_noise = Self::device_measurements(&base_noise);
        info!("[FactoryTest] 底噪数据: {:?}", base_noise);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::BaseNoise,
            success: true,
            message: format!("底噪测试完成, {} 个通道", base_noise.len()),
            data: Self::step_data(&base_noise),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_ppg_noise_step(
        event_bus: &Arc<EventBus>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 PPG 噪声测试: CMD_FACTORY_SET_MODE 0x08");

        let config_file =
            Self::find_config_file(config_dir, "ppg_noise")?.ok_or("未找到 ppg_noise 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.42,
            &format!("加载配置文件: {}", config_file.display()),
        );

        let config_loader = ConfigLoader::from_file(&config_file)?;
        let reg_values = config_loader.get_values();

        info!("[FactoryTest] 解析到 {} 个寄存器", reg_values.len());

        if reg_values.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.44,
            "发送产测模式命令 FS 0x08",
        );

        manager
            .execute_rpc("FS", &["0x08".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x08 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.45,
            "下载配置并写入寄存器",
        );

        manager
            .execute_rpc("D", &["0".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager
            .execute_rpc("L", &params)
            .await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager
            .execute_rpc("D", &["1".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.48,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager
            .execute_rpc("S", &["0x40".to_string(), "0".to_string()])
            .await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager
            .execute_rpc("S", &["0x40".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.53,
            "获取 PPG 噪声结果 FG 0x08",
        );

        let result = manager
            .execute_rpc("FG", &["0x08".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x08 结果失败: {}", e))?;

        let ppg_noise: Vec<u16> = result
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else if chunk.len() == 1 {
                    chunk[0] as u16
                } else {
                    0
                }
            })
            .collect();

        test_result.ppg_noise = Self::device_measurements(&ppg_noise);
        info!("[FactoryTest] PPG 噪声数据: {:?}", ppg_noise);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::PpgNoise,
            success: true,
            message: format!("PPG 噪声测试完成, {} 个通道", ppg_noise.len()),
            data: Self::step_data(&ppg_noise),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_lpctr_step(
        event_bus: &Arc<EventBus>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 LPCTR 测试: CMD_FACTORY_SET_MODE 0x10");

        let config_file =
            Self::find_config_file(config_dir, "lpctr")?.ok_or("未找到 lpctr 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.58,
            &format!("加载配置文件: {}", config_file.display()),
        );

        let config_loader = ConfigLoader::from_file(&config_file)?;
        let reg_values = config_loader.get_values();

        info!("[FactoryTest] 解析到 {} 个寄存器", reg_values.len());

        if reg_values.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.60,
            "发送产测模式命令 FS 0x10",
        );

        manager
            .execute_rpc("FS", &["0x10".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x10 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.62,
            "下载配置并写入寄存器",
        );

        manager
            .execute_rpc("D", &["0".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager
            .execute_rpc("L", &params)
            .await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager
            .execute_rpc("D", &["1".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.65,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager
            .execute_rpc("S", &["0x40".to_string(), "0".to_string()])
            .await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager
            .execute_rpc("S", &["0x40".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.68,
            "获取 LPCTR 结果 FG 0x10",
        );

        let result = manager
            .execute_rpc("FG", &["0x10".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x10 结果失败: {}", e))?;

        let lpctr: Vec<u16> = result
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else if chunk.len() == 1 {
                    chunk[0] as u16
                } else {
                    0
                }
            })
            .collect();

        test_result.lpctr = Self::device_measurements(&lpctr);
        info!("[FactoryTest] LPCTR 数据: {:?}", lpctr);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lpctr,
            success: true,
            message: format!("LPCTR 测试完成, {} 个通道", lpctr.len()),
            data: Self::step_data(&lpctr),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_lplctr_step(
        event_bus: &Arc<EventBus>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 LPLCTR 测试: CMD_FACTORY_SET_MODE 0x20");

        let config_file =
            Self::find_config_file(config_dir, "lplctr")?.ok_or("未找到 lplctr 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.78,
            &format!("加载配置文件: {}", config_file.display()),
        );

        let config_loader = ConfigLoader::from_file(&config_file)?;
        let reg_values = config_loader.get_values();

        info!("[FactoryTest] 解析到 {} 个寄存器", reg_values.len());

        if reg_values.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.80,
            "发送产测模式命令 FS 0x20",
        );

        manager
            .execute_rpc("FS", &["0x20".to_string()])
            .await
            .map_err(|e| format!("设置产测模式 0x20 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.82,
            "下载配置并写入寄存器",
        );

        manager
            .execute_rpc("D", &["0".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager
            .execute_rpc("L", &params)
            .await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager
            .execute_rpc("D", &["1".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.85,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager
            .execute_rpc("S", &["0x40".to_string(), "0".to_string()])
            .await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager
            .execute_rpc("S", &["0x40".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.88,
            "获取 LPLCTR 结果 FG 0x20",
        );

        let result = manager
            .execute_rpc("FG", &["0x20".to_string()])
            .await
            .map_err(|e| format!("获取产测模式 0x20 结果失败: {}", e))?;

        let lplctr: Vec<u16> = result
            .chunks(2)
            .map(|chunk| {
                if chunk.len() == 2 {
                    u16::from_le_bytes([chunk[0], chunk[1]])
                } else if chunk.len() == 1 {
                    chunk[0] as u16
                } else {
                    0
                }
            })
            .collect();

        test_result.lplctr = Self::device_measurements(&lplctr);
        info!("[FactoryTest] LPLCTR 数据: {:?}", lplctr);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lplctr,
            success: true,
            message: format!("LPLCTR 测试完成, {} 个通道", lplctr.len()),
            data: Self::step_data(&lplctr),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    async fn execute_cleanup_step(
        event_bus: &Arc<EventBus>,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行清理步骤: 切换工作模式回 0");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Cleanup,
            FactoryTestStatus::Running,
            0.92,
            "关闭全部功能",
        );

        manager
            .execute_rpc("S", &["0x0".to_string(), "1".to_string()])
            .await
            .map_err(|e| format!("关闭功能失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Cleanup,
            FactoryTestStatus::Running,
            0.94,
            "切换工作模式为 0",
        );

        manager
            .execute_rpc("M", &["0".to_string()])
            .await
            .map_err(|e| format!("设置工作模式失败: {}", e))?;

        info!("[FactoryTest] 工作模式已切换回 0");

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Cleanup,
            success: true,
            message: "清理步骤完成".to_string(),
            data: Vec::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn save_result_to_csv(result: &FactoryTestResult, project_name: &str) -> Result<(), String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .ok_or("获取可执行文件目录失败")?;

        let output_dir = exe_dir.join("data").join("factory");
        std::fs::create_dir_all(&output_dir).map_err(|e| format!("创建输出目录失败: {}", e))?;

        let today = Local::now().format("%Y-%m-%d").to_string();
        let file_name = format!("factory_{}_{}.csv", project_name, today);
        let file_path = output_dir.join(&file_name);

        let file_exists = file_path.exists();

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;

        let mut writer = csv::WriterBuilder::new()
            .has_headers(!file_exists)
            .from_writer(file);

        if !file_exists {
            let headers = Self::generate_csv_headers();
            writer
                .write_record(&headers)
                .map_err(|e| format!("写入文件头失败: {}", e))?;
        }

        let row = Self::build_csv_row(result);
        writer
            .write_record(&row)
            .map_err(|e| format!("写入数据失败: {}", e))?;

        writer.flush().map_err(|e| format!("刷新文件失败: {}", e))?;

        info!("[FactoryTest] 结果已保存到: {}", file_path.display());

        Ok(())
    }

    fn generate_csv_headers() -> Vec<String> {
        let mut headers: Vec<String> = vec![
            "timestamp".to_string(),
            "datetime".to_string(),
            "overall_result".to_string(),
            "error_code".to_string(),
            "device_info".to_string(),
            "chip_init_status".to_string(),
            "uuid".to_string(),
        ];

        for i in 0..4 {
            headers.push(format!("base_noise_{}", i));
        }

        for i in 0..32 {
            headers.push(format!("ppg_noise_{}", i));
            headers.push(format!("lpctr_{}", i));
            headers.push(format!("lplctr_{}", i));
        }

        headers
    }

    fn build_csv_row(result: &FactoryTestResult) -> Vec<String> {
        let datetime = chrono::DateTime::from_timestamp_millis(result.timestamp as i64)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        let uuid_str = result
            .uuid
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join("");

        let base_noise = Self::pad_channels(&result.base_noise, 4);
        let ppg_noise = Self::pad_channels(&result.ppg_noise, 32);
        let lpctr = Self::pad_channels(&result.lpctr, 32);
        let lplctr = Self::pad_channels(&result.lplctr, 32);

        let mut row = vec![
            result.timestamp.to_string(),
            datetime,
            result.overall_result.clone(),
            result.error_code.clone(),
            result.device_info.clone(),
            result.chip_init_status.to_string(),
            uuid_str,
        ];

        row.extend(base_noise);

        for i in 0..32 {
            row.push(ppg_noise[i].clone());
            row.push(lpctr[i].clone());
            row.push(lplctr[i].clone());
        }

        row
    }

    fn device_measurements(values: &[u16]) -> Vec<ChannelMeasurement> {
        values
            .iter()
            .copied()
            .map(|device_value| ChannelMeasurement {
                computed_value: None,
                device_value: Some(device_value),
            })
            .collect()
    }

    fn step_data(values: &[u16]) -> Vec<Option<f64>> {
        values.iter().copied().map(f64::from).map(Some).collect()
    }

    fn pad_channels(data: &[ChannelMeasurement], max_count: usize) -> Vec<String> {
        let mut result: Vec<String> = data
            .iter()
            .map(ChannelMeasurement::evaluation_value)
            .map(|value| value.map_or_else(|| "0".to_string(), |value| value.to_string()))
            .collect();

        while result.len() < max_count {
            result.push("0".to_string());
        }

        result
    }

    fn publish_progress(
        &self,
        step: FactoryTestStep,
        status: FactoryTestStatus,
        progress: f32,
        message: &str,
    ) {
        Self::publish_progress_static(&self.event_bus, step, status, progress, message);
    }

    fn set_state(
        status_state: &Arc<Mutex<FactoryTestStatus>>,
        current_step_state: &Arc<Mutex<FactoryTestStep>>,
        status: FactoryTestStatus,
        step: FactoryTestStep,
    ) {
        *status_state.lock() = status;
        *current_step_state.lock() = step;
    }

    fn publish_progress_static(
        event_bus: &Arc<EventBus>,
        step: FactoryTestStep,
        status: FactoryTestStatus,
        progress: f32,
        message: &str,
    ) {
        let event = FactoryTestProgressEvent {
            current_step: step,
            status,
            step_result: None,
            progress,
            message: message.to_string(),
        };

        event_bus.publish_msgpack("gh3036:factory_test_progress", &event);

        info!(
            "[FactoryTest] 进度: step={:?}, status={:?}, progress={:.2}%, message={}",
            step,
            status,
            progress * 100.0,
            message
        );
    }

    fn publish_step_result_static(
        event_bus: &Arc<EventBus>,
        status: FactoryTestStatus,
        progress: f32,
        step_result: FactoryTestStepResult,
    ) {
        let event = FactoryTestProgressEvent {
            current_step: step_result.step,
            status,
            message: step_result.message.clone(),
            step_result: Some(step_result),
            progress,
        };

        event_bus.publish_msgpack("gh3036:factory_test_progress", &event);
    }
}

impl Drop for FactoryTestManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        let mut handle = self.thread_handle.lock();
        if let Some(thread) = handle.take() {
            let _ = thread.join();
        }

        self.reset_frame_collection();
    }
}

#[cfg(test)]
mod tests {
    use super::super::threshold_config::{GlobalConfig, TestsConfig};
    use super::*;

    fn item(enabled: bool) -> TestItemConfig {
        TestItemConfig {
            enabled,
            description: None,
            unit: None,
            mode: Some(4),
            channels: Some(1),
            compute: None,
            global_threshold: None,
            channel_rules: None,
        }
    }

    fn config(fail_action: FailAction) -> FactoryThresholdConfig {
        FactoryThresholdConfig {
            project: "GH3036".to_string(),
            version: "1.0".to_string(),
            description: None,
            chip: None,
            global: Some(GlobalConfig {
                fail_action,
                ..GlobalConfig::default()
            }),
            tests: TestsConfig {
                chip_init: None,
                chip_uid: None,
                base_noise: Some(item(false)),
                ppg_noise: Some(item(true)),
                lpctr: Some(item(true)),
                lplctr: Some(item(false)),
            },
        }
    }

    #[test]
    fn disabled_steps_and_environment_switch_are_skipped() {
        let config = Some(config(FailAction::Stop));

        assert!(!FactoryTestManager::should_execute_step(
            &config,
            FactoryTestStep::BaseNoise
        ));
        assert!(FactoryTestManager::should_execute_step(
            &config,
            FactoryTestStep::PpgNoise
        ));
        assert!(!FactoryTestManager::should_execute_step(
            &config,
            FactoryTestStep::EnvironmentSwitch
        ));
        assert!(!FactoryTestManager::should_execute_step(
            &config,
            FactoryTestStep::Lplctr
        ));
    }

    #[test]
    fn missing_threshold_config_preserves_all_steps() {
        assert!(FactoryTestManager::should_execute_step(
            &None,
            FactoryTestStep::BaseNoise
        ));
        assert!(FactoryTestManager::should_execute_step(
            &None,
            FactoryTestStep::EnvironmentSwitch
        ));
    }

    #[test]
    fn disabled_items_are_present_in_initial_evaluation() {
        let config = config(FailAction::Continue);

        let result = FactoryTestManager::initial_evaluation_result(&config);

        assert_eq!(result.test_results.len(), 2);
        assert!(result.test_results.iter().all(|item| !item.enabled));
        assert!(result.overall_pass);
    }

    #[test]
    fn stop_policy_stops_only_after_threshold_failure() {
        assert!(FactoryTestManager::should_stop_after_threshold_failure(
            FailAction::Stop,
            false
        ));
        assert!(!FactoryTestManager::should_stop_after_threshold_failure(
            FailAction::Stop,
            true
        ));
    }

    #[test]
    fn continue_policy_keeps_running_after_threshold_failure() {
        assert!(!FactoryTestManager::should_stop_after_threshold_failure(
            FailAction::Continue,
            false
        ));
    }

    #[test]
    fn append_failure_reason_preserves_first_reason_and_appends_later_reason() {
        let mut failure_reason = Some("首个失败原因".to_string());

        FactoryTestManager::append_failure_reason(&mut failure_reason, "后续失败原因");

        assert_eq!(
            failure_reason.as_deref(),
            Some("首个失败原因；后续失败原因")
        );
    }
}
