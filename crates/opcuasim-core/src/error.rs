use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize, Clone)]
pub enum OpcUaSimError {
    // Connection layer
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Session timeout")]
    SessionTimeout,
    #[error("Security rejected: {0}")]
    SecurityRejected(String),
    #[error("Authentication failed")]
    AuthenticationFailed,

    // Protocol layer
    #[error("Browse error: {0}")]
    BrowseError(String),
    #[error("Read error: {0}")]
    ReadError(String),
    #[error("Write error: {0}")]
    WriteError(String),
    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    // Application layer
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Project file error: {0}")]
    ProjectFileError(String),
    #[error("Output error: {0}")]
    OutputError(String),

    // Generic
    #[error("IO error: {0}")]
    Io(String),
    #[error("Internal error: {0}")]
    Internal(String),
}

impl OpcUaSimError {
    pub fn category(&self) -> &'static str {
        match self {
            Self::ConnectionFailed(_) | Self::SessionTimeout
            | Self::SecurityRejected(_) | Self::AuthenticationFailed => "connection",
            Self::BrowseError(_) | Self::ReadError(_)
            | Self::WriteError(_) | Self::SubscriptionError(_) => "protocol",
            Self::ConfigError(_) | Self::ProjectFileError(_)
            | Self::OutputError(_) => "application",
            Self::Io(_) | Self::Internal(_) => "generic",
        }
    }
}

impl From<std::io::Error> for OpcUaSimError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
