use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub version: String,
    pub log_level: String,
    pub log_file: PathBuf,
    pub max_log_size: u64,
    pub max_log_files: usize,
    pub data_queue_size: usize,
    pub event_bus_capacity: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "ComBridge".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            log_level: "info".to_string(),
            log_file: PathBuf::from("logs/combridge.log"),
            max_log_size: 10 * 1024 * 1024,
            max_log_files: 5,
            data_queue_size: 1024,
            event_bus_capacity: 256,
        }
    }
}

pub struct ConfigService {
    config: Arc<RwLock<AppConfig>>,
    config_path: PathBuf,
}

impl ConfigService {
    pub fn new() -> Self {
        Self::with_path("config/app_config.json")
    }

    pub fn with_path<P: AsRef<Path>>(path: P) -> Self {
        let config_path = path.as_ref().to_path_buf();
        let config = Self::load_from_file(&config_path).unwrap_or_default();
        Self {
            config: Arc::new(RwLock::new(config)),
            config_path,
        }
    }

    fn load_from_file(path: &Path) -> Result<AppConfig, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Err("Config file not found".into());
        }
        let content = std::fs::read_to_string(path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub async fn get(&self) -> AppConfig {
        self.config.read().await.clone()
    }

    pub async fn set(&self, config: AppConfig) {
        *self.config.write().await = config;
    }

    pub async fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut config = self.config.write().await;
        f(&mut config);
    }

    pub async fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.save_to(&self.config_path).await
    }

    pub async fn save_to<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let config = self.config.read().await;
        let content = serde_json::to_string_pretty(&*config)?;
        tokio::fs::write(path, content).await?;
        Ok(())
    }

    pub async fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config = Self::load_from_file(&self.config_path)?;
        *self.config.write().await = config;
        Ok(())
    }

    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}
