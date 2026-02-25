//! IP-Relay — Multi-Hop Relay Chain (بدون نیاز به سرور مجازی)
//!
//! از IPهای تمیز Cloudflare/CDN به عنوان لایه relay استفاده می‌کند.
//! هر hop یک CDN IP مستقل است که ترافیک را به هم forward می‌کند.
//! این تکنیک "IP-Relay" یا "Daisy-Chaining" نام دارد.

use std::net::{IpAddr, SocketAddr};
use anyhow::{Context, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{timeout, Duration},
};
use tracing::{debug, info, warn};
use rand::seq::SliceRandom;

/// حداکثر تعداد hop
const MAX_HOPS: usize = 5;
const RELAY_TIMEOUT: Duration = Duration::from_secs(10);

/// یک گره در زنجیره relay
#[derive(Debug, Clone)]
pub struct RelayNode {
    pub ip: IpAddr,
    pub port: u16,
    pub cdn_type: String,
    pub latency_ms: u64,
}

impl RelayNode {
    pub fn new(ip: IpAddr, port: u16, cdn_type: &str) -> Self {
        Self { ip, port, cdn_type: cdn_type.to_string(), latency_ms: 0 }
    }

    pub fn addr(&self) -> SocketAddr {
        SocketAddr::new(self.ip, self.port)
    }
}

/// پیکربندی relay chain
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// آیا ترتیب hopها تصادفی باشد؟
    pub shuffle_hops: bool,
    /// فقط از CDNهای مختلف استفاده کن
    pub prefer_diverse_cdns: bool,
    /// حداکثر تأخیر مجاز برای هر hop (ms)
    pub max_hop_latency_ms: u64,
    /// تعداد hopها
    pub hop_count: usize,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            shuffle_hops: true,
            prefer_diverse_cdns: true,
            max_hop_latency_ms: 300,
            hop_count: 3,
        }
    }
}

/// HTTP CONNECT Relay Chain
pub struct IpRelayChain {
    nodes: Vec<RelayNode>,
    stream: Option<TcpStream>,
    config: RelayConfig,
    active_hops: usize,
}

impl IpRelayChain {
    pub fn new(config: RelayConfig) -> Self {
        Self { nodes: Vec::new(), stream: None, config, active_hops: 0 }
    }

    /// افزودن یک node به زنجیره
    pub fn add_node(mut self, node: RelayNode) -> Self {
        if self.nodes.len() < MAX_HOPS { self.nodes.push(node); }
        self
    }

    /// افزودن چند node یکجا
    pub fn with_nodes(mut self, nodes: Vec<RelayNode>) -> Self {
        for n in nodes.into_iter().take(MAX_HOPS) { self.nodes.push(n); }
        self
    }

    /// ساخت زنجیره با کمترین latency
    pub fn build_optimal_chain(&mut self) {
        // مرتب‌سازی بر اساس latency
        self.nodes.sort_by_key(|n| n.latency_ms);

        if self.config.shuffle_hops && self.nodes.len() > 2 {
            // shuffle کردن میانی‌ها (نه اول و آخر)
            let last = self.nodes.len() - 1;
            let mut rng = rand::thread_rng();
            self.nodes[1..last].shuffle(&mut rng);
        }

        // اعمال hop count
        self.nodes.truncate(self.config.hop_count.min(MAX_HOPS));
        info!("🔗 IP-Relay chain built: {} hops", self.nodes.len());
    }

    /// برقراری اتصال relay زنجیره‌ای
    pub async fn connect(&mut self) -> Result<()> {
        if self.nodes.is_empty() {
            return Err(anyhow::anyhow!("No relay nodes defined"));
        }

        info!("⛓️ برقراری IP-Relay chain ({} hops)...", self.nodes.len());

        // اتصال اول به node[0]
        let first = &self.nodes[0];
        let stream = timeout(RELAY_TIMEOUT, TcpStream::connect(first.addr()))
            .await
            .context("Relay hop #1 timeout")?
            .context("Relay hop #1 TCP failed")?;

        self.stream = Some(stream);
        self.active_hops = 1;
        debug!("✅ Hop #1: {}", first.ip);

        // پیمایش زنجیره: هر hop را با HTTP CONNECT به بعدی متصل کن
        let relay_nodes: Vec<_> = self.nodes[1..].to_vec();
        for (i, node) in relay_nodes.iter().enumerate() {
            self.tunnel_to_next(node, i + 2).await?;
        }

        info!("✅ IP-Relay chain active: {} hops", self.active_hops);
        Ok(())
    }

    /// ایجاد تانل HTTP CONNECT به hop بعدی
    async fn tunnel_to_next(&mut self, next: &RelayNode, hop_num: usize) -> Result<()> {
        let connect_req = format!(
            "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\nProxy-Connection: Keep-Alive\r\n\r\n",
            next.ip, next.port, next.ip, next.port
        );

        let stream = self.stream.as_mut().context("No stream")?;
        stream.write_all(connect_req.as_bytes()).await
            .context(format!("Relay hop #{} CONNECT write failed", hop_num))?;

        // خواندن پاسخ HTTP 200
        let mut buf = vec![0u8; 512];
        let n = timeout(RELAY_TIMEOUT, stream.read(&mut buf)).await?
            .context(format!("Relay hop #{} response timeout", hop_num))?;

        let resp = String::from_utf8_lossy(&buf[..n]);
        if !resp.contains("200") {
            return Err(anyhow::anyhow!(
                "Relay hop #{} rejected: {}", hop_num, resp.lines().next().unwrap_or("")
            ));
        }

        self.active_hops += 1;
        debug!("✅ Hop #{}: {} ({})", hop_num, next.ip, next.cdn_type);
        Ok(())
    }

    /// ارسال داده از طریق زنجیره
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        let stream = self.stream.as_mut().context("No active relay chain")?;
        stream.write_all(data).await.context("Relay send failed")?;
        Ok(())
    }

    /// دریافت داده از طریق زنجیره
    pub async fn recv(&mut self, buf: &mut [u8]) -> Result<usize> {
        let stream = self.stream.as_mut().context("No active relay chain")?;
        let n = timeout(RELAY_TIMEOUT, stream.read(buf)).await??;
        Ok(n)
    }

    /// بستن زنجیره
    pub async fn close(&mut self) {
        if let Some(stream) = self.stream.take() { drop(stream); }
        self.active_hops = 0;
        info!("🔌 IP-Relay chain closed");
    }

    pub fn hop_count(&self) -> usize { self.active_hops }
    pub fn is_active(&self) -> bool { self.stream.is_some() }

    /// دریافت نمای کلی زنجیره (برای لاگ)
    pub fn chain_summary(&self) -> String {
        self.nodes.iter().enumerate()
            .map(|(i, n)| format!("[{}] {}:{} ({})", i + 1, n.ip, n.port, n.cdn_type))
            .collect::<Vec<_>>()
            .join(" → ")
    }
}
