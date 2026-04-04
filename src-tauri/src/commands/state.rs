use tauri::{AppHandle, State};
use tracing::{debug, error, info};

use crate::error::Result;
use crate::state::{
    Action, ActionDispatcherRef, ActionResult, AppStateRef, StatePersistenceRef,
};

#[tauri::command]
pub async fn dispatch_action(
    dispatcher: State<'_, ActionDispatcherRef>,
    app: AppHandle,
    action: Action,
) -> Result<ActionResult> {
    info!("收到 Action: {}", action);
    let dispatcher = dispatcher.inner();
    let result = dispatcher.dispatch(action, &app).await;
    Ok(result)
}

#[tauri::command]
pub async fn get_state(
    state: State<'_, AppStateRef>,
) -> Result<crate::state::AppState> {
    debug!("获取完整状态");
    let state = state.inner().read().await.clone();
    Ok(state)
}

#[tauri::command]
pub async fn get_channel_data(
    state: State<'_, AppStateRef>,
    channel_id: String,
    direction: Option<String>,
    limit: Option<usize>,
) -> Result<serde_json::Value> {
    debug!("获取通道 {} 数据", channel_id);
    
    let state = state.inner().read().await;
    
    match state.get_channel(&channel_id) {
        Some(channel) => {
            let buffer = match direction.as_deref() {
                Some("tx") => &channel.tx_buffer,
                Some("rx") => &channel.rx_buffer,
                _ => &channel.rx_buffer,
            };
            
            let entries = if let Some(limit) = limit {
                buffer.entries.iter().rev().take(limit).rev().cloned().collect::<Vec<_>>()
            } else {
                buffer.entries.clone()
            };
            
            Ok(serde_json::json!({
                "channelId": channel_id,
                "direction": direction,
                "entries": entries,
                "totalBytes": buffer.total_bytes,
            }))
        }
        None => {
            error!("通道不存在: {}", channel_id);
            Ok(serde_json::json!({
                "error": format!("通道不存在: {}", channel_id)
            }))
        }
    }
}

#[tauri::command]
pub async fn restore_state(
    state: State<'_, AppStateRef>,
    persistence: State<'_, StatePersistenceRef>,
) -> Result<()> {
    info!("从持久化存储恢复状态");
    
    let persistence = persistence.inner().read().await;
    let loaded_state = persistence.load().await.map_err(|e| {
        error!("加载状态失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    let mut current_state = state.inner().write().await;
    *current_state = loaded_state;
    
    debug!("状态恢复完成");
    Ok(())
}

#[tauri::command]
pub async fn save_state(
    state: State<'_, AppStateRef>,
    persistence: State<'_, StatePersistenceRef>,
) -> Result<()> {
    info!("手动保存状态");
    
    let current_state = state.inner().read().await.clone();
    let persistence = persistence.inner().read().await;
    
    persistence.save(&current_state).await.map_err(|e| {
        error!("保存状态失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    debug!("状态保存完成");
    Ok(())
}

#[tauri::command]
pub async fn get_connected_channels(
    state: State<'_, AppStateRef>,
) -> Result<Vec<crate::state::DeviceChannel>> {
    debug!("获取已连接的通道");
    let state = state.inner().read().await;
    Ok(state.get_connected_channels().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn get_window_state(
    state: State<'_, AppStateRef>,
) -> Result<crate::state::WindowState> {
    debug!("获取窗口状态");
    let state = state.inner().read().await;
    Ok(state.window_state.clone())
}
