//! SMUX v2 — Stream Multiplexer
//! چندین جریان منطقی را روی یک اتصال TCP واحد مدیریت می‌کند
//! سازگار با smux v1/v2 (همان پروتکل sing-box)

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

// ── SMUX Frame Constants ────────────────────────────────────────
const SMUX_VERSION:   u8 = 2;
const CMD_SYN:        u8 = 0; // Open stream
const CMD_FIN:        u8 = 1; // Close stream
const CMD_PSH:        u8 = 2; // Push data
const CMD_NOP:        u8 = 3; // Keep-alive
const CMD_UPD:        u8 = 4; // Update window (v2 only)
const HEADER_SIZE:   usize = 8;
const MAX_FRAME_SIZE: u32 = 65536;
const INIT_WINDOW:    u32 = 262144; // 256 KB receive window per stream

/// SMUX Frame Header (8 bytes)
#[derive(Debug, Clone)]
pub struct SmuxHeader {
    pub version: u8,
    pub cmd:     u8,
    pub length:  u16,
    pub sid:     u32,
}

impl SmuxHeader {
    pub fn new(cmd: u8, sid: u32, length: u16) -> Self {
        Self { version: SMUX_VERSION, cmd, length, sid }
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut b = [0u8; 8];
        b[0] = self.version;
        b[1] = self.cmd;
        b[2..4].copy_from_slice(&self.length.to_le_bytes());
        b[4..8].copy_from_slice(&self.sid.to_le_bytes());
        b
    }

    pub fn from_bytes(b: &[u8; 8]) -> Self {
        Self {
            version: b[0],
            cmd:     b[1],
            length:  u16::from_le_bytes([b[2], b[3]]),
            sid:     u32::from_le_bytes([b[4], b[5], b[6], b[7]]),
        }
    }
}

/// وضعیت یک Stream
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamState {
    Open,
    HalfClosed,
    Closed,
}

/// یک Stream منطقی
pub struct SmuxStream {
    pub id:     u32,
    pub state:  StreamState,
    pub window: u32,
    pub buffer: Vec<u8>,
}

impl SmuxStream {
    fn new(id: u32) -> Self {
        Self { id, state: StreamState::Open, window: INIT_WINDOW, buffer: Vec::new() }
    }
}

/// SMUX Session (یک اتصال TCP با چند stream)
pub struct SmuxSession {
    stream:      Mutex<TcpStream>,
    streams:     Mutex<HashMap<u32, SmuxStream>>,
    next_sid:    AtomicU32,
    is_client:   bool,
}

impl SmuxSession {
    /// ایجاد session از یک TCP stream
    pub fn new(tcp: TcpStream, is_client: bool) -> Self {
        let start = if is_client { 1u32 } else { 2u32 };
        let s = Self {
            stream:    Mutex::new(tcp),
            streams:   Mutex::new(HashMap::new()),
            next_sid:  AtomicU32::new(start),
            is_client,
        };
        info!("📦 SMUX v2 Session started (client={})", is_client);
        s
    }

    /// باز کردن یک stream جدید
    pub async fn open_stream(&self) -> Result<u32> {
        // شماره stream: کلاینت از اعداد فرد، سرور از اعداد زوج
        let step = 2u32;
        let sid = self.next_sid.fetch_add(step, Ordering::Relaxed);

        // ارسال SYN frame
        self.write_frame(&SmuxHeader::new(CMD_SYN, sid, 0), &[]).await?;

        // ثبت stream
        self.streams.lock().await.insert(sid, SmuxStream::new(sid));
        debug!("📦 SMUX Stream #{} opened", sid);
        Ok(sid)
    }

    /// ارسال داده روی یک stream
    pub async fn send_data(&self, sid: u32, data: &[u8]) -> Result<()> {
        let max = MAX_FRAME_SIZE as usize;
        let mut offset = 0;

        while offset < data.len() {
            let chunk_end = (offset + max).min(data.len());
            let chunk = &data[offset..chunk_end];
            let hdr = SmuxHeader::new(CMD_PSH, sid, chunk.len() as u16);
            self.write_frame(&hdr, chunk).await?;
            offset = chunk_end;
        }
        Ok(())
    }

    /// بستن یک stream
    pub async fn close_stream(&self, sid: u32) -> Result<()> {
        self.write_frame(&SmuxHeader::new(CMD_FIN, sid, 0), &[]).await?;
        if let Some(s) = self.streams.lock().await.get_mut(&sid) {
            s.state = StreamState::Closed;
        }
        debug!("📦 SMUX Stream #{} closed", sid);
        Ok(())
    }

    /// ارسال Keep-alive NOP
    pub async fn ping(&self) -> Result<()> {
        self.write_frame(&SmuxHeader::new(CMD_NOP, 0, 0), &[]).await?;
        Ok(())
    }

    /// خواندن یک frame از TCP
    pub async fn read_frame(&self) -> Result<(SmuxHeader, Vec<u8>)> {
        let mut hdr_buf = [0u8; HEADER_SIZE];
        self.stream.lock().await.read_exact(&mut hdr_buf).await
            .context("SMUX header read failed")?;

        let hdr = SmuxHeader::from_bytes(&hdr_buf);

        if hdr.version != SMUX_VERSION {
            warn!("⚠️ SMUX version mismatch: got {}", hdr.version);
        }

        let mut payload = vec![0u8; hdr.length as usize];
        if hdr.length > 0 {
            self.stream.lock().await.read_exact(&mut payload).await
                .context("SMUX payload read failed")?;
        }

        Ok((hdr, payload))
    }

    /// ارسال frame خام
    async fn write_frame(&self, hdr: &SmuxHeader, payload: &[u8]) -> Result<()> {
        let mut buf = Vec::with_capacity(HEADER_SIZE + payload.len());
        buf.extend_from_slice(&hdr.to_bytes());
        buf.extend_from_slice(payload);
        self.stream.lock().await.write_all(&buf).await
            .context("SMUX frame write failed")?;
        Ok(())
    }

    /// تعداد streamهای فعال
    pub async fn active_streams(&self) -> usize {
        self.streams.lock().await.values()
            .filter(|s| s.state == StreamState::Open).count()
    }
}

/// SMUX Multiplexer — رابط ساده برای استفاده در Matryoshka
pub struct Smux;

impl Smux {
    pub fn new() -> Self { Self }

    /// ساخت SYN frame برای باز کردن اولین stream
    pub fn build_open_frame(sid: u32) -> Vec<u8> {
        SmuxHeader::new(CMD_SYN, sid, 0).to_bytes().to_vec()
    }

    /// بسته‌بندی داده در PSH frame
    pub fn wrap_data(sid: u32, data: &[u8]) -> Vec<u8> {
        let hdr = SmuxHeader::new(CMD_PSH, sid, data.len() as u16);
        let mut frame = hdr.to_bytes().to_vec();
        frame.extend_from_slice(data);
        frame
    }

    /// NOP keep-alive
    pub fn nop_frame() -> Vec<u8> {
        SmuxHeader::new(CMD_NOP, 0, 0).to_bytes().to_vec()
    }
}

impl Default for Smux {
    fn default() -> Self { Self::new() }
}
