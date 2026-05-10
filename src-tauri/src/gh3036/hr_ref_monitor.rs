//! HR金标蓝牙监听器模块
//!
//! 本模块实现从标准心率蓝牙服务设备获取心率数据，并自动写入金标

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::Arc;

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use super::ref_data_manager::RefDataManager;
use crate::device::BleManagerRef;

const HEART_RATE_MEASUREMENT_UUID: &str = "00002a37-0000-1000-8000-00805f9b34fb";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HrRefMonitorState {
    Idle,
    Connecting,
    Subscribing,
    Monitoring,
    Stopping,
    Error,
}

impl std::fmt::Display for HrRefMonitorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HrRefMonitorState::Idle => write!(f, "idle"),
            HrRefMonitorState::Connecting => write!(f, "connecting"),
            HrRefMonitorState::Subscribing => write!(f, "subscribing"),
            HrRefMonitorState::Monitoring => write!(f, "monitoring"),
            HrRefMonitorState::Stopping => write!(f, "stopping"),
            HrRefMonitorState::Error => write!(f, "error"),
        }
    }
}

struct HrRefMonitorInner {
    state: Mutex<HrRefMonitorState>,
    device_address: Mutex<Option<String>>,
    current_hr: AtomicI32,
    collected_count: AtomicI32,
    is_running: AtomicBool,
    hr_values: Mutex<Vec<i32>>,
}

impl HrRefMonitorInner {
    fn new() -> Self {
        Self {
            state: Mutex::new(HrRefMonitorState::Idle),
            device_address: Mutex::new(None),
            current_hr: AtomicI32::new(-1),
            collected_count: AtomicI32::new(0),
            is_running: AtomicBool::new(false),
            hr_values: Mutex::new(Vec::new()),
        }
    }
}

static HR_REF_MONITOR: Lazy<HrRefMonitorInner> = Lazy::new(HrRefMonitorInner::new);
static HR_REF_MONITOR_BLE_MANAGER: Lazy<Mutex<Option<BleManagerRef>>> =
    Lazy::new(|| Mutex::new(None));
static HR_REF_MONITOR_REF_DATA_MANAGER: Lazy<Mutex<Option<Arc<RefDataManager>>>> =
    Lazy::new(|| Mutex::new(None));

pub fn init_hr_ref_monitor(ble_manager: BleManagerRef, ref_data_manager: Arc<RefDataManager>) {
    *HR_REF_MONITOR_BLE_MANAGER.lock() = Some(ble_manager);
    *HR_REF_MONITOR_REF_DATA_MANAGER.lock() = Some(ref_data_manager);
    info!("[HrRefMonitor] 初始化完成");
}

pub fn get_hr_ref_monitor_state() -> HrRefMonitorState {
    *HR_REF_MONITOR.state.lock()
}

pub fn get_hr_ref_monitor_current_hr() -> i32 {
    HR_REF_MONITOR.current_hr.load(Ordering::SeqCst)
}

pub fn get_hr_ref_monitor_collected_count() -> i32 {
    HR_REF_MONITOR.collected_count.load(Ordering::SeqCst)
}

pub fn is_hr_ref_monitor_running() -> bool {
    HR_REF_MONITOR.is_running.load(Ordering::SeqCst)
}

pub fn get_hr_ref_monitor_device_address() -> Option<String> {
    HR_REF_MONITOR.device_address.lock().clone()
}

pub async fn start_hr_ref_monitor(device_address: &str) -> Result<(), String> {
    if HR_REF_MONITOR.is_running.load(Ordering::SeqCst) {
        warn!("[HrRefMonitor] 监听器已在运行中");
        return Err("监听器已在运行中".to_string());
    }

    let ble_manager = HR_REF_MONITOR_BLE_MANAGER.lock().clone();
    let ble_manager = match ble_manager {
        Some(m) => m,
        None => return Err("HR金标监听器未初始化".to_string()),
    };

    let _ref_data_manager = match HR_REF_MONITOR_REF_DATA_MANAGER.lock().clone() {
        Some(m) => m,
        None => return Err("HR金标监听器未初始化".to_string()),
    };

    info!("[HrRefMonitor] 启动HR金标监听，设备: {}", device_address);

    *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Connecting;
    *HR_REF_MONITOR.device_address.lock() = Some(device_address.to_string());
    HR_REF_MONITOR.current_hr.store(-1, Ordering::SeqCst);
    HR_REF_MONITOR.collected_count.store(0, Ordering::SeqCst);
    HR_REF_MONITOR.hr_values.lock().clear();
    HR_REF_MONITOR.is_running.store(true, Ordering::SeqCst);

    let device_address_owned = device_address.to_string();
    let state_for_callback = Arc::new(Mutex::new(HrRefMonitorState::Connecting));
    let current_hr_for_callback = Arc::new(AtomicI32::new(-1));
    let collected_count_for_callback = Arc::new(AtomicI32::new(0));
    let hr_values_for_callback = Arc::new(Mutex::new(Vec::new()));
    let is_running_for_callback = Arc::new(AtomicBool::new(true));

    let current_hr_clone = current_hr_for_callback.clone();
    let collected_count_clone = collected_count_for_callback.clone();
    let hr_values_clone = hr_values_for_callback.clone();
    let is_running_clone = is_running_for_callback.clone();

    let callback = Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
        if !is_running_clone.load(Ordering::SeqCst) {
            return;
        }

        if let Some(hr) = parse_heart_rate_measurement(data) {
            debug!("[HrRefMonitor] 收到心率数据: {} bpm", hr);
            current_hr_clone.store(hr, Ordering::SeqCst);
            HR_REF_MONITOR.current_hr.store(hr, Ordering::SeqCst);

            let mut values = hr_values_clone.lock();
            if values.len() < 4 {
                values.push(hr);
                let count = values.len() as i32;
                collected_count_clone.store(count, Ordering::SeqCst);
                HR_REF_MONITOR
                    .collected_count
                    .store(count, Ordering::SeqCst);
                info!("[HrRefMonitor] 采集心率金标 [{}/4]: {} bpm", count, hr);

                if values.len() == 4 {
                    if let Some(ref_data_mgr) = HR_REF_MONITOR_REF_DATA_MANAGER.lock().as_ref() {
                        if let Err(e) = ref_data_mgr.set_hr_ref(&values) {
                            error!("[HrRefMonitor] 设置HR金标失败: {}", e);
                        } else {
                            info!("[HrRefMonitor] HR金标设置成功: {:?}", *values);
                        }
                    }
                }
            }
        }
    });

    match ble_manager.connect(&device_address_owned).await {
        Ok(_) => {
            info!("[HrRefMonitor] 设备连接成功: {}", device_address_owned);
            *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Subscribing;
            *state_for_callback.lock() = HrRefMonitorState::Subscribing;
        }
        Err(e) => {
            error!("[HrRefMonitor] 设备连接失败: {}", e);
            *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Error;
            HR_REF_MONITOR.is_running.store(false, Ordering::SeqCst);
            return Err(format!("设备连接失败: {}", e));
        }
    }

    match ble_manager
        .subscribe_notify(&device_address_owned, HEART_RATE_MEASUREMENT_UUID, callback)
        .await
    {
        Ok(_) => {
            info!("[HrRefMonitor] 订阅心率特征成功");
            *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Monitoring;
            *state_for_callback.lock() = HrRefMonitorState::Monitoring;
        }
        Err(e) => {
            error!("[HrRefMonitor] 订阅心率特征失败: {}", e);
            *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Error;
            HR_REF_MONITOR.is_running.store(false, Ordering::SeqCst);
            let _ = ble_manager.disconnect(&device_address_owned).await;
            return Err(format!("订阅心率特征失败: {}", e));
        }
    }

    Ok(())
}

pub async fn stop_hr_ref_monitor() -> Result<(), String> {
    if !HR_REF_MONITOR.is_running.load(Ordering::SeqCst) {
        return Ok(());
    }

    info!("[HrRefMonitor] 停止HR金标监听");
    *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Stopping;
    HR_REF_MONITOR.is_running.store(false, Ordering::SeqCst);

    let ble_manager = HR_REF_MONITOR_BLE_MANAGER.lock().clone();
    let address = HR_REF_MONITOR.device_address.lock().take();

    if let Some(ble_manager) = ble_manager {
        if let Some(address) = address {
            if let Err(e) = ble_manager
                .unsubscribe_notify(&address, HEART_RATE_MEASUREMENT_UUID)
                .await
            {
                warn!("[HrRefMonitor] 取消订阅失败: {}", e);
            }

            if let Err(e) = ble_manager.disconnect(&address).await {
                warn!("[HrRefMonitor] 断开设备连接失败: {}", e);
            }
        }
    }

    *HR_REF_MONITOR.state.lock() = HrRefMonitorState::Idle;
    info!("[HrRefMonitor] HR金标监听已停止");

    Ok(())
}

pub struct HrRefMonitor {
    ble_manager: BleManagerRef,
    ref_data_manager: Arc<RefDataManager>,
    state: Mutex<HrRefMonitorState>,
    device_address: Mutex<Option<String>>,
    current_hr: AtomicI32,
    collected_count: AtomicI32,
    is_running: AtomicBool,
    hr_values: Mutex<Vec<i32>>,
}

impl HrRefMonitor {
    pub fn new(ble_manager: BleManagerRef, ref_data_manager: Arc<RefDataManager>) -> Self {
        Self {
            ble_manager,
            ref_data_manager,
            state: Mutex::new(HrRefMonitorState::Idle),
            device_address: Mutex::new(None),
            current_hr: AtomicI32::new(-1),
            collected_count: AtomicI32::new(0),
            is_running: AtomicBool::new(false),
            hr_values: Mutex::new(Vec::new()),
        }
    }

    pub fn get_state(&self) -> HrRefMonitorState {
        *self.state.lock()
    }

    pub fn get_current_hr(&self) -> i32 {
        self.current_hr.load(Ordering::SeqCst)
    }

    pub fn get_collected_count(&self) -> i32 {
        self.collected_count.load(Ordering::SeqCst)
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn get_device_address(&self) -> Option<String> {
        self.device_address.lock().clone()
    }

    pub async fn start(&self, device_address: &str) -> Result<(), String> {
        if self.is_running.load(Ordering::SeqCst) {
            warn!("[HrRefMonitor] 监听器已在运行中");
            return Err("监听器已在运行中".to_string());
        }

        info!("[HrRefMonitor] 启动HR金标监听，设备: {}", device_address);

        *self.state.lock() = HrRefMonitorState::Connecting;
        *self.device_address.lock() = Some(device_address.to_string());
        self.current_hr.store(-1, Ordering::SeqCst);
        self.collected_count.store(0, Ordering::SeqCst);
        self.hr_values.lock().clear();
        self.is_running.store(true, Ordering::SeqCst);

        let ble_manager = self.ble_manager.clone();
        let ref_data_manager = self.ref_data_manager.clone();
        let device_address = device_address.to_string();
        let state = Arc::new(Mutex::new(HrRefMonitorState::Connecting));
        let current_hr = Arc::new(AtomicI32::new(-1));
        let collected_count = Arc::new(AtomicI32::new(0));
        let hr_values = Arc::new(Mutex::new(Vec::new()));
        let is_running = Arc::new(AtomicBool::new(true));

        let state_clone = state.clone();
        let current_hr_clone = current_hr.clone();
        let collected_count_clone = collected_count.clone();
        let hr_values_clone = hr_values.clone();
        let is_running_clone = is_running.clone();

        let callback = Arc::new(move |_addr: &str, _char: &str, data: &[u8]| {
            if !is_running_clone.load(Ordering::SeqCst) {
                return;
            }

            if let Some(hr) = parse_heart_rate_measurement(data) {
                debug!("[HrRefMonitor] 收到心率数据: {} bpm", hr);
                current_hr_clone.store(hr, Ordering::SeqCst);

                let mut values = hr_values_clone.lock();
                if values.len() < 4 {
                    values.push(hr);
                    let count = values.len() as i32;
                    collected_count_clone.store(count, Ordering::SeqCst);
                    info!("[HrRefMonitor] 采集心率金标 [{}/4]: {} bpm", count, hr);

                    if values.len() == 4 {
                        if let Err(e) = ref_data_manager.set_hr_ref(&values) {
                            error!("[HrRefMonitor] 设置HR金标失败: {}", e);
                        } else {
                            info!("[HrRefMonitor] HR金标设置成功: {:?}", *values);
                        }
                    }
                }
            }
        });

        match ble_manager.connect(&device_address).await {
            Ok(_) => {
                info!("[HrRefMonitor] 设备连接成功: {}", device_address);
                *state.lock() = HrRefMonitorState::Subscribing;
                *state_clone.lock() = HrRefMonitorState::Subscribing;
            }
            Err(e) => {
                error!("[HrRefMonitor] 设备连接失败: {}", e);
                *self.state.lock() = HrRefMonitorState::Error;
                self.is_running.store(false, Ordering::SeqCst);
                return Err(format!("设备连接失败: {}", e));
            }
        }

        match ble_manager
            .subscribe_notify(&device_address, HEART_RATE_MEASUREMENT_UUID, callback)
            .await
        {
            Ok(_) => {
                info!("[HrRefMonitor] 订阅心率特征成功");
                *self.state.lock() = HrRefMonitorState::Monitoring;
                *state.lock() = HrRefMonitorState::Monitoring;
            }
            Err(e) => {
                error!("[HrRefMonitor] 订阅心率特征失败: {}", e);
                *self.state.lock() = HrRefMonitorState::Error;
                self.is_running.store(false, Ordering::SeqCst);
                let _ = ble_manager.disconnect(&device_address).await;
                return Err(format!("订阅心率特征失败: {}", e));
            }
        }

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), String> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        info!("[HrRefMonitor] 停止HR金标监听");
        *self.state.lock() = HrRefMonitorState::Stopping;
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(address) = self.device_address.lock().take() {
            if let Err(e) = self
                .ble_manager
                .unsubscribe_notify(&address, HEART_RATE_MEASUREMENT_UUID)
                .await
            {
                warn!("[HrRefMonitor] 取消订阅失败: {}", e);
            }

            if let Err(e) = self.ble_manager.disconnect(&address).await {
                warn!("[HrRefMonitor] 断开设备连接失败: {}", e);
            }
        }

        *self.state.lock() = HrRefMonitorState::Idle;
        info!("[HrRefMonitor] HR金标监听已停止");

        Ok(())
    }
}

fn parse_heart_rate_measurement(data: &[u8]) -> Option<i32> {
    if data.is_empty() {
        return None;
    }

    let flags = data[0];
    let is_16bit = (flags & 0x01) != 0;

    if data.len() < 2 {
        return None;
    }

    if is_16bit {
        if data.len() < 3 {
            return None;
        }
        let hr = u16::from_le_bytes([data[1], data[2]]) as i32;
        Some(hr)
    } else {
        Some(data[1] as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_heart_rate_8bit() {
        let data = [0x00, 72];
        assert_eq!(parse_heart_rate_measurement(&data), Some(72));
    }

    #[test]
    fn test_parse_heart_rate_16bit() {
        let data = [0x01, 0x00, 0x01];
        assert_eq!(parse_heart_rate_measurement(&data), Some(256));
    }

    #[test]
    fn test_parse_heart_rate_empty() {
        let data: [u8; 0] = [];
        assert_eq!(parse_heart_rate_measurement(&data), None);
    }

    #[test]
    fn test_parse_heart_rate_too_short() {
        let data = [0x00];
        assert_eq!(parse_heart_rate_measurement(&data), None);
    }
}
