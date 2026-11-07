//! NetPulse: Sub-millisecond network jitter, RTT latency & bufferbloat analyzer CLI.

pub mod cli;
pub mod engine;
pub mod error;
pub mod reporter;
pub mod stats;

pub use cli::Cli;
pub use engine::{tcp::TcpProber, udp::UdpProber, ProbeConfig, ProbeProtocol, ProbeSample};
pub use error::{NetPulseError, Result};
pub use reporter::{json_export::JsonReporter, terminal::TerminalReporter, OutputFormat};
pub use stats::{
    bufferbloat::{BufferbloatAssessment, BufferbloatGrade},
    jitter::{compute_final_jitter_ms, compute_jitter_series, Rfc3550Jitter},
    percentile::{interpolate_percentile, Percentiles},
    SessionSummary,
};
