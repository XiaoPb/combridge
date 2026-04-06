use crate::error::{ComBridgeError, ErrorResponse};
use crate::waveform::{
    ParserConfig, ParserManager, WaveformBuffer, WaveformBufferConfig, WaveformData, WaveformStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

pub struct WaveformManager {
    buffers: RwLock<HashMap<String, Arc<WaveformBuffer>>>,
    parser_manager: Arc<ParserManager>,
}

impl WaveformManager {
    pub fn new() -> Self {
        Self {
            buffers: RwLock::new(HashMap::new()),
            parser_manager: Arc::new(ParserManager::new()),
        }
    }

    pub fn create_buffer(&self, buffer_id: &str, config: WaveformBufferConfig) -> Result<(), ComBridgeError> {
        let mut buffers = self.buffers.write();
        if buffers.contains_key(buffer_id) {
            return Err(ComBridgeError::parse(format!("Buffer '{}' already exists", buffer_id)));
        }
        buffers.insert(buffer_id.to_string(), Arc::new(WaveformBuffer::new(config)));
        Ok(())
    }

    pub fn get_buffer(&self, buffer_id: &str) -> Result<Arc<WaveformBuffer>, ComBridgeError> {
        let buffers = self.buffers.read();
        buffers.get(buffer_id).cloned().ok_or_else(|| {
            ComBridgeError::parse(format!("Buffer '{}' not found", buffer_id))
        })
    }

    pub fn remove_buffer(&self, buffer_id: &str) {
        let mut buffers = self.buffers.write();
        buffers.remove(buffer_id);
    }

    pub fn configure_parser(&self, buffer_id: &str, config: ParserConfig) -> Result<(), ComBridgeError> {
        self.parser_manager.create_parser(buffer_id, config)
    }

    pub fn parse_and_store(&self, buffer_id: &str, data: &str) -> Result<(), ComBridgeError> {
        let values = self.parser_manager.parse(buffer_id, data)?;
        let buffer = self.get_buffer(buffer_id)?;
        buffer.append_row_from_strings(values)
    }

    pub fn read_data(&self, buffer_id: &str, rows: usize) -> Result<WaveformData, ComBridgeError> {
        let buffer = self.get_buffer(buffer_id)?;
        let columns = buffer.get_column_names();
        let rows_data = buffer.read_last_n_rows(rows);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(WaveformData {
            columns,
            rows: rows_data,
            timestamp,
        })
    }

    pub fn get_status(&self, buffer_id: &str) -> Result<WaveformStatus, ComBridgeError> {
        let buffer = self.get_buffer(buffer_id)?;
        let mut status = buffer.get_status();
        status.buffer_id = buffer_id.to_string();
        
        if let Some(config) = self.parser_manager.get_parser_config(buffer_id) {
            status.parser_type = Some(config.parser_type);
        }
        
        Ok(status)
    }

    pub fn clear_buffer(&self, buffer_id: &str) -> Result<(), ComBridgeError> {
        let buffer = self.get_buffer(buffer_id)?;
        buffer.clear();
        Ok(())
    }

    pub fn list_buffers(&self) -> Vec<String> {
        let buffers = self.buffers.read();
        buffers.keys().cloned().collect()
    }
}

impl Default for WaveformManager {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
pub async fn waveform_create_buffer(
    buffer_id: String,
    config: WaveformBufferConfig,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .create_buffer(&buffer_id, config)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_remove_buffer(
    buffer_id: String,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<(), ErrorResponse> {
    manager.remove_buffer(&buffer_id);
    Ok(())
}

#[tauri::command]
pub async fn waveform_configure_parser(
    buffer_id: String,
    config: ParserConfig,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .configure_parser(&buffer_id, config)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_parse_and_store(
    buffer_id: String,
    data: String,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .parse_and_store(&buffer_id, &data)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_read_data(
    buffer_id: String,
    rows: usize,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<WaveformData, ErrorResponse> {
    manager
        .read_data(&buffer_id, rows)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_get_status(
    buffer_id: String,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<WaveformStatus, ErrorResponse> {
    manager
        .get_status(&buffer_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_clear_buffer(
    buffer_id: String,
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .clear_buffer(&buffer_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn waveform_list_buffers(
    manager: tauri::State<'_, Arc<WaveformManager>>,
) -> Result<Vec<String>, ErrorResponse> {
    Ok(manager.list_buffers())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveform_manager() {
        let manager = WaveformManager::new();
        let config = WaveformBufferConfig::default();

        manager.create_buffer("test", config).unwrap();
        
        let parser_config = ParserConfig::default();
        manager.configure_parser("test", parser_config).unwrap();
        
        manager.parse_and_store("test", "1,2,3,4,5").unwrap();
        
        let data = manager.read_data("test", 10).unwrap();
        assert_eq!(data.rows.len(), 1);
        assert_eq!(data.rows[0], vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    }
}
