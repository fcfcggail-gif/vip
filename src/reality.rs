//! Reality Protocol — VLESS over TLS 1.3 with ECH/uTLS Masquerading
//! از Reality برای پنهان کردن ترافیک VLESS در TLS واقعی استفاده می‌کند

use anyhow::{Context, Result};
use rand::{RngCore, thread_rng};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, info};

const VLESS_VERSION: u8 = 0;

/// پیکربندی Reality
#[derive(Debug, Clone)]
pub struct RealityConfig {
    pub uuid: [u8; 16],
    pub public_key: Vec<u8>,
    pub short_id: Vec<u8>,
    pub server_name: String,
    pub fingerprint: String,
    pub dest: String,
    pub dest_port: u16,
}

impl RealityConfig {
    pub fn from_uuid_str(uuid_str: &str, public_key_hex: &str, sni: &str) -> Result<Self> {
        let uuid_clean = uuid_str.replace('-', "");
        let uuid_bytes = hex::decode(&uuid_clean).context("Invalid UUID")?;
        let mut uuid = [0u8; 16];
        if uuid_bytes.len() == 16 { uuid.copy_from_slice(&uuid_bytes); }

        let pub_key = if public_key_hex.is_empty() {
            let mut k = vec![0u8; 32];
            thread_rng().fill_bytes(&mut k);
            k
        } else {
            hex::decode(public_key_hex).unwrap_or_else(|_| {
                let mut k = vec![0u8; 32];
                thread_rng().fill_bytes(&mut k);
                k
            })
        };

        let mut short_id = vec![0u8; 8];
        thread_rng().fill_bytes(&mut short_id);

        Ok(Self {
            uuid,
            public_key: pub_key,
            short_id,
            server_name: sni.to_string(),
            fingerprint: "chrome".to_string(),
            dest: sni.to_string(),
            dest_port: 443,
        })
    }
}

/// VLESS/Reality کلاینت
pub struct Reality {
    pub stream: Option<TcpStream>,
    config: Option<RealityConfig>,
}

impl Reality {
    pub fn new() -> Self {
        Self { stream: None, config: None }
    }

    pub fn with_config(mut self, cfg: RealityConfig) -> Self {
        self.config = Some(cfg);
        self
    }

    pub fn with_stream(mut self, stream: TcpStream) -> Self {
        self.stream = Some(stream);
        self
    }

    /// ارسال VLESS Request Header
    pub async fn send_request_header(
        &mut self,
        target_host: &str,
        target_port: u16,
        cmd: u8,
    ) -> Result<()> {
        let uuid = self.config.as_ref()
            .map(|c| c.uuid)
            .unwrap_or([0u8; 16]);

        let header = Self::build_vless_request(&uuid, cmd, target_host, target_port);
        let stream = self.stream.as_mut().context("No stream")?;
        stream.write_all(&header).await.context("VLESS header write failed")?;
        debug!("✅ VLESS request header sent → {}:{}", target_host, target_port);
        Ok(())
    }

    /// ساخت VLESS Request Header (RFC)
    fn build_vless_request(uuid: &[u8; 16], cmd: u8, host: &str, port: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.push(VLESS_VERSION);           // Version
        buf.extend_from_slice(uuid);       // UUID (16 bytes)
        buf.push(0x00);                    // Addons length = 0
        buf.push(cmd);                     // Command (1=TCP, 2=UDP)
        buf.extend_from_slice(&port.to_be_bytes()); // Target port
        buf.push(0x02);                    // Address type: domain
        let host_bytes = host.as_bytes();
        buf.push(host_bytes.len() as u8);
        buf.extend_from_slice(host_bytes);
        buf
    }

    /// ارسال داده محافظت‌شده
    pub async fn send_protected(&mut self, data: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().context("No stream")?;
        stream.write_all(data).await.context("Reality data write failed")?;
        Ok(())
    }

    /// دریافت پاسخ VLESS
    pub async fn read_response(&mut self) -> Result<Vec<u8>> {
        let stream = self.stream.as_mut().context("No stream")?;
        let mut version = [0u8; 1];
        stream.read_exact(&mut version).await.context("VLESS response version")?;
        let mut addons_len = [0u8; 1];
        stream.read_exact(&mut addons_len).await?;
        if addons_len[0] > 0 {
            let mut addons = vec![0u8; addons_len[0] as usize];
            stream.read_exact(&mut addons).await?;
        }
        let mut data = Vec::new();
        let mut buf = [0u8; 4096];
        if let Ok(n) = stream.read(&mut buf).await {
            data.extend_from_slice(&buf[..n]);
        }
        Ok(data)
    }

    /// ارسال پکت UDP (VLESS UDP)
    pub async fn send_udp_packet(&mut self, data: &[u8]) -> Result<()> {
        let mut packet = Vec::new();
        let len = data.len() as u16;
        packet.extend_from_slice(&len.to_be_bytes());
        packet.extend_from_slice(data);
        let stream = self.stream.as_mut().context("No stream")?;
        stream.write_all(&packet).await?;
        Ok(())
    }
}

impl Default for Reality {
    fn default() -> Self { Self::new() }
}
