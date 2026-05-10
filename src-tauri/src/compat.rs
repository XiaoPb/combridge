use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct CompatInfo {
    pub transparent_supported: bool,
    pub webview2_installed: bool,
    pub webview2_version: Option<String>,
}

impl Default for CompatInfo {
    fn default() -> Self {
        Self {
            transparent_supported: true,
            webview2_installed: true,
            webview2_version: None,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn check_transparent_window_support() -> bool {
    use winapi::shared::minwindef::BOOL;
    use winapi::um::dwmapi::DwmIsCompositionEnabled;

    unsafe {
        let mut enabled: BOOL = 0;
        let result = DwmIsCompositionEnabled(&mut enabled);
        if result == 0 {
            let is_enabled = enabled != 0;
            info!("DWM 组合效果状态: {}", is_enabled);
            is_enabled
        } else {
            warn!("无法检测 DWM 组合效果状态，默认不支持透明窗口");
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn check_transparent_window_support() -> bool {
    info!("非 Windows 平台，默认支持透明窗口");
    true
}

#[cfg(target_os = "windows")]
pub fn check_webview2_runtime() -> (bool, Option<String>) {
    let possible_paths = [
        r"SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
        r"SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    ];

    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    for path in &possible_paths {
        if let Ok(key) = hklm.open_subkey(path) {
            if let Ok(version) = key.get_value::<String, _>("pv") {
                info!("检测到 WebView2 运行时 (HKLM): {}", version);
                return (true, Some(version));
            }
        }
        if let Ok(key) = hkcu.open_subkey(path) {
            if let Ok(version) = key.get_value::<String, _>("pv") {
                info!("检测到 WebView2 运行时 (HKCU): {}", version);
                return (true, Some(version));
            }
        }
    }

    let program_files = std::env::var("ProgramFiles(x86)")
        .unwrap_or_else(|_| r"C:\Program Files (x86)".to_string());
    let webview2_paths = [
        format!(r"{}\Microsoft\EdgeWebView\Application", program_files),
        format!(
            r"{}\Microsoft\EdgeWebView\Application",
            std::env::var("ProgramFiles").unwrap_or_else(|_| r"C:\Program Files".to_string())
        ),
    ];

    for path in &webview2_paths {
        let path_buf = PathBuf::from(path);
        if path_buf.exists() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    let version_path = entry.path();
                    if let Some(version) = version_path.file_name().and_then(|n| n.to_str()) {
                        if version.contains('.') {
                            info!("检测到 WebView2 运行时 (文件系统): {}", version);
                            return (true, Some(version.to_string()));
                        }
                    }
                }
            }
        }
    }

    warn!("未检测到 WebView2 运行时");
    (false, None)
}

#[cfg(not(target_os = "windows"))]
pub fn check_webview2_runtime() -> (bool, Option<String>) {
    info!("非 Windows 平台，跳过 WebView2 检测");
    (true, None)
}

pub fn check_compatibility() -> CompatInfo {
    info!("开始兼容性检测...");

    let transparent_supported = check_transparent_window_support();
    if !transparent_supported {
        warn!("系统不支持透明窗口特性，将自动降级为非透明模式");
    }

    let (webview2_installed, webview2_version) = check_webview2_runtime();
    if !webview2_installed {
        warn!("WebView2 运行时未安装，应用可能无法正常运行");
    }

    let info = CompatInfo {
        transparent_supported,
        webview2_installed,
        webview2_version: webview2_version.clone(),
    };

    info!(
        "兼容性检测完成: 透明窗口={}, WebView2={} ({})",
        transparent_supported,
        webview2_installed,
        webview2_version.as_deref().unwrap_or("未知版本")
    );

    info
}

pub fn get_webview2_download_url() -> &'static str {
    "https://developer.microsoft.com/en-us/microsoft-edge/webview2/"
}

pub fn get_webview2_bootstrapper_url() -> &'static str {
    "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
}
