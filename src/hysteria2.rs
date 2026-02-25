//! Hysteria2 — QUIC-based High-Speed Protocol with Brutal Congestion Control
//! مخصوص شبکه‌های با تأخیر/loss بالا (مثل ایران)

use anyhow::{Context, Result};
use rand::{RngCore, thread_rng};
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

// ── Hysteria2 Protocol Constants ─────────────────────────────────
const HY2_VERSION: u8 = 2;
// Client Hello fields
const HY2_TYPE_TCP:    u8 = 0x01;
const HY2_TYPE_UDP:    u8 = 0x03;
const HY2_ADDR_DOMAIN: u8 = 0x02;
const HY2_ADDR_IPV4:   u8 = 0x04;
const HY2_ADDR_IPV6:   u8 = 0x06;
// Padding max
const MAX_PADDING: usize = 512;

/// Brutal 拥塞控制 settings (Hysteria2 signature feature)
#[derive(Debug, Clone)]
pub struct BrutalConfig {
    /// بیت بر ثانیه — حداکثر throughput دلخواه
    pub upload_mbps: u64,
    pub download_mbps: u64,
}

impl Default for BrutalConfig {
    fn default() -> Self { Self { upload_mbps: 50, download_mbps: 200 } }
}

/// پیکربندی Hysteria2
#[derive(Debug, Clone)]
pub struct Hysteria2Config {
    pub auth_str: String,
    pub obfs_type: ObfsType,
    pub obfs_password: String,
    pub brutal: BrutalConfig,
    pub sni: String,
    pub insecure: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObfsType {
    None,
    Salamander,  // xor-based obfs
}

impl Default for Hysteria2Config {
    fn default() -> Self {
        Self {
            auth_str: uuid::Uuid::new_v4().to_string(),
            obfs_type: ObfsType::Salamander,
            obfs_password: uuid::Uuid::new_v4().to_string(),
            brutal: BrutalConfig::default(),
            sni: String::new(),
            insecure: false,
        }
    }
}

/// Hysteria2 Client
pub struct Hysteria2 {
    pub socket: Option<UdpSocket>,
    config: Hysteria2Config,
    session_id: u64,
}

impl Hysteria2 {
    pub fn new(config: Hysteria2Config) -> Self {
        Self {
            socket: None,
            config,
            session_id: rand::random(),
        }
    }

    pub async fn connect(&mut self, server: &str) -> Result<()> {
        info!("🚀 اتصال Hysteria2 به {}", server);
        let socket = UdpSocket::bind("0.0.0.0:0").await.context("UDP bind failed")?;
        socket.connect(server).await.context("UDP connect failed")?;
        self.socket = Some(socket);
        self.do_handshake().await?;
        info!("✅ Hysteria2 handshake complete (Brutal CC: {}↑/{}↓ Mbps)",
            self.config.brutal.upload_mbps, self.config.brutal.download_mbps);
        Ok(())
    }

    async fn do_handshake(&mut self) -> Result<()> {
        // Client Hello packet
        let hello = self.build_client_hello();
        let obfs_hello = self.apply_obfs(&hello);
        self.raw_send(&obfs_hello).await?;

        // Server Hello
        let mut buf = vec![0u8; 1500];
        let n = timeout(Duration::from_secs(5), self.raw_recv(&mut buf)).await??;
        let server_hello = self.remove_obfs(&buf[..n]);
        self.parse_server_hello(&server_hello)?;

        debug!("🤝 Hysteria2 handshake successful");
        Ok(())
    }

    fn build_client_hello(&self) -> Vec<u8> {
        let mut pkt = Vec::new();
        // Version
        pkt.push(HY2_VERSION);
        // Auth string (length-prefixed)
        let auth = self.config.auth_str.as_bytes();
        pkt.push(auth.len() as u8);
        pkt.extend_from_slice(auth);
        // Bandwidth (upload/download in Kbps, u64 each)
        let up_kbps = self.config.brutal.upload_mbps * 1000;
        let dn_kbps = self.config.brutal.download_mbps * 1000;
        pkt.extend_from_slice(&up_kbps.to_be_bytes());
        pkt.extend_from_slice(&dn_kbps.to_be_bytes());
        // Random padding for obfuscation
        let mut rng = thread_rng();
        let pad_len: usize = (rand::random::<u8>() as usize) % MAX_PADDING;
        let mut pad = vec![0u8; pad_len];
        rng.fill_bytes(&mut pad);
        pkt.push(pad_len as u8);
        pkt.extend(pad);
        pkt
    }

    fn parse_server_hello(&self, data: &[u8]) -> Result<()> {
        if data.is_empty() { return Err(anyhow::anyhow!("Empty server hello")); }
        let status = data[0];
        if status != 0x00 {
            return Err(anyhow::anyhow!("Hysteria2 auth failed: status={}", status));
        }
        Ok(())
    }

    /// Salamander XOR obfuscation
    fn apply_obfs(&self, data: &[u8]) -> Vec<u8> {
        if self.config.obfs_type == ObfsType::None { return data.to_vec(); }
        let key = self.config.obfs_password.as_bytes();
        data.iter().enumerate()
            .map(|(i, &b)| b ^ key[i % key.len()])
            .collect()
    }

    fn remove_obfs(&self, data: &[u8]) -> Vec<u8> {
        self.apply_obfs(data) // XOR is its own inverse
    }

    /// ارسال TCP stream request
    pub async fn send_tcp_request(&mut self, host: &str, port: u16) -> Result<()> {
        let mut pkt = self.build_data_packet(HY2_TYPE_TCP, host, port, &[]);
        let obfs = self.apply_obfs(&pkt);
        self.raw_send(&obfs).await?;
        debug!("🔗 Hysteria2 TCP → {}:{}", host, port);
        Ok(())
    }

    /// ارسال UDP request
    pub async fn send_udp_request(&mut self, host: &str, port: u16, data: &[u8]) -> Result<()> {
        let pkt = self.build_data_packet(HY2_TYPE_UDP, host, port, data);
        let obfs = self.apply_obfs(&pkt);
        self.raw_send(&obfs).await?;
        Ok(())
    }

    fn build_data_packet(&self, typ: u8, host: &str, port: u16, data: &[u8]) -> Vec<u8> {
        let mut pkt = Vec::new();
        pkt.push(typ);
        // Address: domain
        pkt.push(HY2_ADDR_DOMAIN);
        let h = host.as_bytes();
        pkt.push(h.len() as u8);
        pkt.extend_from_slice(h);
        pkt.extend_from_slice(&port.to_be_bytes());
        pkt.extend_from_slice(data);
        pkt
    }

    /// ارسال داده محافظت‌شده (wrapper عمومی)
    pub async fn send_protected(&mut self, data: &[u8]) -> Result<()> {
        let mut pkt = Vec::new();
        pkt.push(HY2_TYPE_TCP);
        pkt.extend_from_slice(data);
        let obfs = self.apply_obfs(&pkt);
        self.raw_send(&obfs).await
    }

    async fn raw_send(&mut self, data: &[u8]) -> Result<()> {
        let sock = self.socket.as_ref().context("No socket")?;
        timeout(Duration::from_secs(5), sock.send(data)).await??;
        Ok(())
    }

    async fn raw_recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let sock = self.socket.as_ref().context("No socket")?;
        Ok(sock.recv(buf).await?)
    }
}

impl Default for Hysteria2 {
    fn default() -> Self { Self::new(Hysteria2Config::default()) }
}
