use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

pub const GITHUB_API_URL: &str = "https://api.github.com/repos/XiaoPb/combridge/releases/latest";
pub const DEFAULT_PROXY_NODES: &[&str] = &[
    "https://v4.gh-proxy.org",
    "https://gh-proxy.org",
    "https://v6.gh-proxy.org",
];
pub const PROXY_CONFIG_FILE_NAME: &str = "update_proxies.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAsset {
    pub name: String,
    pub original_url: String,
    pub proxy_urls: Vec<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    #[serde(default)]
    pub proxies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_name: String,
    pub published_at: Option<String>,
    pub body: Option<String>,
    pub asset: UpdateAsset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadResult {
    pub path: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    WindowsX64,
    WindowsArm64,
    MacOsX64,
    MacOsArm64,
    LinuxX64,
    LinuxArm64,
}

pub fn detect_platform() -> Result<Platform, String> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => Ok(Platform::WindowsX64),
        ("windows", "aarch64") => Ok(Platform::WindowsArm64),
        ("macos", "x86_64") => Ok(Platform::MacOsX64),
        ("macos", "aarch64") => Ok(Platform::MacOsArm64),
        ("linux", "x86_64") => Ok(Platform::LinuxX64),
        ("linux", "aarch64") => Ok(Platform::LinuxArm64),
        (os, arch) => Err(format!("unsupported platform: {os}/{arch}")),
    }
}

fn platform_candidates(platform: Platform) -> (&'static [&'static str], &'static [&'static str]) {
    match platform {
        Platform::WindowsX64 => (&["x64", "x86_64", "amd64"], &[".msi", ".exe"]),
        Platform::WindowsArm64 => (&["arm64", "aarch64"], &[".msi", ".exe"]),
        Platform::MacOsX64 => (&["x64", "x86_64", "amd64"], &[".dmg"]),
        Platform::MacOsArm64 => (&["arm64", "aarch64"], &[".dmg"]),
        Platform::LinuxX64 => (&["x64", "x86_64", "amd64"], &[".appimage", ".deb"]),
        Platform::LinuxArm64 => (&["arm64", "aarch64"], &[".appimage", ".deb"]),
    }
}

pub fn select_asset(
    assets: &[ReleaseAsset],
    platform: Platform,
    proxy_nodes: &[String],
) -> Result<UpdateAsset, String> {
    let (arch_tokens, extensions) = platform_candidates(platform);
    let matches: Vec<&ReleaseAsset> = assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            extensions.iter().any(|ext| name.ends_with(ext))
                && arch_tokens.iter().any(|token| name.contains(token))
        })
        .collect();
    for ext in extensions {
        if let Some(asset) = matches
            .iter()
            .find(|a| a.name.to_ascii_lowercase().ends_with(ext))
        {
            return Ok(UpdateAsset {
                name: asset.name.clone(),
                original_url: asset.browser_download_url.clone(),
                proxy_urls: proxy_urls(&asset.browser_download_url, proxy_nodes),
                size: asset.size,
            });
        }
    }
    let platform_assets: Vec<&ReleaseAsset> = assets
        .iter()
        .filter(|asset| {
            let name = asset.name.to_ascii_lowercase();
            extensions.iter().any(|ext| name.ends_with(ext))
        })
        .collect();
    let has_any_architecture_marker = assets.iter().any(|asset| {
        let name = asset.name.to_ascii_lowercase();
        [
            "x64",
            "x86_64",
            "amd64",
            "arm64",
            "aarch64",
            "universal",
            "universal2",
        ]
        .iter()
        .any(|token| name.contains(token))
    });
    if platform_assets.len() == 1 && !has_any_architecture_marker {
        let asset = platform_assets[0];
        return Ok(UpdateAsset {
            name: asset.name.clone(),
            original_url: asset.browser_download_url.clone(),
            proxy_urls: proxy_urls(&asset.browser_download_url, proxy_nodes),
            size: asset.size,
        });
    }
    Err(format!("no installer asset found for {platform:?}"))
}

pub fn api_url() -> &'static str {
    GITHUB_API_URL
}

pub fn normalize_proxy_node(node: &str) -> Option<String> {
    let node = node.trim().trim_end_matches('/');
    if (node.starts_with("https://") || node.starts_with("http://")) && node.len() > 8 {
        Some(node.to_string())
    } else {
        None
    }
}

pub fn proxy_nodes_from_yaml(yaml: &str) -> Result<Vec<String>, String> {
    let config: ProxyConfig =
        serde_yaml::from_str(yaml).map_err(|e| format!("invalid update proxy config: {e}"))?;
    let mut nodes = Vec::new();
    for node in config.proxies {
        if let Some(node) = normalize_proxy_node(&node) {
            if !nodes.contains(&node) {
                nodes.push(node);
            }
        }
    }
    Ok(nodes)
}

pub fn proxy_urls(original: &str, nodes: &[String]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| format!("{node}/{original}"))
        .collect()
}

pub fn load_proxy_nodes(config_path: &Path) -> Result<Vec<String>, String> {
    if !config_path.exists() {
        let config = ProxyConfig {
            proxies: DEFAULT_PROXY_NODES
                .iter()
                .map(|node| node.to_string())
                .collect(),
        };
        let yaml = serde_yaml::to_string(&config)
            .map_err(|e| format!("failed to serialize default proxy config: {e}"))?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create proxy config directory: {e}"))?;
        }
        std::fs::write(config_path, yaml)
            .map_err(|e| format!("failed to write default proxy config: {e}"))?;
    }
    let yaml = std::fs::read_to_string(config_path)
        .map_err(|e| format!("failed to read proxy config: {e}"))?;
    let nodes = proxy_nodes_from_yaml(&yaml)?;
    if nodes.is_empty() {
        Ok(DEFAULT_PROXY_NODES
            .iter()
            .map(|node| node.to_string())
            .collect())
    } else {
        Ok(nodes)
    }
}

#[derive(Clone)]
pub struct UpdateService {
    client: reqwest::Client,
    cache_dir: PathBuf,
    current_version: String,
    proxy_nodes: Vec<String>,
}

impl UpdateService {
    pub fn new(cache_dir: PathBuf) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent(format!("ComBridge/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| format!("failed to create update client: {e}"))?;
        let proxy_config_path = cache_dir.join(PROXY_CONFIG_FILE_NAME);
        let proxy_nodes = match load_proxy_nodes(&proxy_config_path) {
            Ok(nodes) => nodes,
            Err(error) => {
                tracing::warn!(
                    path = %proxy_config_path.display(),
                    %error,
                    "更新代理配置无效，将使用默认代理节点"
                );
                DEFAULT_PROXY_NODES
                    .iter()
                    .map(|node| node.to_string())
                    .collect()
            }
        };
        Ok(Self {
            client,
            cache_dir,
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            proxy_nodes,
        })
    }

    pub fn validate_cached_path(&self, path: &Path) -> Result<(), String> {
        let cache_root = self.cache_dir.join("updates");
        if !cache_root.exists() {
            return Err("update cache unavailable".to_string());
        }
        let cache = std::fs::canonicalize(cache_root)
            .map_err(|e| format!("update cache unavailable: {e}"))?;
        let candidate =
            std::fs::canonicalize(path).map_err(|e| format!("installer file unavailable: {e}"))?;
        if !candidate.starts_with(&cache) || !candidate.is_file() {
            return Err("installer path is outside update cache".to_string());
        }
        Ok(())
    }

    pub async fn check_for_update(&self) -> Result<Option<UpdateInfo>, String> {
        #[derive(Deserialize)]
        struct Release {
            tag_name: String,
            name: Option<String>,
            body: Option<String>,
            published_at: Option<String>,
            assets: Vec<ReleaseAsset>,
        }
        let response = self
            .client
            .get(api_url())
            .send()
            .await
            .map_err(|e| format!("GitHub API request failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("GitHub API returned {}", response.status()));
        }
        let release: Release = response
            .json()
            .await
            .map_err(|e| format!("invalid GitHub release response: {e}"))?;
        if !is_newer_version(&release.tag_name, &self.current_version)? {
            return Ok(None);
        }
        let asset = select_asset(&release.assets, detect_platform()?, &self.proxy_nodes)?;
        Ok(Some(UpdateInfo {
            current_version: self.current_version.clone(),
            latest_version: release.tag_name,
            release_name: release.name.unwrap_or_default(),
            published_at: release.published_at,
            body: release.body,
            asset,
        }))
    }

    pub async fn download_update<F>(
        &self,
        asset: &UpdateAsset,
        progress: F,
    ) -> Result<DownloadResult, String>
    where
        F: Fn(DownloadProgress) + Send + Sync + 'static,
    {
        let update_dir = self.cache_dir.join("updates");
        tokio::fs::create_dir_all(&update_dir)
            .await
            .map_err(|e| format!("failed to create update cache: {e}"))?;
        validate_asset_name(&asset.name)?;
        let final_path = update_dir.join(&asset.name);
        let part_path = update_dir.join(format!("{}.part", asset.name));
        let progress = Arc::new(progress);
        let mut downloaded = false;
        let mut last_error = None;
        let configured_proxy_urls = proxy_urls(&asset.original_url, &self.proxy_nodes);
        for proxy_url in &configured_proxy_urls {
            match self
                .download_from(proxy_url, &part_path, asset.size, progress.clone())
                .await
            {
                Ok(()) => {
                    downloaded = true;
                    break;
                }
                Err(error) => {
                    last_error = Some(error);
                    let _ = tokio::fs::remove_file(&part_path).await;
                }
            }
        }
        if !downloaded {
            if let Err(error) = self
                .download_from(&asset.original_url, &part_path, asset.size, progress)
                .await
            {
                let _ = tokio::fs::remove_file(&part_path).await;
                return Err(format!(
                    "proxy downloads and direct download failed: {}",
                    last_error.unwrap_or(error)
                ));
            }
        }
        let _ = tokio::fs::remove_file(&final_path).await;
        tokio::fs::rename(&part_path, &final_path)
            .await
            .map_err(|e| format!("failed to finalize update: {e}"))?;
        Ok(DownloadResult {
            path: final_path.to_string_lossy().into_owned(),
            name: asset.name.clone(),
        })
    }

    async fn download_from(
        &self,
        url: &str,
        part_path: &Path,
        fallback_total: u64,
        progress: Arc<dyn Fn(DownloadProgress) + Send + Sync>,
    ) -> Result<(), String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(format!("download returned {}", response.status()));
        }
        let total = response
            .content_length()
            .or((fallback_total > 0).then_some(fallback_total));
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(part_path)
            .await
            .map_err(|e| e.to_string())?;
        let mut downloaded = 0u64;
        use futures_util::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| e.to_string())?;
            file.write_all(&chunk).await.map_err(|e| e.to_string())?;
            downloaded += chunk.len() as u64;
            progress(DownloadProgress {
                downloaded,
                total,
                percent: total.map(|t| downloaded as f64 * 100.0 / t as f64),
            });
        }
        file.flush().await.map_err(|e| e.to_string())?;
        if downloaded == 0 {
            return Err("downloaded file is empty".to_string());
        }
        Ok(())
    }
}

pub fn validate_asset_name(name: &str) -> Result<(), String> {
    let path = Path::new(name);
    if name.is_empty()
        || path.file_name().and_then(|n| n.to_str()) != Some(name)
        || name.contains("..")
    {
        return Err("invalid update asset name".to_string());
    }
    Ok(())
}

pub fn is_newer_version(latest: &str, current: &str) -> Result<bool, String> {
    let latest = semver::Version::parse(latest.trim().trim_start_matches('v'))
        .map_err(|e| format!("invalid latest version: {e}"))?;
    let current = semver::Version::parse(current.trim().trim_start_matches('v'))
        .map_err(|e| format!("invalid current version: {e}"))?;
    Ok(latest > current)
}

#[cfg(test)]
mod tests;
