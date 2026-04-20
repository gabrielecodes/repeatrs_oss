use serde::Deserialize;

pub const SCHEDULER_GRPC_URL: &str = "http://localhost:50051";
pub const REQUEST_TIMEOUT_MS: u32 = 5000;

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    /// gRPC address to communicate with the scheduler.
    scheduler_grpc_url: String,

    /// gRPC request timeout
    request_timeout_ms: u32,
}

impl Config {
    pub fn new() -> Result<Self, config::ConfigError> {
        let s = config::Config::builder()
            .set_default("scheduler_grpc_url", SCHEDULER_GRPC_URL)?
            .set_default("request_timeout_ms", REQUEST_TIMEOUT_MS)?
            .add_source(config::File::with_name("config/common").required(false))
            .add_source(config::Environment::with_prefix("REPEATRS"))
            .build()?;

        s.try_deserialize()
    }

    pub fn with_scheduler_grpc_url(mut self, url: Option<String>) -> Self {
        if let Some(u) = url {
            self.scheduler_grpc_url = u;
        } else {
            self.scheduler_grpc_url = SCHEDULER_GRPC_URL.to_string();
        }
        self
    }

    pub fn with_request_timeout_ms(mut self, timeout: Option<u32>) -> Self {
        if let Some(t) = timeout {
            self.request_timeout_ms = t;
        } else {
            self.request_timeout_ms = REQUEST_TIMEOUT_MS;
        }
        self
    }

    pub fn scheduler_grpc_url(&self) -> String {
        self.scheduler_grpc_url.to_owned()
    }

    pub fn request_timeout_ms(&self) -> u32 {
        self.request_timeout_ms
    }
}
