use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::FormatTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};

static LOGGER: OnceLock<LoggerService> = OnceLock::new();
static TIMEZONE: OnceLock<std::sync::RwLock<String>> = OnceLock::new();

fn get_timezone_lock() -> &'static std::sync::RwLock<String> {
    TIMEZONE.get_or_init(|| std::sync::RwLock::new("Asia/Shanghai".to_string()))
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoggerConfig {
    pub level: String,
    pub console_enabled: bool,
    pub file_enabled: bool,
    pub file_path: PathBuf,
    pub max_file_size: u64,
    pub max_files: usize,
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
        }
    }
}

pub struct LoggerService {
    config: LoggerConfig,
    log_path: Option<PathBuf>,
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
        let level = Self::parse_level(&config.level);
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

        let env_filter = EnvFilter::builder()
            .with_default_directive(level.into())
            .from_env_lossy();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(layers)
            .try_init()?;

        Ok((
            Self {
                config,
                log_path: log_path.clone(),
            },
            log_path,
        ))
    }

    fn parse_level(level: &str) -> Level {
        match level.to_uppercase().as_str() {
            "TRACE" => Level::TRACE,
            "DEBUG" => Level::DEBUG,
            "INFO" => Level::INFO,
            "WARN" => Level::WARN,
            "ERROR" => Level::ERROR,
            _ => Level::INFO,
        }
    }

    pub fn global() -> Option<&'static LoggerService> {
        LOGGER.get()
    }

    pub fn config(&self) -> &LoggerConfig {
        &self.config
    }

    pub fn log_path(&self) -> Option<&PathBuf> {
        self.log_path.as_ref()
    }
}

impl std::fmt::Debug for LoggerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggerService")
            .field("config", &self.config)
            .field("log_path", &self.log_path)
            .finish()
    }
}
