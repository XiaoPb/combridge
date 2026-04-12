use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetConfig {
    pub index: usize,
    pub title: String,
    pub units: String,
    pub widget: String,
    pub graph: bool,
    pub min: f64,
    pub max: f64,
    #[serde(default)]
    pub color: Option<String>,
    pub led: bool,
    pub led_high: f64,
    pub log: bool,
    pub alarm: f64,
    pub fft: bool,
    pub fft_samples: usize,
    pub fft_sampling_rate: f64,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetGroup {
    pub title: String,
    pub widget: String,
    pub datasets: Vec<DatasetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardJsonConfig {
    pub title: String,
    pub decoder: i32,
    pub frame_detection: i32,
    pub frame_start: String,
    pub frame_end: String,
    pub frame_parser: String,
    pub groups: Vec<WidgetGroup>,
    #[serde(default)]
    pub map_tiler_api_key: Option<String>,
    #[serde(default)]
    pub thunderforest_api_key: Option<String>,
}

pub struct JsonConfigManager {
    json_dir: PathBuf,
}

impl JsonConfigManager {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let json_dir = app_data_dir.join("plugins").join("json");
        if !json_dir.exists() {
            fs::create_dir_all(&json_dir).ok();
        }
        Self { json_dir }
    }

    pub fn get_json_files(&self) -> Result<Vec<String>, String> {
        let mut files = Vec::new();
        let entries = fs::read_dir(&self.json_dir)
            .map_err(|e| format!("Failed to read json directory: {}", e))?;
        
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().map(|e| e == "json").unwrap_or(false) {
                    if let Some(name) = path.file_name() {
                        files.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }
        
        files.sort();
        Ok(files)
    }

    pub fn save_json_file(&self, file_name: &str, config: &DashboardJsonConfig) -> Result<(), String> {
        let path = self.json_dir.join(file_name);
        let content = serde_json::to_string_pretty(config)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(&path, content)
            .map_err(|e| format!("Failed to write json file: {}", e))?;
        
        info!("Saved json config to: {:?}", path);
        Ok(())
    }

    pub fn delete_json_file(&self, file_name: &str) -> Result<(), String> {
        let path = self.json_dir.join(file_name);
        
        if !path.exists() {
            return Err(format!("File not found: {}", file_name));
        }
        
        fs::remove_file(&path)
            .map_err(|e| format!("Failed to delete json file: {}", e))?;
        
        info!("Deleted json config: {:?}", path);
        Ok(())
    }

    pub fn load_json_file(&self, file_name: &str) -> Result<DashboardJsonConfig, String> {
        let path = self.json_dir.join(file_name);
        
        if !path.exists() {
            return Err(format!("File not found: {}", file_name));
        }
        
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read json file: {}", e))?;
        
        let config: DashboardJsonConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse json config: {}", e))?;
        
        Ok(config)
    }
}
