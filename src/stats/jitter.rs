use serde::{Deserialize, Serialize};

/// RFC 3550 Interarrival Jitter Estimator.
///
/// Implements the exact algorithm defined in RFC 3550 (RTP: A Transport Protocol for
/// Real-Time Applications), Section 6.4.1 and Appendix A.8:
///
///   D(i, j) = (R_j - S_j) - (R_i - S_i)
///   J(i)    = J(i-1) + (|D(i-1, i)| - J(i-1)) / 16
///
/// where (R - S) represents packet transit duration (e.g. measured RTT),
/// and J is smoothed via an exponential moving average with weight 1/16 (0.0625).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rfc3550Jitter {
    prev_transit: Option<f64>,
    jitter_micros: f64,
    samples_processed: usize,
}

impl Default for Rfc3550Jitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Rfc3550Jitter {
    /// Create a new jitter estimator initialized to zero.
    pub fn new() -> Self {
        Self {
            prev_transit: None,
            jitter_micros: 0.0,
            samples_processed: 0,
        }
    }

    /// Process a new transit time sample (in microseconds) and update smoothed jitter.
    /// Returns the updated jitter value in microseconds.
    pub fn update(&mut self, transit_micros: f64) -> f64 {
        if let Some(prev) = self.prev_transit {
            let diff = (transit_micros - prev).abs();
            self.jitter_micros += (diff - self.jitter_micros) / 16.0;
        }
        self.prev_transit = Some(transit_micros);
        self.samples_processed += 1;
        self.jitter_micros
    }

    /// Current smoothed jitter in microseconds.
    pub fn jitter_micros(&self) -> f64 {
        self.jitter_micros
    }

    /// Current smoothed jitter in milliseconds.
    pub fn jitter_ms(&self) -> f64 {
        self.jitter_micros / 1000.0
    }

    /// Total number of transit samples incorporated into this calculation.
    pub fn samples_processed(&self) -> usize {
        self.samples_processed
    }

    /// Reset internal state.
    pub fn reset(&mut self) {
        self.prev_transit = None;
        self.jitter_micros = 0.0;
        self.samples_processed = 0;
    }
}

/// Compute running jitter series across an array of transit times in microseconds.
pub fn compute_jitter_series(transits: &[f64]) -> Vec<f64> {
    let mut estimator = Rfc3550Jitter::new();
    transits.iter().map(|&t| estimator.update(t)).collect()
}

/// Compute the final RFC 3550 jitter value in milliseconds given an array of RTTs in microseconds.
pub fn compute_final_jitter_ms(transits_micros: &[f64]) -> f64 {
    if transits_micros.len() < 2 {
        return 0.0;
    }
    let mut estimator = Rfc3550Jitter::new();
    for &t in transits_micros {
        estimator.update(t);
    }
    estimator.jitter_ms()
}
