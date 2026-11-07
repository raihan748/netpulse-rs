use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, UdpSocket};

use netpulse_rs::engine::{tcp::TcpProber, udp::UdpProber, ProbeConfig, ProbeProtocol, ProbeSample};
use netpulse_rs::reporter::json_export::JsonReporter;
use netpulse_rs::stats::SessionSummary;

#[tokio::test]
async fn test_tcp_probe_integration_loopback() {
    // 1. Bind loopback TCP listener on an OS-assigned ephemeral port
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind loopback TCP listener");
    let local_addr = listener.local_addr().expect("Failed to get local address");
    let port = local_addr.port();

    let is_running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&is_running);

    // 2. Spawn async background task to accept incoming probe handshakes
    let server_task = tokio::spawn(async move {
        while running_clone.load(Ordering::Relaxed) {
            match tokio::time::timeout(Duration::from_millis(50), listener.accept()).await {
                Ok(Ok((stream, _peer))) => {
                    // Close connection gracefully
                    drop(stream);
                }
                Ok(Err(_)) => break,
                Err(_) => {
                    // Accept loop timeout tick
                    continue;
                }
            }
        }
    });

    // 3. Execute 10 real TCP probes against localhost listener
    let mut prober = TcpProber::new("127.0.0.1".to_string(), port, Duration::from_millis(1000));
    let mut samples: Vec<ProbeSample> = Vec::with_capacity(10);

    for seq in 1..=10 {
        let sample = prober.probe(seq).await;
        samples.push(sample);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // 4. Terminate background server task
    is_running.store(false, Ordering::Relaxed);
    let _ = server_task.await;

    // 5. Verify telemetry invariants
    assert_eq!(samples.len(), 10);
    for (i, s) in samples.iter().enumerate() {
        assert_eq!(s.sequence, (i + 1) as u32);
        assert!(s.success, "Probe seq {} failed: {:?}", s.sequence, s.error_message);
        assert!(s.rtt_micros.is_some());
        let rtt_ms = s.rtt_ms().unwrap();
        assert!(rtt_ms > 0.0, "RTT must be positive");
        assert!(rtt_ms < 15.0, "Loopback RTT should be < 15ms, got {}ms", rtt_ms);
    }

    // 6. Aggregate into SessionSummary and verify statistics
    let config = ProbeConfig {
        target: "127.0.0.1".to_string(),
        port,
        protocol: ProbeProtocol::Tcp,
        count: 10,
        interval: Duration::from_millis(10),
        timeout: Duration::from_millis(1000),
    };

    let summary = SessionSummary::from_samples(&config, &samples, Some(5.0), Some(0.0));
    assert_eq!(summary.total_sent, 10);
    assert_eq!(summary.total_received, 10);
    assert_eq!(summary.loss_percent, 0.0);
    assert!(summary.sla_breach.is_none(), "SLA breach unexpected on loopback: {:?}", summary.sla_breach);

    let p = summary.percentiles.as_ref().expect("Percentiles must be present");
    assert!(p.min_ms <= p.p50_ms);
    assert!(p.p50_ms <= p.p95_ms);
    assert!(p.p95_ms <= p.max_ms);

    // Verify JSON serialization of the session
    let json_output = JsonReporter::to_pretty_json(&summary).expect("Serialization failed");
    assert!(json_output.contains("\"target\": \"127.0.0.1\""));
    assert!(json_output.contains("\"loss_percent\": 0.0"));
}

#[tokio::test]
async fn test_udp_probe_integration_loopback() {
    // 1. Bind loopback UDP socket on ephemeral port
    let server_socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind UDP server");
    let local_addr = server_socket.local_addr().expect("Failed to get UDP addr");
    let port = local_addr.port();

    let is_running = Arc::new(AtomicBool::new(true));
    let running_clone = Arc::clone(&is_running);

    // 2. Spawn echo reflector task
    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 64];
        while running_clone.load(Ordering::Relaxed) {
            match tokio::time::timeout(Duration::from_millis(50), server_socket.recv_from(&mut buf)).await {
                Ok(Ok((len, peer))) => {
                    let _ = server_socket.send_to(&buf[..len], peer).await;
                }
                Ok(Err(_)) => break,
                Err(_) => continue,
            }
        }
    });

    // 3. Execute 5 UDP burst probes
    let mut prober = UdpProber::new("127.0.0.1".to_string(), port, Duration::from_millis(500));
    let mut samples: Vec<ProbeSample> = Vec::with_capacity(5);

    for seq in 1..=5 {
        let sample = prober.probe(seq).await;
        samples.push(sample);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // 4. Terminate echo reflector
    is_running.store(false, Ordering::Relaxed);
    let _ = server_task.await;

    // 5. Invariant assertions
    assert_eq!(samples.len(), 5);
    for s in &samples {
        assert!(s.success, "UDP probe seq {} failed: {:?}", s.sequence, s.error_message);
        assert!(s.rtt_micros.is_some());
        assert!(s.rtt_ms().unwrap() < 15.0);
    }
}
