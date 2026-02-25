//! XHTTP Client — HTTP/2 Chunked-Transfer Obfuscation Layer
//! ترافیک را در قالب HTTP/2 WebTransport پوشش می‌دهد

use std::net::{IpAddr, SocketAddr};
use anyhow::{Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};
use tracing::{debug, info};

const HTTP2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
const XHTTP_VERSION: &str = "2.0";

/// کلاینت XHTTP (HTTP/2 Chunked Obfuscation)
pub struct XhttpClient {
    server: IpAddr,
    port: u16,
    stream: Option<TcpStream>,
    session_id: String,
    path: String,
    host: String,
    stream_counter: u32,
}

impl XhttpClient {
    pub fn new(server: IpAddr, port: u16) -> Self {
        Self {
            server,
            port,
            stream: None,
            session_id: uuid::Uuid::new_v4().to_string(),
            path: format!("/{}", uuid::Uuid::new_v4().to_string().replace('-', "")),
            host: String::new(),
            stream_counter: 0,
        }
    }

    pub fn with_path(mut self, path: &str) -> Self {
        self.path = path.to_string();
        self
    }

    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_string();
        self
    }

    pub async fn connect(&mut self) -> Result<()> {
        info!("📄 اتصال XHTTP v{} به {}:{}", XHTTP_VERSION, self.server, self.port);
        let addr = SocketAddr::new(self.server, self.port);
        let stream = timeout(Duration::from_secs(10), TcpStream::connect(addr))
            .await
            .context("Connection timeout")?
            .context("TCP connection failed")?;
        self.stream = Some(stream);
        self.send_preface().await?;
        info!("✅ XHTTP connection established");
        Ok(())
    }

    async fn send_preface(&mut self) -> Result<()> {
        let stream = self.stream.as_mut().context("No connection")?;
        // HTTP/2 Connection Preface
        stream.write_all(HTTP2_PREFACE).await.context("HTTP/2 preface failed")?;

        // SETTINGS frame (type=0x4, flags=0x0, stream_id=0, length=18)
        let settings: &[u8] = &[
            0x00, 0x00, 0x12, // Length = 18 bytes
            0x04, 0x00,       // Type=SETTINGS, Flags=0
            0x00, 0x00, 0x00, 0x00, // Stream ID = 0
            // HEADER_TABLE_SIZE = 4096
            0x00, 0x01, 0x00, 0x00, 0x10, 0x00,
            // INITIAL_WINDOW_SIZE = 65535
            0x00, 0x04, 0x00, 0x00, 0xFF, 0xFF,
            // MAX_FRAME_SIZE = 16384
            0x00, 0x05, 0x00, 0x00, 0x40, 0x00,
        ];
        stream.write_all(settings).await.context("HTTP/2 SETTINGS failed")?;
        debug!("✅ HTTP/2 preface sent");
        Ok(())
    }

    /// ارسال درخواست CONNECT via HTTP/2 tunnel
    pub async fn send_connect_request(&mut self, target_host: &str, target_port: u16) -> Result<()> {
        self.stream_counter += 2; // HTTP/2 client streams are odd: 1, 3, 5...
        let stream_id = self.stream_counter - 1; // 1, 3, 5...

        let host_header = if self.host.is_empty() {
            format!("{}:{}", target_host, target_port)
        } else {
            self.host.clone()
        };

        // تولید HEADERS frame برای CONNECT
        let headers_payload = self.build_connect_headers(&host_header, target_host, target_port);
        let frame = self.build_http2_frame(0x01, 0x04, stream_id, &headers_payload);

        let stream = self.stream.as_mut().context("No connection")?;
        stream.write_all(&frame).await.context("CONNECT headers failed")?;

        // خواندن پاسخ
        let mut buf = vec![0u8; 1024];
        let _ = timeout(Duration::from_secs(5), stream.read(&mut buf)).await;
        debug!("✅ XHTTP CONNECT sent for stream #{}", stream_id);
        Ok(())
    }

    fn build_connect_headers(&self, authority: &str, host: &str, port: u16) -> Vec<u8> {
        // HPACK-encoded headers (simplified)
        let mut payload = Vec::new();
        // :method CONNECT
        payload.extend(&self.encode_header(":method", "CONNECT"));
        payload.extend(&self.encode_header(":authority", authority));
        payload.extend(&self.encode_header("x-session-id", &self.session_id));
        payload.extend(&self.encode_header("user-agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/120.0.0.0"));
        payload
    }

    fn encode_header(&self, name: &str, value: &str) -> Vec<u8> {
        // Literal Header Field - Never Indexed
        let mut h = Vec::new();
        h.push(0x10); // Never indexed
        // Name length + name
        h.push(name.len() as u8);
        h.extend_from_slice(name.as_bytes());
        // Value length + value
        h.push(value.len() as u8);
        h.extend_from_slice(value.as_bytes());
        h
    }

    fn build_http2_frame(&self, frame_type: u8, flags: u8, stream_id: u32, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(9 + payload.len());
        let len = payload.len() as u32;
        frame.push((len >> 16) as u8);
        frame.push((len >> 8) as u8);
        frame.push(len as u8);
        frame.push(frame_type);
        frame.push(flags);
        // Stream ID (31-bit, MSB reserved=0)
        frame.push(((stream_id >> 24) & 0x7F) as u8);
        frame.push((stream_id >> 16) as u8);
        frame.push((stream_id >> 8) as u8);
        frame.push(stream_id as u8);
        frame.extend_from_slice(payload);
        frame
    }

    /// ارسال DATA frame
    pub async fn send_data(&mut self, data: &[u8]) -> Result<()> {
        let stream_id = if self.stream_counter == 0 { 1 } else { self.stream_counter - 1 };
        let frame = self.build_http2_frame(0x00, 0x00, stream_id, data);
        let stream = self.stream.as_mut().context("No connection")?;
        stream.write_all(&frame).await.context("DATA frame failed")?;
        Ok(())
    }

    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.send_data(data).await
    }

    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let stream = self.stream.as_mut().context("No connection")?;
        // Skip 9-byte HTTP/2 frame header
        let mut header = [0u8; 9];
        let n = timeout(Duration::from_secs(10), stream.read(&mut header)).await??;
        if n < 9 { return Ok(0); }
        let payload_len = ((header[0] as u32) << 16 | (header[1] as u32) << 8 | header[2] as u32) as usize;
        let read_len = payload_len.min(buf.len());
        let n = stream.read(&mut buf[..read_len]).await?;
        Ok(n)
    }

    pub async fn close(&mut self) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            // GOAWAY frame
            let goaway: &[u8] = &[0x00, 0x00, 0x08, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
            let _ = stream.write_all(goaway).await;
        }
        info!("🔌 XHTTP connection closed");
        Ok(())
    }
}
