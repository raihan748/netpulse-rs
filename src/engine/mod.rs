pub mod tcp;
pub mod udp;

use std::time::Duration;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Supported transport protocols for active probing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProbeProtocol {
    Tcp,
    Udp,
}

impl std::str::FromStr for ProbeProtocol {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "tcp" => Ok(ProbeProtocol::Tcp),
            "udp" => Ok(ProbeProtocol::Udp),
            other => Err(format!("Unsupported protocol '{}'. Use 'tcp' or 'udp'", other)),
        }
    }
}

/// Active probe configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub target: String,
    pub port: u16,
    pub protocol: ProbeProtocol,
    pub count: usize,
    pub interval: Duration,
    pub timeout: Duration,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            target: "127.0.0.1".to_string(),
            port: 80,
            protocol: ProbeProtocol::Tcp,
            count: 10,
            interval: Duration::from_millis(100),
            timeout: Duration::from_secs(1),
        }
    }
}

/// A single probe telemetry measurement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeSample {
    pub sequence: u32,
    pub timestamp: DateTime<Utc>,
    pub rtt_micros: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
}

impl ProbeSample {
    /// Helper to convert round trip time to milliseconds.
    pub fn rtt_ms(&self) -> Option<f64> {
        self.rtt_micros.map(|us| us as f64 / 1000.0)
    }
}
