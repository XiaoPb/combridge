//! GH3036 协议模块
//!
//! 本模块提供 GH3036 芯片协议的 Rust 实现，包括：
//! - 协议管理器
//! - 数据类型定义
//! - CSV 写入器
//! - 产测流程管理
//! - 配置文件加载
//! - 卡控配置管理
//! - 金标数据管理
//! - HR金标蓝牙监听器

pub mod config_loader;
pub mod efuse;
pub mod factory_compute;
pub mod csv_writer;
pub mod factory_test;
pub mod hr_ref_monitor;
pub mod manager;
pub mod ref_data_manager;
pub mod threshold_config;
pub mod types;

pub use config_loader::{ConfigLoader, RegisterItem};
pub use factory_test::FactoryTestManager;
pub use hr_ref_monitor::{
    get_hr_ref_monitor_collected_count, get_hr_ref_monitor_current_hr,
    get_hr_ref_monitor_device_address, get_hr_ref_monitor_state, init_hr_ref_monitor,
    is_hr_ref_monitor_running, start_hr_ref_monitor, stop_hr_ref_monitor, HrRefMonitor,
    HrRefMonitorState,
};
pub use efuse::{Gh3036EfuseReader, RegisterAccess};
pub use manager::{
    ChannelConfig, ChannelType, CsvConfig, Gh3036Manager, HrRefStatus, HrvRefStatus, RefDataStatus,
    Spo2RefStatus,
};
pub use ref_data_manager::{RefDataError, RefDataManager, REF_DATA_TIMEOUT_SECS};
pub use threshold_config::{
    evaluate_test_data, evaluate_test_item, validate_threshold_config_file,
    ChannelEvaluationResult, FactoryEvaluationResult, FactoryThresholdConfig, TestEvaluationResult,
    ThresholdConfigValidation, ThresholdOperator,
};
pub use types::{
    get_rpc_commands, get_version_types, ConfigValidationResult, FactoryTestProgressEvent,
    FactoryTestResult, FactoryTestStatus, FactoryTestStep, FactoryTestStepResult,
    Gh3036ConfigPreview, Gh3036ConfigRegisterPreview, Gh3036EventData, Gh3036FrameData, RpcCommand,
    RpcParam, VersionTypeConfig,
};

pub type Gh3036ManagerRef = std::sync::Arc<Gh3036Manager>;
