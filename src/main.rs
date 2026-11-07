use std::process::ExitCode;
use clap::Parser;
use tokio::time::sleep;

use netpulse_rs::cli::Cli;
use netpulse_rs::engine::{tcp::TcpProber, udp::UdpProber, ProbeProtocol, ProbeSample};
use netpulse_rs::reporter::{json_export::JsonReporter, terminal::TerminalReporter, OutputFormat};
use netpulse_rs::stats::{Rfc3550Jitter, SessionSummary};

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let config = match cli.to_probe_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            return ExitCode::from(1);
        }
    };

    let format = cli.output_format();

    if format == OutputFormat::Text && !cli.quiet {
        TerminalReporter::print_banner(
            &config.target,
            config.port,
            &format!("{:?}", config.protocol),
            config.count,
        );
    }

    let mut samples: Vec<ProbeSample> = Vec::with_capacity(config.count);
    let mut jitter_estimator = Rfc3550Jitter::new();

    match config.protocol {
        ProbeProtocol::Tcp => {
            let mut prober = TcpProber::new(config.target.clone(), config.port, config.timeout);
            for seq in 1..=(config.count as u32) {
                let sample = prober.probe(seq).await;

                let current_jitter = if let Some(us) = sample.rtt_micros {
                    jitter_estimator.update(us as f64);
                    Some(jitter_estimator.jitter_ms())
                } else {
                    None
                };

                match format {
                    OutputFormat::Text => {
                        if !cli.quiet {
                            TerminalReporter::print_probe_tick(&sample, current_jitter);
                        }
                    }
                    OutputFormat::Ndjson => {
                        if let Ok(line) = JsonReporter::to_ndjson_line(&sample) {
                            println!("{}", line);
                        }
                    }
                    OutputFormat::Json => {}
                }

                samples.push(sample);

                if (seq as usize) < config.count {
                    sleep(config.interval).await;
                }
            }
        }
        ProbeProtocol::Udp => {
            let mut prober = UdpProber::new(config.target.clone(), config.port, config.timeout);
            for seq in 1..=(config.count as u32) {
                let sample = prober.probe(seq).await;

                let current_jitter = if let Some(us) = sample.rtt_micros {
                    jitter_estimator.update(us as f64);
                    Some(jitter_estimator.jitter_ms())
                } else {
                    None
                };

                match format {
                    OutputFormat::Text => {
                        if !cli.quiet {
                            TerminalReporter::print_probe_tick(&sample, current_jitter);
                        }
                    }
                    OutputFormat::Ndjson => {
                        if let Ok(line) = JsonReporter::to_ndjson_line(&sample) {
                            println!("{}", line);
                        }
                    }
                    OutputFormat::Json => {}
                }

                samples.push(sample);

                if (seq as usize) < config.count {
                    sleep(config.interval).await;
                }
            }
        }
    }

    let summary = SessionSummary::from_samples(&config, &samples, cli.max_jitter, cli.max_loss);

    match format {
        OutputFormat::Text => {
            if !cli.quiet {
                let latencies: Vec<f64> = samples.iter().filter_map(|s| s.rtt_ms()).collect();
                TerminalReporter::render_histogram(&latencies);
            }
            TerminalReporter::print_summary(&summary);
        }
        OutputFormat::Json => match JsonReporter::to_pretty_json(&summary) {
            Ok(json_str) => println!("{}", json_str),
            Err(e) => {
                eprintln!("JSON serialization error: {}", e);
                return ExitCode::from(1);
            }
        },
        OutputFormat::Ndjson => {
            if let Ok(summary_line) = JsonReporter::to_ndjson_line(&summary) {
                println!("{}", summary_line);
            }
        }
    }

    if summary.sla_breach.is_some() {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    }
}
