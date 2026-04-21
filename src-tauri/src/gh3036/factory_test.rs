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
use tracing::{error, info};

use crate::service::EventBus;
use super::config_loader::ConfigLoader;
use super::manager::Gh3036Manager;
use super::types::{
    FactoryTestStep, FactoryTestStatus, FactoryTestResult, FactoryTestStepResult,
    FactoryTestProgressEvent, ConfigValidationResult,
};

pub struct FactoryTestManager {
    config_dir: Mutex<PathBuf>,
    status: Mutex<FactoryTestStatus>,
    current_step: Mutex<FactoryTestStep>,
    result: Mutex<Option<FactoryTestResult>>,
    running: Arc<AtomicBool>,
    event_bus: Arc<EventBus>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
    status_clone: Arc<Mutex<FactoryTestStatus>>,
}

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
            status: Mutex::new(FactoryTestStatus::Idle),
            current_step: Mutex::new(FactoryTestStep::Idle),
            result: Mutex::new(None),
            running: Arc::new(AtomicBool::new(false)),
            event_bus,
            thread_handle: Mutex::new(None),
            status_clone: Arc::new(Mutex::new(FactoryTestStatus::Idle)),
        }
    }

    pub fn set_config_dir(&self, dir: PathBuf) {
        info!("[FactoryTest] 配置目录设置为: {}", dir.display());
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

        let mut matches: Vec<PathBuf> = Vec::new();
        
        for entry in entries {
            if let Ok(entry) = entry {
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

    pub fn start_test(&self, gh3036_manager: Arc<Gh3036Manager>) -> Result<(), String> {
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
            let mut status = self.status_clone.lock();
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

        let running = self.running.clone();
        let event_bus = self.event_bus.clone();
        let config_dir = self.config_dir.lock().clone();
        let status_clone = self.status_clone.clone();
        let manager = gh3036_manager;

        let thread_handle = thread::spawn(move || {
            info!("[FactoryTest] 产测流程线程启动");

            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    error!("[FactoryTest] 创建 Tokio runtime 失败: {}", e);
                    return;
                }
            };

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

            let steps: [(FactoryTestStep, f32, f32); 10] = [
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
                                running.store(false, Ordering::SeqCst);
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
                        running.store(false, Ordering::SeqCst);
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
        {
            let mut status = self.status_clone.lock();
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
        {
            let mut status = self.status_clone.lock();
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

    async fn execute_step(
        step: FactoryTestStep,
        config_dir: &Path,
        test_result: &mut FactoryTestResult,
        event_bus: &Arc<EventBus>,
        manager: &Gh3036Manager,
    ) -> Result<Option<FactoryTestStepResult>, String> {
        match step {
            FactoryTestStep::Prepare => Self::execute_prepare_step(event_bus, manager).await,
            FactoryTestStep::ChipInit => Self::execute_chip_init_step(event_bus, test_result, manager).await,
            FactoryTestStep::Uuid => Self::execute_uuid_step(event_bus, test_result, manager).await,
            FactoryTestStep::BaseNoise => Self::execute_base_noise_step(event_bus, config_dir, test_result, manager).await,
            FactoryTestStep::PpgNoise => Self::execute_ppg_noise_step(event_bus, config_dir, test_result, manager).await,
            FactoryTestStep::Lpctr => Self::execute_lpctr_step(event_bus, config_dir, test_result, manager).await,
            FactoryTestStep::EnvironmentSwitch => Ok(None),
            FactoryTestStep::Lplctr => Self::execute_lplctr_step(event_bus, config_dir, test_result, manager).await,
            FactoryTestStep::Cleanup => Self::execute_cleanup_step(event_bus, manager).await,
            FactoryTestStep::Idle | FactoryTestStep::Completed => Ok(None),
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

        manager.execute_rpc("M", &["2".to_string()]).await
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

        manager.execute_rpc("S", &["0x0".to_string(), "0".to_string()]).await
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

        manager.execute_rpc("FS", &["0x01".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x01 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::ChipInit,
            FactoryTestStatus::Running,
            0.10,
            "获取芯片初始化结果 FG 0x01",
        );

        let result = manager.execute_rpc("FG", &["0x01".to_string()]).await
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
            data: vec![status],
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

        manager.execute_rpc("FS", &["0x02".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x02 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Uuid,
            FactoryTestStatus::Running,
            0.20,
            "获取 UUID 结果 FG 0x02",
        );

        let result = manager.execute_rpc("FG", &["0x02".to_string()]).await
            .map_err(|e| format!("获取产测模式 0x02 结果失败: {}", e))?;

        let uuid: Vec<u8> = result.iter().map(|&b| b as u8).collect();
        test_result.uuid = uuid.clone();

        let uuid_str: String = uuid.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(":");
        info!("[FactoryTest] UUID: {}", uuid_str);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Uuid,
            success: true,
            message: format!("UUID: {}", uuid_str),
            data: uuid.iter().map(|&b| b as u16).collect(),
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

        manager.execute_rpc("FS", &["0x04".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x04 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.29,
            "下载配置 D 0",
        );

        manager.execute_rpc("D", &["0".to_string()]).await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.30,
            &format!("写入寄存器列表 ({} 个)", reg_values.len()),
        );

        let params = config_loader.format_for_download();
        manager.execute_rpc("L", &params).await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.31,
            "下载配置 D 1",
        );

        manager.execute_rpc("D", &["1".to_string()]).await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.32,
            "启动 TEST1 功能",
        );

        manager.execute_rpc("S", &["0x1".to_string(), "1".to_string()]).await
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

        manager.execute_rpc("S", &["0x1".to_string(), "0".to_string()]).await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::BaseNoise,
            FactoryTestStatus::Running,
            0.39,
            "获取底噪结果 FG 0x04",
        );

        let result = manager.execute_rpc("FG", &["0x04".to_string()]).await
            .map_err(|e| format!("获取产测模式 0x04 结果失败: {}", e))?;

        let base_noise: Vec<u16> = result.chunks(2)
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

        test_result.base_noise = base_noise.clone();
        info!("[FactoryTest] 底噪数据: {:?}", base_noise);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::BaseNoise,
            success: true,
            message: format!("底噪测试完成, {} 个通道", base_noise.len()),
            data: base_noise,
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

        let config_file = Self::find_config_file(config_dir, "ppg_noise")?
            .ok_or("未找到 ppg_noise 配置文件")?;

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

        manager.execute_rpc("FS", &["0x08".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x08 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.45,
            "下载配置并写入寄存器",
        );

        manager.execute_rpc("D", &["0".to_string()]).await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager.execute_rpc("L", &params).await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager.execute_rpc("D", &["1".to_string()]).await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.48,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager.execute_rpc("S", &["0x1".to_string(), "1".to_string()]).await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager.execute_rpc("S", &["0x1".to_string(), "0".to_string()]).await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::PpgNoise,
            FactoryTestStatus::Running,
            0.53,
            "获取 PPG 噪声结果 FG 0x08",
        );

        let result = manager.execute_rpc("FG", &["0x08".to_string()]).await
            .map_err(|e| format!("获取产测模式 0x08 结果失败: {}", e))?;

        let ppg_noise: Vec<u16> = result.chunks(2)
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

        test_result.ppg_noise = ppg_noise.clone();
        info!("[FactoryTest] PPG 噪声数据: {:?}", ppg_noise);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::PpgNoise,
            success: true,
            message: format!("PPG 噪声测试完成, {} 个通道", ppg_noise.len()),
            data: ppg_noise,
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

        let config_file = Self::find_config_file(config_dir, "lpctr")?
            .ok_or("未找到 lpctr 配置文件")?;

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

        manager.execute_rpc("FS", &["0x10".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x10 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.62,
            "下载配置并写入寄存器",
        );

        manager.execute_rpc("D", &["0".to_string()]).await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager.execute_rpc("L", &params).await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager.execute_rpc("D", &["1".to_string()]).await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.65,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager.execute_rpc("S", &["0x1".to_string(), "1".to_string()]).await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager.execute_rpc("S", &["0x1".to_string(), "0".to_string()]).await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lpctr,
            FactoryTestStatus::Running,
            0.68,
            "获取 LPCTR 结果 FG 0x10",
        );

        let result = manager.execute_rpc("FG", &["0x10".to_string()]).await
            .map_err(|e| format!("获取产测模式 0x10 结果失败: {}", e))?;

        let lpctr: Vec<u16> = result.chunks(2)
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

        test_result.lpctr = lpctr.clone();
        info!("[FactoryTest] LPCTR 数据: {:?}", lpctr);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lpctr,
            success: true,
            message: format!("LPCTR 测试完成, {} 个通道", lpctr.len()),
            data: lpctr,
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

        let config_file = Self::find_config_file(config_dir, "lplctr")?
            .ok_or("未找到 lplctr 配置文件")?;

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

        manager.execute_rpc("FS", &["0x20".to_string()]).await
            .map_err(|e| format!("设置产测模式 0x20 失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.82,
            "下载配置并写入寄存器",
        );

        manager.execute_rpc("D", &["0".to_string()]).await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        manager.execute_rpc("L", &params).await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        manager.execute_rpc("D", &["1".to_string()]).await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.85,
            "启动 TEST1 并采集数据 (3秒)",
        );

        manager.execute_rpc("S", &["0x1".to_string(), "1".to_string()]).await
            .map_err(|e| format!("启动 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(3));

        manager.execute_rpc("S", &["0x1".to_string(), "0".to_string()]).await
            .map_err(|e| format!("停止 TEST1 失败: {}", e))?;

        thread::sleep(Duration::from_secs(1));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Lplctr,
            FactoryTestStatus::Running,
            0.88,
            "获取 LPLCTR 结果 FG 0x20",
        );

        let result = manager.execute_rpc("FG", &["0x20".to_string()]).await
            .map_err(|e| format!("获取产测模式 0x20 结果失败: {}", e))?;

        let lplctr: Vec<u16> = result.chunks(2)
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

        test_result.lplctr = lplctr.clone();
        info!("[FactoryTest] LPLCTR 数据: {:?}", lplctr);

        Ok(Some(FactoryTestStepResult {
            step: FactoryTestStep::Lplctr,
            success: true,
            message: format!("LPLCTR 测试完成, {} 个通道", lplctr.len()),
            data: lplctr,
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

        manager.execute_rpc("S", &["0x0".to_string(), "0".to_string()]).await
            .map_err(|e| format!("关闭功能失败: {}", e))?;

        thread::sleep(Duration::from_millis(100));

        Self::publish_progress_static(
            event_bus,
            FactoryTestStep::Cleanup,
            FactoryTestStatus::Running,
            0.94,
            "切换工作模式为 0",
        );

        manager.execute_rpc("M", &["0".to_string()]).await
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
