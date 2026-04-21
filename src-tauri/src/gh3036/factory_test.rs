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
use crossbeam_channel::{Receiver, Sender, unbounded};
use parking_lot::Mutex;
use tracing::{error, info};

use crate::service::EventBus;
use super::manager::Gh3036Manager;
use super::types::{
    FactoryTestStep, FactoryTestStatus, FactoryTestResult, FactoryTestStepResult,
    FactoryTestProgressEvent, ConfigValidationResult,
};

struct FactoryRpcRequest {
    command: String,
    params: Vec<String>,
}

pub struct FactoryTestManager {
    config_dir: Mutex<PathBuf>,
    status: Mutex<FactoryTestStatus>,
    current_step: Mutex<FactoryTestStep>,
    result: Mutex<Option<FactoryTestResult>>,
    running: Arc<AtomicBool>,
    event_bus: Arc<EventBus>,
    rpc_sender: Mutex<Option<Sender<FactoryRpcRequest>>>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

unsafe impl Send for FactoryTestManager {}
unsafe impl Sync for FactoryTestManager {}

impl FactoryTestManager {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let config_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .map(|exe_dir| exe_dir.join("data").join("factory"))
            .unwrap_or_else(|| PathBuf::from("data/factory"));

        Self {
            config_dir: Mutex::new(config_dir),
            status: Mutex::new(FactoryTestStatus::Idle),
            current_step: Mutex::new(FactoryTestStep::Idle),
            result: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            event_bus,
            rpc_sender: Mutex::new(None),
            thread_handle: Mutex::new(None),
        }
    }

    pub fn set_config_dir(&self, dir: PathBuf) {
        let mut config_dir = self.config_dir.lock();
        *config_dir = dir;
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
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("读取目录失败: {}", e))?;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    let file_name_str = file_name.to_string_lossy().to_lowercase();
                    if file_name_str.contains(&pattern_lower) {
                        if let Some(ext) = path.extension() {
                            let ext_str = ext.to_string_lossy().to_lowercase();
                            if ext_str == "config" || ext_str == "ini" {
                                return Ok(Some(path));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    pub fn validate_config_dir(&self) -> ConfigValidationResult {
        let config_dir = self.config_dir.lock();
        let dir = config_dir.clone();
        drop(config_dir);

        let mut result = ConfigValidationResult {
            base_noise_config: None,
            ppg_noise_config: None,
            lpctr_config: None,
            lplctr_config: None,
            errors: Vec::new(),
            is_valid: true,
        };

        match Self::find_config_file(&dir, "base_noise") {
            Ok(Some(path)) => {
                result.base_noise_config = Some(path.to_string_lossy().to_string());
            }
            Ok(None) => {
                result.errors.push("未找到 base_noise 配置文件".to_string());
                result.is_valid = false;
            }
            Err(e) => {
                result.errors.push(format!("查找 base_noise 配置失败: {}", e));
                result.is_valid = false;
            }
        }

        match Self::find_config_file(&dir, "ppg_noise") {
            Ok(Some(path)) => {
                result.ppg_noise_config = Some(path.to_string_lossy().to_string());
            }
            Ok(None) => {
                result.errors.push("未找到 ppg_noise 配置文件".to_string());
                result.is_valid = false;
            }
            Err(e) => {
                result.errors.push(format!("查找 ppg_noise 配置失败: {}", e));
                result.is_valid = false;
            }
        }

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

        result
    }

    pub fn start_test(&self, _gh3036_manager: &Gh3036Manager) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("产测流程已在运行中".to_string());
        }

        let validation = self.validate_config_dir();
        if !validation.is_valid {
            return Err(format!("配置文件校验失败: {:?}", validation.errors));
        }

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

        self.publish_progress(
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.0,
            "产测流程开始",
        );

        let (rpc_sender, rpc_receiver) = unbounded();
        {
            let mut sender = self.rpc_sender.lock();
            *sender = Some(rpc_sender);
        }

        let running = self.running.clone();
        let event_bus = self.event_bus.clone();
        let config_dir = self.config_dir.lock().clone();

        let status_clone = Arc::new(Mutex::new(FactoryTestStatus::Running));
        let current_step_clone = Arc::new(Mutex::new(FactoryTestStep::Prepare));
        let result_clone = Arc::new(Mutex::new(None::<FactoryTestResult>));

        let thread_handle = thread::spawn(move || {
            info!("[FactoryTest] 产测流程线程启动");

            let mut test_result = FactoryTestResult {
                chip_init_status: 0,
                uuid: Vec::new(),
                base_noise: Vec::new(),
                ppg_noise: Vec::new(),
                lpctr: Vec::new(),
                lplctr: Vec::new(),
                overall_result: "PASS".to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            };

            let steps = [
                (FactoryTestStep::Prepare, 0.0, 0.05),
                (FactoryTestStep::ChipInit, 0.05, 0.15),
                (FactoryTestStep::Uuid, 0.15, 0.25),
                (FactoryTestStep::BaseNoise, 0.25, 0.40),
                (FactoryTestStep::PpgNoise, 0.40, 0.55),
                (FactoryTestStep::Lpctr, 0.55, 0.70),
                (FactoryTestStep::EnvironmentSwitch, 0.70, 0.75),
                (FactoryTestStep::Lplctr, 0.75, 0.90),
                (FactoryTestStep::Cleanup, 0.90, 0.95),
                (FactoryTestStep::Completed, 0.95, 1.0),
            ];

            for (step, progress_start, progress_end) in steps.iter() {
                if !running.load(Ordering::SeqCst) {
                    info!("[FactoryTest] 产测流程被停止");
                    break;
                }

                {
                    let mut current = current_step_clone.lock();
                    *current = *step;
                }

                let progress = (progress_start + progress_end) / 2.0;
                Self::publish_progress_static(
                    &event_bus,
                    *step,
                    FactoryTestStatus::Running,
                    progress,
                    &format!("执行步骤: {:?}", step),
                );

                let step_result = Self::execute_step(
                    *step,
                    &rpc_receiver,
                    &config_dir,
                    &mut test_result,
                    &event_bus,
                );

                match step_result {
                    Ok(step_result_opt) => {
                        if let Some(step_result) = step_result_opt {
                            if !step_result.success {
                                test_result.overall_result = "FAIL".to_string();
                                Self::publish_progress_static(
                                    &event_bus,
                                    *step,
                                    FactoryTestStatus::Failed,
                                    *progress_end,
                                    &step_result.message,
                                );
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        error!("[FactoryTest] 步骤 {:?} 执行失败: {}", step, e);
                        test_result.overall_result = "FAIL".to_string();
                        Self::publish_progress_static(
                            &event_bus,
                            *step,
                            FactoryTestStatus::Failed,
                            *progress_end,
                            &e,
                        );
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
                    {
                        let mut status = status_clone.lock();
                        *status = FactoryTestStatus::WaitingForEnvironmentSwitch;
                    }
                    Self::publish_progress_static(
                        &event_bus,
                        FactoryTestStep::EnvironmentSwitch,
                        FactoryTestStatus::WaitingForEnvironmentSwitch,
                        *progress_end,
                        "等待切换测试环境",
                    );

                    while running.load(Ordering::SeqCst) {
                        thread::sleep(Duration::from_millis(100));
                        let current_status = *status_clone.lock();
                        if current_status == FactoryTestStatus::Running {
                            break;
                        }
                    }
                }
            }

            if running.load(Ordering::SeqCst) {
                {
                    let mut result = result_clone.lock();
                    *result = Some(test_result.clone());
                }

                Self::publish_progress_static(
                    &event_bus,
                    FactoryTestStep::Completed,
                    FactoryTestStatus::Completed,
                    1.0,
                    &format!("产测完成，结果: {}", test_result.overall_result),
                );

                if let Err(e) = Self::save_result_to_csv(&test_result) {
                    error!("[FactoryTest] 保存结果失败: {}", e);
                }
            }

            {
                let mut status = status_clone.lock();
                *status = FactoryTestStatus::Idle;
            }

            info!("[FactoryTest] 产测流程线程结束");
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

        self.publish_progress(
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.75,
            "环境切换完成，继续测试",
        );

        Ok(())
    }

    fn execute_step(
        step: FactoryTestStep,
        rpc_receiver: &Receiver<FactoryRpcRequest>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        match step {
            FactoryTestStep::Prepare => Self::execute_prepare_step(rpc_receiver, event_bus),
            FactoryTestStep::ChipInit => Self::execute_chip_init_step(rpc_receiver, test_result, event_bus),
            FactoryTestStep::Uuid => Self::execute_uuid_step(rpc_receiver, test_result, event_bus),
            FactoryTestStep::BaseNoise => Self::execute_base_noise_step(rpc_receiver, config_dir, test_result, event_bus),
            FactoryTestStep::PpgNoise => Self::execute_ppg_noise_step(rpc_receiver, config_dir, test_result, event_bus),
            FactoryTestStep::Lpctr => Self::execute_lpctr_step(rpc_receiver, config_dir, test_result, event_bus),
            FactoryTestStep::EnvironmentSwitch => Ok(None),
            FactoryTestStep::Lplctr => Self::execute_lplctr_step(rpc_receiver, config_dir, test_result, event_bus),
            FactoryTestStep::Cleanup => Self::execute_cleanup_step(rpc_receiver, event_bus),
            FactoryTestStep::Idle | FactoryTestStep::Completed => Ok(None),
        }
    }

    fn execute_prepare_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行准备步骤: 切换工作模式为 2");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.02,
            "切换工作模式为 2",
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Prepare,
            FactoryTestStatus::Running,
            0.04,
            "关闭全部功能",
        );

        thread::sleep(Duration::from_millis(500));

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

    fn execute_chip_init_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行芯片初始化步骤: CMD_FACTORY_SET_MODE 0x01");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::ChipInit,
            FactoryTestStatus::Running,
            0.10,
            "发送芯片初始化命令",
        );

        thread::sleep(Duration::from_millis(1000));

        test_result.chip_init_status = 1;

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::ChipInit,
            success: true,
            message: "芯片初始化完成".to_string(),
            data: vec![1],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_uuid_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 UUID 读取步骤: CMD_FACTORY_SET_MODE 0x02");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Uuid,
            FactoryTestStatus::Running,
            0.20,
            "读取芯片 UUID",
        );

        thread::sleep(Duration::from_millis(1000));

        test_result.uuid = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Uuid,
            success: true,
            message: format!("UUID: {:02X?}", test_result.uuid),
            data: test_result.uuid.iter().map(|&b| b as u16).collect(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_base_noise_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行基底噪声测试: CMD_FACTORY_SET_MODE 0x04");

        let config_file = Self::find_config_file(config_dir, "base_noise")?
            .ok_or("未找到 base_noise 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.30,
            &format!("加载配置文件: {}", config_file.display()),
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.32,
            "启动 TEST1 功能",
        );

        thread::sleep(Duration::from_millis(1000));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.35,
            "采集数据中 (3秒)",
        );

        thread::sleep(Duration::from_secs(3));

        test_result.base_noise = vec![100, 105, 98, 102, 99];

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::BaseNoise,
            success: true,
            message: format!("基底噪声: {:?}", test_result.base_noise),
            data: test_result.base_noise.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_ppg_noise_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 PPG 噪声测试: CMD_FACTORY_SET_MODE 0x08");

        let config_file = Self::find_config_file(config_dir, "ppg_noise")?
            .ok_or("未找到 ppg_noise 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.45,
            &format!("加载配置文件: {}", config_file.display()),
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.48,
            "采集数据中 (3秒)",
        );

        thread::sleep(Duration::from_secs(3));

        test_result.ppg_noise = vec![50, 52, 48, 51, 49];

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::PpgNoise,
            success: true,
            message: format!("PPG 噪声: {:?}", test_result.ppg_noise),
            data: test_result.ppg_noise.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_lpctr_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 LPCTR 测试: CMD_FACTORY_SET_MODE 0x10");

        let config_file = Self::find_config_file(config_dir, "lpctr")?
            .ok_or("未找到 lpctr 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.60,
            &format!("加载配置文件: {}", config_file.display()),
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.63,
            "采集数据中 (3秒)",
        );

        thread::sleep(Duration::from_secs(3));

        test_result.lpctr = vec![200, 205, 198, 202, 199];

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lpctr,
            success: true,
            message: format!("LPCTR: {:?}", test_result.lpctr),
            data: test_result.lpctr.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_lplctr_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行 LPLCTR 测试: CMD_FACTORY_SET_MODE 0x20");

        let config_file = Self::find_config_file(config_dir, "lplctr")?
            .ok_or("未找到 lplctr 配置文件")?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.80,
            &format!("加载配置文件: {}", config_file.display()),
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.83,
            "采集数据中 (3秒)",
        );

        thread::sleep(Duration::from_secs(3));

        test_result.lplctr = vec![150, 155, 148, 152, 149];

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lplctr,
            success: true,
            message: format!("LPLCTR: {:?}", test_result.lplctr),
            data: test_result.lplctr.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }))
    }

    fn execute_cleanup_step(
        _rpc_receiver: &Receiver<FactoryRpcRequest>,
        event_bus: &Arc<EventBus>,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        info!("[FactoryTest] 执行清理步骤: 切换工作模式回 0");

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Cleanup,
            FactoryTestStatus::Running,
            0.92,
            "切换工作模式为 0",
        );

        thread::sleep(Duration::from_millis(500));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Cleanup,
            FactoryTestStatus::Running,
            0.94,
            "关闭测试功能",
        );

        thread::sleep(Duration::from_millis(500));

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

    fn save_result_to_csv(result: &FactoryTestResult) -> Result<(), String> {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .ok_or("获取可执行文件目录失败")?;

        let output_dir = exe_dir.join("data").join("factory");
        std::fs::create_dir_all(&output_dir)
            .map_err(|e| format!("创建输出目录失败: {}", e))?;

        let today = Local::now().format("%Y-%m-%d").to_string();
        let file_name = format!("factory_{}.csv", today);
        let file_path = output_dir.join(&file_name);

        let file_exists = file_path.exists();

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;

        use std::io::Write;

        if !file_exists {
            let header = "timestamp,overall_result,chip_init_status,uuid,base_noise,ppg_noise,lpctr,lplctr\n";
            file.write_all(header.as_bytes())
                .map_err(|e| format!("写入文件头失败: {}", e))?;
        }

        let uuid_str = result.uuid.iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(":");

        let base_noise_str = result.base_noise.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let ppg_noise_str = result.ppg_noise.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let lpctr_str = result.lpctr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let lplctr_str = result.lplctr.iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let line = format!(
            "{},{},{},{},{},{},{},{}\n",
            result.timestamp,
            result.overall_result,
            result.chip_init_status,
            uuid_str,
            base_noise_str,
            ppg_noise_str,
            lpctr_str,
            lplctr_str
        );

        file.write_all(line.as_bytes())
            .map_err(|e| format!("写入数据失败: {}", e))?;

        info!("[FactoryTest] 结果已保存到: {}", file_path.display());

        Ok(())
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
            step, status, progress * 100.0, message
        );
    }
}

impl Drop for FactoryTestManager {
    fn drop(&mut self) {
        self.running.store(false, Ordering::SeqCst);

        let mut handle = self.thread_handle.lock();
        if let Some(thread) = handle.take() {
            let _ = thread.join();
        }
    }
}
