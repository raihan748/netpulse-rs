//! NetPulse: Sub-millisecond network jitter, RTT latency & bufferbloat analyzer CLI.

pub mod engine;
pub mod error;
pub mod stats;

pub use engine::{ProbeConfig, ProbeProtocol, ProbeSample};
pub use error::{NetPulseError, Result};
pub use stats::{BufferbloatAssessment, BufferbloatGrade, Percentiles, Rfc3550Jitter, SessionSummary};
