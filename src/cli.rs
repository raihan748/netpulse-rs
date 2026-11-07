use std::time::Duration;
use clap::Parser;

use crate::engine::{ProbeConfig, ProbeProtocol};
use crate::reporter::OutputFormat;

/// NetPulse: Sub-millisecond network jitter, RTT latency & bufferbloat analyzer CLI.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "netpulse",
    author = "Developer <developer@telkom.sch.id>",
    version = "0.1.0",
    about = "High-precision network jitter (RFC 3550), latency percentiles & bufferbloat analyzer",
    long_about = "NetPulse is an authentic telecommunications telemetry CLI built in Rust.\n\
                  It performs sub-millisecond non-blocking TCP and UDP round-trip probes,\n\
                  computes exact RFC 3550 interarrival jitter, calculates latency percentiles\n\
                  (P50, P90, P95, P99), detects bufferbloat latency-under-load degradation,\n\
                  and streams ANSI terminal visuals, structured JSON, or NDJSON records."
)]
pub struct Cli {
    /// Destination hostname or IP address to probe
    #[arg(short = 't', long = "target", default_value = "127.0.0.1")]
    pub target: String,

    /// Destination port number
    #[arg(short = 'p', long = "port", default_value_t = 80)]
    pub port: u16,

    /// Transport protocol for active probing (tcp or udp)
    #[arg(long = "protocol", default_value = "tcp")]
    pub protocol: String,

    /// Total number of probe samples to send
    #[arg(short = 'c', long = "count", default_value_t = 10)]
    pub count: usize,

    /// Inter-probe transmission interval in milliseconds
    #[arg(short = 'i', long = "interval", default_value_t = 100)]
    pub interval_ms: u64,

    /// Probe socket timeout in milliseconds
    #[arg(long = "timeout", default_value_t = 1000)]
    pub timeout_ms: u64,

    /// Telemetry output format (text, json, ndjson)
    #[arg(short = 'f', long = "format", default_value = "text")]
    pub format: String,

    /// Maximum allowable RFC 3550 jitter in milliseconds (SLA threshold)
    #[arg(long = "max-jitter")]
    pub max_jitter: Option<f64>,

    /// Maximum allowable packet loss percentage (SLA threshold)
    #[arg(long = "max-loss")]
    pub max_loss: Option<f64>,

    /// Quiet mode (suppresses live probe tick lines in text format)
    #[arg(short = 'q', long = "quiet", default_value_t = false)]
    pub quiet: bool,
}

impl Cli {
    /// Convert CLI options to internal engine configuration.
    pub fn to_probe_config(&self) -> Result<ProbeConfig, String> {
        let protocol = self.protocol.parse::<ProbeProtocol>()?;
        Ok(ProbeConfig {
            target: self.target.clone(),
            port: self.port,
            protocol,
            count: self.count,
            interval: Duration::from_millis(self.interval_ms),
            timeout: Duration::from_millis(self.timeout_ms),
        })
    }

    /// Resolve requested output format.
    pub fn output_format(&self) -> OutputFormat {
        match self.format.to_ascii_lowercase().as_str() {
            "json" => OutputFormat::Json,
            "ndjson" => OutputFormat::Ndjson,
            _ => OutputFormat::Text,
        }
    }
}
