use thiserror::Error;

/// NetPulse domain errors.
#[derive(Error, Debug)]
pub enum NetPulseError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Socket timeout after {0:?}")]
    Timeout(std::time::Duration),

    #[error("DNS resolution failed for target '{target}': {source}")]
    DnsResolution {
        target: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Invalid target format or address: {0}")]
    InvalidTarget(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("SLA threshold breach: {0}")]
    SlaBreach(String),

    #[error("Probe engine error: {0}")]
    Engine(String),
}

pub type Result<T> = std::result::Result<T, NetPulseError>;
