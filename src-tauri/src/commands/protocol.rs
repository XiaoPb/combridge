use crate::error::{ComBridgeError, ErrorResponse};
use crate::protocol::{PluginInfo, PluginManager};
use std::path::PathBuf;
use std::sync::Arc;

#[tauri::command]
pub async fn load_protocol(
    plugin_id: String,
    path: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<PluginInfo, ErrorResponse> {
    let path_buf = PathBuf::from(&path);

    if !path_buf.exists() {
        return Err(ComBridgeError::protocol(format!("Script file not found: {}", path)).to_error_response());
    }

    manager
        .load_plugin(&plugin_id, path_buf)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn unload_protocol(
    plugin_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .unload_plugin(&plugin_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn enable_protocol(
    plugin_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .enable_plugin(&plugin_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn disable_protocol(
    plugin_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .disable_plugin(&plugin_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn bind_protocol(
    plugin_id: String,
    device_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .bind_protocol(&plugin_id, &device_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn unbind_protocol(
    plugin_id: String,
    device_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<(), ErrorResponse> {
    manager
        .unbind_protocol(&plugin_id, &device_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn list_protocols(
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<Vec<PluginInfo>, ErrorResponse> {
    manager.list_protocols().map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn get_protocol(
    plugin_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<PluginInfo, ErrorResponse> {
    manager
        .get_plugin(&plugin_id)
        .map_err(|e| e.to_error_response())
}

#[tauri::command]
pub async fn get_bound_protocols(
    device_id: String,
    manager: tauri::State<'_, Arc<PluginManager>>,
) -> Result<Vec<PluginInfo>, ErrorResponse> {
    manager
        .get_bound_plugins(&device_id)
        .map_err(|e| e.to_error_response())
}
