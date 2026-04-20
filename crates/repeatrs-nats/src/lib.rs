//! Repository layer for NATS

mod config;
mod error;
mod nats;

pub use crate::nats::{JetstreamContextBuilder, NatsJobQueueRepository};
pub use error::NatsError;

pub type Result<T> = core::result::Result<T, error::NatsError>;

pub fn nats_config() -> config::Config {
    config::Config::load().unwrap_or_default()
}
