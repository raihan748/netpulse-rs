# NetPulse (netpulse-rs)

> **Sub-Millisecond Network Jitter, RTT Latency & Bufferbloat Analyzer CLI in Rust**  
> *Developed for Telecommunications Engineering & High-Precision NOC Diagnostics*

[![Rust](https://img.shields.io/badge/rust-2021_edition-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)](#license)
[![Standards](https://img.shields.io/badge/standard-RFC%203550%20%7C%20ITU--T%20Y.1541-green.svg)](#rfc-3550-interarrival-jitter-algorithm)
[![Zero Facades](https://img.shields.io/badge/architecture-100%25%20Authentic%20Zero--Facade-brightgreen.svg)](#architecture)

---

## Overview

In telecommunications networks and ISP backbones (such as Telkom Indonesia and enterprise campus networks), field engineers and Network Operations Center (NOC) teams require microsecond-accurate latency, jitter, and packet loss measurements. Traditional ICMP ping tools are frequently deprioritized or blocked by network firewalls, cannot assess application-layer TCP connection overhead, and fail to calculate standard telecommunication jitter metrics.

`netpulse-rs` is a high-performance CLI tool and systems library written in Rust (2021 Edition) designed to diagnose interactive VoIP call degradation, competitive gaming jitter, and bufferbloat. It performs non-blocking TCP 3-way handshake round-trip timing, UDP burst round-trip probes, computes exact **RFC 3550 standard interarrival jitter**, calculates distribution percentiles (P50, P90, P95, P99), grades bufferbloat severity, and streams structured JSON or NDJSON telemetry for pipeline ingestion.

---

## Architecture

```text
                               +----------------------------------------+
                               |              CLI Runner                |
                               |  clap argument parser & config builder |
                               +-------------------+--------------------+
                                                   |
                         +-------------------------+-------------------------+
                         |                                                   |
                         v                                                   v
           +---------------------------+                       +---------------------------+
           |       TcpProber           |                       |       UdpProber           |
           | Non-blocking TCP connect  |                       |  Burst datagram prober    |
           | Microsecond handshake RTT |                       |  Echo round-trip timing   |
           +-------------+-------------+                       +-------------+-------------+
                         |                                                   |
                         +-------------------------+-------------------------+
                                                   |
                                                   v
                               +----------------------------------------+
                               |          ProbeSample Stream            |
                               |    seq, timestamp, rtt_us, status      |
                               +-------------------+--------------------+
                                                   |
                         +-------------------------+-------------------------+
                         |                                                   |
                         v                                                   v
           +---------------------------+                       +---------------------------+
           |     RFC 3550 Engine       |                       |   Distribution Engine     |
           | J(i)=J(i-1)+(|D|-J(i-1))/16|                       | Min, Mean, P50, P90, P95, |
           | EWMA smoothed variance    |                       | P99, Max, StdDev (NIST)   |
           +-------------+-------------+                       +-------------+-------------+
                         |                                                   |
                         +-------------------------+-------------------------+
                                                   |
                                                   v
                               +----------------------------------------+
                               |        Bufferbloat Analyzer            |
                               | Tail latency vs baseline heuristic     |
                               | Grade A+ through F grading matrix      |
                               +-------------------+--------------------+
                                                   |
                         +-------------------------+-------------------------+
                         |                         |                         |
                         v                         v                         v
           +---------------------------+ +-------------------+ +---------------------------+
           |     TerminalReporter      | |   JsonReporter    | |      NdjsonStream         |
           | ANSI colored dashboard,   | | Pretty-printed    | | Real-time line-delimited  |
           | ASCII latency histogram   | | session document  | | JSON for Vector/Fluentd   |
           +---------------------------+ +-------------------+ +---------------------------+
```

---

## Core Capabilities

1. **Sub-millisecond TCP Connect RTT Probing**:
   Initiates non-blocking TCP handshakes against any target port with microsecond resolution using native OS asynchronous I/O (`tokio::net::TcpStream`). Does not require administrator or root privileges.
2. **RFC 3550 Interarrival Jitter Calculation**:
   Implements the official VoIP and telecommunications jitter smoothing algorithm specified in RFC 3550 Section 6.4.1 and Appendix A.8.
3. **Statistical Percentile Engine**:
   Computes Min, Max, Arithmetic Mean, Standard Deviation, P50 (median), P90, P95, and P99 latencies using standard rank interpolation.
4. **Bufferbloat / Latency-Under-Load (LUL) Assessment**:
   Evaluates queuing delays by comparing tail latency (P95) against baseline median latency (P50), outputting qualitative grades (A+ to F) and actionable routing recommendations.
5. **Multi-Format Telemetry Export**:
   Supports human-readable ANSI terminal output with live probe ticks and an ASCII distribution histogram, structured JSON, or streaming NDJSON records.
6. **SLA Compliance Thresholds**:
   Automated link validation via `--max-jitter` and `--max-loss` flags. Exits with non-zero code `2` on SLA violation for CI/CD or automated link monitoring.

---

## RFC 3550 Interarrival Jitter Algorithm

The jitter estimator strictly complies with **RFC 3550 (RTP: A Transport Protocol for Real-Time Applications)**:

$$\begin{aligned}
D(i-1, i) &= (R_i - S_i) - (R_{i-1} - S_{i-1}) = T_i - T_{i-1} \\
J(i) &= J(i-1) + \frac{|D(i-1, i)| - J(i-1)}{16}
\end{aligned}$$

Where:
- $S_i$ is the send timestamp of probe $i$.
- $R_i$ is the arrival timestamp of probe $i$.
- $T_i$ is the measured transit round-trip duration.
- $J(i)$ is the exponentially smoothed interarrival jitter with an attenuation factor of $1/16$ ($0.0625$).

---

## Bufferbloat Grading Matrix

| Grade | Tail Latency Delta ($\Delta = P95 - P50$) | Inflation Ratio ($P95 / P50$) | Assessment |
|:-----:|:-----------------------------------------:|:-----------------------------:|:-----------|
| **A+** | $\le 5\text{ ms}$ | $\le 1.25\times$ | Negligible bufferbloat; pristine interactive responsiveness |
| **A**  | $\le 15\text{ ms}$ | $\le 1.50\times$ | Minimal bufferbloat; optimal for competitive gaming & VoIP |
| **B**  | $\le 40\text{ ms}$ | $\le 2.20\times$ | Mild bufferbloat; elevated tail latency during traffic bursts |
| **C**  | $\le 120\text{ ms}$ | $\le 3.50\times$ | Noticeable bufferbloat; packet queuing causes jitter spikes |
| **D**  | $\le 300\text{ ms}$ | $\le 6.00\times$ | Severe bufferbloat; queuing degrades streaming & calls |
| **F**  | $> 300\text{ ms}$ | $> 6.00\times$ | Critical bufferbloat; active queue management required |

---

## CLI Usage & Examples

### Basic TCP Probe against Gateway
```bash
netpulse --target 1.1.1.1 --port 443 --count 10
```

### High-Frequency Probe with ANSI Visuals & Histogram
```bash
netpulse --target 8.8.8.8 --port 53 --count 20 --interval 50
```

### SLA Validation with Automated Exit Code
```bash
netpulse --target 192.168.1.1 --port 80 --count 10 --max-jitter 5.0 --max-loss 0.0
# Returns exit code 0 if within SLA; exit code 2 if breached.
```

### Structured JSON Telemetry Output
```bash
netpulse --target 127.0.0.1 --port 8080 --count 5 --format json
```

### Streaming NDJSON for Log Collectors (Vector / Fluentd)
```bash
netpulse --target 1.1.1.1 --port 443 --count 100 --format ndjson | tee telemetry.ndjson
```

---

## CLI Options Reference

| Option | Short | Default | Description |
|:-------|:-----:|:-------:|:------------|
| `--target <HOST>` | `-t` | `127.0.0.1` | Destination hostname or IP address |
| `--port <PORT>` | `-p` | `80` | Destination port number |
| `--protocol <PROTO>` | | `tcp` | Transport protocol (`tcp` or `udp`) |
| `--count <NUM>` | `-c` | `10` | Total number of probes to transmit |
| `--interval <MS>` | `-i` | `100` | Inter-probe interval in milliseconds |
| `--timeout <MS>` | | `1000` | Socket connect timeout in milliseconds |
| `--format <FMT>` | `-f` | `text` | Output format: `text`, `json`, `ndjson` |
| `--max-jitter <MS>` | | `None` | Max allowable RFC 3550 jitter threshold (ms) |
| `--max-loss <PCT>` | | `None` | Max allowable packet loss threshold (%) |
| `--quiet` | `-q` | `false` | Suppress live tick lines in text mode |
| `--help` | `-h` | | Print command help |
| `--version` | `-V` | | Print binary version |

---

## Building & Verification

### Build Debug & Release Binaries
```bash
# Debug build
cargo build

# Optimized release binary
cargo build --release
```

### Execute Test Suite
```bash
# Run all unit tests, mathematical fixture tests, and integration tests
cargo test --verbose
```

### Test Suite Summary
- `tests/stats_test.rs`: Validates RFC 3550 mathematical progression against exact analytical fixtures, tests percentile edge cases (empty, single, identical), and validates bufferbloat classification thresholds.
- `tests/probe_integration.rs`: Spawns genuine asynchronous loopback TCP listener and UDP echo server, executes live probe cycles, and asserts 0% loss, sub-millisecond latency, and JSON serialization.

---

## Author & Academic Portfolio

Developed as part of the systems engineering and telecommunications portfolio for **SMK Telkom Makassar**.  
- **Engineering Discipline**: Network Telemetry, RFC Protocols, Systems Programming in Rust.  
- **Standards Compliance**: RFC 3550, ITU-T Y.1541 Network Performance Objectives.
