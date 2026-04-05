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
    /// 事件订阅状态
    events_subscribed: std::sync::atomic::AtomicBool,
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
            events_subscribed: std::sync::atomic::AtomicBool::new(false),
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
        info!("GH3036 handle_send_request 开始处理: {} bytes", request.data.len());
        
        let tx_channel = CALLBACK_CONTEXT.tx_channel.lock();
        let Some(channel) = tx_channel.as_ref() else {
            warn!("GH3036 TX 通道未配置，无法发送数据");
            return;
        };

        let route_id = match channel.channel_type {
            ChannelType::Serial => format!("serial-{}", channel.device_id),
            ChannelType::Ble => format!("ble-{}", channel.device_id),
        };

        info!("GH3036 handle_send_request 路由ID: {}, 数据: {:02X?}", route_id, request.data);

        let device_manager_clone = Arc::clone(device_manager);
        let data = request.data;
        
        if let Some(handle) = CALLBACK_CONTEXT.runtime_handle.lock().as_ref() {
            info!("GH3036 handle_send_request 在异步运行时中发送");
            handle.spawn(async move {
                info!("GH3036 handle_send_request 异步任务开始, route_id={}", route_id);
                if let Err(e) = device_manager_clone.route_data(&route_id, &data).await {
                    error!("GH3036 发送数据失败: {}", e);
                } else {
                    info!("GH3036 handle_send_request 发送成功: {} bytes", data.len());
                }
            });
        } else {
            warn!("GH3036 异步运行时不可用，跳过发送");
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
        info!("GH3036 send_callback 被调用: size={}", size);
        
        if data.is_null() || size <= 0 {
            warn!("GH3036 send_callback 参数无效: data.is_null={}, size={}", data.is_null(), size);
            return;
        }

        let data_slice = std::slice::from_raw_parts(data as *const u8, size as usize);
        let data_vec = data_slice.to_vec();

        info!("GH3036 send_callback 数据: {:02X?}", data_vec);

        let request = SendRequest { data: data_vec };
        if let Err(e) = CALLBACK_CONTEXT.send_data_request(request) {
            error!("GH3036 发送请求入队失败: {}", e);
        } else {
            info!("GH3036 send_callback 发送请求已入队");
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
        info!("GH3036 send_data 被调用: {} bytes, data={:02X?}", data.len(), data);

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
            info!("GH3036 send_data 路由ID: {}, 设备ID: {}", route_id, channel.device_id);
            (route_id, channel.device_id.clone())
        };

        info!("GH3036 send_data 调用 device_manager.route_data");
        self.device_manager
            .route_data(&route_id, data)
            .await
            .map_err(|e| {
                error!("GH3036 send_data 失败: {}", e);
                e.to_string()
            })?;

        info!("GH3036 发送数据成功: {} bytes to {}", data.len(), device_id);
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

    /// 执行 RPC 指令
    ///
    /// # 功能
    /// 根据命令键和参数执行对应的 RPC 指令
    ///
    /// # 参数
    /// - `command_key`: 命令键（V、W、R、B、C、D、L、S、P、M、TS、TM）
    /// - `params`: 参数列表
    ///
    /// # 返回
    /// - `Ok(Vec<u8>)`: 执行成功，返回响应数据
    /// - `Err(String)`: 执行失败
    pub fn execute_rpc(&self, command_key: &str, params: &[String]) -> Result<Vec<u8>, String> {
        info!("GH3036 execute_rpc 开始: key={}, params={:?}", command_key, params);

        if !ffi::is_linked() {
            error!("GH3036 execute_rpc C 库未链接");
            return Err("C 库未链接，无法执行 RPC 指令".to_string());
        }

        let handle_opt = *self.handle.lock();
        let handle = handle_opt.ok_or_else(|| {
            error!("GH3036 execute_rpc 协议实例未初始化");
            "协议实例未初始化".to_string()
        })?;

        if handle.is_null() {
            error!("GH3036 execute_rpc 协议句柄无效");
            return Err("协议句柄无效".to_string());
        }

        info!("GH3036 execute_rpc 协议句柄有效，开始执行命令");

        let result = match command_key {
            "V" => self.execute_version_cmd(handle, params),
            "W" => self.execute_regs_write_cmd(handle, params),
            "R" => self.execute_regs_read_cmd(handle, params),
            "B" => self.execute_reg_bitfield_write_cmd(handle, params),
            "C" => self.execute_chip_ctrl_cmd(handle, params),
            "D" => self.execute_download_config_cmd(handle, params),
            "L" => self.execute_regs_list_write_cmd(handle, params),
            "S" => self.execute_sw_function_cmd(handle, params),
            "P" => self.execute_low_power_cmd(handle, params),
            "M" => self.execute_set_work_mode_cmd(handle, params),
            "TS" => self.execute_timestamp_set_cmd(handle, params),
            "TM" => self.execute_time_set_cmd(handle, params),
            _ => {
                error!("GH3036 execute_rpc 不支持的命令键: {}", command_key);
                Err(format!("不支持的命令键: {}", command_key))
            }
        };

        info!("GH3036 execute_rpc 完成: key={}, result={:?}", command_key, result.is_ok());
        result
    }

    fn execute_version_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let ver_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("GH3036 execute_version_cmd: ver_type={}", ver_type);

        let key = std::ffi::CString::new("V").map_err(|e| e.to_string())?;
        let format = std::ffi::CString::new("%d").map_err(|e| e.to_string())?;

        info!("GH3036 execute_version_cmd 调用 gh_protocol_send_raw: key={:?}, format={:?}, ver_type={}", key, format, ver_type);

        unsafe {
            let result = ffi::gh_protocol_send_raw(handle, key.as_ptr(), format.as_ptr(), ver_type as i32);
            info!("GH3036 gh_protocol_send_raw 返回: {}", result);
            if result < 0 {
                return Err(format!("发送版本命令失败: {}", result));
            }
        }

        info!("GH3036 execute_version_cmd 完成");
        Ok(vec![])
    }

    fn execute_regs_write_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() || regs.len() % 2 != 0 {
            return Err("寄存器数据格式错误，需要成对的地址和值".to_string());
        }

        let mut regs_mut = regs.clone();
        unsafe {
            ffi::gh_protocol_regs_write(handle, regs_mut.as_mut_ptr(), (regs.len() / 2) as i32);
        }

        info!("寄存器写入: {} 个寄存器", regs.len() / 2);
        Ok(vec![])
    }

    fn execute_regs_read_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let read_len: i32 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        let mut reg_values = vec![0u16; read_len as usize];
        let mut actual_len: i32 = 0;

        unsafe {
            ffi::gh_protocol_regs_read(
                handle,
                reg_addr,
                read_len,
                reg_values.as_mut_ptr(),
                &mut actual_len,
            );
        }

        reg_values.truncate(actual_len as usize);
        let result: Vec<u8> = reg_values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();

        info!("寄存器读取: addr=0x{:04X}, len={}", reg_addr, actual_len);
        Ok(result)
    }

    fn execute_reg_bitfield_write_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
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

        unsafe {
            ffi::gh_protocol_reg_bitfield_write(handle, reg_addr, lsb, msb, reg_val);
        }

        info!("位域写入: addr=0x{:04X}, lsb={}, msb={}, val=0x{:04X}", reg_addr, lsb, msb, reg_val);
        Ok(vec![])
    }

    fn execute_chip_ctrl_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let ctrl_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少控制类型参数")?;

        unsafe {
            ffi::gh_protocol_chip_ctrl(handle, ctrl_type);
        }

        info!("芯片控制: type={}", ctrl_type);
        Ok(vec![])
    }

    fn execute_download_config_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let stage: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        unsafe {
            ffi::gh_protocol_download_config(handle, stage);
        }

        info!("下载配置: stage={}", stage);
        Ok(vec![])
    }

    fn execute_regs_list_write_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        let mut regs_mut = regs.clone();
        unsafe {
            ffi::gh_protocol_regs_list_write(handle, regs_mut.as_mut_ptr(), regs.len() as u16);
        }

        info!("寄存器列表写入: {} 个值", regs.len());
        Ok(vec![])
    }

    fn execute_sw_function_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        unsafe {
            ffi::gh_protocol_sw_function_cmd(handle, target_func_mode, ctrl_type);
        }

        info!("软件功能命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);
        Ok(vec![])
    }

    fn execute_low_power_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        unsafe {
            ffi::gh_protocol_low_power_cmd(handle, target_func_mode, ctrl_type);
        }

        info!("低功耗命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);
        Ok(vec![])
    }

    fn execute_set_work_mode_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let work_mode: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少工作模式参数")?;

        unsafe {
            ffi::gh_protocol_set_work_mode(handle, work_mode);
        }

        info!("设置工作模式: mode={}", work_mode);
        Ok(vec![])
    }

    fn execute_timestamp_set_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        unsafe {
            ffi::gh_protocol_timestamp_set(handle, timestamp);
        }

        info!("设置时间戳: {}", timestamp);
        Ok(vec![])
    }

    fn execute_time_set_cmd(
        &self,
        handle: *mut ffi::GhProtocolHandle,
        params: &[String],
    ) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        let hour_offset: i8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);

        unsafe {
            ffi::gh_protocol_time_set(handle, timestamp, hour_offset);
        }

        info!("设置时间: timestamp={}, offset={}", timestamp, hour_offset);
        Ok(vec![])
    }

    /// 订阅事件
    ///
    /// # 功能
    /// 标记前端已准备好接收事件
    ///
    /// # 返回
    /// 是否订阅成功
    pub fn subscribe_events(&self) -> bool {
        self.events_subscribed.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("GH3036 事件订阅已启用");
        true
    }

    /// 检查是否已订阅事件
    ///
    /// # 返回
    /// - `true`: 已订阅
    /// - `false`: 未订阅
    pub fn is_events_subscribed(&self) -> bool {
        self.events_subscribed.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// 获取库状态
    ///
    /// # 返回
    /// (是否已链接, 是否已初始化)
    pub fn get_library_status(&self) -> (bool, bool) {
        (ffi::is_linked(), self.is_initialized())
    }

    /// RX 数据接收（供设备管理器调用）
    ///
    /// # 功能
    /// 将接收的数据传递给协议库处理
    ///
    /// # 参数
    /// - `device_id`: 设备 ID
    /// - `data`: 接收的数据
    pub fn on_rx_data(&self, device_id: &str, data: &[u8]) {
        self.on_data_received(device_id, data);
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
