use serde::{Deserialize, Serialize};

/// Statistical percentile and distribution metrics for latency telemetry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Percentiles {
    pub min_ms: f64,
    pub max_ms: f64,
    pub mean_ms: f64,
    pub stddev_ms: f64,
    pub p50_ms: f64,
    pub p90_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: usize,
}

impl Percentiles {
    /// Compute distribution metrics from a slice of latency measurements in milliseconds.
    pub fn compute(latencies_ms: &[f64]) -> Option<Self> {
        if latencies_ms.is_empty() {
            return None;
        }

        let mut sorted = latencies_ms.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let count = sorted.len();
        let min_ms = sorted[0];
        let max_ms = sorted[count - 1];

        let sum: f64 = sorted.iter().sum();
        let mean_ms = sum / (count as f64);

        let variance = if count > 1 {
            let sum_sq_diff: f64 = sorted.iter().map(|&x| (x - mean_ms).powi(2)).sum();
            sum_sq_diff / ((count - 1) as f64)
        } else {
            0.0
        };
        let stddev_ms = variance.sqrt();

        let p50_ms = interpolate_percentile(&sorted, 50.0);
        let p90_ms = interpolate_percentile(&sorted, 90.0);
        let p95_ms = interpolate_percentile(&sorted, 95.0);
        let p99_ms = interpolate_percentile(&sorted, 99.0);

        Some(Percentiles {
            min_ms,
            max_ms,
            mean_ms,
            stddev_ms,
            p50_ms,
            p90_ms,
            p95_ms,
            p99_ms,
            sample_count: count,
        })
    }
}

/// Compute percentile with linear interpolation on a pre-sorted slice.
pub fn interpolate_percentile(sorted: &[f64], percentile: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 || percentile <= 0.0 {
        return sorted[0];
    }
    if percentile >= 100.0 {
        return sorted[sorted.len() - 1];
    }

    let rank = (percentile / 100.0) * ((sorted.len() - 1) as f64);
    let lower_index = rank.floor() as usize;
    let fraction = rank - (lower_index as f64);
    let upper_index = (lower_index + 1).min(sorted.len() - 1);

    sorted[lower_index] + fraction * (sorted[upper_index] - sorted[lower_index])
}
