//!The scheduler reads its configuration from three sources simultaneously,
//! in this order of priority:
//!
//! 1. Environment Variables (highest priority - overrides everything)
//! 2. Configuration File toml or yaml)
//! 3. Default Values

use crate::ApiResult;
use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use std::{fmt::Display, str::FromStr};

const DATABASE_URL: &str = "postgres://postgres:password@localhost/postgres";
const ACQUIRE_TIMEOUT_SECONDS: u32 = 5;
const MAX_CONNECTIONS: u32 = 10;
const IDLE_TIMEOUT_SECONDS: u32 = 300;
const MAX_LIFETIME_SECONDS: u32 = 1800;
const HEARTBEAT_INTERVAL_SECONDS: u32 = 60;
const HEARTBEAT_THRESHOLD_SECONDS: u32 = 120;
const MAX_RETRIES: u32 = 3;
const TIMEOUT: u32 = 7200; // 2 hours
const SCHEDULER_GRPC_URL: &str = "127.0.0.1:50051";

pub static CONFIG: OnceLock<Config> = OnceLock::new();

/// Returns the configuration or initializes it with default values.
pub fn config() -> &'static Config {
    CONFIG.get_or_init(Config::default)
}

/// Defines the format for the scheduler logging output.
#[derive(Debug, Default, Clone, Deserialize)]
pub enum LogFormat {
    /// A concise, single-line format.
    #[default]
    Compact,
    /// Rich, color-coded, multi-line output.
    Pretty,
    /// Structured JSON for machine parsing.
    Json,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let format = match s {
            "compact" => LogFormat::Compact,
            "pretty" => LogFormat::Pretty,
            "json" => LogFormat::Json,
            &_ => LogFormat::Compact,
        };

        Ok(format)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

impl Default for &LogLevel {
    fn default() -> Self {
        &LogLevel::Info
    }
}

impl LogLevel {
    pub fn to_tracing_level(&self) -> tracing::Level {
        match self {
            LogLevel::Error => tracing::Level::ERROR,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Trace => tracing::Level::TRACE,
        }
    }
}

impl FromStr for LogLevel {
    type Err = ApiError;

    fn from_str(s: &str) -> ApiResult<Self> {
        let format = match s {
            "erorr" => Self::Error,
            "warn" => Self::Warn,
            "info" => Self::Info,
            "debug" => Self::Debug,
            "trace" => Self::Trace,
            &_ => Self::Info,
        };

        Ok(format)
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

impl AsRef<str> for LogLevel {
    fn as_ref(&self) -> &str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
            Self::Trace => "trace",
        }
    }
}

/// Settings for the workers. They can be specified with environment variables with format:
/// REPEATRS_WORKER_DEFAULTS__<struct_field>. They can be specified in a file called
/// "scheduler" (toml, yaml) in the directory "config", under the table [worker_defaults]
/// (or key "worker_defaults").
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerConfig {
    /// Cleanup interval to check for stale jobs in seconds. Default: 60 seconds.    
    heartbeat_interval_seconds: u32,

    /// Workes that have not reported a hearbeat for this interval of time are terminated.
    heartbeat_threshold_seconds: u32,

    /// Maximum number of failed job executions before considering a complete job failure
    max_retries: u32,

    /// Maximum job duration in seconds before the job is terminated
    timeout: u32,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_seconds: HEARTBEAT_INTERVAL_SECONDS,
            heartbeat_threshold_seconds: HEARTBEAT_THRESHOLD_SECONDS,
            max_retries: MAX_RETRIES,
            timeout: TIMEOUT,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PostgresConfig {
    /// Postgres database URL.
    url: String,

    /// Set the maximum number of connections for the postgres connection pool. Default 10.
    max_connections: u32,

    /// Set the maximum amount of time to spend waiting for a connection. Default 5 seconds.
    acquire_timeout_seconds: u32,

    /// Set a maximum idle duration for individual database connections. Default: 5 minutes.
    idle_timeout_seconds: u32,

    /// Set the maximum lifetime of individual database connections in seconds. Default: 30 minutes.
    max_lifetime_seconds: u32,
}

impl PostgresConfig {
    pub fn url(&self) -> &str {
        &self.url
    }
    pub fn max_connections(&self) -> u32 {
        self.max_connections
    }

    pub fn acquire_timeout_secs(&self) -> u32 {
        self.acquire_timeout_seconds
    }

    pub fn idle_timeout_secs(&self) -> u32 {
        self.idle_timeout_seconds
    }

    pub fn max_lifetime_secs(&self) -> u32 {
        self.max_lifetime_seconds
    }
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: DATABASE_URL.to_string(),
            max_connections: MAX_CONNECTIONS,
            acquire_timeout_seconds: ACQUIRE_TIMEOUT_SECONDS,
            idle_timeout_seconds: IDLE_TIMEOUT_SECONDS,
            max_lifetime_seconds: MAX_LIFETIME_SECONDS,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Postgres database settings.
    postgres: PostgresConfig,

    /// URL for gRPC communications with the worker and cli.
    scheduler_grpc_url: String,

    /// Settings for the workers.
    worker: WorkerConfig,

    log_level: LogLevel,
    // /// Log format
    // log_format: LogFormat,
}

impl Default for Config {
    fn default() -> Self {
        let postgres = PostgresConfig::default();
        let worker = WorkerConfig::default();

        Config {
            postgres,
            scheduler_grpc_url: SCHEDULER_GRPC_URL.to_string(),
            worker,
            log_level: LogLevel::Debug,
        }
    }
}

impl Config {
    pub fn init() -> ApiResult<()> {
        let config = config::Config::builder()
            .set_default("scheduler_grpc_url", SCHEDULER_GRPC_URL)?
            .set_default("log_level", "info")?
            .set_default("postgres.url", DATABASE_URL)?
            .set_default("postgres.max_connections", MAX_CONNECTIONS)?
            .set_default("postgres.acquire_timeout_seconds", ACQUIRE_TIMEOUT_SECONDS)?
            .set_default("postgres.idle_timeout_seconds", IDLE_TIMEOUT_SECONDS)?
            .set_default("postgres.max_lifetime_seconds", MAX_LIFETIME_SECONDS)?
            .set_default(
                "worker.heartbeat_interval_seconds",
                HEARTBEAT_INTERVAL_SECONDS,
            )?
            .set_default(
                "worker.heartbeat_threshold_seconds",
                HEARTBEAT_THRESHOLD_SECONDS,
            )?
            .set_default("worker.max_retries", MAX_RETRIES)?
            .set_default("worker.timeout", TIMEOUT)?
            .add_source(config::File::with_name("../../config/scheduler.toml").required(true))
            .add_source(config::Environment::with_prefix("REPEATRS"))
            .build()
            .expect("Failed setting configuration.");

        let de: Config = config
            .try_deserialize()
            .map_err(ApiError::Config)
            .expect("Failed deserializing configuration");

        CONFIG.set(de).expect("Config already initialized.");

        Ok(())
    }

    /// Returns the Postgres configuration
    pub fn postgres(&self) -> &PostgresConfig {
        &self.postgres
    }

    pub fn scheduler_grpc_url(&self) -> &str {
        &self.scheduler_grpc_url
    }

    // pub fn log_format(&self) -> &LogFormat {
    //     &self.log_format
    // }

    pub fn log_level(&self) -> &LogLevel {
        &self.log_level
    }
}
