//! GH3036 协议管理器
//!
//! 本模块实现 GH3036 协议的核心管理功能：
//! - 协议实例生命周期管理
//! - 回调函数实现（send、frame、event）
//! - RX 数据处理线程
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
use super::csv_writer::CsvWriter;
use super::ffi;
use super::sync;
use super::types::{Gh3036EventData, Gh3036FrameData};

/// 通道类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    Serial,
    Ble,
}

/// 通道配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// 通道类型
    pub channel_type: ChannelType,
    /// 设备 ID（串口名或蓝牙地址）
    pub device_id: String,
    /// 蓝牙特征值 UUID（仅蓝牙通道需要）
    pub characteristic_uuid: Option<String>,
}

/// CSV 配置结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvConfig {
    /// 是否启用 CSV 保存
    pub enabled: bool,
    /// 输出目录
    pub output_dir: String,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            output_dir: String::from("."),
        }
    }
}

/// 发送数据请求
struct SendRequest {
    data: Vec<u8>,
}

/// 全局回调上下文
///
/// 用于在 C 回调函数中访问 Rust 状态
struct GlobalContext {
    /// TX 通道配置
    tx_channel: Mutex<Option<ChannelConfig>>,
    /// 设备管理器
    device_manager: Mutex<Option<Arc<DeviceManager>>>,
    /// AppHandle 用于发送事件
    app_handle: Mutex<Option<AppHandle>>,
    /// CSV 配置
    csv_config: Mutex<CsvConfig>,
    /// CSV 写入器集合
    csv_writers: Mutex<HashMap<i32, CsvWriter>>,
    /// 发送数据请求通道
    send_sender: Mutex<Option<Sender<SendRequest>>>,
    /// 事件数据发送器
    event_sender: Mutex<Option<Sender<Gh3036EventData>>>,
    /// 帧数据发送器
    frame_sender: Mutex<Option<Sender<Gh3036FrameData>>>,
    /// 运行时句柄
    runtime_handle: Mutex<Option<Handle>>,
}

impl GlobalContext {
    fn new() -> Self {
        Self {
            tx_channel: Mutex::new(None),
            device_manager: Mutex::new(None),
            app_handle: Mutex::new(None),
            csv_config: Mutex::new(CsvConfig::default()),
            csv_writers: Mutex::new(HashMap::new()),
            send_sender: Mutex::new(None),
            event_sender: Mutex::new(None),
            frame_sender: Mutex::new(None),
            runtime_handle: Mutex::new(None),
        }
    }

    fn setup_channels(&self) -> (Receiver<SendRequest>, Receiver<Gh3036EventData>, Receiver<Gh3036FrameData>) {
        let (send_sender, send_receiver) = unbounded();
        let (event_sender, event_receiver) = unbounded();
        let (frame_sender, frame_receiver) = unbounded();
        
        *self.send_sender.lock() = Some(send_sender);
        *self.event_sender.lock() = Some(event_sender);
        *self.frame_sender.lock() = Some(frame_sender);
        
        (send_receiver, event_receiver, frame_receiver)
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

    fn send_data_request(&self, request: SendRequest) -> Result<(), crossbeam_channel::SendError<SendRequest>> {
        if let Some(ref sender) = *self.send_sender.lock() {
            sender.send(request)
        } else {
            Err(crossbeam_channel::SendError(request))
        }
    }

    fn send_event_data(&self, event_data: Gh3036EventData) -> Result<(), crossbeam_channel::SendError<Gh3036EventData>> {
        if let Some(ref sender) = *self.event_sender.lock() {
            sender.send(event_data)
        } else {
            Err(crossbeam_channel::SendError(event_data))
        }
    }

    fn send_frame_data(&self, frame_data: Gh3036FrameData) -> Result<(), crossbeam_channel::SendError<Gh3036FrameData>> {
        if let Some(ref sender) = *self.frame_sender.lock() {
            sender.send(frame_data)
        } else {
            Err(crossbeam_channel::SendError(frame_data))
        }
    }
}

static CALLBACK_CONTEXT: once_cell::sync::Lazy<GlobalContext> = once_cell::sync::Lazy::new(GlobalContext::new);

/// GH3036 协议管理器
pub struct Gh3036Manager {
    /// C 库协议句柄
    handle: Mutex<Option<*mut ffi::GhProtocolHandle>>,
    /// 设备管理器
    device_manager: Arc<DeviceManager>,
    /// 初始化状态
    initialized: Mutex<bool>,
    /// 处理线程运行标志
    running: Arc<std::sync::atomic::AtomicBool>,
    /// 处理线程句柄
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
}

unsafe impl Send for Gh3036Manager {}
unsafe impl Sync for Gh3036Manager {}

impl Gh3036Manager {
    /// 创建新的 GH3036 管理器
    ///
    /// # 参数
    /// - `device_manager`: 设备管理器引用
    ///
    /// # 返回
    /// GH3036 管理器实例
    pub fn new(device_manager: Arc<DeviceManager>) -> Self {
        Self {
            handle: Mutex::new(None),
            device_manager,
            initialized: Mutex::new(false),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
        }
    }

    /// 设置 AppHandle
    ///
    /// # 参数
    /// - `handle`: Tauri AppHandle
    pub fn set_app_handle(&self, handle: AppHandle) {
        CALLBACK_CONTEXT.set_app_handle(handle);
    }

    /// 检查是否已初始化
    ///
    /// # 返回
    /// - `true`: 已初始化
    /// - `false`: 未初始化
    pub fn is_initialized(&self) -> bool {
        *self.initialized.lock()
    }

    /// 检查 C 库是否已链接
    ///
    /// # 返回
    /// - `true`: C 库已链接
    /// - `false`: C 库未链接
    pub fn is_library_linked() -> bool {
        ffi::is_linked()
    }

    /// 初始化协议管理器
    ///
    /// # 功能
    /// - 创建 C 库协议实例
    /// - 配置回调函数
    /// - 启动处理线程
    ///
    /// # 返回
    /// - `Ok(())`: 初始化成功
    /// - `Err(String)`: 初始化失败
    pub fn initialize(&self) -> Result<(), String> {
        if !ffi::is_linked() {
            info!("GH3036 C 库未链接，使用纯 Rust 模式");
            let mut initialized = self.initialized.lock();
            *initialized = true;
            return Ok(());
        }

        {
            let handle_guard = self.handle.lock();
            if handle_guard.is_some() {
                return Ok(());
            }
        }

        CALLBACK_CONTEXT.set_device_manager(Arc::clone(&self.device_manager));
        
        if let Ok(handle) = Handle::try_current() {
            CALLBACK_CONTEXT.set_runtime_handle(handle);
        }

        let config = ffi::GhProtocolConfig {
            lock: Some(sync::gh_protocol_lock),
            unlock: Some(sync::gh_protocol_unlock),
            delay: Some(sync::gh_protocol_delay),
            send: Some(Self::send_callback),
            event_callback: Some(Self::event_callback),
            frame_callback: Some(Self::frame_callback),
        };

        let new_handle = unsafe { ffi::gh_protocol_create(&config) };
        if new_handle.is_null() {
            return Err("创建 GH3036 协议实例失败".to_string());
        }

        {
            let mut handle_guard = self.handle.lock();
            *handle_guard = Some(new_handle);
        }

        {
            let mut initialized = self.initialized.lock();
            *initialized = true;
        }

        self.start_processing_thread()?;

        info!("GH3036 协议管理器初始化成功 (C 库模式)");
        Ok(())
    }

    /// 启动处理线程
    ///
    /// # 功能
    /// 启动独立线程处理发送请求、事件和帧数据
    fn start_processing_thread(&self) -> Result<(), String> {
        let running = self.running.clone();
        running.store(true, std::sync::atomic::Ordering::SeqCst);

        let (send_receiver, event_receiver, frame_receiver) = CALLBACK_CONTEXT.setup_channels();
        let device_manager = Arc::clone(&self.device_manager);
        let running_clone = running.clone();
        
        let thread_handle = std::thread::spawn(move || {
            info!("GH3036 处理线程启动");

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
                    default(std::time::Duration::from_millis(10)) => {
                        // 超时后继续循环检查 running 标志
                    }
                }
            }

            info!("GH3036 处理线程停止");
        });

        {
            let mut thread_guard = self.thread_handle.lock();
            *thread_guard = Some(thread_handle);
        }

        Ok(())
    }

    /// 处理发送请求
    fn handle_send_request(device_manager: &Arc<DeviceManager>, request: SendRequest) {
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        let Some(channel) = tx_channel.as_ref() else {
            warn!("GH3036 TX 通道未配置，无法发送数据");
            return;
        };

        let route_id = match channel.channel_type {
            ChannelType::Serial => format!("serial-{}", channel.device_id),
            ChannelType::Ble => format!("ble-{}", channel.device_id),
        };

        let device_manager_clone = Arc::clone(device_manager);
        let data = request.data;
        
        if let Some(handle) = CALLBACK_CONTEXT.runtime_handle.lock().as_ref() {
            handle.spawn(async move {
                if let Err(e) = device_manager_clone.route_data(&route_id, &data).await {
                    error!("GH3036 发送数据失败: {}", e);
                }
            });
        } else {
            debug!("GH3036 异步运行时不可用，跳过发送");
        }
    }

    /// 处理事件数据
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

    /// 处理帧数据
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

    /// 保存帧数据到 CSV
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

    /// 停止处理线程
    fn stop_processing_thread(&self) {
        self.running.store(false, std::sync::atomic::Ordering::SeqCst);
        
        let mut thread_guard = self.thread_handle.lock();
        if let Some(thread) = thread_guard.take() {
            let _ = thread.join();
        }
    }

    /// 发送回调函数
    ///
    /// # 功能
    /// C 库调用此函数发送数据
    ///
    /// # 参数
    /// - `data`: 数据指针
    /// - `size`: 数据长度
    ///
    /// # 线程安全
    /// 可能在 C 库线程中调用
    unsafe extern "C" fn send_callback(data: *mut std::ffi::c_void, size: std::os::raw::c_int) {
        if data.is_null() || size <= 0 {
            return;
        }

        let data_slice = std::slice::from_raw_parts(data as *const u8, size as usize);
        let data_vec = data_slice.to_vec();

        debug!("GH3036 send_callback: {} bytes", data_vec.len());

        let request = SendRequest { data: data_vec };
        if let Err(e) = CALLBACK_CONTEXT.send_data_request(request) {
            error!("GH3036 发送请求入队失败: {}", e);
        }
    }

    /// 帧数据回调函数
    ///
    /// # 功能
    /// C 库解析完成帧数据后调用此函数
    ///
    /// # 参数
    /// - `frame`: 帧数据指针
    ///
    /// # 线程安全
    /// 可能在 C 库线程中调用
    unsafe extern "C" fn frame_callback(frame: *mut ffi::DataFrame) {
        if frame.is_null() {
            return;
        }

        let frame_ref = &*frame;
        let frame_data = Gh3036FrameData::from_c_frame(frame_ref);

        debug!(
            "GH3036 frame_callback: func_id={}, frame_id={}, timestamp={}",
            frame_data.function_id, frame_data.frame_id, frame_data.timestamp
        );

        if let Err(e) = CALLBACK_CONTEXT.send_frame_data(frame_data) {
            error!("GH3036 帧数据入队失败: {}", e);
        }
    }

    /// 事件回调函数
    ///
    /// # 功能
    /// C 库产生事件时调用此函数
    ///
    /// # 参数
    /// - `event_type`: 事件类型
    /// - `data`: 事件数据指针
    /// - `size`: 数据长度
    ///
    /// # 线程安全
    /// 可能在 C 库线程中调用
    unsafe extern "C" fn event_callback(event_type: u8, data: *mut u8, size: u32) {
        if data.is_null() || size == 0 {
            return;
        }

        let data_slice = std::slice::from_raw_parts(data, size as usize);
        let event_data = Gh3036EventData::new(event_type, data_slice);

        debug!(
            "GH3036 event_callback: type={}, size={}",
            event_type, size
        );

        if let Err(e) = CALLBACK_CONTEXT.send_event_data(event_data) {
            error!("GH3036 事件数据入队失败: {}", e);
        }
    }

    /// 配置 TX 通道
    ///
    /// # 参数
    /// - `config`: 通道配置
    ///
    /// # 返回
    /// - `Ok(())`: 配置成功
    /// - `Err(String)`: 配置失败
    pub fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_tx_channel(config.clone());
        info!("GH3036 TX 通道配置成功: {:?}", config);
        Ok(())
    }

    /// 配置 RX 通道
    ///
    /// # 参数
    /// - `config`: 通道配置
    ///
    /// # 返回
    /// - `Ok(())`: 配置成功
    /// - `Err(String)`: 配置失败
    pub fn configure_rx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        info!("GH3036 RX 通道配置成功: {:?}", config);
        Ok(())
    }

    /// 获取 TX 通道配置
    ///
    /// # 返回
    /// TX 通道配置（如果已配置）
    pub fn get_tx_channel(&self) -> Option<ChannelConfig> {
        CALLBACK_CONTEXT.tx_channel.lock().clone()
    }

    /// 获取 RX 通道配置
    ///
    /// # 返回
    /// RX 通道配置（如果已配置）
    pub fn get_rx_channel(&self) -> Option<ChannelConfig> {
        None
    }

    /// 设置 CSV 配置
    ///
    /// # 参数
    /// - `config`: CSV 配置
    ///
    /// # 返回
    /// - `Ok(())`: 设置成功
    /// - `Err(String)`: 设置失败
    pub fn set_csv_config(&self, config: CsvConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_csv_config(config);
        info!("GH3036 CSV 配置更新成功");
        Ok(())
    }

    /// 获取 CSV 配置
    ///
    /// # 返回
    /// 当前 CSV 配置
    pub fn get_csv_config(&self) -> CsvConfig {
        CALLBACK_CONTEXT.csv_config.lock().clone()
    }

    /// 发送数据
    ///
    /// # 功能
    /// 通过配置的 TX 通道发送数据
    ///
    /// # 参数
    /// - `data`: 待发送的数据
    ///
    /// # 返回
    /// - `Ok(())`: 发送成功
    /// - `Err(String)`: 发送失败
    pub async fn send_data(&self, data: &[u8]) -> Result<(), String> {
        let (route_id, device_id) = {
            let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
            let channel = tx_channel.as_ref().ok_or("TX 通道未配置")?;
            
            let route_id = match channel.channel_type {
                ChannelType::Serial => format!("serial-{}", channel.device_id),
                ChannelType::Ble => {
                    let _char_uuid = channel.characteristic_uuid.as_ref()
                        .ok_or("蓝牙 TX 通道缺少特征 UUID")?;
                    format!("ble-{}", channel.device_id)
                }
            };
            (route_id, channel.device_id.clone())
        };

        self.device_manager
            .route_data(&route_id, data)
            .await
            .map_err(|e| e.to_string())?;

        debug!("GH3036 发送数据: {} bytes to {}", data.len(), device_id);
        Ok(())
    }

    /// 处理接收数据
    ///
    /// # 功能
    /// 当 RX 通道收到数据时调用此函数
    ///
    /// # 参数
    /// - `device_id`: 设备 ID
    /// - `data`: 接收的数据
    pub fn on_data_received(&self, device_id: &str, data: &[u8]) {
        debug!("GH3036 接收数据: {} bytes from {}", data.len(), device_id);

        if !ffi::is_linked() {
            return;
        }

        let handle_opt = *self.handle.lock();
        if let Some(handle) = handle_opt {
            if !handle.is_null() {
                let mut data_mut = data.to_vec();
                unsafe {
                    let result = ffi::gh_protocol_receive(
                        handle,
                        data_mut.as_mut_ptr(),
                        data_mut.len() as u32,
                    );
                    if result < 0 {
                        error!("gh_protocol_receive 失败: {}", result);
                    }
                }
            }
        }
    }
}

impl Drop for Gh3036Manager {
    fn drop(&mut self) {
        self.stop_processing_thread();

        if let Some(mut handle_guard) = self.handle.try_lock() {
            if let Some(handle) = handle_guard.take() {
                if !handle.is_null() && ffi::is_linked() {
                    unsafe {
                        ffi::gh_protocol_destroy(handle);
                    }
                }
            }
        }
    }
}
