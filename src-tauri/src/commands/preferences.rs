use tauri::State;
use tracing::{debug, error, info};

use crate::error::Result;
use crate::state::{StatePersistenceRef, Preferences};

#[tauri::command]
pub async fn get_preferences(
    persistence: State<'_, StatePersistenceRef>,
) -> Result<Preferences> {
    debug!("获取偏好设置");
    
    let persistence = persistence.inner().read().await;
    let prefs = persistence.load_preferences().await.map_err(|e| {
        error!("加载偏好设置失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    Ok(prefs)
}

#[tauri::command]
pub async fn save_preferences(
    persistence: State<'_, StatePersistenceRef>,
    prefs: Preferences,
) -> Result<()> {
    info!("保存偏好设置");
    
    let persistence = persistence.inner().read().await;
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    debug!("偏好设置保存完成");
    Ok(())
}

#[tauri::command]
pub async fn update_serial_preferences(
    persistence: State<'_, StatePersistenceRef>,
    display_format: String,
    display_mode: String,
    send_format: String,
    append_newline: bool,
    newline_type: String,
    auto_scroll: bool,
) -> Result<()> {
    debug!("更新串口偏好设置");
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    prefs.serial.display_format = display_format;
    prefs.serial.display_mode = display_mode;
    prefs.serial.send_format = send_format;
    prefs.serial.append_newline = append_newline;
    prefs.serial.newline_type = newline_type;
    prefs.serial.auto_scroll = auto_scroll;
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_ble_subscriptions(
    persistence: State<'_, StatePersistenceRef>,
    device_id: String,
    subscribed_uuids: Vec<String>,
) -> Result<()> {
    debug!("更新BLE订阅状态: device={}, uuids={:?}", device_id, subscribed_uuids);
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    if subscribed_uuids.is_empty() {
        prefs.ble.subscribed_characteristics.remove(&device_id);
    } else {
        prefs.ble.subscribed_characteristics.insert(device_id, subscribed_uuids);
    }
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_ble_preferences(
    persistence: State<'_, StatePersistenceRef>,
    display_format: String,
    auto_scroll: bool,
    input_format: String,
    without_response: bool,
    config_collapsed: bool,
    gatt_collapsed: bool,
    panel_collapsed: bool,
) -> Result<()> {
    debug!("更新BLE偏好设置");
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    let existing_subscriptions = prefs.ble.subscribed_characteristics.clone();
    
    prefs.ble.display_format = display_format;
    prefs.ble.auto_scroll = auto_scroll;
    prefs.ble.input_format = input_format;
    prefs.ble.without_response = without_response;
    prefs.ble.config_collapsed = config_collapsed;
    prefs.ble.gatt_collapsed = gatt_collapsed;
    prefs.ble.panel_collapsed = panel_collapsed;
    prefs.ble.subscribed_characteristics = existing_subscriptions;
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        crate::error::ComBridgeError::config(e)
    })?;
    
    Ok(())
}
