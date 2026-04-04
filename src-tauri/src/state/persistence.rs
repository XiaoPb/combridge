use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::app_state::AppState;
use super::types::Preferences;

const STATE_FILE_NAME: &str = "app_state.json";
const PREFERENCES_FILE_NAME: &str = "preferences.yaml";
const CONFIG_DIR_NAME: &str = "config";

pub struct StatePersistence {
    state_path: PathBuf,
    preferences_path: PathBuf,
    preferences: RwLock<Option<Preferences>>,
}

impl StatePersistence {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let state_path = app_data_dir.join(STATE_FILE_NAME);
        
        let preferences_path = std::env::current_dir()
            .unwrap_or_else(|_| app_data_dir.clone())
            .join(CONFIG_DIR_NAME)
            .join(PREFERENCES_FILE_NAME);
        
        debug!("状态持久化路径: {:?}", state_path);
        debug!("偏好设置持久化路径: {:?}", preferences_path);
        Self {
            state_path,
            preferences_path,
            preferences: RwLock::new(None),
        }
    }

    pub async fn save(&self, state: &AppState) -> Result<(), String> {
        let json = serde_json::to_string_pretty(state)
            .map_err(|e| format!("序列化状态失败: {}", e))?;
        
        if let Some(parent) = self.state_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }
        
        tokio::fs::write(&self.state_path, json)
            .await
            .map_err(|e| format!("写入状态文件失败: {}", e))?;
        
        debug!("状态已保存到: {:?}", self.state_path);
        Ok(())
    }

    pub async fn load(&self) -> Result<AppState, String> {
        if !self.state_path.exists() {
            info!("状态文件不存在，使用默认状态");
            return Ok(AppState::default());
        }
        
        let content = tokio::fs::read_to_string(&self.state_path)
            .await
            .map_err(|e| format!("读取状态文件失败: {}", e))?;
        
        let state: AppState = serde_json::from_str(&content)
            .map_err(|e| format!("解析状态文件失败: {}", e))?;
        
        info!("状态已从 {:?} 加载", self.state_path);
        Ok(state)
    }

    pub async fn save_if_changed(
        &self,
        state: &AppState,
        last_saved: &mut Option<String>,
    ) -> bool {
        let current = match serde_json::to_string(state) {
            Ok(s) => s,
            Err(e) => {
                error!("序列化状态失败: {}", e);
                return false;
            }
        };
        
        let should_save = match last_saved {
            Some(last) => last != &current,
            None => true,
        };
        
        if should_save {
            match self.save(state).await {
                Ok(()) => {
                    *last_saved = Some(current);
                    true
                }
                Err(e) => {
                    error!("保存状态失败: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    pub async fn save_preferences(&self, prefs: &Preferences) -> Result<(), String> {
        let yaml = serde_yaml::to_string(prefs)
            .map_err(|e| format!("序列化偏好设置失败: {}", e))?;
        
        if let Some(parent) = self.preferences_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("创建目录失败: {}", e))?;
        }
        
        tokio::fs::write(&self.preferences_path, yaml)
            .await
            .map_err(|e| format!("写入偏好设置文件失败: {}", e))?;
        
        let mut prefs_guard = self.preferences.write().await;
        *prefs_guard = Some(prefs.clone());
        
        debug!("偏好设置已保存到: {:?}", self.preferences_path);
        Ok(())
    }

    pub async fn load_preferences(&self) -> Result<Preferences, String> {
        if !self.preferences_path.exists() {
            info!("偏好设置文件不存在，创建默认配置文件");
            let default_prefs = Preferences::default();
            self.save_preferences(&default_prefs).await?;
            return Ok(default_prefs);
        }
        
        let content = tokio::fs::read_to_string(&self.preferences_path)
            .await
            .map_err(|e| format!("读取偏好设置文件失败: {}", e))?;
        
        let prefs: Preferences = match serde_yaml::from_str(&content) {
            Ok(p) => p,
            Err(e) => {
                warn!("解析偏好设置文件失败: {}, 使用默认值", e);
                return Ok(Preferences::default());
            }
        };
        
        let mut prefs_guard = self.preferences.write().await;
        *prefs_guard = Some(prefs.clone());
        
        info!("偏好设置已从 {:?} 加载", self.preferences_path);
        Ok(prefs)
    }

    pub async fn get_cached_preferences(&self) -> Option<Preferences> {
        let prefs_guard = self.preferences.read().await;
        prefs_guard.clone()
    }
}

pub type StatePersistenceRef = Arc<RwLock<StatePersistence>>;

pub fn create_state_persistence(app_data_dir: PathBuf) -> StatePersistenceRef {
    Arc::new(RwLock::new(StatePersistence::new(app_data_dir)))
}
