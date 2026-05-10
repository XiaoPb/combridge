use tauri::{AppHandle, State};
use tracing::{debug, error, info};

use crate::error::Result;
use crate::state::{
    Action, ActionDispatcherRef, ActionResult, AppStateRef, Device, StatePersistenceRef,
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
pub async fn get_state(state: State<'_, AppStateRef>) -> Result<crate::state::AppState> {
    debug!("获取完整状态");
    let state = state.inner().read().await.clone();
    Ok(state)
}

#[tauri::command]
pub async fn get_channel_data(
    state: State<'_, AppStateRef>,
    device_id: String,
    channel_id: String,
    limit: Option<usize>,
) -> Result<serde_json::Value> {
    debug!("获取通道 {} 数据", channel_id);

    let state = state.inner().read().await;

    match state.get_channel(&device_id, &channel_id) {
        Some(channel) => {
            let entries: Vec<_> = if let Some(limit) = limit {
                channel
                    .buffer
                    .entries
                    .iter()
                    .rev()
                    .take(limit)
                    .rev()
                    .cloned()
                    .collect()
            } else {
                channel.buffer.entries.iter().cloned().collect()
            };

            Ok(serde_json::json!({
                "deviceId": device_id,
                "channelId": channel_id,
                "entries": entries,
                "totalBytes": channel.buffer.total_bytes,
                "subscribed": channel.subscribed,
            }))
        }
        None => {
            error!("通道不存在: {}/{}", device_id, channel_id);
            Ok(serde_json::json!({
                "error": format!("通道不存在: {}/{}", device_id, channel_id)
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
        e
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
        e
    })?;

    debug!("状态保存完成");
    Ok(())
}

#[tauri::command]
pub async fn get_connected_devices(state: State<'_, AppStateRef>) -> Result<Vec<Device>> {
    debug!("获取已连接的设备");
    let state = state.inner().read().await;
    Ok(state.get_connected_devices().into_iter().cloned().collect())
}

#[tauri::command]
pub async fn get_window_state(state: State<'_, AppStateRef>) -> Result<crate::state::WindowState> {
    debug!("获取窗口状态");
    let state = state.inner().read().await;
    Ok(state.window_state.clone())
}
