use std::path::PathBuf;
use std::sync::{Mutex, OnceLock, RwLock};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::FormatTime},
    layer::SubscriberExt,
    reload,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

static LOGGER: OnceLock<LoggerService> = OnceLock::new();
static TIMEZONE: OnceLock<RwLock<String>> = OnceLock::new();

fn get_timezone_lock() -> &'static RwLock<String> {
    TIMEZONE.get_or_init(|| RwLock::new("Asia/Shanghai".to_string()))
}

pub fn set_timezone(timezone: &str) {
    if let Ok(mut tz) = get_timezone_lock().write() {
        *tz = timezone.to_string();
    }
}

pub fn get_timezone() -> String {
    get_timezone_lock()
        .read()
        .map(|tz| tz.clone())
        .unwrap_or_else(|_| "Asia/Shanghai".to_string())
}

struct TimezoneTime;

impl FormatTime for TimezoneTime {
    fn format_time(&self, w: &mut tracing_subscriber::fmt::format::Writer<'_>) -> std::fmt::Result {
        let timezone_str = get_timezone();
        let now = chrono::Utc::now();

        if let Ok(tz) = timezone_str.parse::<chrono_tz::Tz>() {
            let local_time = now.with_timezone(&tz);
            write!(w, "{}", local_time.format("%Y-%m-%d %H:%M:%S%.3f"))
        } else {
            write!(w, "{}", now.format("%Y-%m-%d %H:%M:%S%.3f UTC"))
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct LogModuleConfig {
    pub name: String,
    pub enabled: bool,
    pub level: String,
}

impl LogModuleConfig {
    pub fn new(name: &str, enabled: bool, level: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled,
            level: level.to_string(),
        }
    }
}

fn default_log_modules() -> Vec<LogModuleConfig> {
    vec![
        LogModuleConfig::new("rpc-core", true, "info"),
        LogModuleConfig::new("gh3036", true, "info"),
        LogModuleConfig::new("ble", true, "info"),
        LogModuleConfig::new("event-bus", true, "warn"),
        LogModuleConfig::new("device", true, "info"),
        LogModuleConfig::new("frontend", true, "info"),
    ]
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoggerConfig {
    pub level: String,
    pub console_enabled: bool,
    pub file_enabled: bool,
    pub file_path: PathBuf,
    pub max_file_size: u64,
    pub max_files: usize,
    #[serde(default = "default_log_modules")]
    pub modules: Vec<LogModuleConfig>,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            console_enabled: true,
            file_enabled: true,
            file_path: PathBuf::from("logs/combridge.log"),
            max_file_size: 10 * 1024 * 1024,
            max_files: 5,
            modules: default_log_modules(),
        }
    }
}

pub struct LoggerService {
    config: RwLock<LoggerConfig>,
    log_path: Option<PathBuf>,
    filter_handle: reload::Handle<EnvFilter, tracing_subscriber::Registry>,
}

impl LoggerService {
    pub fn init(
        config: LoggerConfig,
    ) -> Result<&'static LoggerService, Box<dyn std::error::Error>> {
        if LOGGER.get().is_some() {
            return Err("Logger already initialized".into());
        }

        let (service, log_path) = Self::create_service(config.clone())?;

        LOGGER
            .set(service)
            .map_err(|_| "Logger already initialized")?;

        let _ = tracing_log::LogTracer::init();

        if let Some(path) = log_path {
            tracing::info!("日志系统初始化完成，日志文件: {}", path.display());
        }

        Ok(LOGGER.get().expect("LOGGER must be initialized after set"))
    }

    pub fn init_default() -> Result<&'static LoggerService, Box<dyn std::error::Error>> {
        Self::init(LoggerConfig::default())
    }

    fn create_service(
        config: LoggerConfig,
    ) -> Result<(Self, Option<PathBuf>), Box<dyn std::error::Error>> {
        let mut layers = Vec::new();

        if config.console_enabled {
            let console_layer = fmt::layer()
                .with_timer(TimezoneTime)
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .pretty();
            layers.push(console_layer.boxed());
        }

        let mut log_path = None;

        if config.file_enabled {
            let log_dir = config
                .file_path
                .parent()
                .unwrap_or(&PathBuf::from("."))
                .to_path_buf();

            if !log_dir.exists() {
                std::fs::create_dir_all(&log_dir)?;
            }

            let timezone_str = get_timezone();
            let now = if let Ok(tz) = timezone_str.parse::<chrono_tz::Tz>() {
                chrono::Utc::now().with_timezone(&tz)
            } else {
                chrono::Utc::now().with_timezone(&chrono_tz::Asia::Shanghai)
            };
            let timestamp = now.format("%Y-%m-%d-%H-%M-%S");
            let log_filename = format!("combridge_system_{}.log", timestamp);
            let path = log_dir.join(&log_filename);
            log_path = Some(path.clone());

            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)?;

            let file_layer = fmt::layer()
                .with_timer(TimezoneTime)
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(Mutex::new(file));
            layers.push(file_layer.boxed());
        }

        let env_filter = Self::build_filter(&config);
        let (env_filter, filter_handle) = reload::Layer::new(env_filter);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(layers)
            .try_init()?;

        Ok((
            Self {
                config: RwLock::new(config),
                log_path: log_path.clone(),
                filter_handle,
            },
            log_path,
        ))
    }

    fn parse_level_filter(level: &str) -> LevelFilter {
        match level.to_ascii_lowercase().as_str() {
            "trace" => LevelFilter::TRACE,
            "debug" => LevelFilter::DEBUG,
            "info" => LevelFilter::INFO,
            "warn" => LevelFilter::WARN,
            "error" => LevelFilter::ERROR,
            "off" => LevelFilter::OFF,
            _ => LevelFilter::INFO,
        }
    }

    fn module_target(name: &str) -> Option<&'static str> {
        match name {
            "rpc-core" => Some("rpc_core"),
            "gh3036" => Some("combridge_rust_lib::gh3036"),
            "ble" => Some("combridge_rust_lib::device::ble"),
            "event-bus" => Some("combridge_rust_lib::service::event_bus"),
            "device" => Some("combridge_rust_lib::device"),
            "frontend" => Some("frontend"),
            _ => None,
        }
    }

    fn build_filter(config: &LoggerConfig) -> EnvFilter {
        let mut filter = EnvFilter::default().add_directive(Self::parse_level_filter(&config.level).into());

        for module in &config.modules {
            let Some(target) = Self::module_target(&module.name) else {
                continue;
            };
            let level = if module.enabled {
                Self::parse_level_filter(&module.level)
            } else {
                LevelFilter::OFF
            };
            if let Ok(directive) = format!("{}={}", target, level).parse() {
                filter = filter.add_directive(directive);
            }
        }

        filter
    }

    pub fn global() -> Option<&'static LoggerService> {
        LOGGER.get()
    }

    pub fn config(&self) -> LoggerConfig {
        self.config
            .read()
            .map(|config| config.clone())
            .unwrap_or_default()
    }

    pub fn update_config(&self, config: LoggerConfig) -> Result<(), Box<dyn std::error::Error>> {
        let filter = Self::build_filter(&config);
        self.filter_handle.reload(filter)?;

        if let Ok(mut current) = self.config.write() {
            *current = config;
        }

        Ok(())
    }

    pub fn log_path(&self) -> Option<&PathBuf> {
        self.log_path.as_ref()
    }
}

impl std::fmt::Debug for LoggerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggerService")
            .field("config", &self.config())
            .field("log_path", &self.log_path)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_modules_include_rpc_core() {
        let config = LoggerConfig::default();
        assert!(config.modules.iter().any(|module| module.name == "rpc-core"));
    }

    #[test]
    fn test_build_filter_accepts_disabled_module() {
        let mut config = LoggerConfig::default();
        config.modules = vec![LogModuleConfig::new("rpc-core", false, "trace")];
        let filter = LoggerService::build_filter(&config);
        let display = filter.to_string();
        assert!(display.contains("rpc_core=off"));
    }
}
