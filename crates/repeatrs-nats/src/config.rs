use crate::Result;
use crate::error::NatsError;
use async_nats::ServerAddr;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

const JOB_QUEUES_PREFIX: &str = "job_queues";
const NATS_URL: &str = "127.0.0.1:4222";
const MAX_CONSUMERS: i32 = 10;
const MAX_BYTES: i64 = 1024 * 1024;
const MAX_MESSAGES: i64 = 1000000;
const ENVIRONMENT: &str = "DEV";
const DUPLICATE_WINDOW_SECS: u64 = 120;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    address: String,
    job_queues_prefix: String,
    max_consumers: i32,
    max_bytes: i64,
    max_messages: i64,
    environment: String,
    duplicate_window_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            address: NATS_URL.to_string(),
            job_queues_prefix: JOB_QUEUES_PREFIX.to_string(),
            max_consumers: MAX_CONSUMERS,
            max_bytes: MAX_BYTES,
            max_messages: MAX_MESSAGES,
            environment: ENVIRONMENT.to_string(),
            duplicate_window_seconds: DUPLICATE_WINDOW_SECS,
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let config = config::Config::builder()
            .set_default("nats.address", NATS_URL)?
            .set_default("nats.job_queues_prefix", JOB_QUEUES_PREFIX)?
            .set_default("nats.max_consumers", MAX_CONSUMERS)?
            .set_default("nats.max_bytes", MAX_BYTES)?
            .set_default("nats.max_messages", MAX_MESSAGES)?
            .set_default("nats.environment", ENVIRONMENT)?
            .set_default("nats.duplicate_window_seconds", DUPLICATE_WINDOW_SECS)?
            .add_source(config::File::with_name("nats").required(false))
            .add_source(config::Environment::with_prefix("REPEATRS"))
            .build()?;

        let de: Config = config.try_deserialize()?;

        de.validated()?;

        Ok(de)
    }

    pub fn job_queues_prefix(&self) -> &str {
        &self.job_queues_prefix
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn max_consumers(&self) -> i32 {
        self.max_consumers
    }

    pub fn max_messages(&self) -> i64 {
        self.max_messages
    }

    pub fn max_bytes(&self) -> i64 {
        self.max_bytes
    }

    pub fn environment(&self) -> &str {
        &self.environment
    }

    pub fn duplicate_window_seconds(&self) -> u64 {
        self.duplicate_window_seconds
    }

    fn validated(&self) -> Result<()> {
        ServerAddr::from_str(&self.address).map_err(NatsError::InvalidAddress)?;

        if self.job_queues_prefix.is_empty() {
            return Err(NatsError::Internal("Prefix cannot be empty".to_string()));
        }

        if self.max_consumers <= 0 {
            return Err(NatsError::Internal(
                "Configuration parameter 'max_consumers' must be greater than zero.".to_string(),
            ));
        }

        if self.max_bytes <= 0 {
            return Err(NatsError::Internal(
                "Configuration parameter 'max_bytes' must be greater than zero.".to_string(),
            ));
        }

        if self.max_messages <= 0 {
            return Err(NatsError::Internal(
                "Configuration parameter 'max_messages' must be greater than zero.".to_string(),
            ));
        }

        Ok(())
    }
}
