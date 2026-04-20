//! Utilities for logging. Provides asynchronous log writing and reading capabilities,
//! facilitating efficient logging for job executions.
//!
//! ## Features:
//!
//! - **Log Event Enumeration**: Defines different types of log events.
//! - **Multiple Log Destinations**: Writes logs to multiple destinations at once.
//! - **Log Frame Control**: Use codecs to write frames in different formats.
//! - **Real-time Logging**: Supports near real-time logging balancing performance and immediacy.
//! - **Backpressured Writing**: Async reader task adaptively slows down the reading of the pipe.
//!      Control via the buffer Logger parameter
//! - **Asynchronous Reader Task**: Implements a task that reads log lines from a source and sends them as log events.
//! - **Error Handling**: Includes error handling for file operations and inter-task communication.
//!

use crate::config;
use crate::error::{Error, Result};

use chrono::DateTime;
use chrono::Utc;
use std::fmt::Display;
// use std::marker::PhantomData;
use std::path::Path;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::{AsyncRead, AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};
use tracing::{error, info};

/*
// TODO make reader and sender structs.
- reader reads from Stdin Stderr...
- sender sends to multiple destinations
- logger subscribes senders and dispatches log messages with broadcast
 */

/// Reperesnts an event to be logged. Events are written to the
/// destination specifid in the [`Logger`].
pub enum LogEvent {
    /// A log line with a timestamp and a message
    Line(LogMessage),

    /// Sent when a shutdown signal is received
    Shutdown(DateTime<Utc>),
}

impl Display for LogEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogEvent::Line(log) => {
                if log.with_timestamp {
                    let now = log
                        .timestamp
                        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    let line = format!("[{}] {}\n", now, log.message);
                    return write!(f, "[{}] {}", now, line);
                } else {
                    let line = format!("{}\n", log.message);
                    return write!(f, "{}", line);
                };
            }
            LogEvent::Shutdown(timestamp) => {
                let now = timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                let line = format!("[{}] {}\n", now, "Shutdown signal received. Exiting.");
                write!(f, "{}", line)
            }
        }
    }
}

impl LogEvent {
    pub fn log_line(message: &str, with_timestamp: bool) -> Self {
        let msg = LogMessage {
            with_timestamp,
            timestamp: Utc::now(),
            message: message.to_string(),
        };
        LogEvent::Line(msg)
    }

    pub fn shutdown() -> Self {
        LogEvent::Shutdown(Utc::now())
    }
}

#[derive(Clone)]
pub enum StreamType {
    Stdout,
    Stderr,
}

impl Display for StreamType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamType::Stdout => write!(f, "[stdout]"),
            StreamType::Stderr => write!(f, "[stderr]"),
        }
    }
}

struct LogMessage<'m> {
    timestamp: DateTime<Utc>,
    message: &'m str,
}

impl LogMessage {
    pub fn new(message: &str) -> Self {
        Self {
            timestamp: Utc::now(),
            message: message,
        }
    }
}

// not exaustive
pub enum LogDestination {
    /// Write logs to a local file
    File(FileDestination),
    Stdout,
}

impl LogDestination {
    /// Sets a new file destination
    pub fn file() -> Self {
        let fd = FileDestination::new_file_dest();
        Self::File(fd)
    }
}

struct FileDestination {
    path: PathBuf,
}

impl FileDestination {
    pub fn new_file_dest() -> Self {
        let config = config::get_config();
        let path = config.job_logs_base_path();

        FileDestination {
            path: path.to_owned(),
        }
    }

    /// Creates a file for a specific job run at the input log path.
    async fn get_log_file_handle(&self) -> Result<File> {
        let timestamp = chrono::Utc::now().format("%Y-%m-%d_%H-%M-%S");
        let file_name = format!("{}.log", timestamp);

        let dir_path = self.path.join(file_name.replace(" ", "_"));

        let log_file = OpenOptions::new()
            .mode(0o660)
            .append(true)
            .create(true)
            .open(&dir_path)
            .await?;

        Ok(log_file)
    }
}

/// Manages logging. It uses an internal receiver, that reads
/// log messages from the job stdin/stderr and sends them to
/// a receiver task that writes them to a specified destination.
///
/// The mpsc channel is used in order to have:
/// - explicit backpressure control via the buffer parameter.
/// - Ability to spawn multiple writers.
/// - No need for Mutex on the File handle, avoids contention.
pub struct Logger {
    /// Log channel size
    sender: mpsc::Sender<LogEvent>,
    receiver: mpsc::Receiver<LogEvent>,
    destination: Option<LogDestination>,
}

impl Logger {
    pub fn new() -> Self {
        // TODO: get logger buffer size from CONFIG
        let buffer: usize = 1_000;
        let (sender, receiver) = mpsc::channel::<LogEvent>(buffer);

        Self {
            sender,
            receiver,
            destination: None,
        }
    }

    /// Spawns a reader task that reads framed lines from the provided source
    pub fn with_source<R>(self, source: R) -> Self
    where
        R: AsyncRead + Unpin + Send + 'static,
    {
        let sender = self.sender.clone();
        let mut framed = FramedRead::new(source, LinesCodec::new());

        tokio::spawn(async move {
            while let Some(line) = framed.next().await {
                match line {
                    Ok(msg) => {
                        let log = LogMessage::new(&msg);
                        if sender.send(LogEvent::Line(log)).await.is_err() {
                            error!("Could not send log event")
                        };
                    }
                    Err(e) => error!("Failed decoding log frame: {}", e),
                }
            }
        });
        self
    }

    /// Sets the log destination
    pub fn with_destination(mut self, destination: LogDestination) -> Self {
        self.destination = Some(destination);
        self
    }

    /// Sends a single message to the log writer task
    pub async fn send_message(&self, msg: &str) -> Result<()> {
        let log = LogMessage::new(msg);
        if let Err(e) = self.sender.send(LogEvent::Line(log)).await {
            error!("Could not send log event: {}", e.to_string());
            // return Err(e);
        }
        Ok(())
    }

    /// Receives log messages from the child subprocess and writes them to the
    /// log destination.
    pub async fn start(self) -> Result<()> {
        // To compromise between expensive, frequent syscalls and near real-time logging,
        // we flush to file either when the buffer is full (64Kib capacity) or every 5 seconds.
        // TODO: use env variable to control the flush interval.

        let Some(ref destination) = self.destination else {
            return Err(Error("Error: log destination not set.".into()));
        };

        match destination {
            LogDestination::File(fd) => {
                let fh = fd.get_log_file_handle().await?;
                let _ = self.write_to_file(fh).await;
            }
            _ => todo!(),
        };

        Ok(())
    }

    async fn write_to_file(mut self, file: File) -> Result<()> {
        let mut file = BufWriter::with_capacity(64 * 1024, file);
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(5));
        ticker.tick().await;

        loop {
            tokio::select! {
                msg = self.receiver.recv() => {
                    match msg {
                        Some(event) =>  {
                            let line = event.to_string();
                            file.write_all(line.as_bytes()).await?;
                        }
                        None => {
                            break;
                        }
                    }
                }

                _ = ticker.tick() => {
                    file.flush().await?;
                }
            }
        }

        file.flush().await?;
        Ok(())
    }
}

/// Creates the directory for the current job and all of its parents, if it's missing.
pub async fn create_job_log_dir(path: impl AsRef<Path>) {
    if let Err(e) = tokio::fs::create_dir_all(&path).await {
        error!(
            error = format!(
                "Could not create logs directory at path: {}. {}",
                path.as_ref().display(),
                e
            )
        )
    }

    info!("Created logs directory: {}", &path.as_ref().display())
}
