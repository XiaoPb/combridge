use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::SystemTime;
use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
use tauri::Manager;

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

#[tauri::command]
pub async fn show_main_window(app: tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| ComBridgeError::io(format!("显示窗口失败: {}", e)))?;
        window.set_focus().map_err(|e| ComBridgeError::io(format!("设置焦点失败: {}", e)))?;
    }
    Ok(())
}
