use netpulse_rs::stats::{
    compute_final_jitter_ms, compute_jitter_series,
    interpolate_percentile, BufferbloatAssessment, BufferbloatGrade, Percentiles, Rfc3550Jitter,
};

#[test]
fn test_rfc3550_jitter_exact_mathematical_progression() {
    // RFC 3550 Appendix A.8 specification:
    // D(i-1, i) = |T_i - T_{i-1}|
    // J(i) = J(i-1) + (|D| - J(i-1)) / 16
    let mut estimator = Rfc3550Jitter::new();
    assert_eq!(estimator.samples_processed(), 0);
    assert_eq!(estimator.jitter_micros(), 0.0);

    // First sample T0 = 1000 us: establishes baseline, no jitter yet
    let j0 = estimator.update(1000.0);
    assert_eq!(j0, 0.0);
    assert_eq!(estimator.samples_processed(), 1);

    // Second sample T1 = 1160 us: D = |1160 - 1000| = 160 us
    // J(1) = 0 + (160 - 0) / 16 = 10.0 us
    let j1 = estimator.update(1160.0);
    assert!((j1 - 10.0).abs() < 1e-6, "Expected 10.0 us, got {}", j1);

    // Third sample T2 = 1320 us: D = |1320 - 1160| = 160 us
    // J(2) = 10.0 + (160 - 10.0) / 16 = 10.0 + 150/16 = 10.0 + 9.375 = 19.375 us
    let j2 = estimator.update(1320.0);
    assert!((j2 - 19.375).abs() < 1e-6, "Expected 19.375 us, got {}", j2);

    // Fourth sample T3 = 1320 us: D = |1320 - 1320| = 0 us (no difference)
    // J(3) = 19.375 + (0 - 19.375) / 16 = 19.375 * (15/16) = 18.1640625 us
    let j3 = estimator.update(1320.0);
    let expected_j3 = 19.375 * 15.0 / 16.0;
    assert!((j3 - expected_j3).abs() < 1e-6, "Expected {}, got {}", expected_j3, j3);

    // Verify ms conversion helper
    assert!((estimator.jitter_ms() - (j3 / 1000.0)).abs() < 1e-6);
}

#[test]
fn test_compute_jitter_series_and_final_helper() {
    let transits = vec![1000.0, 1160.0, 1320.0, 1320.0];
    let series = compute_jitter_series(&transits);
    assert_eq!(series.len(), 4);
    assert_eq!(series[0], 0.0);
    assert!((series[1] - 10.0).abs() < 1e-6);
    assert!((series[2] - 19.375).abs() < 1e-6);

    let final_jitter_ms = compute_final_jitter_ms(&transits);
    assert!((final_jitter_ms - (series[3] / 1000.0)).abs() < 1e-6);
}

#[test]
fn test_percentiles_calculation_known_fixture() {
    // 10 elements from 1.0 to 10.0
    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
    let p = Percentiles::compute(&values).expect("Failed to compute percentiles");

    assert_eq!(p.sample_count, 10);
    assert!((p.min_ms - 1.0).abs() < 1e-6);
    assert!((p.max_ms - 10.0).abs() < 1e-6);
    assert!((p.mean_ms - 5.5).abs() < 1e-6);

    // For 10 elements (indices 0..9):
    // 50th percentile rank = 0.50 * 9 = 4.5 -> index 4 (5.0) and 5 (6.0) -> 5.5
    assert!((p.p50_ms - 5.5).abs() < 1e-6);

    // 90th percentile rank = 0.90 * 9 = 8.1 -> index 8 (9.0) + 0.1 * (10.0 - 9.0) = 9.1
    assert!((p.p90_ms - 9.1).abs() < 1e-6);

    // Monotonicity assertion
    assert!(p.min_ms <= p.p50_ms);
    assert!(p.p50_ms <= p.p90_ms);
    assert!(p.p90_ms <= p.p95_ms);
    assert!(p.p95_ms <= p.p99_ms);
    assert!(p.p99_ms <= p.max_ms);
}

#[test]
fn test_percentiles_edge_cases() {
    // Empty case
    let empty: Vec<f64> = vec![];
    assert!(Percentiles::compute(&empty).is_none());

    // Single item case
    let single = vec![42.0];
    let p_single = Percentiles::compute(&single).unwrap();
    assert_eq!(p_single.sample_count, 1);
    assert_eq!(p_single.min_ms, 42.0);
    assert_eq!(p_single.max_ms, 42.0);
    assert_eq!(p_single.mean_ms, 42.0);
    assert_eq!(p_single.p50_ms, 42.0);
    assert_eq!(p_single.stddev_ms, 0.0);

    // All identical items
    let identical = vec![5.0, 5.0, 5.0, 5.0];
    let p_ident = Percentiles::compute(&identical).unwrap();
    assert_eq!(p_ident.min_ms, 5.0);
    assert_eq!(p_ident.max_ms, 5.0);
    assert_eq!(p_ident.mean_ms, 5.0);
    assert_eq!(p_ident.p50_ms, 5.0);
    assert_eq!(p_ident.stddev_ms, 0.0);
}

#[test]
fn test_interpolate_percentile_boundaries() {
    let sorted = vec![10.0, 20.0, 30.0];
    assert_eq!(interpolate_percentile(&sorted, 0.0), 10.0);
    assert_eq!(interpolate_percentile(&sorted, 100.0), 30.0);
    assert_eq!(interpolate_percentile(&sorted, 50.0), 20.0);
}

#[test]
fn test_bufferbloat_grading_heuristics() {
    // Scenario 1: A+ pristine connection (P50 = 10ms, P95 = 12ms, delta = 2ms)
    let p_aplus = Percentiles {
        min_ms: 9.5,
        max_ms: 13.0,
        mean_ms: 10.2,
        stddev_ms: 0.8,
        p50_ms: 10.0,
        p90_ms: 11.5,
        p95_ms: 12.0,
        p99_ms: 12.8,
        sample_count: 50,
    };
    let bb_aplus = BufferbloatAssessment::analyze(&p_aplus);
    assert_eq!(bb_aplus.grade, BufferbloatGrade::APlus);
    assert!(bb_aplus.delta_ms <= 5.0);

    // Scenario 2: Noticeable bufferbloat Grade C (P50 = 25ms, P95 = 70ms, delta = 45ms, ratio = 2.8x)
    let p_c = Percentiles {
        min_ms: 22.0,
        max_ms: 85.0,
        mean_ms: 35.0,
        stddev_ms: 15.0,
        p50_ms: 25.0,
        p90_ms: 60.0,
        p95_ms: 70.0,
        p99_ms: 80.0,
        sample_count: 50,
    };
    let bb_c = BufferbloatAssessment::analyze(&p_c);
    assert_eq!(bb_c.grade, BufferbloatGrade::C);

    // Scenario 3: Severe bufferbloat Grade D (P50 = 20ms, P95 = 90ms, delta = 70ms, ratio = 4.5x)
    let p_d = Percentiles {
        min_ms: 18.0,
        max_ms: 110.0,
        mean_ms: 45.0,
        stddev_ms: 25.0,
        p50_ms: 20.0,
        p90_ms: 75.0,
        p95_ms: 90.0,
        p99_ms: 105.0,
        sample_count: 50,
    };
    let bb_d = BufferbloatAssessment::analyze(&p_d);
    assert_eq!(bb_d.grade, BufferbloatGrade::D);

    // Scenario 4: Critical bufferbloat Grade F (P50 = 15ms, P95 = 350ms, delta = 335ms)
    let p_f = Percentiles {
        min_ms: 14.0,
        max_ms: 450.0,
        mean_ms: 95.0,
        stddev_ms: 80.0,
        p50_ms: 15.0,
        p90_ms: 280.0,
        p95_ms: 350.0,
        p99_ms: 420.0,
        sample_count: 50,
    };
    let bb_f = BufferbloatAssessment::analyze(&p_f);
    assert_eq!(bb_f.grade, BufferbloatGrade::F);
    assert!(bb_f.delta_ms > 250.0);
}
