use std::fmt::Display;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    Cli(&'static str),
    Io(#[from] std::io::Error),
    TomlParse(#[from] toml::de::Error),
    Transport(#[from] tonic::transport::Error),
    TonicStatus(#[from] tonic::Status),
    Serialization(#[from] serde_json::Error),
    Config(#[from] config::ConfigError),
    Unknown(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cli(msg) => write!(f, "{}", msg),
            Self::Io(msg) => write!(f, "{}", msg),
            Self::TomlParse(msg) => write!(f, "Could not parse TOML file. {}", msg),
            Self::Transport(_) => write!(f, "gRPC connection error"),
            Self::TonicStatus(msg) => write!(f, "{}", msg.message()),
            Self::Serialization(msg) => write!(f, "Serialization error. {}", msg),
            Self::Config(msg) => write!(f, "Invalid configuration. {}", msg),
            Self::Unknown(msg) => write!(f, "Unknown error. {}", msg),
        }
    }
}
