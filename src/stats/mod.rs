pub mod bufferbloat;
pub mod jitter;
pub mod percentile;

use serde::{Deserialize, Serialize};

use crate::engine::{ProbeConfig, ProbeSample};
pub use bufferbloat::{BufferbloatAssessment, BufferbloatGrade};
pub use jitter::{compute_final_jitter_ms, compute_jitter_series, Rfc3550Jitter};
pub use percentile::{interpolate_percentile, Percentiles};

/// Aggregate telemetry summary of an active probing session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub target: String,
    pub port: u16,
    pub protocol: String,
    pub total_sent: usize,
    pub total_received: usize,
    pub loss_percent: f64,
    pub percentiles: Option<Percentiles>,
    pub rfc3550_jitter_ms: Option<f64>,
    pub bufferbloat: Option<BufferbloatAssessment>,
    pub sla_breach: Option<String>,
}

impl SessionSummary {
    /// Aggregate raw probe telemetry samples into a complete analytical summary.
    pub fn from_samples(
        config: &ProbeConfig,
        samples: &[ProbeSample],
        max_jitter_ms: Option<f64>,
        max_loss_percent: Option<f64>,
    ) -> Self {
        let total_sent = samples.len();
        let successful_samples: Vec<&ProbeSample> = samples.iter().filter(|s| s.success).collect();
        let total_received = successful_samples.len();

        let loss_percent = if total_sent > 0 {
            ((total_sent - total_received) as f64 / total_sent as f64) * 100.0
        } else {
            0.0
        };

        let latencies_ms: Vec<f64> = successful_samples
            .iter()
            .filter_map(|s| s.rtt_ms())
            .collect();

        let latencies_us: Vec<f64> = successful_samples
            .iter()
            .filter_map(|s| s.rtt_micros.map(|us| us as f64))
            .collect();

        let percentiles = Percentiles::compute(&latencies_ms);
        let rfc3550_jitter_ms = if latencies_us.len() >= 2 {
            Some(compute_final_jitter_ms(&latencies_us))
        } else {
            None
        };

        let bufferbloat = percentiles.as_ref().map(BufferbloatAssessment::analyze);

        // Evaluate SLA constraints
        let mut sla_breach = None;
        if let Some(limit) = max_loss_percent {
            if loss_percent > limit {
                sla_breach = Some(format!(
                    "Packet loss {:.2}% exceeded SLA limit of {:.2}%",
                    loss_percent, limit
                ));
            }
        }
        if let Some(limit) = max_jitter_ms {
            if let Some(actual_jitter) = rfc3550_jitter_ms {
                if actual_jitter > limit {
                    let msg = format!(
                        "RFC 3550 Jitter {:.3}ms exceeded SLA limit of {:.3}ms",
                        actual_jitter, limit
                    );
                    sla_breach = match sla_breach {
                        Some(prev) => Some(format!("{}; {}", prev, msg)),
                        None => Some(msg),
                    };
                }
            }
        }

        Self {
            target: config.target.clone(),
            port: config.port,
            protocol: format!("{:?}", config.protocol).to_lowercase(),
            total_sent,
            total_received,
            loss_percent,
            percentiles,
            rfc3550_jitter_ms,
            bufferbloat,
            sla_breach,
        }
    }
}
