//! 金标数据管理模块
//!
//! 本模块实现金标数据的持久化写入接口，支持：
//! - HR 金标：时间持久化（4秒超时）
//! - HRV 金标：时间持久化（4秒超时）
//! - SpO2 金标：会话级持久化（frame_count 重置时清除）

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tracing::{debug, info, warn};

use super::types::REF_DATA_COUNT;

pub const HR_REF_START: usize = 0;
pub const HR_REF_COUNT: usize = 4;
pub const HR_REF_NUM_IDX: usize = 4;

pub const SPO2_REF_START: usize = 5;
pub const SPO2_REF_COUNT: usize = 4;
pub const SPO2_REF_NUM_IDX: usize = 9;

pub const HRV_REF_START: usize = 10;
pub const HRV_REF_COUNT: usize = 4;
pub const HRV_REF_NUM_IDX: usize = 14;

pub const REF_DATA_TIMEOUT_SECS: u64 = 4;

#[derive(Debug, Clone)]
pub struct TimeBasedRefData {
    pub values: Vec<i32>,
    pub count: i32,
    pub last_update: Instant,
}

impl Default for TimeBasedRefData {
    fn default() -> Self {
        Self {
            values: vec![0; 4],
            count: 0,
            last_update: Instant::now(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionRefData {
    pub values: Vec<i32>,
    pub count: i32,
}

impl Default for SessionRefData {
    fn default() -> Self {
        Self {
            values: vec![0; 4],
            count: 0,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RefDataError {
    #[error("Invalid value count: expected {expected}, got {actual}")]
    InvalidCount { expected: usize, actual: usize },
    #[error("Value out of range: {value} at index {index}")]
    OutOfRange { value: i32, index: usize },
    #[error("Empty values provided")]
    EmptyValues,
}

#[derive(Debug, Clone)]
pub struct RefDataManager {
    hr_ref: Arc<Mutex<TimeBasedRefData>>,
    hrv_ref: Arc<Mutex<TimeBasedRefData>>,
    spo2_ref: Arc<Mutex<SessionRefData>>,
    last_frame_count: Arc<Mutex<usize>>,
}

impl Default for RefDataManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RefDataManager {
    pub fn new() -> Self {
        info!("[RefDataManager] 初始化金标数据管理器");
        Self {
            hr_ref: Arc::new(Mutex::new(TimeBasedRefData::default())),
            hrv_ref: Arc::new(Mutex::new(TimeBasedRefData::default())),
            spo2_ref: Arc::new(Mutex::new(SessionRefData::default())),
            last_frame_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn set_hr_ref(&self, values: &[i32]) -> Result<(), RefDataError> {
        if values.is_empty() {
            return Err(RefDataError::EmptyValues);
        }
        if values.len() > HR_REF_COUNT {
            return Err(RefDataError::InvalidCount {
                expected: HR_REF_COUNT,
                actual: values.len(),
            });
        }

        let mut hr_ref = self.hr_ref.lock().map_err(|e| {
            warn!("[RefDataManager] HR金标锁获取失败: {}", e);
            RefDataError::InvalidCount {
                expected: 0,
                actual: 0,
            }
        })?;

        let count = values.len() as i32;
        let mut padded_values = vec![0; HR_REF_COUNT];
        for (i, &v) in values.iter().enumerate() {
            padded_values[i] = v;
        }

        hr_ref.values = padded_values;
        hr_ref.count = count;
        hr_ref.last_update = Instant::now();

        info!(
            "[RefDataManager] HR金标已更新: values={:?}, count={}",
            values, count
        );
        Ok(())
    }

    pub fn set_hrv_ref(&self, values: &[i32]) -> Result<(), RefDataError> {
        if values.is_empty() {
            return Err(RefDataError::EmptyValues);
        }
        if values.len() > HRV_REF_COUNT {
            return Err(RefDataError::InvalidCount {
                expected: HRV_REF_COUNT,
                actual: values.len(),
            });
        }

        let mut hrv_ref = self.hrv_ref.lock().map_err(|e| {
            warn!("[RefDataManager] HRV金标锁获取失败: {}", e);
            RefDataError::InvalidCount {
                expected: 0,
                actual: 0,
            }
        })?;

        let count = values.len() as i32;
        let mut padded_values = vec![0; HRV_REF_COUNT];
        for (i, &v) in values.iter().enumerate() {
            padded_values[i] = v;
        }

        hrv_ref.values = padded_values;
        hrv_ref.count = count;
        hrv_ref.last_update = Instant::now();

        info!(
            "[RefDataManager] HRV金标已更新: values={:?}, count={}",
            values, count
        );
        Ok(())
    }

    pub fn set_spo2_ref(&self, values: &[i32]) -> Result<(), RefDataError> {
        if values.is_empty() {
            return Err(RefDataError::EmptyValues);
        }
        if values.len() > SPO2_REF_COUNT {
            return Err(RefDataError::InvalidCount {
                expected: SPO2_REF_COUNT,
                actual: values.len(),
            });
        }

        let mut spo2_ref = self.spo2_ref.lock().map_err(|e| {
            warn!("[RefDataManager] SpO2金标锁获取失败: {}", e);
            RefDataError::InvalidCount {
                expected: 0,
                actual: 0,
            }
        })?;

        let count = values.len() as i32;
        let mut padded_values = vec![0; SPO2_REF_COUNT];
        for (i, &v) in values.iter().enumerate() {
            padded_values[i] = v;
        }

        spo2_ref.values = padded_values;
        spo2_ref.count = count;

        info!(
            "[RefDataManager] SpO2金标已更新: values={:?}, count={}",
            values, count
        );
        Ok(())
    }

    pub fn get_ref_data(&self, current_frame_count: usize) -> Vec<i32> {
        let mut ref_data = vec![0; REF_DATA_COUNT];

        {
            let mut last_fc = self.last_frame_count.lock().unwrap_or_else(|e| {
                warn!("[RefDataManager] last_frame_count锁获取失败: {}", e);
                e.into_inner()
            });

            if current_frame_count == 0 && *last_fc > 0 {
                info!(
                    "[RefDataManager] frame_count重置为0，清除SpO2金标数据 (prev={})",
                    *last_fc
                );
                let mut spo2_ref = self.spo2_ref.lock().unwrap();
                spo2_ref.values = vec![0; SPO2_REF_COUNT];
                spo2_ref.count = 0;
            }
            *last_fc = current_frame_count;
        }

        {
            let hr_ref = self.hr_ref.lock().unwrap();
            let elapsed = hr_ref.last_update.elapsed();
            if elapsed < Duration::from_secs(REF_DATA_TIMEOUT_SECS) && hr_ref.count > 0 {
                ref_data[HR_REF_START..(HR_REF_COUNT + HR_REF_START)]
                    .copy_from_slice(&hr_ref.values[..HR_REF_COUNT]);
                ref_data[HR_REF_NUM_IDX] = hr_ref.count;
                debug!(
                    "[RefDataManager] HR金标有效: elapsed={:?}s, count={}",
                    elapsed.as_secs_f64(),
                    hr_ref.count
                );
            } else if hr_ref.count > 0 && elapsed >= Duration::from_secs(REF_DATA_TIMEOUT_SECS) {
                debug!(
                    "[RefDataManager] HR金标已过期: elapsed={:?}s",
                    elapsed.as_secs_f64()
                );
            }
        }

        {
            let hrv_ref = self.hrv_ref.lock().unwrap();
            let elapsed = hrv_ref.last_update.elapsed();
            if elapsed < Duration::from_secs(REF_DATA_TIMEOUT_SECS) && hrv_ref.count > 0 {
                ref_data[HRV_REF_START..(HRV_REF_COUNT + HRV_REF_START)]
                    .copy_from_slice(&hrv_ref.values[..HRV_REF_COUNT]);
                ref_data[HRV_REF_NUM_IDX] = hrv_ref.count;
                debug!(
                    "[RefDataManager] HRV金标有效: elapsed={:?}s, count={}",
                    elapsed.as_secs_f64(),
                    hrv_ref.count
                );
            } else if hrv_ref.count > 0 && elapsed >= Duration::from_secs(REF_DATA_TIMEOUT_SECS) {
                debug!(
                    "[RefDataManager] HRV金标已过期: elapsed={:?}s",
                    elapsed.as_secs_f64()
                );
            }
        }

        {
            let spo2_ref = self.spo2_ref.lock().unwrap();
            if spo2_ref.count > 0 {
                ref_data[SPO2_REF_START..(SPO2_REF_COUNT + SPO2_REF_START)]
                    .copy_from_slice(&spo2_ref.values[..SPO2_REF_COUNT]);
                ref_data[SPO2_REF_NUM_IDX] = spo2_ref.count;
                debug!("[RefDataManager] SpO2金标有效: count={}", spo2_ref.count);
            }
        }

        ref_data
    }

    pub fn clear_spo2_ref(&self) {
        let mut spo2_ref = self.spo2_ref.lock().unwrap();
        spo2_ref.values = vec![0; SPO2_REF_COUNT];
        spo2_ref.count = 0;
        info!("[RefDataManager] SpO2金标已手动清除");
    }

    pub fn clear_hr_ref(&self) {
        let mut hr_ref = self.hr_ref.lock().unwrap();
        hr_ref.values = vec![0; HR_REF_COUNT];
        hr_ref.count = 0;
        info!("[RefDataManager] HR金标已手动清除");
    }

    pub fn clear_hrv_ref(&self) {
        let mut hrv_ref = self.hrv_ref.lock().unwrap();
        hrv_ref.values = vec![0; HRV_REF_COUNT];
        hrv_ref.count = 0;
        info!("[RefDataManager] HRV金标已手动清除");
    }

    pub fn clear_all(&self) {
        self.clear_hr_ref();
        self.clear_hrv_ref();
        self.clear_spo2_ref();
        info!("[RefDataManager] 所有金标数据已清除");
    }

    pub fn get_hr_ref_status(&self) -> (Vec<i32>, i32, Duration) {
        let hr_ref = self.hr_ref.lock().unwrap();
        let elapsed = hr_ref.last_update.elapsed();
        (hr_ref.values.clone(), hr_ref.count, elapsed)
    }

    pub fn get_hrv_ref_status(&self) -> (Vec<i32>, i32, Duration) {
        let hrv_ref = self.hrv_ref.lock().unwrap();
        let elapsed = hrv_ref.last_update.elapsed();
        (hrv_ref.values.clone(), hrv_ref.count, elapsed)
    }

    pub fn get_spo2_ref_status(&self) -> (Vec<i32>, i32) {
        let spo2_ref = self.spo2_ref.lock().unwrap();
        (spo2_ref.values.clone(), spo2_ref.count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_set_hr_ref() {
        let manager = RefDataManager::new();
        let result = manager.set_hr_ref(&[72, 75, 70, 73]);
        assert!(result.is_ok());

        let (values, count, _) = manager.get_hr_ref_status();
        assert_eq!(values, vec![72, 75, 70, 73]);
        assert_eq!(count, 4);
    }

    #[test]
    fn test_set_hr_ref_partial() {
        let manager = RefDataManager::new();
        let result = manager.set_hr_ref(&[72, 75]);
        assert!(result.is_ok());

        let (values, count, _) = manager.get_hr_ref_status();
        assert_eq!(values, vec![72, 75, 0, 0]);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_set_hr_ref_empty() {
        let manager = RefDataManager::new();
        let result = manager.set_hr_ref(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_set_hr_ref_too_many() {
        let manager = RefDataManager::new();
        let result = manager.set_hr_ref(&[1, 2, 3, 4, 5]);
        assert!(result.is_err());
    }

    #[test]
    fn test_hr_ref_timeout() {
        let manager = RefDataManager::new();
        manager.set_hr_ref(&[72, 75]).unwrap();

        let ref_data = manager.get_ref_data(10);
        assert_eq!(ref_data[HR_REF_START], 72);
        assert_eq!(ref_data[HR_REF_START + 1], 75);
        assert_eq!(ref_data[HR_REF_NUM_IDX], 2);

        thread::sleep(Duration::from_millis(100));

        let ref_data = manager.get_ref_data(10);
        assert_eq!(ref_data[HR_REF_START], 72);
        assert_eq!(ref_data[HR_REF_NUM_IDX], 2);
    }

    #[test]
    fn test_set_hrv_ref() {
        let manager = RefDataManager::new();
        let result = manager.set_hrv_ref(&[800, 820, 790, 810]);
        assert!(result.is_ok());

        let (values, count, _) = manager.get_hrv_ref_status();
        assert_eq!(values, vec![800, 820, 790, 810]);
        assert_eq!(count, 4);
    }

    #[test]
    fn test_set_spo2_ref() {
        let manager = RefDataManager::new();
        let result = manager.set_spo2_ref(&[98, 97, 99, 96]);
        assert!(result.is_ok());

        let (values, count) = manager.get_spo2_ref_status();
        assert_eq!(values, vec![98, 97, 99, 96]);
        assert_eq!(count, 4);
    }

    #[test]
    fn test_spo2_ref_persistence() {
        let manager = RefDataManager::new();
        manager.set_spo2_ref(&[98, 97]).unwrap();

        let ref_data = manager.get_ref_data(100);
        assert_eq!(ref_data[SPO2_REF_START], 98);
        assert_eq!(ref_data[SPO2_REF_START + 1], 97);
        assert_eq!(ref_data[SPO2_REF_NUM_IDX], 2);

        let ref_data = manager.get_ref_data(200);
        assert_eq!(ref_data[SPO2_REF_START], 98);
        assert_eq!(ref_data[SPO2_REF_NUM_IDX], 2);
    }

    #[test]
    fn test_spo2_ref_clear_on_frame_count_reset() {
        let manager = RefDataManager::new();
        manager.set_spo2_ref(&[98, 97]).unwrap();

        let ref_data = manager.get_ref_data(100);
        assert_eq!(ref_data[SPO2_REF_NUM_IDX], 2);

        let ref_data = manager.get_ref_data(0);
        assert_eq!(ref_data[SPO2_REF_NUM_IDX], 0);
        assert_eq!(ref_data[SPO2_REF_START], 0);
    }

    #[test]
    fn test_get_ref_data_combined() {
        let manager = RefDataManager::new();
        manager.set_hr_ref(&[72, 75]).unwrap();
        manager.set_hrv_ref(&[800, 820]).unwrap();
        manager.set_spo2_ref(&[98]).unwrap();

        let ref_data = manager.get_ref_data(10);

        assert_eq!(ref_data[HR_REF_START], 72);
        assert_eq!(ref_data[HR_REF_START + 1], 75);
        assert_eq!(ref_data[HR_REF_NUM_IDX], 2);

        assert_eq!(ref_data[SPO2_REF_START], 98);
        assert_eq!(ref_data[SPO2_REF_NUM_IDX], 1);

        assert_eq!(ref_data[HRV_REF_START], 800);
        assert_eq!(ref_data[HRV_REF_START + 1], 820);
        assert_eq!(ref_data[HRV_REF_NUM_IDX], 2);
    }

    #[test]
    fn test_clear_all() {
        let manager = RefDataManager::new();
        manager.set_hr_ref(&[72]).unwrap();
        manager.set_hrv_ref(&[800]).unwrap();
        manager.set_spo2_ref(&[98]).unwrap();

        manager.clear_all();

        let (_hr_values, hr_count, _) = manager.get_hr_ref_status();
        assert_eq!(hr_count, 0);

        let (_hrv_values, hrv_count, _) = manager.get_hrv_ref_status();
        assert_eq!(hrv_count, 0);

        let (_spo2_values, spo2_count) = manager.get_spo2_ref_status();
        assert_eq!(spo2_count, 0);
    }

    #[test]
    fn test_time_based_expiration() {
        let manager = RefDataManager::new();

        manager.set_hr_ref(&[72]).unwrap();

        let ref_data = manager.get_ref_data(10);
        assert_eq!(ref_data[HR_REF_NUM_IDX], 1);

        thread::sleep(Duration::from_millis(500));

        let ref_data = manager.get_ref_data(10);
        assert_eq!(ref_data[HR_REF_NUM_IDX], 1);
    }
}
