use serde::{Deserialize, Serialize};
use crate::stats::percentile::Percentiles;

/// Qualitative bufferbloat grade based on latency inflation under load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BufferbloatGrade {
    #[serde(rename = "A+")]
    APlus,
    #[serde(rename = "A")]
    A,
    #[serde(rename = "B")]
    B,
    #[serde(rename = "C")]
    C,
    #[serde(rename = "D")]
    D,
    #[serde(rename = "F")]
    F,
}

impl BufferbloatGrade {
    pub fn as_str(&self) -> &'static str {
        match self {
            BufferbloatGrade::APlus => "A+",
            BufferbloatGrade::A => "A",
            BufferbloatGrade::B => "B",
            BufferbloatGrade::C => "C",
            BufferbloatGrade::D => "D",
            BufferbloatGrade::F => "F",
        }
    }
}

/// Bufferbloat & Latency-Under-Load (LUL) assessment result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferbloatAssessment {
    pub baseline_ms: f64,
    pub loaded_p95_ms: f64,
    pub delta_ms: f64,
    pub inflation_ratio: f64,
    pub grade: BufferbloatGrade,
    pub rating_summary: String,
    pub recommendation: String,
}

impl BufferbloatAssessment {
    /// Assess bufferbloat severity by comparing baseline latency (P50 / Min) with tail latency (P95).
    pub fn analyze(p: &Percentiles) -> Self {
        let baseline_ms = p.p50_ms.min(p.min_ms).max(0.001);
        let loaded_p95_ms = p.p95_ms;
        let delta_ms = (loaded_p95_ms - baseline_ms).max(0.0);
        let inflation_ratio = loaded_p95_ms / baseline_ms;

        let (grade, summary, recommendation) = if delta_ms <= 5.0 && inflation_ratio <= 1.25 {
            (
                BufferbloatGrade::APlus,
                "Negligible queue accumulation. Pristine interactive latency.".to_string(),
                "No queuing adjustments required. Excellent for competitive gaming & real-time audio.".to_string(),
            )
        } else if delta_ms <= 15.0 && inflation_ratio <= 1.50 {
            (
                BufferbloatGrade::A,
                "Minimal bufferbloat. Stable under nominal transmission load.".to_string(),
                "Optimal performance for VoIP, video conferencing, and low-latency workloads.".to_string(),
            )
        } else if delta_ms <= 40.0 && inflation_ratio <= 2.20 {
            (
                BufferbloatGrade::B,
                "Mild bufferbloat detected. Tail latency elevated during bursts.".to_string(),
                "Consider enabling Smart Queue Management (SQM) or CAKE on router if experiencing jitter.".to_string(),
            )
        } else if delta_ms <= 120.0 && inflation_ratio <= 3.50 {
            (
                BufferbloatGrade::C,
                "Noticeable bufferbloat. High queue buffering inflates tail RTT.".to_string(),
                "Deploy FQ-CoDel or CAKE rate-limiting to mitigate queue buildup on gateway router.".to_string(),
            )
        } else if delta_ms <= 300.0 || inflation_ratio <= 6.0 {
            (
                BufferbloatGrade::D,
                "Severe bufferbloat. Extreme latency spikes under concurrent network traffic.".to_string(),
                "Aggressive bufferbloat detected. Significant packet queuing degrades streaming and VoIP.".to_string(),
            )
        } else {
            (
                BufferbloatGrade::F,
                "Critical bufferbloat. Network buffers unmanaged; major latency degradation.".to_string(),
                "Immediate remediation required. Enable active queue management (AQM) on uplink interface.".to_string(),
            )
        };

        Self {
            baseline_ms,
            loaded_p95_ms,
            delta_ms,
            inflation_ratio,
            grade,
            rating_summary: summary,
            recommendation,
        }
    }
}
