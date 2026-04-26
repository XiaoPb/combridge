use tauri::State;
use tracing::{debug, error, info};

use crate::error::Result;
use crate::state::{StatePersistenceRef, Preferences};
use crate::gh3036::Gh3036ManagerRef;
use crate::gh3036::CsvConfig;

#[tauri::command]
pub async fn get_preferences(
    persistence: State<'_, StatePersistenceRef>,
) -> Result<Preferences> {
    debug!("获取偏好设置");
    
    let persistence = persistence.inner().read().await;
    let prefs = persistence.load_preferences().await.map_err(|e| {
        error!("加载偏好设置失败: {}", e);
        e
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
        e
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
        e
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
    
    prefs.ble.display_format = display_format;
    prefs.ble.auto_scroll = auto_scroll;
    prefs.ble.input_format = input_format;
    prefs.ble.without_response = without_response;
    prefs.ble.config_collapsed = config_collapsed;
    prefs.ble.gatt_collapsed = gatt_collapsed;
    prefs.ble.panel_collapsed = panel_collapsed;
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        e
    })?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_waveform_preferences(
    persistence: State<'_, StatePersistenceRef>,
    display_rows: u32,
    refresh_interval: u32,
    sidebar_collapsed: bool,
) -> Result<()> {
    debug!("更新波形偏好设置");
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    prefs.waveform.display_rows = display_rows;
    prefs.waveform.refresh_interval = refresh_interval;
    prefs.waveform.sidebar_collapsed = sidebar_collapsed;
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        e
    })?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_gh3036_channel_preferences(
    persistence: State<'_, StatePersistenceRef>,
    connection_type: String,
    serial_port: String,
    ble_device: String,
    tx_char: String,
    rx_char: String,
) -> Result<()> {
    debug!("更新GH3036通道偏好设置");
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    prefs.gh3036_channel.connection_type = connection_type;
    prefs.gh3036_channel.serial_port = serial_port;
    prefs.gh3036_channel.ble_device = ble_device;
    prefs.gh3036_channel.tx_char = tx_char;
    prefs.gh3036_channel.rx_char = rx_char;
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        e
    })?;
    
    Ok(())
}

#[tauri::command]
pub async fn update_gh3036_csv_preferences(
    persistence: State<'_, StatePersistenceRef>,
    manager: State<'_, Gh3036ManagerRef>,
    enabled: bool,
    output_dir: String,
) -> Result<()> {
    debug!("更新GH3036 CSV偏好设置");
    
    let persistence = persistence.inner().read().await;
    let mut prefs = persistence.load_preferences().await.unwrap_or_else(|_| {
        Preferences::default()
    });
    
    prefs.gh3036_csv.enabled = enabled;
    prefs.gh3036_csv.output_dir = output_dir.clone();
    
    persistence.save_preferences(&prefs).await.map_err(|e| {
        error!("保存偏好设置失败: {}", e);
        e
    })?;
    
    let config = CsvConfig { enabled, output_dir };
    if let Err(e) = manager.set_csv_config(config) {
        error!("同步更新后端CSV配置失败: {}", e);
    }
    
    info!("GH3036 CSV偏好设置已更新并同步到后端");
    Ok(())
}
