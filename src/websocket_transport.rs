//! WebSocket & HTTP Obfuscation Transport
//!
//! پوشاندن ترافیک در قالب WebSocket یا HTTP برای دور زدن DPI.
//! تمام ترافیک به‌صورت جریان WebSocket عادی به نظر می‌رسد.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::net::{IpAddr, SocketAddr};
use anyhow::{Context, Result};
use base64::Engine as _;
use rand::{Rng, RngCore};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};
use tracing::{debug, info};

// ── WebSocket Frame ────────────────────────────────────────────────────────

const WS_FIN: u8 = 0x80;
const WS_OPCODE_BINARY: u8 = 0x02;
const WS_OPCODE_TEXT: u8 = 0x01;
const WS_OPCODE_PING: u8 = 0x09;
const WS_OPCODE_PONG: u8 = 0x0A;
const WS_OPCODE_CLOSE: u8 = 0x08;
const WS_MASK_BIT: u8 = 0x80;

/// یک فریم WebSocket
#[derive(Debug, Clone)]
pub struct WsFrame {
    pub fin: bool,
    pub opcode: u8,
    pub masked: bool,
    pub payload: Vec<u8>,
}

impl WsFrame {
    /// ساخت فریم Binary برای ارسال داده
    pub fn binary(payload: Vec<u8>) -> Self {
        Self { fin: true, opcode: WS_OPCODE_BINARY, masked: true, payload }
    }

    /// ساخت فریم Ping
    pub fn ping() -> Self {
        Self { fin: true, opcode: WS_OPCODE_PING, masked: true, payload: vec![] }
    }

    /// ساخت فریم Pong
    pub fn pong(data: Vec<u8>) -> Self {
        Self { fin: true, opcode: WS_OPCODE_PONG, masked: false, payload: data }
    }

    /// Encode فریم به bytes
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let first_byte = if self.fin { WS_FIN } else { 0 } | self.opcode;
        buf.push(first_byte);

        let payload_len = self.payload.len();
        if payload_len < 126 {
            buf.push(if self.masked { WS_MASK_BIT | payload_len as u8 } else { payload_len as u8 });
        } else if payload_len < 65536 {
            buf.push(if self.masked { WS_MASK_BIT | 126 } else { 126 });
            buf.extend((payload_len as u16).to_be_bytes());
        } else {
            buf.push(if self.masked { WS_MASK_BIT | 127 } else { 127 });
            buf.extend((payload_len as u64).to_be_bytes());
        }

        if self.masked {
            let mask: [u8; 4] = rand::random();
            buf.extend_from_slice(&mask);
            for (i, byte) in self.payload.iter().enumerate() {
                buf.push(byte ^ mask[i % 4]);
            }
        } else {
            buf.extend_from_slice(&self.payload);
        }

        buf
    }

    /// Decode فریم از bytes
    pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
        if data.len() < 2 { return None; }
        
        let fin = data[0] & 0x80 != 0;
        let opcode = data[0] & 0x0F;
        let masked = data[1] & 0x80 != 0;
        let mut payload_len = (data[1] & 0x7F) as usize;
        let mut offset = 2;

        if payload_len == 126 {
            if data.len() < offset + 2 { return None; }
            payload_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
            offset += 2;
        } else if payload_len == 127 {
            if data.len() < offset + 8 { return None; }
            payload_len = u64::from_be_bytes(data[offset..offset + 8].try_into().ok()?) as usize;
            offset += 8;
        }

        let mask_key = if masked {
            if data.len() < offset + 4 { return None; }
            let m = [data[offset], data[offset+1], data[offset+2], data[offset+3]];
            offset += 4;
            Some(m)
        } else { None };

        if data.len() < offset + payload_len { return None; }
        let raw_payload = &data[offset..offset + payload_len];
        let payload = if let Some(mask) = mask_key {
            raw_payload.iter().enumerate().map(|(i, &b)| b ^ mask[i % 4]).collect()
        } else {
            raw_payload.to_vec()
        };

        Some((Self { fin, opcode, masked: masked, payload }, offset + payload_len))
    }
}

// ── WebSocket Handshake ────────────────────────────────────────────────────

/// تولید Sec-WebSocket-Accept از Sec-WebSocket-Key
pub fn ws_accept_key(key: &str) -> String {
    let combined = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key);
    let hash = Sha1::digest(combined.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hash)
}

/// تولید Sec-WebSocket-Key تصادفی
pub fn generate_ws_key() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

// ── WebSocket Transport ────────────────────────────────────────────────────

/// تنظیمات WebSocket Transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsTransportConfig {
    pub host: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub ping_interval_secs: u64,
    pub use_tls: bool,
    pub early_data: bool,
}

impl Default for WsTransportConfig {
    fn default() -> Self {
        Self {
            host: "cdn.cloudflare.com".to_string(),
            path: "/ws".to_string(),
            headers: vec![
                ("User-Agent".to_string(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string()),
                ("Origin".to_string(), "https://cdn.cloudflare.com".to_string()),
            ],
            ping_interval_secs: 30,
            use_tls: true,
            early_data: false,
        }
    }
}

/// کلاینت WebSocket Transport
pub struct WsTransport {
    server: IpAddr,
    port: u16,
    config: WsTransportConfig,
    stream: Option<TcpStream>,
    connected: bool,
}

impl WsTransport {
    pub fn new(server: IpAddr, port: u16, config: WsTransportConfig) -> Self {
        Self { server, port, config, stream: None, connected: false }
    }

    /// اتصال و WebSocket Handshake
    pub async fn connect(&mut self) -> Result<()> {
        info!("🔌 اتصال WebSocket به {}:{}{}", self.server, self.port, self.config.path);
        
        let addr = SocketAddr::new(self.server, self.port);
        let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(addr))
            .await
            .context("WebSocket connection timeout")??;

        // ارسال HTTP Upgrade request
        let ws_key = generate_ws_key();
        let upgrade_req = self.build_upgrade_request(&ws_key);
        stream.write_all(upgrade_req.as_bytes()).await
            .context("WebSocket upgrade request failed")?;

        // دریافت و تأیید پاسخ
        let mut buf = vec![0u8; 4096];
        let n = timeout(Duration::from_secs(10), stream.read(&mut buf)).await??;
        let response = String::from_utf8_lossy(&buf[..n]);

        if !response.contains("101 Switching Protocols") {
            return Err(anyhow::anyhow!("WebSocket handshake rejected: {}", &response[..response.len().min(200)]));
        }

        // تأیید Sec-WebSocket-Accept
        let expected_accept = ws_accept_key(&ws_key);
        if !response.contains(&expected_accept) {
            debug!("⚠️ WebSocket accept key mismatch (ignored in Ghost mode)");
        }

        self.stream = Some(stream);
        self.connected = true;
        info!("✅ WebSocket connected ({}{})", self.config.host, self.config.path);
        Ok(())
    }

    /// ساخت HTTP Upgrade request
    fn build_upgrade_request(&self, key: &str) -> String {
        let mut req = format!(
            "GET {} HTTP/1.1\r\n\
             Host: {}\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Key: {}\r\n\
             Sec-WebSocket-Version: 13\r\n",
            self.config.path, self.config.host, key
        );

        for (name, value) in &self.config.headers {
            req.push_str(&format!("{}: {}\r\n", name, value));
        }
        req.push_str("\r\n");
        req
    }

    /// ارسال داده در قالب WebSocket binary frame
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let frame = WsFrame::binary(data.to_vec());
        let encoded = frame.encode();
        let stream = self.stream.as_mut().context("WebSocket not connected")?;
        stream.write_all(&encoded).await.context("WebSocket send failed")?;
        Ok(())
    }

    /// دریافت داده از WebSocket
    pub async fn recv(&mut self) -> Result<Vec<u8>> {
        let stream = self.stream.as_mut().context("WebSocket not connected")?;
        let mut buf = vec![0u8; 65536];
        let n = stream.read(&mut buf).await.context("WebSocket recv failed")?;
        
        if let Some((frame, _)) = WsFrame::decode(&buf[..n]) {
            match frame.opcode {
                WS_OPCODE_PING => {
                    // پاسخ به Ping
                    let pong = WsFrame::pong(frame.payload).encode();
                    stream.write_all(&pong).await.ok();
                    return Ok(vec![]);
                }
                WS_OPCODE_CLOSE => {
                    self.connected = false;
                    return Err(anyhow::anyhow!("WebSocket closed by server"));
                }
                _ => return Ok(frame.payload),
            }
        }
        
        Ok(buf[..n].to_vec())
    }

    /// ارسال Ping برای نگه‌داشتن اتصال
    pub async fn ping(&mut self) -> Result<()> {
        let frame = WsFrame::ping().encode();
        if let Some(stream) = self.stream.as_mut() {
            stream.write_all(&frame).await.ok();
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool { self.connected }
}

// ── HTTP/2 gRPC Transport ──────────────────────────────────────────────────

/// تنظیمات gRPC Transport (مشابه xhttp اما با Content-Type: application/grpc)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcTransportConfig {
    pub host: String,
    pub service_name: String,
    pub use_tls: bool,
    pub idle_timeout_secs: u64,
}

impl Default for GrpcTransportConfig {
    fn default() -> Self {
        Self {
            host: "grpc.google.com".to_string(),
            service_name: "GunService".to_string(),
            use_tls: true,
            idle_timeout_secs: 60,
        }
    }
}

/// کلاینت gRPC Transport (برای پوشاندن ترافیک در gRPC)
pub struct GrpcTransport {
    server: IpAddr,
    port: u16,
    config: GrpcTransportConfig,
    stream: Option<TcpStream>,
}

impl GrpcTransport {
    pub fn new(server: IpAddr, port: u16, config: GrpcTransportConfig) -> Self {
        Self { server, port, config, stream: None }
    }

    /// ارسال داده در قالب gRPC frame
    pub fn encode_grpc_frame(data: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(5 + data.len());
        frame.push(0); // compression flag: none
        frame.extend((data.len() as u32).to_be_bytes()); // message length
        frame.extend_from_slice(data);
        frame
    }

    /// Decode گرفتن frame گرفتن gRPC
    pub fn decode_grpc_frame(data: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 5 { return None; }
        let _compressed = data[0];
        let msg_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;
        if data.len() < 5 + msg_len { return None; }
        Some(data[5..5 + msg_len].to_vec())
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("🔌 اتصال gRPC به {}:{}", self.server, self.port);
        let addr = SocketAddr::new(self.server, self.port);
        let mut stream = timeout(Duration::from_secs(10), TcpStream::connect(addr)).await??;

        let preface = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
        stream.write_all(preface).await?;

        self.stream = Some(stream);
        info!("✅ gRPC transport connected");
        Ok(())
    }
}
