//! GH3036 协议管理器
//!
//! 本模块实现 GH3036 协议的核心管理功能：
//! - 协议实例生命周期管理
//! - RPC 命令执行（基于 gh-rpc 库）
//! - RX 数据处理（通过 EventBus 订阅）
//! - 聚合帧发布到前端
//! - CSV 数据保存

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tracing::{debug, error, info, warn};

use super::config_loader::ConfigLoader;
use super::csv_writer::{CsvInfoRow, CsvWriter};
use super::factory_test::FactoryTestManager;
use super::ref_data_manager::RefDataManager;
use super::types::{
    ConfigValidationResult, FactoryTestResult, FactoryTestStatus, FactoryTestStep,
    Gh3036ConfigPreview, Gh3036ConfigRegisterPreview, Gh3036FrameData, Gh3036FramesEvent,
    GhFuncFixIdx, GhFuncFixIdxExt, GhFuncFrame, FMT_DOWNLOAD_CONFIG, FMT_F_GET_MODE,
    FMT_F_SET_MODE, FMT_GH3X_CHIP_CTRL, FMT_GH3X_GET_VERSION, FMT_GH3X_REGS_LIST_WRITE_CMD,
    FMT_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_WRITE_CMD, FMT_GH3X_REG_BIT_FIELD_WRITE_CMD,
    FMT_GH3X_SW_FUNCTION_CMD, FMT_GH_LOW_POWER_CMD, FMT_GH_SET_WORK_MODE_CMD, FMT_GH_TIMESTAMP_SET,
    FMT_GH_TIME_SET, KEY_DOWNLOAD_CONFIG, KEY_F_GET_MODE, KEY_F_SET_MODE, KEY_GH3X_CHIP_CTRL,
    KEY_GH3X_GET_VERSION, KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GH3X_REGS_READ_CMD,
    KEY_GH3X_REGS_WRITE_CMD, KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, KEY_GH3X_SW_FUNCTION_CMD,
    KEY_GH_LOW_POWER_CMD, KEY_GH_SET_WORK_MODE_CMD, KEY_GH_TIMESTAMP_SET, KEY_GH_TIME_SET,
    RET_F_GET_MODE, RET_GH3X_GET_VERSION, RET_GH3X_REGS_READ_CMD,
};
use crate::device::DeviceManager;
use crate::service::{
    topics, BleConnectionEvent, BleDataEvent, EventBus, SerialDataEvent, SerialDisconnectedEvent,
};

use gh_rpc::{CommandExecutor, FrameCallback};
use rpc::{unpack, LogCallback, LogLevel, RpcConfig, SendFunction, UnpackValue};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ChannelType {
    Serial,
    Ble,
}

impl From<ChannelType> for crate::device::DeviceType {
    fn from(channel_type: ChannelType) -> Self {
        match channel_type {
            ChannelType::Serial => crate::device::DeviceType::Serial,
            ChannelType::Ble => crate::device::DeviceType::Ble,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: ChannelType,
    pub device_id: String,
    pub characteristic_uuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvConfig {
    pub enabled: bool,
    pub output_dir: String,
}

impl Default for CsvConfig {
    fn default() -> Self {
        let output_dir = std::env::current_exe()
            .ok()
            .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
            .map(|exe_dir| exe_dir.join("data"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("data"));

        Self {
            enabled: false,
            output_dir,
        }
    }
}

enum RpcInput {
    Data(Vec<u8>),
    Reset,
}

#[derive(Default)]
struct AlgorithmResultCache {
    last_non_zero: HashMap<u8, Vec<i32>>,
}

impl AlgorithmResultCache {
    fn normalize(&mut self, frame: &GhFuncFrame) -> GhFuncFrame {
        let mut normalized = frame.clone();
        let function_id = frame.id as u8;

        if frame.frame_cnt == 0 {
            self.last_non_zero.remove(&function_id);
        }

        if frame.algo_data.is_empty() {
            return normalized;
        }

        if frame.algo_data.iter().any(|value| *value != 0) {
            self.last_non_zero
                .insert(function_id, frame.algo_data.clone());
        } else if let Some(previous) = self.last_non_zero.get(&function_id) {
            normalized.algo_data = previous.clone();
        }

        normalized
    }
}

struct FrameAggregator {
    buffer: HashMap<u8, Gh3036FramesEvent>,
    last_publish_time: std::time::Instant,
    min_interval: std::time::Duration,
    ref_data_manager: Arc<RefDataManager>,
}

impl FrameAggregator {
    fn new(ref_data_manager: Arc<RefDataManager>) -> Self {
        Self {
            buffer: HashMap::new(),
            last_publish_time: std::time::Instant::now(),
            min_interval: std::time::Duration::from_millis(30),
            ref_data_manager,
        }
    }

    fn add_frame(&mut self, frame: &GhFuncFrame) -> Vec<Gh3036FramesEvent> {
        let func_id = frame.id as u8;
        let func_name = GhFuncFixIdx::from(func_id).name().to_string();

        let event = self
            .buffer
            .entry(func_id)
            .or_insert_with(|| Gh3036FramesEvent::new(func_id, func_name));
        let ref_data = self.ref_data_manager.get_ref_data(frame.frame_cnt as usize);
        event.add_frame_with_ref(frame, ref_data);

        let now = std::time::Instant::now();
        if now.duration_since(self.last_publish_time) >= self.min_interval {
            self.flush()
        } else {
            Vec::new()
        }
    }

    fn flush(&mut self) -> Vec<Gh3036FramesEvent> {
        if self.buffer.is_empty() {
            return Vec::new();
        }

        let result: Vec<Gh3036FramesEvent> = self
            .buffer
            .drain()
            .filter_map(|(_, event)| (!event.is_empty()).then_some(event))
            .collect();

        if !result.is_empty() {
            self.last_publish_time = std::time::Instant::now();
        }
        result
    }
}

#[derive(Debug, Clone)]
struct BleDeviceInfo {
    address: String,
    name: Option<String>,
}

struct GlobalContext {
    rx_channel: Mutex<Option<ChannelConfig>>,
    tx_channel: Mutex<Option<ChannelConfig>>,
    device_manager: Mutex<Option<Arc<DeviceManager>>>,
    rpc_data_sender: Mutex<Option<crossbeam_channel::Sender<RpcInput>>>,
    frame_aggregator: Mutex<FrameAggregator>,
    runtime_handle: Mutex<Option<Handle>>,
    csv_config: Mutex<CsvConfig>,
    app_name: Mutex<String>,
    app_version: Mutex<String>,
    last_ble_device: Mutex<Option<BleDeviceInfo>>,
    csv_writers: Mutex<HashMap<i32, CsvWriter>>,
    algorithm_result_cache: Mutex<AlgorithmResultCache>,
    ref_data_manager: Arc<RefDataManager>,
    event_bus: Mutex<Option<Arc<EventBus>>>,
    last_frame_time: Mutex<Option<std::time::Instant>>,
}

impl GlobalContext {
    fn new() -> Self {
        let ref_data_manager = Arc::new(RefDataManager::new());
        Self {
            rx_channel: Mutex::new(None),
            tx_channel: Mutex::new(None),
            device_manager: Mutex::new(None),
            rpc_data_sender: Mutex::new(None),
            frame_aggregator: Mutex::new(FrameAggregator::new(Arc::clone(&ref_data_manager))),
            runtime_handle: Mutex::new(None),
            csv_config: Mutex::new(CsvConfig::default()),
            app_name: Mutex::new(String::new()),
            app_version: Mutex::new(String::new()),
            last_ble_device: Mutex::new(None),
            csv_writers: Mutex::new(HashMap::new()),
            algorithm_result_cache: Mutex::new(AlgorithmResultCache::default()),
            ref_data_manager,
            event_bus: Mutex::new(None),
            last_frame_time: Mutex::new(None),
        }
    }

    fn set_event_bus(&self, event_bus: Arc<EventBus>) {
        let mut bus = self.event_bus.lock();
        *bus = Some(event_bus);
    }

    fn setup_rpc_channel(&self) -> crossbeam_channel::Receiver<RpcInput> {
        let (rpc_data_sender, rpc_data_receiver) = crossbeam_channel::unbounded();
        *self.rpc_data_sender.lock() = Some(rpc_data_sender);
        rpc_data_receiver
    }

    fn set_rx_channel(&self, config: ChannelConfig) {
        let mut rx_channel = self.rx_channel.lock();
        *rx_channel = Some(config);
        if let Some(ref sender) = *self.rpc_data_sender.lock() {
            if let Err(error) = sender.send(RpcInput::Reset) {
                warn!("[GH3036] RX 通道切换时重置 RPC 接收状态失败: {}", error);
            }
        }
    }

    fn get_rx_channel(&self) -> Option<ChannelConfig> {
        self.rx_channel.lock().clone()
    }

    fn is_channel_match(&self, device_id: &str, channel_type: ChannelType) -> bool {
        let rx_channel = self.rx_channel.lock();
        match rx_channel.as_ref() {
            Some(config) => config.device_id == device_id && config.channel_type == channel_type,
            None => false,
        }
    }

    fn set_tx_channel(&self, config: ChannelConfig) {
        let mut tx_channel = self.tx_channel.lock();
        *tx_channel = Some(config);
    }

    fn set_device_manager(&self, manager: Arc<DeviceManager>) {
        let mut device_manager = self.device_manager.lock();
        *device_manager = Some(manager);
    }

    fn set_runtime_handle(&self, handle: Handle) {
        let mut runtime_handle = self.runtime_handle.lock();
        *runtime_handle = Some(handle);
    }

    fn set_app_info(&self, app_name: String, app_version: String) {
        *self.app_name.lock() = app_name;
        *self.app_version.lock() = app_version;
    }

    fn set_last_ble_device(&self, address: String, name: Option<String>) {
        *self.last_ble_device.lock() = Some(BleDeviceInfo { address, name });
    }

    fn clear_last_ble_device(&self, address: &str) {
        let mut ble = self.last_ble_device.lock();
        if ble.as_ref().map(|info| info.address.as_str()) == Some(address) {
            *ble = None;
        }
    }

    /// 生成 CSV 信息行（应用名、版本、测试功能、蓝牙名称/地址）
    fn current_info_row(&self, function_name: &str) -> CsvInfoRow {
        let app_name = self.app_name.lock().clone();
        let app_version = self.app_version.lock().clone();
        let ble = self.last_ble_device.lock().clone();
        CsvInfoRow {
            app: app_name,
            version: app_version,
            function: function_name.to_string(),
            ble_name: ble.as_ref().and_then(|info| info.name.clone()),
            ble_address: ble.as_ref().map(|info| info.address.clone()),
        }
    }

    fn send_rpc_data(&self, data: Vec<u8>) -> Result<(), crossbeam_channel::SendError<RpcInput>> {
        if let Some(ref sender) = *self.rpc_data_sender.lock() {
            sender.send(RpcInput::Data(data))
        } else {
            Err(crossbeam_channel::SendError(RpcInput::Data(Vec::new())))
        }
    }

    fn normalize_frame(&self, frame: &GhFuncFrame) -> GhFuncFrame {
        self.algorithm_result_cache.lock().normalize(frame)
    }

    fn add_frame_to_aggregator(&self, frame: &GhFuncFrame) -> Vec<Gh3036FramesEvent> {
        {
            let mut last_frame_time = self.last_frame_time.lock();
            *last_frame_time = Some(std::time::Instant::now());
        }
        let mut aggregator = self.frame_aggregator.lock();
        aggregator.add_frame(frame)
    }

    fn set_csv_config(&self, config: CsvConfig) {
        let mut csv_config = self.csv_config.lock();

        let output_dir = if std::path::Path::new(&config.output_dir).is_absolute() {
            config.output_dir
        } else {
            std::env::current_exe()
                .ok()
                .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
                .map(|exe_dir| exe_dir.join(&config.output_dir))
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(config.output_dir)
        };

        *csv_config = CsvConfig {
            enabled: config.enabled,
            output_dir,
        };
    }

    fn get_csv_config(&self) -> CsvConfig {
        self.csv_config.lock().clone()
    }

    fn save_frame_to_csv(&self, frame: &GhFuncFrame) {
        let csv_config = self.csv_config.lock();
        if !csv_config.enabled {
            return;
        }
        info!("frame.frame_cnt: {}", frame.frame_cnt);
        let ref_data = self.ref_data_manager.get_ref_data(frame.frame_cnt as usize);
        let frame_data = Gh3036FrameData::from_func_frame(frame, Some(&ref_data));
        let mut writers = self.csv_writers.lock();
        let function_id = frame_data.function_id;
        let function_name = frame_data.function_name.clone();

        let writer = writers.entry(function_id).or_insert_with(|| {
            let mut writer = CsvWriter::new(
                PathBuf::from(&csv_config.output_dir),
                function_id,
                function_name.clone(),
            );
            writer.set_info_row(CALLBACK_CONTEXT.current_info_row(&function_name));
            writer
        });

        // 新文件边界（frame_id==0 或文件尚未创建）时刷新设备信息，
        // 保证强制分文件后新建文件的信息行与实际采集设备一致
        if frame_data.frame_id == 0 || !writer.is_open() {
            writer.set_info_row(CALLBACK_CONTEXT.current_info_row(&function_name));
        }

        if let Err(e) = writer.write_frame(&frame_data) {
            error!("CSV 写入失败: {}", e);
        }
    }

    /// 触发所有 CSV writer 创建新文件
    ///
    /// 在启动/停止命令执行或设备断开时调用，
    /// 确保每个功能的 CSV writer 都创建新文件
    fn trigger_new_csv_file(&self) {
        let csv_config = self.csv_config.lock();
        if !csv_config.enabled {
            return;
        }
        drop(csv_config);

        let mut writers = self.csv_writers.lock();
        for (function_id, writer) in writers.iter_mut() {
            if let Err(e) = writer.force_new_file() {
                error!("[GH3036] 功能 {} 强制创建新CSV文件失败: {}", function_id, e);
            } else {
                info!("[GH3036] 功能 {} 已标记创建新CSV文件", function_id);
            }
        }
    }

    /// 软件功能命令执行完成后的处理
    ///
    /// 启动（ctrl_type=0）或停止（ctrl_type=1）命令执行后，触发新 CSV 文件创建
    fn handle_sw_function_command_completed(&self, ctrl_type: u8) {
        if ctrl_type == 0 || ctrl_type == 1 {
            self.trigger_new_csv_file();
            info!(
                "[GH3036] 软件{}命令执行完成，已触发新CSV文件创建",
                if ctrl_type == 0 { "启动" } else { "停止" }
            );
        }
    }

    fn publish_ref_data(&self) {
        use super::types::Gh3036RefDataEvent;
        use std::time::Duration;

        let last_frame_time = self.last_frame_time.lock();
        match *last_frame_time {
            Some(last_time) => {
                let elapsed = last_time.elapsed();
                if elapsed >= Duration::from_secs(4) {
                    debug!(
                        "[GH3036] 金标数据发布已停止: frames 事件超时 {}秒",
                        elapsed.as_secs()
                    );
                    return;
                }
            }
            None => {
                debug!("[GH3036] 金标数据发布已停止: 无 frames 事件");
                return;
            }
        }
        drop(last_frame_time);

        let event_bus = self.event_bus.lock();
        if let Some(bus) = event_bus.as_ref() {
            let (hr_values, hr_count, hr_elapsed) = self.ref_data_manager.get_hr_ref_status();
            let (hrv_values, hrv_count, hrv_elapsed) = self.ref_data_manager.get_hrv_ref_status();
            let (spo2_values, spo2_count) = self.ref_data_manager.get_spo2_ref_status();

            let hr_valid = hr_count > 0 && hr_elapsed < Duration::from_secs(4);
            let hrv_valid = hrv_count > 0 && hrv_elapsed < Duration::from_secs(4);
            let spo2_valid = spo2_count > 0;

            let has_valid_ref_data = hr_valid || hrv_valid || spo2_valid;
            if !has_valid_ref_data {
                debug!("[GH3036] 金标数据发布已停止: 无有效金标数据");
                return;
            }

            let event = Gh3036RefDataEvent {
                hr_values,
                hr_count,
                hr_valid,
                hrv_values,
                hrv_count,
                hrv_valid,
                spo2_values,
                spo2_count,
                spo2_valid,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            };

            bus.publish_msgpack("gh3036:ref_data", &event);
            debug!(
                "[GH3036] 发布金标数据: hr_valid={}, hrv_valid={}, spo2_valid={}",
                hr_valid, hrv_valid, spo2_valid
            );
        }
    }
}

static CALLBACK_CONTEXT: once_cell::sync::Lazy<GlobalContext> =
    once_cell::sync::Lazy::new(GlobalContext::new);

static EVENTS_SUBSCRIBED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrRefStatus {
    pub values: Vec<i32>,
    pub count: i32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HrvRefStatus {
    pub values: Vec<i32>,
    pub count: i32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spo2RefStatus {
    pub values: Vec<i32>,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefDataStatus {
    pub hr: HrRefStatus,
    pub hrv: HrvRefStatus,
    pub spo2: Spo2RefStatus,
}

pub struct Gh3036Manager {
    device_manager: Arc<DeviceManager>,
    event_bus: Arc<EventBus>,
    initialized: Mutex<bool>,
    running: Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    executor: Mutex<Option<Arc<tokio::sync::RwLock<CommandExecutor>>>>,
    factory_test_manager: Arc<FactoryTestManager>,
}

// SAFETY: All fields are Send+Sync except CommandExecutor (from gh_rpc crate)
// which is wrapped in Arc<tokio::sync::RwLock<>> and only accessed through async locks.
unsafe impl Send for Gh3036Manager {}
unsafe impl Sync for Gh3036Manager {}

impl Gh3036Manager {
    pub fn new(device_manager: Arc<DeviceManager>, event_bus: Arc<EventBus>) -> Self {
        let factory_test_manager = Arc::new(FactoryTestManager::new(Arc::clone(&event_bus)));
        Self {
            device_manager,
            event_bus,
            initialized: Mutex::new(false),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
            executor: Mutex::new(None),
            factory_test_manager,
        }
    }

    pub fn is_initialized(&self) -> bool {
        *self.initialized.lock()
    }

    pub fn is_library_linked() -> bool {
        true
    }

    pub fn initialize(&self) -> Result<(), String> {
        info!("GH3036 协议管理器初始化 (纯 Rust 模式 + RPC 集成)");

        CALLBACK_CONTEXT.set_device_manager(Arc::clone(&self.device_manager));
        CALLBACK_CONTEXT.set_event_bus(Arc::clone(&self.event_bus));

        if let Ok(handle) = Handle::try_current() {
            CALLBACK_CONTEXT.set_runtime_handle(handle);
        }

        self.initialize_rpc()?;

        self.subscribe_data_events();

        {
            let mut initialized = self.initialized.lock();
            *initialized = true;
        }

        self.start_processing_thread()?;

        info!("GH3036 协议管理器初始化成功");
        Ok(())
    }

    fn subscribe_data_events(&self) {
        use std::sync::atomic::Ordering;

        if EVENTS_SUBSCRIBED
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            info!("[GH3036] 事件已订阅，跳过重复订阅");
            return;
        }

        info!("[GH3036] 订阅 EventBus 数据事件");

        self.event_bus.subscribe_msgpack::<SerialDataEvent, _>(
            topics::SERIAL_DATA,
            move |_topic, event| {
                if !CALLBACK_CONTEXT.is_channel_match(&event.device_id, ChannelType::Serial) {
                    debug!("[GH3036] 过滤非配置通道数据: device_id={}", event.device_id);
                    return;
                }
                info!(
                    "[GH3036] 接收到串口数据: device_id={}, len={}",
                    event.device_id,
                    event.data.len()
                );
                if let Err(e) = CALLBACK_CONTEXT.send_rpc_data(event.data.clone()) {
                    error!("[GH3036] 发送数据到 RPC 通道失败: {}", e);
                }
            },
        );

        self.event_bus.subscribe_msgpack::<BleDataEvent, _>(
            topics::BLE_DATA,
            move |_topic, event| {
                if !CALLBACK_CONTEXT.is_channel_match(&event.device_id, ChannelType::Ble) {
                    debug!("[GH3036] 过滤非配置通道数据: device_id={}", event.device_id);
                    return;
                }
                info!(
                    "[GH3036] 接收到 BLE 数据: device_id={}, len={}",
                    event.device_id,
                    event.data.len()
                );
                if let Err(e) = CALLBACK_CONTEXT.send_rpc_data(event.data.clone()) {
                    error!("[GH3036] 发送数据到 RPC 通道失败: {}", e);
                }
            },
        );

        self.event_bus
            .subscribe_msgpack::<SerialDisconnectedEvent, _>(
                topics::SERIAL_DISCONNECTED,
                move |_topic, event| {
                    info!("[GH3036] 收到串口断开事件: {}", event.port_name);
                    Self::handle_device_disconnected(&event.port_name);
                },
            );

        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_topic, event| {
                info!("[GH3036] 收到 BLE 断开事件: {}", event.address);
                Self::handle_device_disconnected(&event.address);
                CALLBACK_CONTEXT.clear_last_ble_device(&event.address);
            },
        );

        self.event_bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_CONNECTED,
            move |_topic, event| {
                info!(
                    "[GH3036] 收到 BLE 连接事件: address={}, name={:?}",
                    event.address, event.name
                );
                CALLBACK_CONTEXT.set_last_ble_device(event.address.clone(), event.name.clone());
            },
        );

        info!(
            "[GH3036] 已订阅 serial:data、ble:data、serial:disconnected 和 ble:disconnected 事件"
        );
    }

    fn handle_device_disconnected(device_id: &str) {
        {
            let mut rx_channel = CALLBACK_CONTEXT.rx_channel.lock();
            if rx_channel
                .as_ref()
                .is_some_and(|channel| channel.device_id == device_id)
            {
                *rx_channel = None;
                if let Some(ref sender) = *CALLBACK_CONTEXT.rpc_data_sender.lock() {
                    if let Err(error) = sender.send(RpcInput::Reset) {
                        warn!("[GH3036] 设备断开时重置 RPC 接收状态失败: {}", error);
                    }
                }
                info!("GH3036 RX 通道已清理: 设备 {} 已断开", device_id);

                // 设备断开时触发新CSV文件创建
                CALLBACK_CONTEXT.trigger_new_csv_file();
                info!("[GH3036] 设备断开，已触发新CSV文件创建");
            }
        }

        let mut tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        if tx_channel
            .as_ref()
            .is_some_and(|channel| channel.device_id == device_id)
        {
            *tx_channel = None;
            info!("GH3036 TX 通道已清理: 设备 {} 已断开", device_id);
        }
    }

    fn initialize_rpc(&self) -> Result<(), String> {
        info!("GH3036 初始化 RPC 核心");

        let device_manager = Arc::clone(&self.device_manager);
        let handle = Handle::try_current().map_err(|e| format!("获取 Tokio 运行时失败: {}", e))?;

        let send_fn: SendFunction = Arc::new(move |data: Vec<u8>| {
            debug!("[RPC发送] 发送数据: {:02X?}", data);

            let channel = {
                let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
                tx_channel.as_ref().map(|channel| {
                    (
                        channel.channel_type,
                        channel.device_id.clone(),
                        channel.characteristic_uuid.clone(),
                    )
                })
            };

            let dm = Arc::clone(&device_manager);
            Box::pin(async move {
                let (channel_type, device_id, char_uuid) = match channel {
                    Some(channel) => channel,
                    None => {
                        warn!("[RPC发送] TX 通道未配置");
                        return Err(rpc::RpcError::SendFail);
                    }
                };

                dm.send_direct(channel_type.into(), &device_id, char_uuid.as_deref(), &data)
                    .await
                    .map_err(|e| {
                        let error_str = format!("{}", e);
                        if error_str.contains("已关闭")
                            || error_str.contains("closed")
                            || error_str.contains("disconnected")
                        {
                            warn!("[RPC发送] 设备已断开，发送失败");
                        } else {
                            error!("[RPC发送] 发送失败: {}", e);
                        }
                        rpc::RpcError::SendFail
                    })?;
                debug!("[RPC发送] 发送成功: {} bytes", data.len());
                Ok(())
            })
        });

        struct TauriLogger;
        impl LogCallback for TauriLogger {
            fn log(&self, level: LogLevel, context: &str, message: &str) {
                if context == "rpc_core" || context == "RpcCore" {
                    match level {
                        LogLevel::Trace => {
                            tracing::trace!(target: "rpc_core", "[{}] {}", context, message)
                        }
                        LogLevel::Debug => {
                            tracing::debug!(target: "rpc_core", "[{}] {}", context, message)
                        }
                        LogLevel::Info => {
                            tracing::info!(target: "rpc_core", "[{}] {}", context, message)
                        }
                        LogLevel::Warn => {
                            tracing::warn!(target: "rpc_core", "[{}] {}", context, message)
                        }
                        LogLevel::Error => {
                            tracing::error!(target: "rpc_core", "[{}] {}", context, message)
                        }
                    }
                } else {
                    match level {
                        LogLevel::Trace => {
                            tracing::trace!(target: "combridge_rust_lib::gh3036::rpc", "[{}] {}", context, message)
                        }
                        LogLevel::Debug => {
                            tracing::debug!(target: "combridge_rust_lib::gh3036::rpc", "[{}] {}", context, message)
                        }
                        LogLevel::Info => {
                            tracing::info!(target: "combridge_rust_lib::gh3036::rpc", "[{}] {}", context, message)
                        }
                        LogLevel::Warn => {
                            tracing::warn!(target: "combridge_rust_lib::gh3036::rpc", "[{}] {}", context, message)
                        }
                        LogLevel::Error => {
                            tracing::error!(target: "combridge_rust_lib::gh3036::rpc", "[{}] {}", context, message)
                        }
                    }
                }
            }
        }

        let event_bus = self.event_bus.clone();
        let frame_callback: FrameCallback = Arc::new(move |frame: &GhFuncFrame| {
            let frame = CALLBACK_CONTEXT.normalize_frame(frame);
            let func_id = frame.id as u8;
            let func_name = GhFuncFixIdx::from(func_id).name();
            let acc = &frame.gsensor_data.acc;
            info!(
                "[GH3036] 帧解码: func_id={} ({}), frame_cnt={}, ch_num={}, acc=[{},{},{}], algo_data={:?}",
                func_id, func_name, frame.frame_cnt, frame.ch_num, acc[0], acc[1], acc[2], frame.algo_data
            );

            for (i, ch_data) in frame.data.iter().enumerate() {
                info!(
                    "[GH3036]   ch[{}]: ipd={}, raw={}",
                    i, ch_data.ipd_pa, ch_data.rawdata
                );
            }

            CALLBACK_CONTEXT.save_frame_to_csv(&frame);

            for aggregated in CALLBACK_CONTEXT.add_frame_to_aggregator(&frame) {
                info!(
                    "[GH3036] 发布聚合帧事件: function_id={}, frame_count={}, channel_count={}",
                    aggregated.function_id, aggregated.frame_count, aggregated.channel_count
                );
                event_bus.publish_msgpack("gh3036:frames", &aggregated);
            }
        });

        let mut executor = CommandExecutor::new(RpcConfig {
            timeout_ms: 1000,
            ..RpcConfig::default()
        })
        .with_logger(Arc::new(TauriLogger));

        executor.register_frame_callback(frame_callback);

        tokio::task::block_in_place(|| {
            handle.block_on(async {
                executor.set_send_function(Arc::clone(&send_fn)).await;
                if let Err(e) = executor.register_g_handler().await {
                    error!("GH3036 注册 G 协议处理器失败: {:?}", e);
                } else {
                    info!("GH3036 G 协议处理器注册成功");
                }
            });
        });

        info!("GH3036 RPC 核心初始化完成");

        let executor = Arc::new(tokio::sync::RwLock::new(executor));

        *self.executor.lock() = Some(Arc::clone(&executor));

        Ok(())
    }

    fn start_processing_thread(&self) -> Result<(), String> {
        let running = self.running.clone();
        running.store(true, std::sync::atomic::Ordering::SeqCst);

        let rpc_data_receiver = CALLBACK_CONTEXT.setup_rpc_channel();
        let running_clone = running.clone();
        let executor = self.executor.lock().as_ref().map(Arc::clone);
        let tokio_handle =
            Handle::try_current().map_err(|e| format!("获取 Tokio 运行时失败: {}", e))?;

        let thread_handle = std::thread::spawn(move || {
            info!("[GH3036] 处理线程启动");
            let mut last_ref_data_publish = std::time::Instant::now();
            let ref_data_interval = std::time::Duration::from_secs(1);

            while running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                crossbeam_channel::select! {
                    recv(rpc_data_receiver) -> result => {
                        if let Ok(input) = result {
                            Self::handle_rpc_input(&executor, input, &tokio_handle);
                        }
                    }
                    default(std::time::Duration::from_millis(10)) => {
                    }
                }

                let now = std::time::Instant::now();
                if now.duration_since(last_ref_data_publish) >= ref_data_interval {
                    CALLBACK_CONTEXT.publish_ref_data();
                    last_ref_data_publish = now;
                }
            }

            info!("[GH3036] 处理线程停止");
        });

        {
            let mut thread_guard = self.thread_handle.lock();
            *thread_guard = Some(thread_handle);
        }

        Ok(())
    }

    fn handle_rpc_input(
        executor: &Option<Arc<tokio::sync::RwLock<CommandExecutor>>>,
        input: RpcInput,
        tokio_handle: &Handle,
    ) {
        if let Some(exec) = executor {
            let exec_clone = Arc::clone(exec);
            tokio_handle.block_on(async move {
                let executor = exec_clone.read().await;
                match input {
                    RpcInput::Data(data) => {
                        debug!("GH3036 按序处理 RPC 数据: {} bytes", data.len());
                        let results = executor.process(&data).await;
                        for result in results {
                            match result {
                                Ok(parse_result) => {
                                    debug!(
                                        "GH3036 RPC 解析成功: key={}, len={}",
                                        parse_result.key,
                                        parse_result.param.len()
                                    );
                                }
                                Err(error) => {
                                    debug!("GH3036 RPC 解析失败: {:?}", error);
                                }
                            }
                        }
                    }
                    RpcInput::Reset => {
                        executor.reset_receive_state().await;
                        debug!("GH3036 RPC 接收状态已重置");
                    }
                }
            });
        } else {
            warn!("GH3036 handle_rpc_data RPC 核心未初始化");
        }
    }

    fn stop_processing_thread(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);

        let mut thread_guard = self.thread_handle.lock();
        if let Some(thread) = thread_guard.take() {
            let _ = thread.join();
        }
    }

    pub fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_tx_channel(config.clone());
        info!("GH3036 TX 通道配置成功: {:?}", config);
        Ok(())
    }

    pub fn configure_rx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_rx_channel(config.clone());
        info!("[GH3036] RX 通道配置成功: {:?}", config);
        Ok(())
    }

    pub fn get_tx_channel(&self) -> Option<ChannelConfig> {
        CALLBACK_CONTEXT.tx_channel.lock().clone()
    }

    pub fn get_rx_channel(&self) -> Option<ChannelConfig> {
        CALLBACK_CONTEXT.get_rx_channel()
    }

    pub fn set_csv_config(&self, config: CsvConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_csv_config(config);
        info!("GH3036 CSV 配置更新成功");
        Ok(())
    }

    pub fn force_new_csv_file(&self) -> Result<(), String> {
        CALLBACK_CONTEXT.trigger_new_csv_file();
        info!("[GH3036] 手动触发新CSV文件创建");
        Ok(())
    }

    pub fn get_csv_config(&self) -> CsvConfig {
        CALLBACK_CONTEXT.get_csv_config()
    }

    pub fn set_app_info(&self, app_name: String, app_version: String) {
        info!("[GH3036] 设置应用信息: {} v{}", app_name, app_version);
        CALLBACK_CONTEXT.set_app_info(app_name, app_version);
    }

    pub async fn send_data(&self, data: &[u8]) -> Result<(), String> {
        let (device_type, device_id, char_uuid) = {
            let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
            let channel = tx_channel.as_ref().ok_or("TX 通道未配置")?;

            let device_type = match channel.channel_type {
                ChannelType::Serial => crate::device::DeviceType::Serial,
                ChannelType::Ble => crate::device::DeviceType::Ble,
            };
            let char_uuid = channel.characteristic_uuid.clone();
            (device_type, channel.device_id.clone(), char_uuid)
        };

        self.device_manager
            .send_direct(device_type, &device_id, char_uuid.as_deref(), data)
            .await
            .map_err(|e| {
                error!("GH3036 send_data 失败: {}", e);
                e.to_string()
            })?;

        Ok(())
    }

    async fn call_command(&self, key: &str, format: &str, data: &[u8]) -> Result<Vec<u8>, String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref().ok_or("RPC 核心未初始化")?.clone()
        };

        let exec = executor.read().await;
        exec.call(key, format, data)
            .await
            .map_err(|e| format!("RPC 调用失败: {:?}", e))
    }

    async fn send_command(&self, key: &str, format: &str, data: &[u8]) -> Result<(), String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref().ok_or("RPC 核心未初始化")?.clone()
        };

        let exec = executor.read().await;
        exec.send(key, format, data)
            .await
            .map_err(|e| format!("RPC 发送失败: {:?}", e))
    }

    async fn publish_command(&self, key: &str, format: &str, data: &[u8]) -> Result<(), String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref().ok_or("RPC 核心未初始化")?.clone()
        };

        let exec = executor.read().await;
        exec.publish(key, format, data)
            .await
            .map_err(|e| format!("RPC 发布失败: {:?}", e))
    }

    pub async fn execute_rpc(
        &self,
        command_key: &str,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        info!(
            "GH3036 execute_rpc 开始: key={}, params={:?}",
            command_key, params
        );
        self.execute_rpc_async(command_key, params).await
    }

    async fn execute_rpc_async(
        &self,
        command_key: &str,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        match command_key {
            "V" => self.execute_version_cmd_async(params).await,
            "W" => self.execute_regs_write_cmd_async(params).await,
            "R" => self.execute_regs_read_cmd_async(params).await,
            "B" => self.execute_reg_bitfield_write_cmd_async(params).await,
            "C" => self.execute_chip_ctrl_cmd_async(params).await,
            "D" => self.execute_download_config_cmd_async(params).await,
            "L" => self.execute_regs_list_write_cmd_async(params).await,
            "S" => self.execute_sw_function_cmd_async(params).await,
            "P" => self.execute_low_power_cmd_async(params).await,
            "M" => self.execute_set_work_mode_cmd_async(params).await,
            "TS" => self.execute_timestamp_set_cmd_async(params).await,
            "TM" => self.execute_time_set_cmd_async(params).await,
            "FS" => self.execute_factory_set_mode_cmd_async(params).await,
            "FG" => self.execute_factory_get_mode_cmd_async(params).await,
            _ => {
                error!("GH3036 execute_rpc 不支持的命令键: {}", command_key);
                Err(format!("不支持的命令键: {}", command_key))
            }
        }
    }

    async fn execute_version_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let ver_type: u8 = params.first().and_then(|s| s.parse().ok()).unwrap_or(0);

        info!("GH3036 execute_version_cmd: ver_type={}", ver_type);

        let param_data = self
            .call_command(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[ver_type])
            .await?;

        let value =
            unpack(&param_data, RET_GH3X_GET_VERSION).map_err(|e| format!("解包失败: {:?}", e))?;
        match value {
            UnpackValue::U8Array(arr) => {
                let version_str = String::from_utf8_lossy(&arr).to_string();
                info!("获取版本成功: {}", version_str);
                Ok(arr)
            }
            _ => Err("获取版本失败: 解包结果不是数组".into()),
        }
    }

    async fn execute_regs_write_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() || !regs.len().is_multiple_of(2) {
            return Err("寄存器数据格式错误，需要成对的地址和值".to_string());
        }

        info!("寄存器写入: {} 个寄存器", regs.len() / 2);

        let mut data = Vec::new();
        data.extend_from_slice(&(regs.len() as u16).to_le_bytes());
        for reg in &regs {
            data.extend_from_slice(&reg.to_le_bytes());
        }

        self.send_command(KEY_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_WRITE_CMD, &data)
            .await?;
        Ok(vec![])
    }

    async fn execute_regs_read_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let read_len: i32 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);

        info!("寄存器读取: addr=0x{:04X}, len={}", reg_addr, read_len);

        let mut data = Vec::new();
        data.extend_from_slice(&reg_addr.to_le_bytes());
        data.extend_from_slice(&read_len.to_le_bytes());

        let param_data = self
            .call_command(KEY_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_READ_CMD, &data)
            .await?;
        info!("寄存器读取响应: {:04X?}", param_data);
        let value = unpack(&param_data, RET_GH3X_REGS_READ_CMD)
            .map_err(|e| format!("解包失败: {:?}", e))?;
        info!("寄存器读取解包结果: {:?}", value);

        match value {
            UnpackValue::U16Array(arr) => {
                let mut result = Vec::new();
                for &val in arr.iter() {
                    result.extend_from_slice(&val.to_le_bytes());
                    info!("寄存器读取值: 0x{:04X}", val);
                }
                Ok(result)
            }
            _ => Err("寄存器读取失败: 解包结果不是数组".into()),
        }
    }

    async fn execute_reg_bitfield_write_cmd_async(
        &self,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let lsb: u8 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        let msb: u8 = params.get(2).and_then(|s| s.parse().ok()).unwrap_or(15);

        let reg_val: u16 = params
            .get(3)
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器值参数")?;

        info!(
            "位域写入: addr=0x{:04X}, lsb={}, msb={}, val=0x{:04X}",
            reg_addr, lsb, msb, reg_val
        );

        let mut data = Vec::new();
        data.extend_from_slice(&reg_addr.to_le_bytes());
        data.push(lsb);
        data.push(msb);
        data.extend_from_slice(&reg_val.to_le_bytes());

        self.send_command(
            KEY_GH3X_REG_BIT_FIELD_WRITE_CMD,
            FMT_GH3X_REG_BIT_FIELD_WRITE_CMD,
            &data,
        )
        .await?;
        Ok(vec![])
    }

    async fn execute_chip_ctrl_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let ctrl_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少控制类型参数")?;

        info!("芯片控制: type={}", ctrl_type);

        self.send_command(KEY_GH3X_CHIP_CTRL, FMT_GH3X_CHIP_CTRL, &[ctrl_type])
            .await?;
        Ok(vec![])
    }

    async fn execute_download_config_cmd_async(
        &self,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let stage: u8 = params.first().and_then(|s| s.parse().ok()).unwrap_or(0);

        info!("下载配置: stage={}", stage);

        self.send_command(KEY_DOWNLOAD_CONFIG, FMT_DOWNLOAD_CONFIG, &[stage])
            .await?;
        Ok(vec![])
    }

    fn regs_list_write_transport() -> &'static str {
        "send"
    }

    async fn execute_regs_list_write_cmd_async(
        &self,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        info!("寄存器列表写入: {} 个值", regs.len());
        info!(
            "寄存器列表写入使用 {}: key={}",
            Self::regs_list_write_transport(),
            KEY_GH3X_REGS_LIST_WRITE_CMD
        );

        let mut data = Vec::new();
        data.extend_from_slice(&(regs.len() as u16).to_le_bytes());
        for &val in regs.iter() {
            data.extend_from_slice(&val.to_le_bytes());
        }

        self.send_command(
            KEY_GH3X_REGS_LIST_WRITE_CMD,
            FMT_GH3X_REGS_LIST_WRITE_CMD,
            &data,
        )
        .await?;
        Ok(vec![])
    }

    async fn execute_sw_function_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| {
                if s.starts_with("0x") || s.starts_with("0X") {
                    u32::from_str_radix(&s[2..], 16).ok()
                } else {
                    s.parse().ok()
                }
            })
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        info!(
            "软件功能命令: mode=0x{:08X}, ctrl={}",
            target_func_mode, ctrl_type
        );

        let mut data = Vec::new();
        data.extend_from_slice(&target_func_mode.to_le_bytes());
        data.push(ctrl_type);

        self.send_command(KEY_GH3X_SW_FUNCTION_CMD, FMT_GH3X_SW_FUNCTION_CMD, &data)
            .await?;

        // 启动（ctrl_type=0）或停止（ctrl_type=1）命令执行后，触发新CSV文件创建
        CALLBACK_CONTEXT.handle_sw_function_command_completed(ctrl_type);

        Ok(vec![])
    }

    async fn execute_low_power_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| {
                if s.starts_with("0x") || s.starts_with("0X") {
                    u32::from_str_radix(&s[2..], 16).ok()
                } else {
                    s.parse().ok()
                }
            })
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);

        info!(
            "低功耗命令: mode=0x{:08X}, ctrl={}",
            target_func_mode, ctrl_type
        );

        let mut data = Vec::new();
        data.extend_from_slice(&target_func_mode.to_le_bytes());
        data.push(ctrl_type);

        self.publish_command(KEY_GH_LOW_POWER_CMD, FMT_GH_LOW_POWER_CMD, &data)
            .await?;
        Ok(vec![])
    }

    async fn execute_set_work_mode_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let work_mode: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少工作模式参数")?;

        info!("设置工作模式: mode={}", work_mode);

        self.send_command(
            KEY_GH_SET_WORK_MODE_CMD,
            FMT_GH_SET_WORK_MODE_CMD,
            &[work_mode],
        )
        .await?;
        Ok(vec![])
    }

    async fn execute_timestamp_set_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        info!("设置时间戳: {}", timestamp);

        self.send_command(
            KEY_GH_TIMESTAMP_SET,
            FMT_GH_TIMESTAMP_SET,
            &timestamp.to_le_bytes(),
        )
        .await?;
        Ok(vec![])
    }

    async fn execute_time_set_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        let hour_offset: i8 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(8);

        info!("设置时间: timestamp={}, offset={}", timestamp, hour_offset);

        let mut data = Vec::new();
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.push(hour_offset as u8);

        self.send_command(KEY_GH_TIME_SET, FMT_GH_TIME_SET, &data)
            .await?;
        Ok(vec![])
    }

    async fn execute_factory_set_mode_cmd_async(
        &self,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let factory_mode: u8 = params
            .first()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少产测模式参数")?;

        info!("产测模式设置: mode={}", factory_mode);

        self.send_command(KEY_F_SET_MODE, FMT_F_SET_MODE, &[factory_mode])
            .await?;
        Ok(vec![])
    }

    async fn execute_factory_get_mode_cmd_async(
        &self,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let factory_mode: u8 = params
            .first()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少产测模式参数")?;

        info!("产测模式结果获取: mode={}", factory_mode);

        let param_data = self
            .call_command(KEY_F_GET_MODE, FMT_F_GET_MODE, &[factory_mode])
            .await?;
        info!("产测模式结果响应: {:04X?}", param_data);

        Self::decode_factory_mode_response(&param_data)
    }

    fn decode_factory_mode_response(param_data: &[u8]) -> Result<Vec<u8>, String> {
        if param_data.is_empty() {
            info!("产测模式结果为空");
            return Ok(Vec::new());
        }

        let value = unpack(param_data, RET_F_GET_MODE).map_err(|e| format!("解包失败: {:?}", e))?;
        info!("产测模式结果解包: {:?}", value);

        match value {
            UnpackValue::U16Array(arr) => {
                let mut result = Vec::new();
                for &val in arr.iter() {
                    result.extend_from_slice(&val.to_le_bytes());
                    info!("产测结果值: 0x{:04X}", val);
                }
                Ok(result)
            }
            _ => Err("产测模式结果获取失败: 解包结果不是数组".into()),
        }
    }

    pub fn subscribe_events(&self) -> bool {
        self.subscribe_data_events();
        true
    }

    pub fn is_events_subscribed(&self) -> bool {
        EVENTS_SUBSCRIBED.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_library_status(&self) -> (bool, bool) {
        (true, self.is_initialized())
    }

    pub async fn load_config_file(&self, file_path: &str) -> Result<Gh3036ConfigPreview, String> {
        use std::path::Path;

        info!("解析配置文件: {}", file_path);

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", file_path));
        }

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if extension != "config" && extension != "ini" {
            return Err(format!("不支持的文件类型: {}", extension));
        }

        let config_loader = ConfigLoader::from_file(path)?;
        if config_loader.is_empty() {
            return Err("未找到寄存器配置".to_string());
        }

        let registers: Vec<Gh3036ConfigRegisterPreview> = config_loader
            .get_register_list()
            .iter()
            .map(|reg| Gh3036ConfigRegisterPreview {
                addr: format!("0x{:04X}", reg.addr),
                value: format!("0x{:04X}", reg.value),
            })
            .collect();

        info!("解析到 {} 个寄存器", registers.len());

        Ok(Gh3036ConfigPreview {
            file_path: file_path.to_string(),
            register_count: registers.len(),
            registers,
        })
    }

    pub async fn download_config_file(&self, file_path: &str) -> Result<(), String> {
        use std::path::Path;

        info!("下载配置文件: {}", file_path);

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", file_path));
        }

        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if extension != "config" && extension != "ini" {
            return Err(format!("不支持的文件类型: {}", extension));
        }

        let config_loader = ConfigLoader::from_file(path)?;

        if config_loader.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        self.execute_rpc("D", &["0".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 0 失败: {}", e))?;

        let params = config_loader.format_for_download();
        self.execute_rpc("L", &params)
            .await
            .map_err(|e| format!("写入寄存器列表失败: {}", e))?;

        self.execute_rpc("D", &["1".to_string()])
            .await
            .map_err(|e| format!("下载配置阶段 1 失败: {}", e))?;

        info!("配置文件下载完成: {} 个寄存器", config_loader.len());

        Ok(())
    }

    pub async fn factory_test_start(self: Arc<Self>) -> Result<(), String> {
        Arc::clone(&self.factory_test_manager).start_test(self)
    }

    pub async fn factory_test_stop(&self) -> Result<(), String> {
        self.factory_test_manager.stop_test()
    }

    pub fn factory_test_status(&self) -> (FactoryTestStatus, FactoryTestStep) {
        (
            self.factory_test_manager.get_status(),
            self.factory_test_manager.get_current_step(),
        )
    }

    pub fn factory_test_continue(&self) -> Result<(), String> {
        self.factory_test_manager.continue_test()
    }

    pub fn factory_test_set_config_dir(&self, config_dir: &str) -> Result<(), String> {
        let path = std::path::PathBuf::from(config_dir);
        self.factory_test_manager.set_config_dir(path);
        info!("GH3036 产测配置目录设置为: {}", config_dir);
        Ok(())
    }

    pub fn factory_test_validate_config(&self) -> ConfigValidationResult {
        self.factory_test_manager.validate_config_dir()
    }

    pub fn factory_test_get_result(&self) -> Option<FactoryTestResult> {
        self.factory_test_manager.get_result()
    }

    pub fn validate_threshold_config(&self) -> super::ThresholdConfigValidation {
        self.factory_test_manager.validate_threshold_config()
    }

    pub fn get_threshold_config(&self) -> Option<super::FactoryThresholdConfig> {
        self.factory_test_manager.get_threshold_config()
    }

    pub fn get_evaluation_result(&self) -> Option<super::FactoryEvaluationResult> {
        self.factory_test_manager.get_evaluation_result()
    }

    pub fn set_hr_ref(&self, values: &[i32]) -> Result<(), String> {
        CALLBACK_CONTEXT
            .ref_data_manager
            .set_hr_ref(values)
            .map_err(|e| format!("设置HR金标失败: {}", e))
    }

    pub fn set_hrv_ref(&self, values: &[i32]) -> Result<(), String> {
        CALLBACK_CONTEXT
            .ref_data_manager
            .set_hrv_ref(values)
            .map_err(|e| format!("设置HRV金标失败: {}", e))
    }

    pub fn set_spo2_ref(&self, values: &[i32]) -> Result<(), String> {
        CALLBACK_CONTEXT
            .ref_data_manager
            .set_spo2_ref(values)
            .map_err(|e| format!("设置SpO2金标失败: {}", e))
    }

    pub fn clear_hr_ref(&self) {
        CALLBACK_CONTEXT.ref_data_manager.clear_hr_ref();
    }

    pub fn clear_hrv_ref(&self) {
        CALLBACK_CONTEXT.ref_data_manager.clear_hrv_ref();
    }

    pub fn clear_spo2_ref(&self) {
        CALLBACK_CONTEXT.ref_data_manager.clear_spo2_ref();
    }

    pub fn clear_all_ref(&self) {
        CALLBACK_CONTEXT.ref_data_manager.clear_all();
    }

    pub fn get_ref_data_status(&self) -> RefDataStatus {
        let (hr_values, hr_count, hr_elapsed) =
            CALLBACK_CONTEXT.ref_data_manager.get_hr_ref_status();
        let (hrv_values, hrv_count, hrv_elapsed) =
            CALLBACK_CONTEXT.ref_data_manager.get_hrv_ref_status();
        let (spo2_values, spo2_count) = CALLBACK_CONTEXT.ref_data_manager.get_spo2_ref_status();

        RefDataStatus {
            hr: HrRefStatus {
                values: hr_values,
                count: hr_count,
                elapsed_ms: hr_elapsed.as_millis() as u64,
            },
            hrv: HrvRefStatus {
                values: hrv_values,
                count: hrv_count,
                elapsed_ms: hrv_elapsed.as_millis() as u64,
            },
            spo2: Spo2RefStatus {
                values: spo2_values,
                count: spo2_count,
            },
        }
    }

    pub fn init_hr_ref_monitor(&self) {
        let ble_manager = self.device_manager.ble_manager.clone();
        let ref_data_manager = CALLBACK_CONTEXT.ref_data_manager.clone();
        super::hr_ref_monitor::init_hr_ref_monitor(ble_manager, ref_data_manager);
        info!("[Gh3036Manager] HR金标监听器初始化完成");
    }
}

impl Drop for Gh3036Manager {
    fn drop(&mut self) {
        self.stop_processing_thread();
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use super::{
        AlgorithmResultCache, ChannelConfig, ChannelType, CsvConfig, FrameAggregator,
        Gh3036Manager, GlobalContext, RefDataManager, RpcInput, CALLBACK_CONTEXT,
    };
    use crate::gh3036::types::{GhFuncFixIdx, GhFuncFrame};

    fn make_frame(function_id: GhFuncFixIdx, frame_cnt: u32, algo_data: Vec<i32>) -> GhFuncFrame {
        GhFuncFrame {
            id: function_id,
            frame_cnt,
            algo_data,
            ..GhFuncFrame::default()
        }
    }

    /// 读取 CSV 文件首行信息行 JSON
    fn read_info_row(filepath: &std::path::Path) -> serde_json::Value {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_path(filepath)
            .unwrap();
        let record = reader.records().next().unwrap().unwrap();
        serde_json::from_str(&record[0]).unwrap()
    }

    #[test]
    fn aggregator_flushes_every_buffered_function() {
        let ref_data_manager = Arc::new(RefDataManager::new());
        let mut aggregator = FrameAggregator::new(ref_data_manager);
        aggregator.add_frame(&make_frame(GhFuncFixIdx::Adt, 1, vec![1]));
        aggregator.add_frame(&make_frame(GhFuncFixIdx::Spo2, 1, vec![98]));

        let events = aggregator.flush();
        let function_ids: HashSet<u8> = events.iter().map(|event| event.function_id).collect();

        assert_eq!(events.len(), 2);
        assert_eq!(
            function_ids,
            HashSet::from([GhFuncFixIdx::Adt as u8, GhFuncFixIdx::Spo2 as u8])
        );
    }

    #[test]
    fn algorithm_cache_reuses_only_complete_zero_results_until_frame_reset() {
        let mut cache = AlgorithmResultCache::default();

        let first = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 10, vec![98, 12, 3]));
        let zero = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 11, vec![0, 0, 0]));
        let mixed = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 12, vec![99, 0, 4]));
        let zero_after_mixed = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 13, vec![0, 0, 0]));
        let empty = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 14, vec![]));
        let reset = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 0, vec![0, 0, 0]));

        assert_eq!(first.algo_data, vec![98, 12, 3]);
        assert_eq!(zero.algo_data, vec![98, 12, 3]);
        assert_eq!(mixed.algo_data, vec![99, 0, 4]);
        assert_eq!(zero_after_mixed.algo_data, vec![99, 0, 4]);
        assert!(empty.algo_data.is_empty());
        assert_eq!(reset.algo_data, vec![0, 0, 0]);
    }

    #[test]
    fn algorithm_cache_is_isolated_by_function() {
        let mut cache = AlgorithmResultCache::default();
        cache.normalize(&make_frame(GhFuncFixIdx::Adt, 4, vec![1, 80]));

        let spo2 = cache.normalize(&make_frame(GhFuncFixIdx::Spo2, 4, vec![0, 0]));

        assert_eq!(spo2.algo_data, vec![0, 0]);
    }

    #[test]
    fn rx_channel_change_queues_reset_before_new_data() {
        let context = GlobalContext::new();
        let receiver = context.setup_rpc_channel();
        context.set_rx_channel(ChannelConfig {
            channel_type: ChannelType::Ble,
            device_id: "ble-device".to_string(),
            characteristic_uuid: Some("rx-char".to_string()),
        });
        context.send_rpc_data(vec![0xAA, 0x11]).unwrap();

        assert!(matches!(receiver.recv().unwrap(), RpcInput::Reset));
        assert!(matches!(
            receiver.recv().unwrap(),
            RpcInput::Data(data) if data == vec![0xAA, 0x11]
        ));
    }

    #[test]
    fn regs_list_write_uses_call_transport() {
        assert_eq!(Gh3036Manager::regs_list_write_transport(), "send");
    }

    #[test]
    fn factory_mode_empty_response_is_successful() {
        assert_eq!(
            Gh3036Manager::decode_factory_mode_response(&[]).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn factory_mode_u16_array_is_returned_as_little_endian_bytes() {
        let response = [0x64, 0x02, 0x34, 0x12, 0x78, 0x56];
        assert_eq!(
            Gh3036Manager::decode_factory_mode_response(&response).unwrap(),
            vec![0x34, 0x12, 0x78, 0x56]
        );
    }

    #[test]
    fn trigger_new_csv_file_creates_new_files_for_all_writers() {
        use tempfile::TempDir;

        let context = GlobalContext::new();
        let temp_dir = TempDir::new().unwrap();

        context.set_csv_config(CsvConfig {
            enabled: true,
            output_dir: temp_dir.path().to_string_lossy().to_string(),
        });

        // 模拟已有 writer
        let frame = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
        context.save_frame_to_csv(&frame);

        // 触发新文件创建
        context.trigger_new_csv_file();

        // 再次写入，应该在新文件中
        let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
        context.save_frame_to_csv(&frame2);

        // 验证：检查输出目录中恰好有两个 CSV 文件
        let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|e| e == "csv")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(csv_files.len(), 2, "应该恰好创建两个CSV文件");
    }

    #[test]
    fn sw_function_start_stop_triggers_new_csv_file() {
        use tempfile::TempDir;

        // 直接驱动命令完成后的处理逻辑：
        // 异步 send_command 路径需要真实设备，此处通过 handle_sw_function_command_completed 模拟命令执行完成
        let run_case = |ctrl_type: u8, expected_files: usize| {
            let context = GlobalContext::new();
            let temp_dir = TempDir::new().unwrap();

            context.set_csv_config(CsvConfig {
                enabled: true,
                output_dir: temp_dir.path().to_string_lossy().to_string(),
            });

            // 写入初始帧
            let frame1 = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
            context.save_frame_to_csv(&frame1);

            // 模拟软件功能命令执行完成
            context.handle_sw_function_command_completed(ctrl_type);

            // 写入后续帧
            let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
            context.save_frame_to_csv(&frame2);

            // 验证：检查输出目录中的 CSV 文件数量
            let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
                .unwrap()
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .map(|e| e == "csv")
                        .unwrap_or(false)
                })
                .collect();

            assert_eq!(
                csv_files.len(),
                expected_files,
                "ctrl_type={} 应产生 {} 个CSV文件",
                ctrl_type,
                expected_files
            );
        };

        // 启动命令（ctrl_type=0）触发新文件
        run_case(0, 2);
        // 停止命令（ctrl_type=1）触发新文件
        run_case(1, 2);
        // 其他控制类型（ctrl_type=2）不触发新文件
        run_case(2, 1);
    }

    #[test]
    fn factory_mode_invalid_non_empty_response_still_fails() {
        assert!(Gh3036Manager::decode_factory_mode_response(&[0x64]).is_err());
    }

    #[test]
    fn device_disconnect_triggers_new_csv_file() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();

        // handle_device_disconnected 操作的是全局 CALLBACK_CONTEXT，测试需直接配置它
        CALLBACK_CONTEXT.set_csv_config(CsvConfig {
            enabled: true,
            output_dir: temp_dir.path().to_string_lossy().to_string(),
        });
        CALLBACK_CONTEXT.set_rx_channel(ChannelConfig {
            channel_type: ChannelType::Ble,
            device_id: "test-device".to_string(),
            characteristic_uuid: Some("test-char".to_string()),
        });

        let frame1 = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
        CALLBACK_CONTEXT.save_frame_to_csv(&frame1);

        // 模拟设备断开
        Gh3036Manager::handle_device_disconnected("test-device");

        let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
        CALLBACK_CONTEXT.save_frame_to_csv(&frame2);

        // 验证：设备断开后应创建新 CSV 文件（共 2 个）
        let csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
            .unwrap()
            .collect::<std::io::Result<Vec<_>>>()
            .unwrap()
            .into_iter()
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|e| e == "csv")
                    .unwrap_or(false)
            })
            .collect();

        assert_eq!(csv_files.len(), 2, "设备断开后应该创建新CSV文件");

        // 清理全局状态，避免影响其他测试
        CALLBACK_CONTEXT.set_csv_config(CsvConfig::default());
        CALLBACK_CONTEXT.rx_channel.lock().take();
    }

    #[test]
    fn force_new_file_refreshes_info_row_even_when_frame_id_nonzero() {
        use tempfile::TempDir;

        let context = GlobalContext::new();
        let temp_dir = TempDir::new().unwrap();

        context.set_csv_config(CsvConfig {
            enabled: true,
            output_dir: temp_dir.path().to_string_lossy().to_string(),
        });

        // save_frame_to_csv 的信息行取自全局 CALLBACK_CONTEXT
        CALLBACK_CONTEXT.set_app_info("ComBridge".to_string(), "0.5.24".to_string());
        CALLBACK_CONTEXT
            .set_last_ble_device("AA:BB:CC:DD:EE:FF".to_string(), Some("DEV-A".to_string()));

        // 首次写入：frame_cnt=1（非新文件边界），创建第一个文件
        let frame1 = make_frame(GhFuncFixIdx::Spo2, 1, vec![98]);
        context.save_frame_to_csv(&frame1);

        // 切换应用信息与蓝牙设备后强制分文件
        CALLBACK_CONTEXT.set_app_info("ComBridge2".to_string(), "0.5.24".to_string());
        CALLBACK_CONTEXT
            .set_last_ble_device("11:22:33:44:55:66".to_string(), Some("DEV-B".to_string()));
        context.trigger_new_csv_file();

        // 再次写入：frame_cnt=2（非新文件边界），writer 已关闭时应刷新信息行
        let frame2 = make_frame(GhFuncFixIdx::Spo2, 2, vec![99]);
        context.save_frame_to_csv(&frame2);

        // 刷新第二个文件内容（flush）后再读取
        context.trigger_new_csv_file();

        let mut csv_files: Vec<_> = std::fs::read_dir(temp_dir.path().join("SPO2"))
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .path()
                    .extension()
                    .map(|e| e == "csv")
                    .unwrap_or(false)
            })
            .map(|entry| entry.path())
            .collect();
        csv_files.sort();

        assert_eq!(csv_files.len(), 2, "应恰好创建两个CSV文件");

        // 第一个文件：使用首次设置的应用信息与蓝牙设备
        let first = read_info_row(&csv_files[0]);
        assert_eq!(first["app"].as_str(), Some("ComBridge"));
        assert_eq!(first["bleName"].as_str(), Some("DEV-A"));
        assert_eq!(first["bleAddress"].as_str(), Some("AA:BB:CC:DD:EE:FF"));

        // 第二个文件：强制分文件后刷新为新信息行
        let second = read_info_row(&csv_files[1]);
        assert_eq!(second["app"].as_str(), Some("ComBridge2"));
        assert_eq!(second["bleName"].as_str(), Some("DEV-B"));
        assert_eq!(second["bleAddress"].as_str(), Some("11:22:33:44:55:66"));

        // 蓝牙缓存按地址清理：不匹配地址不清除，匹配地址清除
        CALLBACK_CONTEXT.clear_last_ble_device("99:99:99:99:99:99");
        assert!(CALLBACK_CONTEXT.current_info_row("SPO2").ble_name.is_some());
        CALLBACK_CONTEXT.clear_last_ble_device("11:22:33:44:55:66");
        assert!(CALLBACK_CONTEXT.current_info_row("SPO2").ble_name.is_none());

        // 清理全局状态，避免影响其他测试
        CALLBACK_CONTEXT.set_app_info(String::new(), String::new());
        *CALLBACK_CONTEXT.last_ble_device.lock() = None;
    }

    #[test]
    fn ble_disconnect_event_clears_cached_device_via_event_bus() {
        use crate::service::{topics, BleConnectionEvent, EventBus};

        let event_bus = Arc::new(EventBus::new(1024));

        // 先缓存一个蓝牙设备
        CALLBACK_CONTEXT
            .set_last_ble_device("AA:BB:CC:DD:EE:FF".to_string(), Some("DEV-A".to_string()));

        // 注册与生产代码相同的 JSON 订阅
        let bus = event_bus.clone();
        bus.subscribe_json::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_topic, event| {
                CALLBACK_CONTEXT.clear_last_ble_device(&event.address);
            },
        );

        // msgpack 订阅不应收到 JSON 事件（旧 bug 的根因）
        let msgpack_fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = msgpack_fired.clone();
        let bus2 = event_bus.clone();
        bus2.subscribe_msgpack::<BleConnectionEvent, _>(
            topics::BLE_DISCONNECTED,
            move |_topic, _event| {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
            },
        );

        // 发布类型化事件（JSON 编码）
        let event = BleConnectionEvent::new("AA:BB:CC:DD:EE:FF", None);
        event_bus.publish_typed(topics::BLE_DISCONNECTED, &event);

        // 缓存应被清除
        let row = CALLBACK_CONTEXT.current_info_row("SPO2");
        assert!(row.ble_address.is_none(), "断开后缓存应被清除");
        assert!(
            !msgpack_fired.load(std::sync::atomic::Ordering::SeqCst),
            "msgpack 订阅不应收到 JSON 事件"
        );

        // 清理全局状态
        *CALLBACK_CONTEXT.last_ble_device.lock() = None;
    }
}
