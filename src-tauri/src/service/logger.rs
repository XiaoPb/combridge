use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::Level;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    Layer,
    EnvFilter,
};

static LOGGER: OnceLock<LoggerService> = OnceLock::new();

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
    _guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

impl LoggerService {
    pub fn init(config: LoggerConfig) -> Result<&'static LoggerService, Box<dyn std::error::Error>> {
        let _ = tracing_log::LogTracer::init();
        
        let service = Self::create_service(config.clone())?;
        LOGGER
            .set(service)
            .map_err(|_| "Logger already initialized")?;
        Ok(LOGGER.get().expect("LOGGER must be initialized after set"))
    }

    pub fn init_default() -> Result<&'static LoggerService, Box<dyn std::error::Error>> {
        Self::init(LoggerConfig::default())
    }

    fn create_service(config: LoggerConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let level = Self::parse_level(&config.level);
        let mut layers = Vec::new();

        if config.console_enabled {
            let console_layer = fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .pretty();
            layers.push(console_layer.boxed());
        }

        let mut guard = None;
        if config.file_enabled {
            let log_dir = config.file_path.parent().unwrap_or(&PathBuf::from(".")).to_path_buf();
            
            if !log_dir.exists() {
                std::fs::create_dir_all(&log_dir)?;
            }

            let now = chrono::Local::now();
            let timestamp = now.format("%Y-%m-%d-%H-%M-%S");
            let log_filename = format!("combridge_system_{}.log", timestamp);
            let log_path = log_dir.join(&log_filename);

            let file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&log_path)?;
            
            let (non_blocking, worker_guard) = tracing_appender::non_blocking(file);
            guard = Some(worker_guard);

            let file_layer = fmt::layer()
                .with_span_events(FmtSpan::CLOSE)
                .with_target(true)
                .with_thread_ids(true)
                .with_line_number(true)
                .with_ansi(false)
                .with_writer(non_blocking);
            layers.push(file_layer.boxed());
            
            tracing::info!("日志文件已创建: {}", log_path.display());
        }

        let env_filter = EnvFilter::builder()
            .with_default_directive(level.into())
            .from_env_lossy();

        tracing_subscriber::registry()
            .with(env_filter)
            .with(layers)
            .try_init()?;

        Ok(Self {
            config,
            _guard: guard,
        })
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
}

impl std::fmt::Debug for LoggerService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoggerService")
            .field("config", &self.config)
            .field("_guard", &"WorkerGuard")
            .finish()
    }
}
