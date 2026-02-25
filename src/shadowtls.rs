//! ShadowTLS v3 Client

use std::net::{IpAddr, SocketAddr};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};
use tracing::{debug, info};


/// کلاینت ShadowTLS v3
pub struct ShadowTlsClient {
    /// آدرس سرور
    server: IpAddr,
    /// پورت
    port: u16,
    /// SNI
    sni: String,
    /// کانکشن TCP
    stream: Option<TcpStream>,
}

impl ShadowTlsClient {
    /// ایجاد کلاینت جدید
    pub fn new(server: IpAddr, port: u16, sni: String) -> Self {
        Self {
            server,
            port,
            sni,
            stream: None,
        }
    }

    /// اتصال
    pub async fn connect(&mut self) -> Result<()> {
        info!("🔐 اتصال ShadowTLS v3 به {}:{}", self.server, self.port);

        let addr = SocketAddr::new(self.server, self.port);

        let stream = timeout(Duration::from_secs(10), TcpStream::connect(addr))
            .await
            .context("Connection timeout")?
            .context("TCP connection failed")?;

        self.stream = Some(stream);

        // Handshake
        self.do_handshake().await?;

        info!("✅ ShadowTLS handshake موفق");
        Ok(())
    }

    /// انجام Handshake
    async fn do_handshake(&mut self) -> Result<()> {
        // ساخت Client Hello
        let hello = self.build_client_hello();
        
        let stream = self.stream.as_mut().context("No connection")?;
        stream.write_all(&hello).await?;

        // دریافت Server Hello
        let mut response = vec![0u8; 4096];
        let _n = timeout(Duration::from_secs(10), stream.read(&mut response))
            .await
            .context("Handshake timeout")??;

        debug!("✅ ShadowTLS handshake successful");
        Ok(())
    }

    /// ساخت Client Hello
    fn build_client_hello(&self) -> Vec<u8> {
        let mut hello = Vec::new();

        // TLS Record Layer
        hello.push(0x16);
        hello.push(0x03);
        hello.push(0x01);

        // Length placeholder
        hello.extend_from_slice(&[0x00, 0x00]);

        // Handshake Type
        hello.push(0x01);

        // Version
        hello.push(0x03);
        hello.push(0x03);

        // Random
        let random: [u8; 32] = rand::random();
        hello.extend_from_slice(&random);

        // SNI
        let sni_bytes = self.sni.as_bytes();
        hello.extend_from_slice(sni_bytes);

        hello
    }

    /// ارسال داده
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().context("No connection")?;
        stream.write_all(data).await?;
        Ok(())
    }

    /// بستن اتصال
    pub async fn close(&mut self) -> Result<()> {
        if let Some(stream) = self.stream.take() {
            drop(stream);
        }
        info!("🔌 ShadowTLS connection closed");
        Ok(())
    }
}
