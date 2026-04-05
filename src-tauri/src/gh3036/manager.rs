use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tauri::{AppHandle, Emitter};
use tracing::{debug, error, info};

use crate::device::DeviceManager;
use super::csv_writer::CsvWriter;
use super::types::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    Serial,
    Ble,
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

pub struct Gh3036Manager {
    device_manager: Arc<DeviceManager>,
    app_handle: RwLock<Option<AppHandle>>,
    tx_channel: RwLock<Option<ChannelConfig>>,
    rx_channel: RwLock<Option<ChannelConfig>>,
    csv_config: RwLock<CsvConfig>,
    csv_writers: RwLock<HashMap<i32, CsvWriter>>,
    initialized: RwLock<bool>,
}

unsafe impl Send for Gh3036Manager {}
unsafe impl Sync for Gh3036Manager {}

impl Gh3036Manager {
    pub fn new(device_manager: Arc<DeviceManager>) -> Self {
        Self {
            device_manager,
            app_handle: RwLock::new(None),
            tx_channel: RwLock::new(None),
            rx_channel: RwLock::new(None),
            csv_config: RwLock::new(CsvConfig::default()),
            csv_writers: RwLock::new(HashMap::new()),
            initialized: RwLock::new(false),
        }
    }

    pub async fn set_app_handle(&self, handle: AppHandle) {
        let mut app_handle = self.app_handle.write().await;
        *app_handle = Some(handle);
    }

    pub async fn is_initialized(&self) -> bool {
        *self.initialized.read().await
    }

    pub async fn initialize(&self) -> Result<(), String> {
        let mut initialized = self.initialized.write().await;
        *initialized = true;
        info!("GH3036 协议管理器初始化成功");
        Ok(())
    }

    pub async fn configure_tx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        let mut tx_channel = self.tx_channel.write().await;
        *tx_channel = Some(config.clone());
        info!("GH3036 发送通道配置成功: {:?}", config);
        Ok(())
    }

    pub async fn configure_rx_channel(&self, config: ChannelConfig) -> Result<(), String> {
        let mut rx_channel = self.rx_channel.write().await;
        *rx_channel = Some(config.clone());
        info!("GH3036 接收通道配置成功: {:?}", config);
        Ok(())
    }

    pub async fn get_tx_channel(&self) -> Option<ChannelConfig> {
        self.tx_channel.read().await.clone()
    }

    pub async fn get_rx_channel(&self) -> Option<ChannelConfig> {
        self.rx_channel.read().await.clone()
    }

    pub async fn set_csv_config(&self, config: CsvConfig) -> Result<(), String> {
        let mut csv_config = self.csv_config.write().await;
        *csv_config = config;
        info!("GH3036 CSV 配置更新成功");
        Ok(())
    }

    pub async fn get_csv_config(&self) -> CsvConfig {
        self.csv_config.read().await.clone()
    }

    pub async fn send_data(&self, data: &[u8]) -> Result<(), String> {
        let tx_channel = self.tx_channel.read().await;
        let channel = tx_channel.as_ref().ok_or("发送通道未配置")?;

        match channel.channel_type {
            ChannelType::Serial => {
                let port_name = &channel.device_id;
                self.device_manager
                    .route_data(&format!("serial-{}", port_name), data)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            ChannelType::Ble => {
                let _char_uuid = channel.characteristic_uuid.as_ref()
                    .ok_or("蓝牙发送通道缺少特征UUID")?;
                self.device_manager
                    .route_data(&format!("ble-{}", channel.device_id), data)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        debug!("GH3036 发送数据: {} bytes", data.len());
        Ok(())
    }

    pub async fn on_data_received(&self, device_id: &str, data: &[u8]) {
        debug!("GH3036 接收数据: {} bytes from {}", data.len(), device_id);
        
        if let Some(frame_data) = self.parse_frame_data(data).await {
            self.emit_frame_data(&frame_data).await;
            self.save_to_csv(&frame_data).await;
        }
    }

    async fn parse_frame_data(&self, data: &[u8]) -> Option<Gh3036FrameData> {
        if data.len() < 4 || data[0] != 0xAA || data[1] != 0x11 {
            return None;
        }

        let function_id = if data.len() > 2 { data[2] as i32 } else { 0 };
        let frame_id = if data.len() > 3 { data[3] as i32 } else { 0 };
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let func_name = GhFuncFixIdx::from_i32(function_id)
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| format!("UNKNOWN_{}", function_id));

        Some(Gh3036FrameData {
            function_id,
            function_name: func_name,
            frame_id,
            timestamp,
            gs_data: vec![0, 0, 0],
            rawdata: data.iter().map(|&b| b as i32).collect(),
            flags: vec![],
            algo_data: vec![],
            agc_info: vec![],
            phy_value: vec![],
        })
    }

    async fn emit_frame_data(&self, frame_data: &Gh3036FrameData) {
        let app_handle = self.app_handle.read().await;
        if let Some(handle) = app_handle.as_ref() {
            if let Err(e) = handle.emit("gh3036-frame", frame_data) {
                error!("发送帧数据事件失败: {}", e);
            }
        }
    }

    async fn save_to_csv(&self, frame_data: &Gh3036FrameData) {
        let csv_config = self.csv_config.read().await;
        if !csv_config.enabled {
            return;
        }

        let mut writers = self.csv_writers.write().await;
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

    pub fn get_rpc_commands() -> Vec<RpcCommand> {
        get_rpc_commands()
    }
}
