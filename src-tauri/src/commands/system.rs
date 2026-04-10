use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{Duration, SystemTime};
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tauri::Manager;
use tracing::{info, warn};

use crate::error::{ComBridgeError, Result};

static SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| {
    let mut sys = System::new_all();
    sys.refresh_all();
    Mutex::new(sys)
});

pub fn start_system_monitor() {
    tauri::async_runtime::spawn(async {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut sys = SYSTEM.lock();
            sys.refresh_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything()),
            );
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
    pub cpu_count: usize,
    pub total_memory: u64,
    pub app_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub used_memory: u64,
    pub total_memory: u64,
    pub uptime_secs: u64,
    pub disk_usage: Vec<DiskUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskUsage {
    pub name: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub max_files: usize,
    pub max_size_mb: u64,
    pub console_enabled: bool,
    pub file_enabled: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            max_files: 10,
            max_size_mb: 10,
            console_enabled: true,
            file_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStatus {
    pub active_connections: usize,
    pub serial_ports_open: usize,
    pub ble_connections: usize,
    pub websocket_connections: usize,
    pub protocols_loaded: usize,
    pub uptime_secs: u64,
}

#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo> {
    let sys = SYSTEM.lock();

    let os_name = System::name().unwrap_or_else(|| "Unknown".to_string());
    let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
    let arch = env::consts::ARCH.to_string();
    let hostname = System::host_name().unwrap_or_else(|| "Unknown".to_string());
    let cpu_count = sys.cpus().len();
    let total_memory = sys.total_memory();
    let app_version = env!("CARGO_PKG_VERSION").to_string();

    Ok(SystemInfo {
        os_name,
        os_version,
        arch,
        hostname,
        cpu_count,
        total_memory,
        app_version,
    })
}

#[tauri::command]
pub async fn get_system_status() -> Result<SystemStatus> {
    let sys = SYSTEM.lock();
    
    let cpu_usage = sys.global_cpu_info().cpu_usage();
    let total_memory = sys.total_memory();
    let used_memory = sys.used_memory();
    let memory_usage = if total_memory > 0 {
        (used_memory as f32 / total_memory as f32) * 100.0
    } else {
        0.0
    };

    let uptime_secs = System::uptime();

    let disks = Disks::new_with_refreshed_list();
    let disk_usage: Vec<DiskUsage> = disks
        .iter()
        .map(|disk| {
            let total_space = disk.total_space();
            let available_space = disk.available_space();
            let used_space = total_space.saturating_sub(available_space);
            let usage_percent = if total_space > 0 {
                (used_space as f32 / total_space as f32) * 100.0
            } else {
                0.0
            };
            let name = disk.name().to_string_lossy().to_string();
            DiskUsage {
                name,
                total_space,
                available_space,
                used_space,
                usage_percent,
            }
        })
        .collect();

    Ok(SystemStatus {
        cpu_usage,
        memory_usage,
        used_memory,
        total_memory,
        uptime_secs,
        disk_usage,
    })
}

#[tauri::command]
pub async fn configure_log(config: LogConfig) -> Result<()> {
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.level.to_lowercase().as_str()) {
        return Err(ComBridgeError::config(format!(
            "无效的日志级别: {}，有效值为: {:?}",
            config.level, valid_levels
        )));
    }

    Ok(())
}

#[tauri::command]
pub async fn get_log_config() -> Result<LogConfig> {
    Ok(LogConfig::default())
}

#[tauri::command]
pub async fn get_runtime_status(
    serial_manager: tauri::State<'_, crate::device::SerialManagerRef>,
    ble_manager: tauri::State<'_, crate::device::BleManagerRef>,
    connection_pool: tauri::State<'_, crate::websocket::ConnectionPoolRef>,
    plugin_manager: tauri::State<'_, std::sync::Arc<crate::protocol::PluginManager>>,
) -> Result<RuntimeStatus> {
    let serial_ports_open = serial_manager.inner().get_open_ports().len();
    let ble_connections = ble_manager.inner().get_connections().await?.len();
    let websocket_connections = connection_pool.inner().get_all_status().await.len();
    let protocols_loaded = plugin_manager.inner().list_protocols()?.len();

    let uptime_secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(RuntimeStatus {
        active_connections: serial_ports_open + ble_connections + websocket_connections,
        serial_ports_open,
        ble_connections,
        websocket_connections,
        protocols_loaded,
        uptime_secs,
    })
}

#[tauri::command]
pub async fn get_app_version() -> Result<String> {
    Ok(env!("CARGO_PKG_VERSION").to_string())
}

#[tauri::command]
pub async fn get_platform() -> Result<String> {
    Ok(env::consts::OS.to_string())
}

#[tauri::command]
pub async fn open_url(url: String) -> Result<()> {
    match open::that(&url) {
        Ok(_) => Ok(()),
        Err(e) => Err(ComBridgeError::io(format!("无法打开URL: {}", e))),
    }
}

#[tauri::command]
pub async fn show_in_folder(path: String) -> Result<()> {
    let path = std::path::Path::new(&path);
    if !path.exists() {
        return Err(ComBridgeError::io(format!("路径不存在: {}", path.display())));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .args(["/select,", &path.canonicalize()?.to_string_lossy()])
            .spawn()
            .map_err(|e| ComBridgeError::io(format!("无法打开文件夹: {}", e)))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .args(["-R", &path.canonicalize()?.to_string_lossy()])
            .spawn()
            .map_err(|e| ComBridgeError::io(format!("无法打开文件夹: {}", e)))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .spawn()
            .map_err(|e| ComBridgeError::io(format!("无法打开文件夹: {}", e)))?;
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStatus {
    pub label: String,
    pub title: String,
    pub visible: bool,
    pub focused: bool,
    pub maximized: bool,
    pub minimized: bool,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
}

#[tauri::command]
pub async fn get_window_status(app: tauri::AppHandle) -> Result<WindowStatus> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| ComBridgeError::io("未找到主窗口".to_string()))?;

    let label = window.label().to_string();
    let title = window.title().map_err(|e| ComBridgeError::io(format!("获取窗口标题失败: {}", e)))?;
    let visible = window.is_visible().map_err(|e| ComBridgeError::io(format!("获取窗口可见性失败: {}", e)))?;
    let focused = window.is_focused().map_err(|e| ComBridgeError::io(format!("获取窗口焦点状态失败: {}", e)))?;
    let maximized = window.is_maximized().map_err(|e| ComBridgeError::io(format!("获取窗口最大化状态失败: {}", e)))?;
    let minimized = window.is_minimized().map_err(|e| ComBridgeError::io(format!("获取窗口最小化状态失败: {}", e)))?;
    let fullscreen = window.is_fullscreen().map_err(|e| ComBridgeError::io(format!("获取窗口全屏状态失败: {}", e)))?;

    let scale_factor = window
        .scale_factor()
        .map_err(|e| ComBridgeError::io(format!("获取缩放因子失败: {}", e)))?;
    let inner_size = window
        .inner_size()
        .map_err(|e| ComBridgeError::io(format!("获取窗口大小失败: {}", e)))?;
    let outer_position = window
        .outer_position()
        .map_err(|e| ComBridgeError::io(format!("获取窗口位置失败: {}", e)))?;

    let width = (inner_size.width as f64 / scale_factor as f64) as u32;
    let height = (inner_size.height as f64 / scale_factor as f64) as u32;
    let x = outer_position.x;
    let y = outer_position.y;

    info!(
        "窗口状态诊断: label={}, visible={}, focused={}, maximized={}, minimized={}, size={}x{}, position=({},{})",
        label, visible, focused, maximized, minimized, width, height, x, y
    );

    Ok(WindowStatus {
        label,
        title,
        visible,
        focused,
        maximized,
        minimized,
        fullscreen,
        width,
        height,
        x,
        y,
    })
}

#[tauri::command]
pub async fn show_main_window(app: tauri::AppHandle) -> Result<()> {
    const MAX_RETRIES: u32 = 3;
    const RETRY_DELAY_MS: u64 = 500;

    let window = app
        .get_webview_window("main")
        .ok_or_else(|| ComBridgeError::io("未找到主窗口".to_string()))?;

    info!("开始显示主窗口，最多尝试 {} 次", MAX_RETRIES);

    for attempt in 1..=MAX_RETRIES {
        if attempt > 1 {
            info!("第 {} 次重试显示窗口，等待 {}ms", attempt - 1, RETRY_DELAY_MS);
            tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;
        }

        match window.show() {
            Ok(_) => {
                info!("窗口显示成功（第 {} 次尝试）", attempt);

                match window.set_focus() {
                    Ok(_) => {
                        info!("窗口焦点设置成功");
                        return Ok(());
                    }
                    Err(e) => {
                        warn!("第 {} 次尝试设置焦点失败: {}", attempt, e);
                        if attempt == MAX_RETRIES {
                            return Err(ComBridgeError::io(format!(
                                "设置窗口焦点失败（已尝试 {} 次）: {}",
                                MAX_RETRIES, e
                            )));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("第 {} 次尝试显示窗口失败: {}", attempt, e);

                if attempt == MAX_RETRIES {
                    info!("所有尝试均失败，执行延迟显示窗口的备用方案");
                    return attempt_delayed_show(&window, MAX_RETRIES).await;
                }
            }
        }
    }

    Ok(())
}

async fn attempt_delayed_show(window: &tauri::WebviewWindow, retries: u32) -> Result<()> {
    const DELAY_MS: u64 = 1000;

    info!("执行延迟显示窗口备用方案，延迟 {}ms 后再次尝试", DELAY_MS);
    tokio::time::sleep(Duration::from_millis(DELAY_MS)).await;

    window
        .show()
        .map_err(|e| {
            warn!("延迟显示窗口失败: {}", e);
            ComBridgeError::io(format!(
                "显示窗口失败（已重试 {} 次并尝试延迟显示）: {}",
                retries + 1,
                e
            ))
        })?;

    info!("延迟显示窗口成功");

    window.set_focus().map_err(|e| {
        warn!("延迟显示后设置焦点失败: {}", e);
        ComBridgeError::io(format!("设置窗口焦点失败: {}", e))
    })?;

    info!("延迟显示窗口并设置焦点成功");
    Ok(())
}

#[tauri::command]
#[cfg(feature = "devtools")]
pub async fn open_devtools(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
        info!("已打开开发者工具");
        Ok(())
    } else {
        Err(ComBridgeError::io("未找到主窗口".to_string()))
    }
}

#[tauri::command]
#[cfg(feature = "devtools")]
pub async fn close_devtools(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.close_devtools();
        info!("已关闭开发者工具");
        Ok(())
    } else {
        Err(ComBridgeError::io("未找到主窗口".to_string()))
    }
}
