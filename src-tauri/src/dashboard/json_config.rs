use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetConfig {
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub units: String,
    #[serde(default)]
    pub widget: String,
    #[serde(default)]
    pub graph: bool,
    #[serde(default)]
    pub min: f64,
    #[serde(default = "default_max")]
    pub max: f64,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub led: bool,
    #[serde(default = "default_led_high")]
    pub led_high: f64,
    #[serde(default)]
    pub log: bool,
    #[serde(default)]
    pub alarm: f64,
    #[serde(default)]
    pub fft: bool,
    #[serde(default = "default_fft_samples")]
    pub fft_samples: usize,
    #[serde(default = "default_fft_sampling_rate")]
    pub fft_sampling_rate: f64,
    #[serde(default = "default_value")]
    pub value: String,
}

fn default_max() -> f64 {
    100.0
}
fn default_led_high() -> f64 {
    1.0
}
fn default_fft_samples() -> usize {
    1024
}
fn default_fft_sampling_rate() -> f64 {
    100.0
}
fn default_value() -> String {
    "--.--".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetGroup {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub widget: String,
    #[serde(default)]
    pub datasets: Vec<DatasetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardJsonConfig {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub decoder: i32,
    #[serde(default)]
    pub frame_detection: i32,
    #[serde(default)]
    pub frame_start: String,
    #[serde(default)]
    pub frame_end: String,
    #[serde(default)]
    pub frame_parser: String,
    #[serde(default)]
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
        info!("JsonConfigManager initializing, json_dir: {:?}", json_dir);

        if !json_dir.exists() {
            match fs::create_dir_all(&json_dir) {
                Ok(_) => info!("Created json directory: {:?}", json_dir),
                Err(e) => {
                    info!(
                        "Failed to create json directory: {:?}, error: {}",
                        json_dir, e
                    );
                }
            }
        } else {
            info!("Json directory already exists: {:?}", json_dir);
        }
        Self { json_dir }
    }

    pub fn get_json_files(&self) -> Result<Vec<String>, String> {
        info!("Getting json files from: {:?}", self.json_dir);

        if !self.json_dir.exists() {
            info!("Json directory does not exist, creating it");
            fs::create_dir_all(&self.json_dir)
                .map_err(|e| format!("Failed to create json directory: {}", e))?;
        }

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
        info!("Found {} json files", files.len());
        Ok(files)
    }

    pub fn save_json_file(
        &self,
        file_name: &str,
        config: &DashboardJsonConfig,
    ) -> Result<(), String> {
        info!("Saving json file: {} to {:?}", file_name, self.json_dir);

        if !self.json_dir.exists() {
            info!("Json directory does not exist, creating it");
            fs::create_dir_all(&self.json_dir)
                .map_err(|e| format!("Failed to create json directory: {}", e))?;
        }

        let path = self.json_dir.join(file_name);
        info!("Full path: {:?}", path);

        let content = serde_json::to_string_pretty(config).map_err(|e| {
            let err = format!("Failed to serialize config: {}", e);
            info!("{}", err);
            err
        })?;

        info!("Serialized config, content length: {} bytes", content.len());

        fs::write(&path, content).map_err(|e| {
            let err = format!("Failed to write json file {:?}: {}", path, e);
            info!("{}", err);
            err
        })?;

        info!("Successfully saved json config to: {:?}", path);
        Ok(())
    }

    pub fn delete_json_file(&self, file_name: &str) -> Result<(), String> {
        let path = self.json_dir.join(file_name);

        if !path.exists() {
            return Err(format!("File not found: {}", file_name));
        }

        fs::remove_file(&path).map_err(|e| format!("Failed to delete json file: {}", e))?;

        info!("Deleted json config: {:?}", path);
        Ok(())
    }

    pub fn load_json_file(&self, file_name: &str) -> Result<DashboardJsonConfig, String> {
        let path = self.json_dir.join(file_name);

        if !path.exists() {
            return Err(format!("File not found: {}", file_name));
        }

        let content =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read json file: {}", e))?;

        let config: DashboardJsonConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse json config: {}", e))?;

        Ok(config)
    }
}
