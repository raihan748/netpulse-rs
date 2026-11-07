use std::net::SocketAddr;
use std::time::{Duration, Instant};
use chrono::Utc;
use tokio::net::lookup_host;

use crate::engine::ProbeSample;
use crate::error::{NetPulseError, Result};

/// TCP Connect Prober measuring 3-way handshake round trip time.
pub struct TcpProber {
    target: String,
    port: u16,
    timeout: Duration,
    resolved_addr: Option<SocketAddr>,
}

impl TcpProber {
    pub fn new(target: String, port: u16, timeout: Duration) -> Self {
        Self {
            target,
            port,
            timeout,
            resolved_addr: None,
        }
    }

    /// Resolve destination hostname or IP to SocketAddr.
    pub async fn resolve(&mut self) -> Result<SocketAddr> {
        if let Some(addr) = self.resolved_addr {
            return Ok(addr);
        }

        let address_str = format!("{}:{}", self.target, self.port);
        let mut addrs = lookup_host(&address_str)
            .await
            .map_err(|source| NetPulseError::DnsResolution {
                target: self.target.clone(),
                source,
            })?;

        let addr = addrs.next().ok_or_else(|| {
            NetPulseError::InvalidTarget(format!("No IP address found for {}", address_str))
        })?;

        self.resolved_addr = Some(addr);
        Ok(addr)
    }

    /// Execute a single TCP connect probe and record microsecond RTT.
    pub async fn probe(&mut self, sequence: u32) -> ProbeSample {
        let timestamp = Utc::now();
        let target_addr = match self.resolve().await {
            Ok(addr) => addr,
            Err(e) => {
                return ProbeSample {
                    sequence,
                    timestamp,
                    rtt_micros: None,
                    success: false,
                    error_message: Some(e.to_string()),
                };
            }
        };

        let start = Instant::now();
        let connect_future = tokio::net::TcpStream::connect(target_addr);

        match tokio::time::timeout(self.timeout, connect_future).await {
            Ok(Ok(stream)) => {
                let elapsed = start.elapsed();
                let micros = elapsed.as_micros() as u64;

                // Explicitly disable Nagle's algorithm to ensure immediate closure packet transmission
                let _ = stream.set_nodelay(true);
                drop(stream);

                ProbeSample {
                    sequence,
                    timestamp,
                    rtt_micros: Some(micros),
                    success: true,
                    error_message: None,
                }
            }
            Ok(Err(io_err)) => ProbeSample {
                sequence,
                timestamp,
                rtt_micros: None,
                success: false,
                error_message: Some(format!("Connection failed: {}", io_err)),
            },
            Err(_) => ProbeSample {
                sequence,
                timestamp,
                rtt_micros: None,
                success: false,
                error_message: Some(format!("Timeout after {:?}", self.timeout)),
            },
        }
    }
}
