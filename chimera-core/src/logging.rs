use crate::ApplicationResult;
use std::str::FromStr;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Invalid log level: {}", s)),
        }
    }
}

impl From<LogLevel> for Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

/// 日志格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// 紧凑格式（默认）
    Compact,
    /// 完整格式（带时间、级别、目标）
    Full,
    /// JSON 格式
    Json,
    /// 美化格式（适合开发）
    Pretty,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "compact" => Ok(LogFormat::Compact),
            "full" => Ok(LogFormat::Full),
            "json" => Ok(LogFormat::Json),
            "pretty" => Ok(LogFormat::Pretty),
            _ => Err(format!("Invalid log format: {}", s)),
        }
    }
}

/// 日志配置
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// 日志级别（默认：Info）
    pub level: LogLevel,

    /// 日志格式（默认：Compact）
    pub format: LogFormat,

    /// 是否显示时间戳（默认：true）
    pub show_timestamp: bool,

    /// 是否显示目标（模块路径）（默认：false）
    pub show_target: bool,

    /// 是否显示线程 ID（默认：false）
    pub show_thread_ids: bool,

    /// 是否显示线程名（默认：false）
    pub show_thread_names: bool,

    /// 自定义过滤器（可选）
    /// 例如："my_crate=debug,other_crate=warn"
    pub filter: Option<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Compact,
            show_timestamp: true,
            show_target: false,
            show_thread_ids: false,
            show_thread_names: false,
            filter: None,
        }
    }
}

impl LoggingConfig {
    /// 创建新的日志配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置日志级别
    pub fn level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// 设置日志格式
    pub fn format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    /// 设置是否显示时间戳
    pub fn show_timestamp(mut self, show: bool) -> Self {
        self.show_timestamp = show;
        self
    }

    /// 设置是否显示目标
    pub fn show_target(mut self, show: bool) -> Self {
        self.show_target = show;
        self
    }

    /// 设置自定义过滤器
    pub fn filter(mut self, filter: String) -> Self {
        self.filter = Some(filter);
        self
    }

    /// 从环境变量读取配置
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // 从 RUST_LOG 环境变量读取
        if let Ok(rust_log) = std::env::var("RUST_LOG") {
            config.filter = Some(rust_log);
        }

        // 从 LOG_LEVEL 环境变量读取级别
        if let Ok(level_str) = std::env::var("LOG_LEVEL") {
            if let Ok(level) = level_str.parse() {
                config.level = level;
            }
        }

        // 从 LOG_FORMAT 环境变量读取格式
        if let Ok(format_str) = std::env::var("LOG_FORMAT") {
            if let Ok(format) = format_str.parse() {
                config.format = format;
            }
        }

        config
    }

    /// 初始化日志系统
    pub fn init(self) -> ApplicationResult<()> {
        // 构建环境过滤器
        let env_filter = if let Some(filter) = &self.filter {
            EnvFilter::try_new(filter)
                .unwrap_or_else(|_| EnvFilter::new(&self.level.to_string().to_lowercase()))
        } else {
            // 优先使用 RUST_LOG 环境变量，否则使用配置的级别
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&self.level.to_string().to_lowercase()))
        };

        // 根据格式初始化订阅者
        match self.format {
            LogFormat::Compact => {
                fmt()
                    .with_env_filter(env_filter)
                    .compact()
                    .with_target(self.show_target)
                    .with_thread_ids(self.show_thread_ids)
                    .with_thread_names(self.show_thread_names)
                    .try_init()
                    .map_err(|e| crate::ApplicationError::LoggingInitFailed(e.to_string()))?;
            }
            LogFormat::Full => {
                fmt()
                    .with_env_filter(env_filter)
                    .with_target(self.show_target)
                    .with_thread_ids(self.show_thread_ids)
                    .with_thread_names(self.show_thread_names)
                    .try_init()
                    .map_err(|e| crate::ApplicationError::LoggingInitFailed(e.to_string()))?;
            }
            LogFormat::Json => {
                fmt()
                    .with_env_filter(env_filter)
                    .json()
                    .with_target(self.show_target)
                    .try_init()
                    .map_err(|e| crate::ApplicationError::LoggingInitFailed(e.to_string()))?;
            }
            LogFormat::Pretty => {
                fmt()
                    .with_env_filter(env_filter)
                    .pretty()
                    .with_target(self.show_target)
                    .try_init()
                    .map_err(|e| crate::ApplicationError::LoggingInitFailed(e.to_string()))?;
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl std::fmt::Display for LogFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogFormat::Compact => write!(f, "compact"),
            LogFormat::Full => write!(f, "full"),
            LogFormat::Json => write!(f, "json"),
            LogFormat::Pretty => write!(f, "pretty"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_from_str() {
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("trace".parse::<LogLevel>().unwrap(), LogLevel::Trace);
    }

    #[test]
    fn test_log_format_from_str() {
        assert_eq!("compact".parse::<LogFormat>().unwrap(), LogFormat::Compact);
        assert_eq!("full".parse::<LogFormat>().unwrap(), LogFormat::Full);
        assert_eq!("json".parse::<LogFormat>().unwrap(), LogFormat::Json);
        assert_eq!("pretty".parse::<LogFormat>().unwrap(), LogFormat::Pretty);
    }

    #[test]
    fn test_logging_config_builder() {
        let config = LoggingConfig::new()
            .level(LogLevel::Debug)
            .format(LogFormat::Json)
            .show_timestamp(false)
            .show_target(true);

        assert_eq!(config.level, LogLevel::Debug);
        assert_eq!(config.format, LogFormat::Json);
        assert!(!config.show_timestamp);
        assert!(config.show_target);
    }
}
