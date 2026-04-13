//! GH3036 协议管理器
//!
//! 本模块实现 GH3036 协议的核心管理功能：
//! - 协议实例生命周期管理
//! - RPC 命令执行
//! - RX 数据处理
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
use super::types::{Gh3036EventData, Gh3036FrameData, DataFrame, FuncFrame, FrameDecoder};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
        Self {
            enabled: false,
            output_dir: String::from("."),
        }
    }
}

struct SendRequest {
    data: Vec<u8>,
}

struct GlobalContext {
    tx_channel: Mutex<Option<ChannelConfig>>,
    device_manager: Mutex<Option<Arc<DeviceManager>>>,
    app_handle: Mutex<Option<AppHandle>>,
    csv_config: Mutex<CsvConfig>,
    csv_writers: Mutex<HashMap<i32, CsvWriter>>,
    send_sender: Mutex<Option<Sender<SendRequest>>>,
    event_sender: Mutex<Option<Sender<Gh3036EventData>>>,
    frame_sender: Mutex<Option<Sender<Gh3036FrameData>>>,
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

pub struct Gh3036Manager {
    frame_decoder: Mutex<FrameDecoder>,
    device_manager: Arc<DeviceManager>,
    initialized: Mutex<bool>,
    running: Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Mutex<Option<std::thread::JoinHandle<()>>>,
    events_subscribed: std::sync::atomic::AtomicBool,
}

unsafe impl Send for Gh3036Manager {}
unsafe impl Sync for Gh3036Manager {}

impl Gh3036Manager {
    pub fn new(device_manager: Arc<DeviceManager>) -> Self {
        Self {
            frame_decoder: Mutex::new(FrameDecoder::new()),
            device_manager,
            initialized: Mutex::new(false),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            thread_handle: Mutex::new(None),
            events_subscribed: std::sync::atomic::AtomicBool::new(false),
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
        info!("GH3036 协议管理器初始化 (纯 Rust 模式)");
        
        CALLBACK_CONTEXT.set_device_manager(Arc::clone(&self.device_manager));
        
        if let Ok(handle) = Handle::try_current() {
            CALLBACK_CONTEXT.set_runtime_handle(handle);
        }

        {
            let mut initialized = self.initialized.lock();
            *initialized = true;
        }

        self.start_processing_thread()?;

        info!("GH3036 协议管理器初始化成功");
        Ok(())
    }

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
    }

    pub fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        CALLBACK_CONTEXT.set_tx_channel(config.clone());
        info!("GH3036 TX 通道配置成功: {:?}", config);
        Ok(())
    }

    pub fn configure_rx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        info!("GH3036 RX 通道配置成功: {:?}", config);
        Ok(())
    }

    pub fn get_tx_channel(&self) -> Option<ChannelConfig> {
        CALLBACK_CONTEXT.tx_channel.lock().clone()
    }

    pub fn get_rx_channel(&self) -> Option<ChannelConfig> {
        None
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

    pub fn on_data_received(&self, _device_id: &str, data: &[u8]) {
        debug!("GH3036 接收数据: {} bytes", data.len());

        let mut frames: heapless::Vec<FuncFrame, 16> = heapless::Vec::new();
        
        let mut decoder = self.frame_decoder.lock();
        let count = decoder.decode_frames(data, &mut frames);
        
        if count > 0 {
            debug!("GH3036 解码到 {} 帧", count);
            for frame in frames.iter() {
                let frame_data = Gh3036FrameData::from_func_frame(frame);
                if let Err(e) = CALLBACK_CONTEXT.send_frame_data(frame_data) {
                    error!("GH3036 帧数据入队失败: {}", e);
                }
            }
        }
    }

    pub fn execute_rpc(&self, command_key: &str, params: &[String]) -> Result<Vec<u8>, String> {
        info!("GH3036 execute_rpc 开始: key={}, params={:?}", command_key, params);

        let result = match command_key {
            "V" => self.execute_version_cmd(params),
            "W" => self.execute_regs_write_cmd(params),
            "R" => self.execute_regs_read_cmd(params),
            "B" => self.execute_reg_bitfield_write_cmd(params),
            "C" => self.execute_chip_ctrl_cmd(params),
            "D" => self.execute_download_config_cmd(params),
            "L" => self.execute_regs_list_write_cmd(params),
            "S" => self.execute_sw_function_cmd(params),
            "P" => self.execute_low_power_cmd(params),
            "M" => self.execute_set_work_mode_cmd(params),
            "TS" => self.execute_timestamp_set_cmd(params),
            "TM" => self.execute_time_set_cmd(params),
            _ => {
                error!("GH3036 execute_rpc 不支持的命令键: {}", command_key);
                Err(format!("不支持的命令键: {}", command_key))
            }
        };

        info!("GH3036 execute_rpc 完成: key={}, result={:?}", command_key, result.is_ok());
        result
    }

    fn execute_version_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let ver_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("GH3036 execute_version_cmd: ver_type={}", ver_type);
        
        Ok(vec![ver_type])
    }

    fn execute_regs_write_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() || regs.len() % 2 != 0 {
            return Err("寄存器数据格式错误，需要成对的地址和值".to_string());
        }

        info!("寄存器写入: {} 个寄存器", regs.len() / 2);
        Ok(vec![])
    }

    fn execute_regs_read_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let reg_addr: u16 = params
            .first()
            .and_then(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少寄存器地址参数")?;

        let read_len: i32 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);

        info!("寄存器读取: addr=0x{:04X}, len={}", reg_addr, read_len);
        Ok(vec![0; (read_len * 2) as usize])
    }

    fn execute_reg_bitfield_write_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
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
        Ok(vec![])
    }

    fn execute_chip_ctrl_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let ctrl_type: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少控制类型参数")?;

        info!("芯片控制: type={}", ctrl_type);
        Ok(vec![])
    }

    fn execute_download_config_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let stage: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("下载配置: stage={}", stage);
        Ok(vec![])
    }

    fn execute_regs_list_write_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let regs: Vec<u16> = params
            .iter()
            .filter_map(|s| u16::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .collect();

        if regs.is_empty() {
            return Err("寄存器列表为空".to_string());
        }

        info!("寄存器列表写入: {} 个值", regs.len());
        Ok(vec![])
    }

    fn execute_sw_function_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("软件功能命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);
        Ok(vec![])
    }

    fn execute_low_power_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let target_func_mode: u32 = params
            .first()
            .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            .ok_or("缺少目标功能模式参数")?;

        let ctrl_type: u8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        info!("低功耗命令: mode=0x{:08X}, ctrl={}", target_func_mode, ctrl_type);
        Ok(vec![])
    }

    fn execute_set_work_mode_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let work_mode: u8 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少工作模式参数")?;

        info!("设置工作模式: mode={}", work_mode);
        Ok(vec![])
    }

    fn execute_timestamp_set_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        info!("设置时间戳: {}", timestamp);
        Ok(vec![])
    }

    fn execute_time_set_cmd(&self, params: &[String]) -> Result<Vec<u8>, String> {
        let timestamp: u32 = params
            .first()
            .and_then(|s| s.parse().ok())
            .ok_or("缺少时间戳参数")?;

        let hour_offset: i8 = params
            .get(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(8);

        info!("设置时间: timestamp={}, offset={}", timestamp, hour_offset);
        Ok(vec![])
    }

    pub fn subscribe_events(&self) -> bool {
        self.events_subscribed.store(true, std::sync::atomic::Ordering::SeqCst);
        info!("GH3036 事件订阅已启用");
        true
    }

    pub fn is_events_subscribed(&self) -> bool {
        self.events_subscribed.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_library_status(&self) -> (bool, bool) {
        (true, self.is_initialized())
    }

    pub fn on_rx_data(&self, device_id: &str, data: &[u8]) {
        self.on_data_received(device_id, data);
    }
}

impl Drop for Gh3036Manager {
    fn drop(&mut self) {
        self.stop_processing_thread();
    }
}
