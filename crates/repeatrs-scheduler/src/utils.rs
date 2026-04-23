#![allow(unused)]

use crate::ApiResult;
use crate::config::{self, CONFIG, Config, LogLevel, PostgresConfig, config};
use crate::error::ApiError;

use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::time::Duration;
use tracing::error;
use tracing_journald::Layer as JournaldLayer;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

/// Initializes the global `tracing` subscriber for the application.
///
/// # Examples
///
/// Filter by the structured field we defined in the Rust code
/// ```bash
/// journalctl -u repeatrs-scheduler JOB_ID=12345
/// ```
///
/// Use JSON output to see all the hidden metadata tracing sent
/// ```bash
/// journalctl -u repeatrs-scheduler -o json | jq
/// ```
pub fn initialize_tracing() -> ApiResult<()> {
    println!("initializing tracing");

    let log_level = config().log_level();

    // let journald = JournaldLayer::new()?;

    // let journald_filter = EnvFilter::new(format!(
    //     "repeatrs={},tonic=debug,tower=debug",
    //     log_level.to_string()
    // ));
    // let journald_layer = JournaldLayer::new()?.with_filter(journald_filter);

    // FIX: async_nats::connector prints username and password in the debug trace

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
        .with_target(true)
        .with_level(true)
        .with_writer(std::io::stdout)
        .with_filter(tracing_subscriber::filter::LevelFilter::from_level(
            log_level.to_tracing_level(),
        ));

    tracing_subscriber::registry()
        // .with(journald_layer)
        .with(stdout_layer)
        .init();

    tracing::debug!("Tracing initialized at level: {}", log_level.to_string());
    Ok(())
}
