use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaveformBufferConfig {
    pub capacity: usize,
    pub column_names: Vec<String>,
}

impl Default for WaveformBufferConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            column_names: vec![
                "CH0".to_string(),
                "CH1".to_string(),
                "CH2".to_string(),
                "CH3".to_string(),
                "CH4".to_string(),
            ],
        }
    }
}

pub struct WaveformBuffer {
    config: WaveformBufferConfig,
    data: RwLock<VecDeque<Vec<f64>>>,
    timestamp: RwLock<u64>,
}

impl WaveformBuffer {
    pub fn new(config: WaveformBufferConfig) -> Self {
        Self {
            config,
            data: RwLock::new(VecDeque::new()),
            timestamp: RwLock::new(0),
        }
    }

    pub fn append_row(&self, values: Vec<f64>) {
        let mut data = self.data.write();
        if data.len() >= self.config.capacity {
            data.pop_front();
        }
        data.push_back(values);

        let mut ts = self.timestamp.write();
        *ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
    }

    pub fn append_row_from_strings(
        &self,
        values: Vec<String>,
    ) -> Result<(), crate::error::ComBridgeError> {
        let parsed: Result<Vec<f64>, _> = values.iter().map(|s| s.trim().parse::<f64>()).collect();

        match parsed {
            Ok(nums) => {
                self.append_row(nums);
                Ok(())
            }
            Err(_) => Err(crate::error::ComBridgeError::parse(format!(
                "Failed to parse values: {:?}",
                values
            ))),
        }
    }

    pub fn read_last_n_rows(&self, n: usize) -> Vec<Vec<f64>> {
        let data = self.data.read();
        let len = data.len();
        if n >= len {
            data.iter().cloned().collect()
        } else {
            data.iter().skip(len - n).cloned().collect()
        }
    }

    pub fn get_status(&self) -> super::WaveformStatus {
        let data = self.data.read();
        super::WaveformStatus {
            buffer_id: String::new(),
            row_count: data.len(),
            column_count: self.config.column_names.len(),
            column_names: self.config.column_names.clone(),
            capacity: self.config.capacity,
            parser_type: None,
        }
    }

    pub fn clear(&self) {
        let mut data = self.data.write();
        data.clear();
    }

    pub fn get_column_names(&self) -> Vec<String> {
        self.config.column_names.clone()
    }

    pub fn set_column_names(&mut self, names: Vec<String>) {
        self.config.column_names = names;
    }
}

impl Default for WaveformBuffer {
    fn default() -> Self {
        Self::new(WaveformBufferConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_append_and_read() {
        let buffer = WaveformBuffer::new(WaveformBufferConfig {
            capacity: 10,
            column_names: vec!["CH0".to_string(), "CH1".to_string()],
        });

        buffer.append_row(vec![1.0, 2.0]);
        buffer.append_row(vec![3.0, 4.0]);

        let data = buffer.read_last_n_rows(10);
        assert_eq!(data.len(), 2);
        assert_eq!(data[0], vec![1.0, 2.0]);
        assert_eq!(data[1], vec![3.0, 4.0]);
    }

    #[test]
    fn test_buffer_overflow() {
        let buffer = WaveformBuffer::new(WaveformBufferConfig {
            capacity: 3,
            column_names: vec!["CH0".to_string()],
        });

        buffer.append_row(vec![1.0]);
        buffer.append_row(vec![2.0]);
        buffer.append_row(vec![3.0]);
        buffer.append_row(vec![4.0]);

        let data = buffer.read_last_n_rows(10);
        assert_eq!(data.len(), 3);
        assert_eq!(data[0], vec![2.0]);
        assert_eq!(data[2], vec![4.0]);
    }

    #[test]
    fn test_buffer_clear() {
        let buffer = WaveformBuffer::new(WaveformBufferConfig {
            capacity: 10,
            column_names: vec!["CH0".to_string()],
        });

        buffer.append_row(vec![1.0]);
        buffer.clear();

        let data = buffer.read_last_n_rows(10);
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_buffer_from_strings() {
        let buffer = WaveformBuffer::new(WaveformBufferConfig {
            capacity: 10,
            column_names: vec!["CH0".to_string(), "CH1".to_string()],
        });

        buffer
            .append_row_from_strings(vec!["1.5".to_string(), "2.5".to_string()])
            .unwrap();

        let data = buffer.read_last_n_rows(10);
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], vec![1.5, 2.5]);
    }
}
