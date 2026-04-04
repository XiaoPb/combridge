use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info};

use super::app_state::AppState;

const STATE_FILE_NAME: &str = "app_state.json";

pub struct StatePersistence {
    state_path: PathBuf,
}

impl StatePersistence {
    pub fn new(app_data_dir: PathBuf) -> Self {
        let state_path = app_data_dir.join(STATE_FILE_NAME);
        debug!("状态持久化路径: {:?}", state_path);
        Self { state_path }
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
}

pub type StatePersistenceRef = Arc<RwLock<StatePersistence>>;

pub fn create_state_persistence(app_data_dir: PathBuf) -> StatePersistenceRef {
    Arc::new(RwLock::new(StatePersistence::new(app_data_dir)))
}
