//! MASQUE (HTTP/3 CONNECT-UDP) — RFC 9298/9297
//! ترافیک UDP را در HTTP/3 (QUIC) بسته‌بندی می‌کند

use std::net::{IpAddr, SocketAddr};
use anyhow::{Context, Result};
use tokio::{net::UdpSocket, time::{timeout, Duration}};
use tracing::{debug, info};
use rand::thread_rng;

const CAPSULE_TYPE_DATAGRAM: u32 = 0x00;
const CAPSULE_TYPE_CLOSE:    u32 = 0x01;
const MAX_UDP_PAYLOAD: usize = 1200;

/// Capsule (RFC 9297)
#[derive(Debug)]
pub struct Capsule {
    pub capsule_type: u32,
    pub data: Vec<u8>,
}

impl Capsule {
    pub fn datagram(payload: &[u8]) -> Self {
        Self { capsule_type: CAPSULE_TYPE_DATAGRAM, data: payload.to_vec() }
    }

    pub fn close() -> Self {
        Self { capsule_type: CAPSULE_TYPE_CLOSE, data: vec![] }
    }

    /// Encode with variable-length integer (VarInt per RFC 9000)
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&Self::encode_varint(self.capsule_type as u64));
        out.extend_from_slice(&Self::encode_varint(self.data.len() as u64));
        out.extend_from_slice(&self.data);
        out
    }

    fn encode_varint(v: u64) -> Vec<u8> {
        if v < 64 { vec![v as u8] }
        else if v < 16384 {
            let n = v | 0x4000;
            vec![(n >> 8) as u8, (n & 0xFF) as u8]
        } else if v < 1073741824 {
            let n = v | 0x80000000;
            vec![(n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, (n & 0xFF) as u8]
        } else {
            let n = v | 0xC000000000000000;
            n.to_be_bytes().to_vec()
        }
    }
}

/// پیکربندی MASQUE
#[derive(Debug, Clone)]
pub struct MasqueConfig {
    pub proxy_host: String,
    pub proxy_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub path_template: String,
}

impl Default for MasqueConfig {
    fn default() -> Self {
        Self {
            proxy_host: String::new(),
            proxy_port: 443,
            target_host: String::new(),
            target_port: 443,
            path_template: "/.well-known/masque/udp/{target_host}/{target_port}/".to_string(),
        }
    }
}

/// کلاینت MASQUE
pub struct MasqueClient {
    server: IpAddr,
    port: u16,
    socket: Option<UdpSocket>,
    stream_id: u64,
    config: MasqueConfig,
    context_id: u64,
}

impl MasqueClient {
    pub fn new(server: IpAddr, port: u16) -> Self {
        Self {
            server, port,
            socket: None,
            stream_id: 0,
            config: MasqueConfig::default(),
            context_id: 0,
        }
    }

    pub fn with_config(mut self, cfg: MasqueConfig) -> Self {
        self.config = cfg;
        self
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("📦 اتصال MASQUE (H3 CONNECT-UDP) به {}:{}", self.server, self.port);
        let socket = UdpSocket::bind("0.0.0.0:0").await.context("UDP bind")?;
        socket.connect(SocketAddr::new(self.server, self.port)).await.context("UDP connect")?;
        self.socket = Some(socket);
        self.send_connect().await?;
        info!("✅ MASQUE connection established");
        Ok(())
    }

    async fn send_connect(&mut self) -> Result<()> {
        let connect_pkt = self.build_connect_request();
        let sock = self.socket.as_ref().context("No socket")?;
        timeout(Duration::from_secs(5), sock.send(&connect_pkt)).await??;

        let mut buf = vec![0u8; 2048];
        let n = timeout(Duration::from_secs(5), sock.recv(&mut buf)).await??;
        debug!("📩 MASQUE server response: {} bytes", n);
        Ok(())
    }

    fn build_connect_request(&self) -> Vec<u8> {
        // Simplified QUIC Initial-like packet to establish MASQUE proxy
        let mut pkt = Vec::new();
        // Fake QUIC Initial (for VPN traversal simulation)
        pkt.push(0xC0); // Long Header
        pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]); // QUIC v1
        // CONNECT-UDP request simulation
        let path = self.config.path_template
            .replace("{target_host}", &self.config.target_host)
            .replace("{target_port}", &self.config.target_port.to_string());
        let req = format!(
            "CONNECT {} HTTP/3\r\nHost: {}:{}\r\nUpgrade: connect-udp\r\n\r\n",
            path, self.server, self.port
        );
        pkt.extend_from_slice(req.as_bytes());
        pkt
    }

    /// ارسال UDP payload via Capsule Protocol
    pub async fn send_capsule(&mut self, payload: &[u8]) -> Result<()> {
        // Split oversized payloads
        for chunk in payload.chunks(MAX_UDP_PAYLOAD) {
            let capsule = Capsule::datagram(chunk);
            let encoded = capsule.encode();
            let sock = self.socket.as_ref().context("No socket")?;
            timeout(Duration::from_secs(5), sock.send(&encoded)).await??;
        }
        Ok(())
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send_capsule(data).await
    }

    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let sock = self.socket.as_ref().context("No socket")?;
        let n = timeout(Duration::from_secs(10), sock.recv(buf)).await??;
        // Skip Capsule header (at minimum 2 bytes varint)
        let skip = 2.min(n);
        buf.copy_within(skip..n, 0);
        Ok(n.saturating_sub(skip))
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(sock) = &self.socket {
            let close = Capsule::close().encode();
            let _ = sock.send(&close).await;
        }
        info!("🔌 MASQUE connection closed");
        Ok(())
    }
}
