use std::io::{stdout, Write};
use crossterm::style::{Color, ResetColor, SetForegroundColor};
use crossterm::execute;

use crate::engine::ProbeSample;
use crate::stats::{BufferbloatGrade, SessionSummary};

pub struct TerminalReporter;

impl TerminalReporter {
    /// Render welcome header banner.
    pub fn print_banner(target: &str, port: u16, protocol: &str, count: usize) {
        let mut out = stdout();
        let _ = execute!(out, SetForegroundColor(Color::Cyan));
        println!("================================================================================");
        println!("  NetPulse v0.1.0 — Telecom Network Jitter & Bufferbloat Analyzer CLI");
        println!("================================================================================");
        let _ = execute!(out, ResetColor);
        println!(
            "Target: {}:{} ({}) | Probes: {} | Standards: RFC 3550, ITU-T Y.1541",
            target, port, protocol.to_uppercase(), count
        );
        println!("--------------------------------------------------------------------------------");
    }

    /// Print live probe result line.
    pub fn print_probe_tick(sample: &ProbeSample, current_jitter_ms: Option<f64>) {
        let mut out = stdout();
        if sample.success {
            let rtt = sample.rtt_ms().unwrap_or(0.0);
            let jitter_str = match current_jitter_ms {
                Some(j) => format!("jitter={:.3}ms", j),
                None => "jitter=--".to_string(),
            };

            let _ = execute!(out, SetForegroundColor(Color::Green));
            print!("[OK]  ");
            let _ = execute!(out, ResetColor);
            println!(
                "seq={:<3} rtt={:>7.3}ms  {:>16}  ts={}",
                sample.sequence,
                rtt,
                jitter_str,
                sample.timestamp.format("%H:%M:%S%.3f")
            );
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Red));
            print!("[DROP]");
            let _ = execute!(out, ResetColor);
            let err_msg = sample
                .error_message
                .as_deref()
                .unwrap_or("Timed out / connection failed");
            println!(
                "seq={:<3} rtt=   --- ms  error=\"{}\"",
                sample.sequence, err_msg
            );
        }
        let _ = stdout().flush();
    }

    /// Render ASCII latency distribution histogram.
    pub fn render_histogram(latencies_ms: &[f64]) {
        if latencies_ms.is_empty() {
            return;
        }

        println!("\nLatency Distribution Histogram:");
        let min = latencies_ms.iter().copied().fold(f64::INFINITY, f64::min);
        let max = latencies_ms.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        if (max - min).abs() < 0.001 {
            println!(
                "  [{:>7.2}ms - {:>7.2}ms]  ████████████████████ ({})",
                min, max, latencies_ms.len()
            );
            return;
        }

        let buckets_count = 5;
        let step = (max - min) / (buckets_count as f64);
        let mut counts = vec![0usize; buckets_count];

        for &val in latencies_ms {
            let mut idx = ((val - min) / step).floor() as usize;
            if idx >= buckets_count {
                idx = buckets_count - 1;
            }
            counts[idx] += 1;
        }

        let max_count = *counts.iter().max().unwrap_or(&1);
        let bar_width = 32;

        let mut out = stdout();
        for i in 0..buckets_count {
            let b_start = min + (i as f64) * step;
            let b_end = b_start + step;
            let c = counts[i];
            let bar_len = if max_count > 0 {
                (c * bar_width) / max_count
            } else {
                0
            };
            let bar: String = "█".repeat(bar_len);

            print!("  [{:>7.2}ms - {:>7.2}ms] ", b_start, b_end);
            let _ = execute!(out, SetForegroundColor(Color::Cyan));
            print!("{:<32}", bar);
            let _ = execute!(out, ResetColor);
            println!(" ({:>2})", c);
        }
    }

    /// Render comprehensive session summary.
    pub fn print_summary(summary: &SessionSummary) {
        let mut out = stdout();
        println!("\n================================================================================");
        println!("                             SESSION METRICS SUMMARY                            ");
        println!("================================================================================");

        // Transmission stats
        println!(
            "Packets: Sent = {}, Received = {}, Lost = {} ({:.1}% loss)",
            summary.total_sent,
            summary.total_received,
            summary.total_sent.saturating_sub(summary.total_received),
            summary.loss_percent
        );

        // Percentiles
        if let Some(ref p) = summary.percentiles {
            println!("\nLatency Percentiles (ms):");
            println!(
                "  Min    : {:>8.3} ms  |  Mean   : {:>8.3} ms",
                p.min_ms, p.mean_ms
            );
            println!(
                "  P50    : {:>8.3} ms  |  StdDev : {:>8.3} ms",
                p.p50_ms, p.stddev_ms
            );
            println!(
                "  P90    : {:>8.3} ms  |  P95    : {:>8.3} ms",
                p.p90_ms, p.p95_ms
            );
            println!(
                "  P99    : {:>8.3} ms  |  Max    : {:>8.3} ms",
                p.p99_ms, p.max_ms
            );

            // RFC 3550 Jitter
            if let Some(jitter) = summary.rfc3550_jitter_ms {
                println!("\nRFC 3550 Interarrival Jitter: {:.4} ms", jitter);
            }

            // Bufferbloat
            if let Some(ref bb) = summary.bufferbloat {
                println!("\nBufferbloat Assessment (Latency-Under-Load):");
                print!("  Grade          : ");
                match bb.grade {
                    BufferbloatGrade::APlus => {
                        let _ = execute!(out, SetForegroundColor(Color::Green));
                        print!("A+ (Optimal)");
                    }
                    BufferbloatGrade::A => {
                        let _ = execute!(out, SetForegroundColor(Color::Green));
                        print!("A  (Very Good)");
                    }
                    BufferbloatGrade::B => {
                        let _ = execute!(out, SetForegroundColor(Color::Blue));
                        print!("B  (Good)");
                    }
                    BufferbloatGrade::C => {
                        let _ = execute!(out, SetForegroundColor(Color::Yellow));
                        print!("C  (Fair)");
                    }
                    BufferbloatGrade::D => {
                        let _ = execute!(out, SetForegroundColor(Color::Magenta));
                        print!("D  (Poor)");
                    }
                    BufferbloatGrade::F => {
                        let _ = execute!(out, SetForegroundColor(Color::Red));
                        print!("F  (Critical)");
                    }
                }
                let _ = execute!(out, ResetColor);
                println!();
                println!("  Baseline RTT   : {:.3} ms (P50)", bb.baseline_ms);
                println!("  Tail RTT (P95) : {:.3} ms (Delta: +{:.3} ms, Ratio: {:.2}x)", bb.loaded_p95_ms, bb.delta_ms, bb.inflation_ratio);
                println!("  Diagnosis      : {}", bb.rating_summary);
                println!("  Recommendation : {}", bb.recommendation);
            }
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Red));
            println!("\nNo successful probe telemetry collected (100% loss).");
            let _ = execute!(out, ResetColor);
        }

        // SLA evaluation
        if let Some(ref breach) = summary.sla_breach {
            let _ = execute!(out, SetForegroundColor(Color::Red));
            println!("\n[!] SLA BREACH DETECTED:");
            println!("    {}", breach);
            let _ = execute!(out, ResetColor);
        } else {
            let _ = execute!(out, SetForegroundColor(Color::Green));
            println!("\n[✓] SLA Thresholds: All parameters within operational boundaries.");
            let _ = execute!(out, ResetColor);
        }
        println!("================================================================================\n");
    }
}
