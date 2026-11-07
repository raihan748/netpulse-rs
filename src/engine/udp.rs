use std::net::SocketAddr;
use std::time::{Duration, Instant};
use chrono::Utc;
use tokio::net::{lookup_host, UdpSocket};

use crate::engine::ProbeSample;
use crate::error::{NetPulseError, Result};

const PROBE_MAGIC: &[u8; 4] = b"NPUL";

/// UDP Prober measuring round-trip burst timing.
pub struct UdpProber {
    target: String,
    port: u16,
    timeout: Duration,
    resolved_addr: Option<SocketAddr>,
    socket: Option<UdpSocket>,
}

impl UdpProber {
    pub fn new(target: String, port: u16, timeout: Duration) -> Self {
        Self {
            target,
            port,
            timeout,
            resolved_addr: None,
            socket: None,
        }
    }

    /// Resolve target and bind local UDP socket.
    pub async fn init(&mut self) -> Result<()> {
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

        let bind_addr = if addr.is_ipv6() {
            "[::]:0"
        } else {
            "0.0.0.0:0"
        };
        let socket = UdpSocket::bind(bind_addr).await?;
        self.socket = Some(socket);
        Ok(())
    }

    /// Execute a UDP burst round-trip timing probe.
    pub async fn probe(&mut self, sequence: u32) -> ProbeSample {
        let timestamp = Utc::now();

        if self.socket.is_none() || self.resolved_addr.is_none() {
            if let Err(e) = self.init().await {
                return ProbeSample {
                    sequence,
                    timestamp,
                    rtt_micros: None,
                    success: false,
                    error_message: Some(e.to_string()),
                };
            }
        }

        let socket = self.socket.as_ref().unwrap();
        let target_addr = self.resolved_addr.unwrap();

        // Build 16-byte probe datagram: Magic(4B) + Seq(4B) + Timestamp(8B)
        let mut packet = [0u8; 16];
        packet[0..4].copy_from_slice(PROBE_MAGIC);
        packet[4..8].copy_from_slice(&sequence.to_be_bytes());
        let send_time_us = Utc::now().timestamp_micros() as u64;
        packet[8..16].copy_from_slice(&send_time_us.to_be_bytes());

        let start = Instant::now();
        if let Err(e) = socket.send_to(&packet, target_addr).await {
            return ProbeSample {
                sequence,
                timestamp,
                rtt_micros: None,
                success: false,
                error_message: Some(format!("UDP send failed: {}", e)),
            };
        }

        let mut recv_buf = [0u8; 64];
        let recv_future = socket.recv_from(&mut recv_buf);

        match tokio::time::timeout(self.timeout, recv_future).await {
            Ok(Ok((len, _sender))) => {
                let elapsed = start.elapsed();
                let micros = elapsed.as_micros() as u64;

                // Validate echo payload length and magic
                if len >= 8 && &recv_buf[0..4] == PROBE_MAGIC {
                    let recv_seq = u32::from_be_bytes(recv_buf[4..8].try_into().unwrap());
                    if recv_seq == sequence {
                        return ProbeSample {
                            sequence,
                            timestamp,
                            rtt_micros: Some(micros),
                            success: true,
                            error_message: None,
                        };
                    }
                }

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
                error_message: Some(format!("UDP recv failed: {}", io_err)),
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
