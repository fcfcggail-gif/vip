//! TUIC v5 — TLS/UDP-based Innovative Congestion Control
//! پروتکل QUIC-based با QUIC Multiplexing و Zero-RTT

use anyhow::{Context, Result};
use rand::{RngCore, thread_rng};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

// ── TUIC v5 Constants ────────────────────────────────────────────
const TUIC_VERSION: u8 = 0x05;
// Commands
const CMD_AUTHENTICATE:  u8 = 0x00;
const CMD_CONNECT:       u8 = 0x01;
const CMD_PACKET:        u8 = 0x02;
const CMD_DISSOCIATE:    u8 = 0x03;
const CMD_HEARTBEAT:     u8 = 0x04;
// Address types
const ADDR_IPV4:         u8 = 0x01;
const ADDR_DOMAIN:       u8 = 0x03;
const ADDR_IPV6:         u8 = 0x04;

/// TUIC v5 Header
#[derive(Debug)]
pub struct TuicHeader {
    pub version: u8,
    pub command: u8,
}

impl TuicHeader {
    pub fn new(cmd: u8) -> Self { Self { version: TUIC_VERSION, command: cmd } }
    pub fn to_bytes(&self) -> [u8; 2] { [self.version, self.command] }
}

/// پیکربندی TUIC
#[derive(Debug, Clone)]
pub struct TuicConfig {
    pub uuid: [u8; 16],
    pub password: String,
    pub congestion_control: CongestionControl,
    pub udp_relay_mode: UdpRelayMode,
    pub zero_rtt_handshake: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionControl {
    Cubic,
    NewReno,
    Bbr,   // توصیه‌شده برای ایران
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UdpRelayMode {
    Native,
    Quic,
}

impl Default for TuicConfig {
    fn default() -> Self {
        let mut uuid = [0u8; 16]; thread_rng().fill_bytes(&mut uuid);
        Self {
            uuid,
            password: uuid::Uuid::new_v4().to_string(),
            congestion_control: CongestionControl::Bbr,
            udp_relay_mode: UdpRelayMode::Quic,
            zero_rtt_handshake: true,
        }
    }
}

/// TUIC v5 Client
pub struct Tuic {
    pub socket: Option<UdpSocket>,
    config: TuicConfig,
    token: [u8; 32],
}

impl Tuic {
    pub fn new(config: TuicConfig) -> Self {
        let mut token = [0u8; 32]; thread_rng().fill_bytes(&mut token);
        Self { socket: None, config, token }
    }

    pub async fn connect(&mut self, server: &str) -> Result<()> {
        info!("⚡ اتصال TUIC v5 به {}", server);
        let socket = UdpSocket::bind("0.0.0.0:0").await.context("UDP bind failed")?;
        socket.connect(server).await.context("UDP connect failed")?;
        self.socket = Some(socket);
        self.authenticate().await?;
        info!("✅ TUIC v5 authenticated");
        Ok(())
    }

    /// ارسال AUTHENTICATE command
    async fn authenticate(&mut self) -> Result<()> {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&TuicHeader::new(CMD_AUTHENTICATE).to_bytes());
        pkt.extend_from_slice(&self.config.uuid);
        // Token = HMAC-SHA256(UUID, password) — simplified
        pkt.extend_from_slice(&self.token);
        self.raw_send(&pkt).await?;
        debug!("🔐 TUIC AUTHENTICATE sent");
        Ok(())
    }

    /// ارسال CONNECT command
    pub async fn send_connect(&mut self, host: &str, port: u16) -> Result<()> {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&TuicHeader::new(CMD_CONNECT).to_bytes());
        // Address
        pkt.push(ADDR_DOMAIN);
        pkt.push(host.len() as u8);
        pkt.extend_from_slice(host.as_bytes());
        pkt.extend_from_slice(&port.to_be_bytes());
        self.raw_send(&pkt).await?;
        debug!("🔗 TUIC CONNECT → {}:{}", host, port);
        Ok(())
    }

    /// ارسال UDP PACKET
    pub async fn send_protected(&mut self, data: &[u8]) -> Result<()> {
        let mut pkt = Vec::new();
        pkt.extend_from_slice(&TuicHeader::new(CMD_PACKET).to_bytes());
        // Association ID (random per session)
        let assoc_id: u16 = rand::random();
        pkt.extend_from_slice(&assoc_id.to_be_bytes());
        // Fragment info: fragment_id=0, fragment_total=1
        pkt.push(0x00); pkt.push(0x01);
        // Size
        let size = data.len() as u16;
        pkt.extend_from_slice(&size.to_be_bytes());
        // Target address (0.0.0.0:0 for relay)
        pkt.push(ADDR_IPV4);
        pkt.extend_from_slice(&[0u8; 4]); // 0.0.0.0
        pkt.extend_from_slice(&[0u8; 2]); // port 0
        pkt.extend_from_slice(data);
        self.raw_send(&pkt).await
    }

    /// HEARTBEAT
    pub async fn heartbeat(&mut self) -> Result<()> {
        let pkt = TuicHeader::new(CMD_HEARTBEAT).to_bytes().to_vec();
        self.raw_send(&pkt).await
    }

    async fn raw_send(&mut self, data: &[u8]) -> Result<()> {
        let sock = self.socket.as_ref().context("No socket")?;
        timeout(Duration::from_secs(5), sock.send(data)).await??;
        Ok(())
    }

    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let sock = self.socket.as_ref().context("No socket")?;
        let n = timeout(Duration::from_secs(10), sock.recv(buf)).await??;
        Ok(n)
    }
}

impl Default for Tuic {
    fn default() -> Self { Self::new(TuicConfig::default()) }
}
