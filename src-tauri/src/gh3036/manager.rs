//! GH3036 协议管理器
//!
//! 本模块实现 GH3036 协议的核心管理功能：
//! - 协议实例生命周期管理
//! - RPC 命令执行（基于 gh-rpc 库）
//! - RX 数据处理（通过 EventBus 订阅）
//! - CSV 数据保存

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crossbeam_channel::{Receiver, Sender, unbounded};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio::runtime::Handle;
use tracing::{debug, error, info, warn};

use crate::device::DeviceManager;
use crate::service::{EventBus, topics, SerialDataEvent, BleDataEvent, SerialDisconnectedEvent, BleConnectionEvent};
use super::csv_writer::CsvWriter;
use super::factory_test::FactoryTestManager;
use super::types::{Gh3036EventData, Gh3036FrameData, GhFuncFrame, FrameDecoder, Gh3036FramesEvent, GhFuncFixIdx,
    FactoryTestStep, FactoryTestStatus, FactoryTestResult, ConfigValidationResult,
    KEY_GH3X_GET_VERSION, KEY_GH3X_REGS_WRITE_CMD, KEY_GH3X_REGS_READ_CMD,
    KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, KEY_GH3X_CHIP_CTRL, KEY_GH3X_SW_FUNCTION_CMD,
    KEY_DOWNLOAD_CONFIG, KEY_GH3X_REGS_LIST_WRITE_CMD, KEY_GH_TIMESTAMP_SET,
    KEY_GH_TIME_SET, KEY_GH_SET_WORK_MODE_CMD, KEY_GH_LOW_POWER_CMD,
    KEY_F_SET_MODE, KEY_F_GET_MODE,
    FMT_GH3X_GET_VERSION, FMT_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_READ_CMD,
    FMT_GH3X_REG_BIT_FIELD_WRITE_CMD, FMT_GH3X_CHIP_CTRL, FMT_GH3X_SW_FUNCTION_CMD,
    FMT_DOWNLOAD_CONFIG, FMT_GH3X_REGS_LIST_WRITE_CMD, FMT_GH_TIMESTAMP_SET,
    FMT_GH_TIME_SET, FMT_GH_SET_WORK_MODE_CMD, FMT_GH_LOW_POWER_CMD,
    FMT_F_SET_MODE, FMT_F_GET_MODE,
    RET_GH3X_GET_VERSION, RET_GH3X_REGS_READ_CMD, RET_F_GET_MODE,
    GhFuncFixIdxExt};

use gh_rpc::CommandExecutor;
use rpc::{RpcConfig, SendFunction, LogCallback, LogLevel, unpack, UnpackValue};

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

struct SendRequest {
    data: Vec<u8>,
}

struct RpcDataRequest {
    data: Vec<u8>,
}

struct FrameAggregator {
    buffer: HashMap<u8, Gh3036FramesEvent>,
    last_publish_time: std::time::Instant,
    min_interval: std::time::Duration,
}

impl FrameAggregator {
    fn new() -> Self {
        Self {
            buffer: HashMap::new(),
            last_publish_time: std::time::Instant::now(),
            min_interval: std::time::Duration::from_millis(30),
        }
    }

    fn add_frame(&mut self, frame: &GhFuncFrame) -> Option<Gh3036FramesEvent> {
        let func_id = frame.id as u8;
        let func_name = GhFuncFixIdx::from(func_id).name().to_string();
        
        let event = self.buffer.entry(func_id).or_insert_with(|| {
            Gh3036FramesEvent::new(func_id, func_name)
        });
        event.add_frame(frame);

        let now = std::time::Instant::now();
        if now.duration_since(self.last_publish_time) >= self.min_interval {
            self.flush()
        } else {
            None
        }
    }

    fn flush(&mut self) -> Option<Gh3036FramesEvent> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut result: Option<Gh3036FramesEvent> = None;
        
        for (_, event) in self.buffer.drain() {
            if !event.is_empty() {
                result = Some(event);
                break;
            }
        }

        if result.is_some() {
            self.last_publish_time = std::time::Instant::now();
        }
        result
    }
}

struct GlobalContext {
    rx_channel: Mutex<Option<ChannelConfig>>,
    tx_channel: Mutex<Option<ChannelConfig>>,
    device_manager: Mutex<Option<Arc<DeviceManager>>>,
    app_handle: Mutex<Option<AppHandle>>,
    csv_config: Mutex<CsvConfig>,
    csv_writers: Mutex<HashMap<i32, CsvWriter>>,
    send_sender: Mutex<Option<Sender<SendRequest>>>,
    event_sender: Mutex<Option<Sender<Gh3036EventData>>>,
    frame_sender: Mutex<Option<Sender<Gh3036FrameData>>>,
    rpc_data_sender: Mutex<Option<Sender<RpcDataRequest>>>,
    frame_raw_sender: Mutex<Option<Sender<Vec<u8>>>>,
    frame_aggregator: Mutex<FrameAggregator>,
    runtime_handle: Mutex<Option<Handle>>,
}

impl GlobalContext {
    fn new() -> Self {
        Self {
            rx_channel: Mutex::new(None),
            tx_channel: Mutex::new(None),
            device_manager: Mutex::new(None),
            app_handle: Mutex::new(None),
            csv_config: Mutex::new(CsvConfig::default()),
            csv_writers: Mutex::new(HashMap::new()),
            send_sender: Mutex::new(None),
            event_sender: Mutex::new(None),
            frame_sender: Mutex::new(None),
            rpc_data_sender: Mutex::new(None),
            frame_raw_sender: Mutex::new(None),
            frame_aggregator: Mutex::new(FrameAggregator::new()),
            runtime_handle: Mutex::new(None),
        }
    }

    fn setup_channels(&self) -> (
        Receiver<SendRequest>, 
        Receiver<Gh3036EventData>, 
        Receiver<Gh3036FrameData>,
        Receiver<RpcDataRequest>,
        Receiver<Vec<u8>>,
    ) {
        let (send_sender, send_receiver) = unbounded();
        let (event_sender, event_receiver) = unbounded();
        let (frame_sender, frame_receiver) = unbounded();
        let (rpc_data_sender, rpc_data_receiver) = unbounded();
        let (frame_raw_sender, frame_raw_receiver) = unbounded();
        
        *self.send_sender.lock() = Some(send_sender);
        *self.event_sender.lock() = Some(event_sender);
        *self.frame_sender.lock() = Some(frame_sender);
        *self.rpc_data_sender.lock() = Some(rpc_data_sender);
        *self.frame_raw_sender.lock() = Some(frame_raw_sender);
        
        (send_receiver, event_receiver, frame_receiver, rpc_data_receiver, frame_raw_receiver)
    }

    fn set_rx_channel(&self, config: ChannelConfig) {
        let mut rx_channel = self.rx_channel.lock();
        *rx_channel = Some(config);
    }

    fn get_rx_channel(&self) -> Option<ChannelConfig> {
        self.rx_channel.lock().clone()
    }

    fn is_channel_match(&self, device_id: &str, channel_type: ChannelType) -> bool {
        let rx_channel = self.rx_channel.lock();
        match rx_channel.as_ref() {
            Some(config) => {
                config.device_id == device_id && config.channel_type == channel_type
            }
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

    fn set_app_handle(&self, handle: AppHandle) {
        let mut app_handle = self.app_handle.lock();
        *app_handle = Some(handle);
    }

    fn set_csv_config(&self, config: CsvConfig) {
        let mut csv_config = self.csv_config.lock();
        *csv_config = config;
    }

    fn set_runtime_handle(&self, handle: Handle) {
        let mut runtime_handle = self.runtime_handle.lock();
        *runtime_handle = Some(handle);
    }

    fn send_rpc_data(&self, data: Vec<u8>) -> Result<(), crossbeam_channel::SendError<RpcDataRequest>> {
        if let Some(ref sender) = *self.rpc_data_sender.lock() {
            sender.send(RpcDataRequest { data })
        } else {
            Err(crossbeam_channel::SendError(RpcDataRequest { data: vec![] }))
        }
    }

    fn send_frame_data(&self, frame_data: Gh3036FrameData) -> Result<(), crossbeam_channel::SendError<Gh3036FrameData>> {
        if let Some(ref sender) = *self.frame_sender.lock() {
            sender.send(frame_data)
        } else {
            Err(crossbeam_channel::SendError(frame_data))
        }
    }


    fn add_frame_to_aggregator(&self, frame: &GhFuncFrame) -> Option<Gh3036FramesEvent> {
        let mut aggregator = self.frame_aggregator.lock();
        aggregator.add_frame(frame)
    }

}

static CALLBACK_CONTEXT: once_cell::sync::Lazy<GlobalContext> = once_cell::sync::Lazy::new(GlobalContext::new);

static EVENTS_SUBSCRIBED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);


pub struct Gh3036Manager {
    device_manager: Arc<DeviceManager>,
    event_bus: Arc<EventBus>,
    initialized: Mutex<bool>,
    running: Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    rpc_thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    executor: Mutex<Option<Arc<tokio::sync::RwLock<CommandExecutor>>>>,
    factory_test_manager: Arc<FactoryTestManager>,
}

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
            rpc_thread_handle: Mutex::new(None),
            executor: Mutex::new(None),
            factory_test_manager,
        }
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        CALLBACK_CONTEXT.set_app_handle(handle);
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
        if EVENTS_SUBSCRIBED.load(std::sync::atomic::Ordering::SeqCst) {
            info!("[GH3036] 事件已订阅，跳过重复订阅");
            return;
        }
        
        info!("[GH3036] 订阅 EventBus 数据事件");
        
        self.event_bus.subscribe_msgpack::<SerialDataEvent, _>(topics::SERIAL_DATA, move |_topic, event| {
            if !CALLBACK_CONTEXT.is_channel_match(&event.device_id, ChannelType::Serial) {
                debug!("[GH3036] 过滤非配置通道数据: device_id={}", event.device_id);
                return;
            }
            info!("[GH3036] 接收到串口数据: device_id={}, len={}", event.device_id, event.data.len());
            if let Err(e) = CALLBACK_CONTEXT.send_rpc_data(event.data.clone()) {
                error!("[GH3036] 发送数据到 RPC 通道失败: {}", e);
            }
        });
        
        self.event_bus.subscribe_msgpack::<BleDataEvent, _>(topics::BLE_DATA, move |_topic, event| {
            if !CALLBACK_CONTEXT.is_channel_match(&event.device_id, ChannelType::Ble) {
                debug!("[GH3036] 过滤非配置通道数据: device_id={}", event.device_id);
                return;
            }
            info!("[GH3036] 接收到 BLE 数据: device_id={}, len={}", event.device_id, event.data.len());
            if let Err(e) = CALLBACK_CONTEXT.send_rpc_data(event.data.clone()) {
                error!("[GH3036] 发送数据到 RPC 通道失败: {}", e);
            }
        });
        
        self.event_bus.subscribe_msgpack::<SerialDisconnectedEvent, _>(topics::SERIAL_DISCONNECTED, move |_topic, event| {
            info!("[GH3036] 收到串口断开事件: {}", event.port_name);
            Self::handle_device_disconnected(&event.port_name);
        });
        
        self.event_bus.subscribe_msgpack::<BleConnectionEvent, _>(topics::BLE_DISCONNECTED, move |_topic, event| {
            info!("[GH3036] 收到 BLE 断开事件: {}", event.address);
            Self::handle_device_disconnected(&event.address);
        });
        
        EVENTS_SUBSCRIBED.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("[GH3036] 已订阅 serial:data、ble:data、serial:disconnected 和 ble:disconnected 事件");
    }
    
    fn handle_device_disconnected(device_id: &str) {
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        if let Some(channel) = tx_channel.as_ref() {
            if channel.device_id == device_id {
                drop(tx_channel);
                let mut tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
                *tx_channel = None;
                info!("GH3036 TX 通道已清理: 设备 {} 已断开", device_id);
            }
        }
    }
    
    fn process_frame_data(event_bus: &Arc<EventBus>, decoder: &mut FrameDecoder, data: &[u8]) {
        match decoder.decode_frames(data) {
            Ok(frames) => {
                for frame in frames.iter() {
                    if let Some(aggregated) = CALLBACK_CONTEXT.add_frame_to_aggregator(frame) {
                        info!(
                            "[GH3036] 发布聚合帧事件: function_id={}, frame_count={}, channel_count={}",
                            aggregated.function_id,
                            aggregated.frame_count,
                            aggregated.channel_count
                        );
                        event_bus.publish_msgpack("gh3036:frames", &aggregated);
                    }
                    
                    let frame_data = Gh3036FrameData::from_func_frame(frame);
                    if let Err(e) = CALLBACK_CONTEXT.send_frame_data(frame_data) {
                        error!("[GH3036] 帧数据入队失败: {}", e);
                    }
                }
            }
            Err(e) => {
                error!("[GH3036] 帧解码失败: {:?}", e);
            }
        }
    }

    fn initialize_rpc(&self) -> Result<(), String> {
        info!("GH3036 初始化 RPC 核心");
        
        let device_manager = Arc::clone(&self.device_manager);
        let handle = Handle::try_current().map_err(|e| format!("获取 Tokio 运行时失败: {}", e))?;
        let handle_for_send = handle.clone();
        
        let send_fn: SendFunction = Arc::new(move |data: &[u8]| -> Result<(), rpc::RpcError> {
            debug!("[RPC发送] 发送数据: {:02X?}", data);
            
            let (channel_type, device_id, char_uuid) = {
                let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
                match tx_channel.as_ref() {
                    Some(channel) => (channel.channel_type, channel.device_id.clone(), channel.characteristic_uuid.clone()),
                    None => {
                        warn!("[RPC发送] TX 通道未配置");
                        return Err(rpc::RpcError::SendFail);
                    }
                }
            };
            
            let dm = Arc::clone(&device_manager);
            let data_vec = data.to_vec();
            handle_for_send.spawn(async move {
                let result = dm
                    .send_direct(
                        channel_type.into(),
                        &device_id,
                        char_uuid.as_deref(),
                        &data_vec,
                    )
                    .await;
                
                match result {
                    Ok(_) => debug!("[RPC发送] 发送成功: {} bytes", data_vec.len()),
                    Err(e) => {
                        let error_str = format!("{}", e);
                        if error_str.contains("已关闭") || error_str.contains("closed") || error_str.contains("disconnected") {
                            warn!("[RPC发送] 设备已断开，发送失败");
                        } else {
                            error!("[RPC发送] 发送失败: {}", e);
                        }
                    }
                }
            });
            
            Ok(())
        });

        struct TauriLogger;
        impl LogCallback for TauriLogger {
            fn log(&self, level: LogLevel, context: &str, message: &str) {
                match level {
                    LogLevel::Trace => tracing::trace!("[{}] {}", context, message),
                    LogLevel::Debug => tracing::debug!("[{}] {}", context, message),
                    LogLevel::Info => tracing::info!("[{}] {}", context, message),
                    LogLevel::Warn => tracing::warn!("[{}] {}", context, message),
                    LogLevel::Error => tracing::error!("[{}] {}", context, message),
                }
            }
        }

        let executor = CommandExecutor::new(RpcConfig {
            timeout_ms: 1000,
            ..RpcConfig::default()
        }).with_logger(Arc::new(TauriLogger));
        
        tokio::task::block_in_place(|| {
            handle.block_on(executor.set_send_function(Arc::clone(&send_fn)));
        });
        
        info!("GH3036 RPC 核心初始化完成");
        
        let executor = Arc::new(tokio::sync::RwLock::new(executor));
        
        *self.executor.lock() = Some(Arc::clone(&executor));
        
        Ok(())
    }

    fn start_processing_thread(&self) -> Result<(), String> {
        let running = self.running.clone();
        running.store(true, std::sync::atomic::Ordering::SeqCst);

        let (send_receiver, event_receiver, frame_receiver, rpc_data_receiver, frame_raw_receiver) = 
            CALLBACK_CONTEXT.setup_channels();
        let device_manager = Arc::clone(&self.device_manager);
        let running_clone = running.clone();
        let executor = self.executor.lock().as_ref().map(Arc::clone);
        let event_bus = self.event_bus.clone();
        let tokio_handle = Handle::try_current().map_err(|e| format!("获取 Tokio 运行时失败: {}", e))?;
        
        let thread_handle = std::thread::spawn(move || {
            info!("[GH3036] 处理线程启动");
            let mut frame_decoder = FrameDecoder::new();

            while running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                crossbeam_channel::select! {
                    recv(send_receiver) -> result => {
                        if let Ok(request) = result {
                            Self::handle_send_request(&device_manager, request);
                        }
                    }
                    recv(event_receiver) -> result => {
                        if let Ok(event_data) = result {
                            Self::handle_event_data(event_data);
                        }
                    }
                    recv(frame_receiver) -> result => {
                        if let Ok(frame_data) = result {
                            Self::handle_frame_data(frame_data);
                        }
                    }
                    recv(rpc_data_receiver) -> result => {
                        if let Ok(rpc_data) = result {
                            Self::handle_rpc_data(&executor, rpc_data, &tokio_handle);
                        }
                    }
                    recv(frame_raw_receiver) -> result => {
                        if let Ok(raw_data) = result {
                            Self::process_frame_data(&event_bus, &mut frame_decoder, &raw_data);
                        }
                    }
                    default(std::time::Duration::from_millis(10)) => {
                    }
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

    fn handle_rpc_data(
        executor: &Option<Arc<tokio::sync::RwLock<CommandExecutor>>>,
        request: RpcDataRequest,
        tokio_handle: &Handle,
    ) {
        debug!("GH3036 handle_rpc_data 处理 RPC 数据: {} bytes", request.data.len());
        
        if let Some(exec) = executor {
            let exec_clone = Arc::clone(exec);
            let data = request.data;
            tokio_handle.spawn(async move {
                let executor = exec_clone.read().await;
                let results = executor.process(&data).await;
                for result in results {
                    match result {
                        Ok(parse_result) => {
                            debug!("GH3036 handle_rpc_data 解析成功: key={}, len={}", 
                                parse_result.key, parse_result.param.len());
                        }
                        Err(e) => {
                            debug!("GH3036 handle_rpc_data 解析失败: {:?}", e);
                        }
                    }
                }
            });
        } else {
            warn!("GH3036 handle_rpc_data RPC 核心未初始化");
        }
    }

    fn handle_send_request(device_manager: &Arc<DeviceManager>, request: SendRequest) {
        info!("GH3036 handle_send_request 开始处理: {} bytes", request.data.len());
        
        let (channel_type, device_id, char_uuid) = {
            let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
            let Some(channel) = tx_channel.as_ref() else {
                warn!("GH3036 TX 通道未配置，无法发送数据");
                return;
            };
            (channel.channel_type, channel.device_id.clone(), channel.characteristic_uuid.clone())
        };

        info!("GH3036 handle_send_request 设备: type={:?}, id={}, 数据: {:02X?}", channel_type, device_id, request.data);

        let device_manager_clone = Arc::clone(device_manager);
        let data = request.data;
        
        if let Some(handle) = CALLBACK_CONTEXT.runtime_handle.lock().as_ref() {
            info!("GH3036 handle_send_request 在异步运行时中发送");
            handle.spawn(async move {
                info!("GH3036 handle_send_request 异步任务开始");
                let result = device_manager_clone
                    .send_direct(
                        channel_type.into(),
                        &device_id,
                        char_uuid.as_deref(),
                        &data,
                    )
                    .await;
                
                match result {
                    Ok(_) => info!("GH3036 handle_send_request 发送成功: {} bytes", data.len()),
                    Err(e) => error!("GH3036 handle_send_request 发送失败: {}", e),
                }
            });
        } else {
            warn!("GH3036 异步运行时不可用，跳过发送");
        }
    }

    fn handle_event_data(event_data: Gh3036EventData) {
        let app_handle = CALLBACK_CONTEXT.app_handle.lock();
        if let Some(ref handle) = *app_handle {
            if let Err(e) = handle.emit("gh3036-event", &event_data) {
                error!("GH3036 发送事件到前端失败: {}", e);
            } else {
                debug!("GH3036 事件已发送到前端: type={}", event_data.event_type);
            }
        }
    }

    fn handle_frame_data(frame_data: Gh3036FrameData) {
        let app_handle = CALLBACK_CONTEXT.app_handle.lock();
        if let Some(ref handle) = *app_handle {
            if let Err(e) = handle.emit("gh3036-frame", &frame_data) {
                error!("GH3036 发送帧数据到前端失败: {}", e);
            } else {
                debug!(
                    "GH3036 帧数据已发送到前端: func_id={}, frame_id={}",
                    frame_data.function_id, frame_data.frame_id
                );
            }
        }

        Self::save_frame_to_csv(&frame_data);
    }

    fn save_frame_to_csv(frame_data: &Gh3036FrameData) {
        let csv_config = CALLBACK_CONTEXT.csv_config.lock();
        if !csv_config.enabled {
            return;
        }

        let mut writers = CALLBACK_CONTEXT.csv_writers.lock();
        let function_id = frame_data.function_id;

        let writer = writers.entry(function_id).or_insert_with(|| {
            CsvWriter::new(
                PathBuf::from(&csv_config.output_dir),
                function_id,
                frame_data.function_name.clone(),
            )
        });

        if let Err(e) = writer.write_frame(frame_data) {
            error!("CSV 写入失败: {}", e);
        }
    }

    fn stop_processing_thread(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        let mut thread_guard = self.thread_handle.lock();
        if let Some(thread) = thread_guard.take() {
            let _ = thread.join();
        }
        
        let mut rpc_thread_guard = self.rpc_thread_handle.lock();
        if let Some(thread) = rpc_thread_guard.take() {
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

    pub fn get_csv_config(&self) -> CsvConfig {
        CALLBACK_CONTEXT.csv_config.lock().clone()
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
            executor_guard.as_ref()
                .ok_or("RPC 核心未初始化")?
                .clone()
        };

        let exec = executor.read().await;
        exec.call(key, format, data).await.map_err(|e| format!("RPC 调用失败: {:?}", e))
    }

    async fn send_command(&self, key: &str, format: &str, data: &[u8]) -> Result<(), String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref()
                .ok_or("RPC 核心未初始化")?
                .clone()
        };

        let exec = executor.read().await;
        exec.send(key, format, data).await.map_err(|e| format!("RPC 发送失败: {:?}", e))
    }

    async fn send_command_multi(&self, key: &str, format: &str, data: &[u8]) -> Result<(), String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref()
                .ok_or("RPC 核心未初始化")?
                .clone()
        };

        let exec = executor.read().await;
        exec.sall(key, format, data).await.map_err(|e| format!("RPC 多帧发送失败: {:?}", e))?;
        Ok(())
    }

    async fn publish_command(&self, key: &str, format: &str, data: &[u8]) -> Result<(), String> {
        let executor = {
            let executor_guard = self.executor.lock();
            executor_guard.as_ref()
                .ok_or("RPC 核心未初始化")?
                .clone()
        };

        let exec = executor.read().await;
        exec.publish(key, format, data).await.map_err(|e| format!("RPC 发布失败: {:?}", e))
    }

    pub async fn execute_rpc(&self, command_key: &str, params: &[String]) -> Result<Vec<u8>, String> {
        info!("GH3036 execute_rpc 开始: key={}, params={:?}", command_key, params);
        self.execute_rpc_async(command_key, params).await
    }

    async fn execute_rpc_async(&self, command_key: &str, params: &[String]) -> Result<Vec<u8>, String> {
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
        let ver_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("GH3036 execute_version_cmd: ver_type={}", ver_type);
        
        let param_data = self.call_command(KEY_GH3X_GET_VERSION, FMT_GH3X_GET_VERSION, &[ver_type]).await?;
        
        let value = unpack(&param_data, RET_GH3X_GET_VERSION).map_err(|e| format!("解包失败: {:?}", e))?;
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

        if regs.is_empty() || regs.len() % 2 != 0 {
            return Err("寄存器数据格式错误，需要成对的地址和值".to_string());
        }

        info!("寄存器写入: {} 个寄存器", regs.len() / 2);

        let mut data = Vec::new();
        data.extend_from_slice(&(regs.len() as u16).to_le_bytes());
        for reg in &regs {
            data.extend_from_slice(&reg.to_le_bytes());
        }
        
        self.send_command(KEY_GH3X_REGS_WRITE_CMD, FMT_GH3X_REGS_WRITE_CMD, &data).await?;
        Ok(vec![])
    }

    async fn execute_regs_read_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let read_len: i32 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        info!("寄存器读取: addr=0x{:04X}, len={}", reg_addr, read_len);

        let mut data = Vec::new();
        data.extend_from_slice(&reg_addr.to_le_bytes());
        data.extend_from_slice(&read_len.to_le_bytes());
        
        let param_data = self.call_command(KEY_GH3X_REGS_READ_CMD, FMT_GH3X_REGS_READ_CMD, &data).await?;
        info!("寄存器读取响应: {:04X?}", param_data);
        let value = unpack(&param_data, RET_GH3X_REGS_READ_CMD).map_err(|e| format!("解包失败: {:?}", e))?;
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

    async fn execute_reg_bitfield_write_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let lsb: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let msb: u8 = params
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(15);

        let reg_val: u16 = params
            .get(3)
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器值参数")?;

        info!("位域写入: addr=0x{:04X}, lsb={}, msb={}, val=0x{:04X}", reg_addr, lsb, msb, reg_val);

        let mut data = Vec::new();
        data.extend_from_slice(&reg_addr.to_le_bytes());
        data.push(lsb);
        data.push(msb);
        data.extend_from_slice(&reg_val.to_le_bytes());
        
        self.send_command(KEY_GH3X_REG_BIT_FIELD_WRITE_CMD, FMT_GH3X_REG_BIT_FIELD_WRITE_CMD, &data).await?;
        Ok(vec![])
    }

    async fn execute_chip_ctrl_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let ctrl_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少控制类型参数")?;

        info!("芯片控制: type={}", ctrl_type);

        self.send_command(KEY_GH3X_CHIP_CTRL, FMT_GH3X_CHIP_CTRL, &[ctrl_type]).await?;
        Ok(vec![])
    }

    async fn execute_download_config_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let stage: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("下载配置: stage={}", stage);

        self.send_command(KEY_DOWNLOAD_CONFIG, FMT_DOWNLOAD_CONFIG, &[stage]).await?;
        Ok(vec![])
    }

    async fn execute_regs_list_write_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        info!("寄存器列表写入: {} 个值", regs.len());

        let mut data = Vec::new();
        data.extend_from_slice(&(regs.len() as u16).to_le_bytes());
        for &val in regs.iter() {
            data.extend_from_slice(&val.to_le_bytes());
        }
        
        self.send_command_multi(KEY_GH3X_REGS_LIST_WRITE_CMD, FMT_GH3X_REGS_LIST_WRITE_CMD, &data).await?;
        Ok(vec![])
    }

    async fn execute_sw_function_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("软件功能命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);

        let mut data = Vec::new();
        data.extend_from_slice(&target_func_mode.to_le_bytes());
        data.push(ctrl_type);
        
        self.send_command(KEY_GH3X_SW_FUNCTION_CMD, FMT_GH3X_SW_FUNCTION_CMD, &data).await?;
        Ok(vec![])
    }

    async fn execute_low_power_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("低功耗命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);

        let mut data = Vec::new();
        data.extend_from_slice(&target_func_mode.to_le_bytes());
        data.push(ctrl_type);
        
        self.publish_command(KEY_GH_LOW_POWER_CMD, FMT_GH_LOW_POWER_CMD, &data).await?;
        Ok(vec![])
    }

    async fn execute_set_work_mode_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let work_mode: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少工作模式参数")?;

        info!("设置工作模式: mode={}", work_mode);

        self.send_command(KEY_GH_SET_WORK_MODE_CMD, FMT_GH_SET_WORK_MODE_CMD, &[work_mode]).await?;
        Ok(vec![])
    }

    async fn execute_timestamp_set_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        info!("设置时间戳: {}", timestamp);

        self.send_command(KEY_GH_TIMESTAMP_SET, FMT_GH_TIMESTAMP_SET, &timestamp.to_le_bytes()).await?;
        Ok(vec![])
    }

    async fn execute_time_set_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        let hour_offset: i8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);

        info!("设置时间: timestamp={}, offset={}", timestamp, hour_offset);

        let mut data = Vec::new();
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.push(hour_offset as u8);
        
        self.send_command(KEY_GH_TIME_SET, FMT_GH_TIME_SET, &data).await?;
        Ok(vec![])
    }

    async fn execute_factory_set_mode_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let factory_mode: u8 = params
            .first()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少产测模式参数")?;

        info!("产测模式设置: mode={}", factory_mode);

        self.send_command(KEY_F_SET_MODE, FMT_F_SET_MODE, &[factory_mode]).await?;
        Ok(vec![])
    }

    async fn execute_factory_get_mode_cmd_async(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let factory_mode: u8 = params
            .first()
            .and_then(|s| u8::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少产测模式参数")?;

        info!("产测模式结果获取: mode={}", factory_mode);

        let param_data = self.call_command(KEY_F_GET_MODE, FMT_F_GET_MODE, &[factory_mode]).await?;
        info!("产测模式结果响应: {:04X?}", param_data);
        
        let value = unpack(&param_data, RET_F_GET_MODE).map_err(|e| format!("解包失败: {:?}", e))?;
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

    pub async fn load_config_file(&self, file_path: &str) -> Result<Vec<String>, String> {
        use std::fs;
        use std::path::Path;

        info!("加载配置文件: {}", file_path);

        let path = Path::new(file_path);
        if !path.exists() {
            return Err(format!("文件不存在: {}", file_path));
        }

        let extension = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if extension != "config" && extension != "ini" {
            return Err(format!("不支持的文件类型: {}", extension));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| format!("读取文件失败: {}", e))?;

        let regs = Self::parse_config_registers(&content)?;
        
        info!("解析到 {} 个寄存器", regs.len());
        
        let reg_strings: Vec<String> = regs.iter()
            .map(|(addr, value)| format!("0x{:04X},0x{:04X}", addr, value))
            .collect();

        Ok(reg_strings)
    }

    fn parse_config_registers(content: &str) -> Result<Vec<(u16, u16)>, String> {
        let mut regs = Vec::new();
        let mut in_register_list = false;

        for line in content.lines() {
            let trimmed = line.trim();
            
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let section = trimmed[1..trimmed.len()-1].to_lowercase();
                in_register_list = section == "register_list";
                continue;
            }

            if !in_register_list {
                continue;
            }

            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("#") {
                continue;
            }

            if trimmed.starts_with('{') && trimmed.contains('}') {
                let inner = trimmed.trim_start_matches('{').trim_end_matches('}');
                let parts: Vec<&str> = inner.split(',')
                    .map(|s| s.trim())
                    .collect();

                if parts.len() >= 2 {
                    let addr_str = parts[0].trim_start_matches("0x").trim_start_matches("0X");
                    let value_str = parts[1].split("//")
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_start_matches("0x")
                        .trim_start_matches("0X");

                    if let (Ok(addr), Ok(value)) = (
                        u16::from_str_radix(addr_str, 16),
                        u16::from_str_radix(value_str, 16)
                    ) {
                        regs.push((addr, value));
                    }
                }
            }
        }

        if regs.is_empty() {
            return Err("未找到寄存器配置".to_string());
        }

        Ok(regs)
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
}

impl Drop for Gh3036Manager {
    fn drop(&mut self) {
        self.stop_processing_thread();
    }
}
