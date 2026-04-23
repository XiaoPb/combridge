//! RPC Logging Interface

use crate::types::LogLevel;

pub trait LogCallback: Send + Sync {
    fn log(&self, level: LogLevel, context: &str, message: &str);
}

#[derive(Default)]
pub struct DefaultLogger;

impl LogCallback for DefaultLogger {
    fn log(&self, level: LogLevel, _context: &str, message: &str) {
        match level {
            LogLevel::Trace => log::trace!("{}", message),
            LogLevel::Debug => log::debug!("{}", message),
            LogLevel::Info => log::info!("{}", message),
            LogLevel::Warn => log::warn!("{}", message),
            LogLevel::Error => log::error!("{}", message),
        }
    }
}

#[derive(Default)]
pub struct NullLogger;

impl LogCallback for NullLogger {
    fn log(&self, _level: LogLevel, _context: &str, _message: &str) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_logger() {
        let logger = NullLogger::default();
        logger.log(LogLevel::Info, "test", "test message");
    }

    #[test]
    fn test_default_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
        let logger = DefaultLogger::default();
        logger.log(LogLevel::Info, "test", "test message");
    }
}
