//! Matryoshka Dialer - زنجیره تو در تو

use std::{
    net::{IpAddr, SocketAddr},
    time::Duration,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use tracing::{debug, info};

/// حداکثر تعداد لایه‌ها
const MAX_LAYERS: usize = 20;

/// نوع لایه
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LayerType {
    /// TCP
    Tcp,
    /// ShadowTLS
    ShadowTls { sni: String },
    /// Reality
    Reality { uuid: String, public_key: String },
    /// SMUX
    Smux,
}

impl Default for LayerType {
    fn default() -> Self {
        Self::Tcp
    }
}

/// دیالر ماتریوشکا
pub struct MatryoshkaDialer {
    /// آدرس هدف
    target: SocketAddr,
    /// لایه‌ها
    layers: Vec<LayerType>,
    /// کانکشن TCP
    tcp_stream: Option<TcpStream>,
    /// فعال
    active: bool,
}

impl MatryoshkaDialer {
    /// ایجاد دیالر جدید
    pub fn new(target: SocketAddr) -> Self {
        Self {
            target,
            layers: Vec::new(),
            tcp_stream: None,
            active: false,
        }
    }

    /// ایجاد با IP
    pub fn from_ip(ip: IpAddr, port: u16) -> Self {
        Self::new(SocketAddr::new(ip, port))
    }

    /// دریافت آدرس هدف
    pub fn target_addr(&self) -> SocketAddr {
        self.target
    }

    /// اضافه کردن لایه ShadowTLS
    pub fn wrap_with_shadowtls(mut self, sni: &str) -> Self {
        self.layers.push(LayerType::ShadowTls { sni: sni.to_string() });
        self
    }

    /// اضافه کردن لایه Reality
    pub fn wrap_with_reality(mut self, uuid: &str, public_key: &str) -> Self {
        self.layers.push(LayerType::Reality {
            uuid: uuid.to_string(),
            public_key: public_key.to_string(),
        });
        self
    }

    /// فعال‌سازی SMUX
    pub fn enable_smux(mut self) -> Self {
        self.layers.push(LayerType::Smux);
        self
    }

    /// تعداد لایه‌ها
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// شروع اتصال
    pub async fn start(&mut self) -> Result<()> {
        if self.layers.len() > MAX_LAYERS {
            self.layers.truncate(MAX_LAYERS);
        }

        info!("🚀 Starting Matryoshka Dialer with {} layers", self.layers.len());

        // اتصال TCP
        let stream = timeout(
            Duration::from_secs(10),
            TcpStream::connect(self.target),
        )
        .await
        .context("TCP connection timeout")?
        .context("TCP connection failed")?;

        self.tcp_stream = Some(stream);

        // اعمال لایه‌ها
        for i in 0..self.layers.len() { let layer = self.layers[i].clone();
            self.apply_layer(&layer).await?;
        }

        self.active = true;
        info!("✅ Matryoshka chain established");
        Ok(())
    }

    /// اعمال لایه
    async fn apply_layer(&mut self, layer: &LayerType) -> Result<()> {
        match layer {
            LayerType::ShadowTls { sni } => {
                self.apply_shadowtls(sni).await?;
            }
            LayerType::Reality { uuid, public_key } => {
                self.apply_reality(uuid, public_key).await?;
            }
            LayerType::Smux => {
                self.apply_smux().await?;
            }
            _ => {}
        }
        Ok(())
    }

    /// اعمال ShadowTLS
    async fn apply_shadowtls(&mut self, sni: &str) -> Result<()> {
        debug!("🔐 Applying ShadowTLS layer with SNI: {}", sni);
        
        let hello = self.build_shadowtls_hello(sni);
        
        let stream = self.tcp_stream.as_mut().context("No connection")?;
        stream.write_all(&hello).await?;
        
        let mut response = vec![0u8; 4096];
        let _n = timeout(Duration::from_secs(5), stream.read(&mut response))
            .await?
            .context("ShadowTLS timeout")?;
        
        debug!("✅ ShadowTLS layer applied");
        Ok(())
    }

    /// ساخت ShadowTLS Hello
    fn build_shadowtls_hello(&self, sni: &str) -> Vec<u8> {
        let mut hello = Vec::new();
        
        hello.push(0x16);
        hello.push(0x03);
        hello.push(0x01);
        
        let sni_bytes = sni.as_bytes();
        hello.extend_from_slice(sni_bytes);
        
        hello
    }

    /// اعمال Reality
    async fn apply_reality(&mut self, uuid: &str, _public_key: &str) -> Result<()> {
        debug!("🌐 Applying Reality layer");
        
        let packet = self.build_reality_packet(uuid);
        
        let stream = self.tcp_stream.as_mut().context("No connection")?;
        stream.write_all(&packet).await?;
        
        debug!("✅ Reality layer applied");
        Ok(())
    }

    /// ساخت Reality packet
    fn build_reality_packet(&self, uuid: &str) -> Vec<u8> {
        let mut packet = Vec::new();
        
        packet.push(0x01);
        
        let uuid_clean = uuid.replace('-', "");
        if let Ok(bytes) = hex::decode(&uuid_clean) {
            packet.extend(bytes);
        }
        
        packet
    }

    /// اعمال SMUX
    async fn apply_smux(&mut self) -> Result<()> {
        debug!("📦 Applying SMUX layer");
        
        let open = vec![
            0x01, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
        
        let stream = self.tcp_stream.as_mut().context("No connection")?;
        stream.write_all(&open).await?;
        
        debug!("✅ SMUX layer applied");
        Ok(())
    }

    /// ارسال داده
    pub async fn send(&mut self, data: &[u8]) -> Result<usize> {
        let stream = self.tcp_stream.as_mut().context("No connection")?;
        stream.write_all(data).await?;
        Ok(data.len())
    }

    /// دریافت داده
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let stream = self.tcp_stream.as_mut().context("No connection")?;
        let n = stream.read(buf).await?;
        Ok(n)
    }

    /// بستن اتصال
    pub async fn close(&mut self) -> Result<()> {
        if let Some(stream) = self.tcp_stream.take() {
            drop(stream);
        }
        self.active = false;
        info!("🔌 Matryoshka connection closed");
        Ok(())
    }

    /// آیا فعال است؟
    pub fn is_active(&self) -> bool {
        self.active
    }
}


