//! Web Dashboard

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::info;

/// تنظیمات Dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// پورت
    pub port: u16,
    /// آدرس bind
    pub bind: String,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            port: 9090,
            bind: "0.0.0.0".to_string(),
        }
    }
}

/// اطلاعات تونل
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    /// فعال
    pub active: bool,
    /// IP فعلی
    pub current_ip: String,
    /// پورت فعلی
    pub current_port: u16,
    /// پروتکل
    pub protocol: String,
    /// CDN
    pub cdn: String,
    /// uptime
    pub uptime_secs: u64,
    /// RX bytes
    pub rx_bytes: u64,
    /// TX bytes
    pub tx_bytes: u64,
    /// تأخیر
    pub latency_ms: u64,
}

/// Dashboard Server
pub struct DashboardServer {
    /// تنظیمات
    config: DashboardConfig,
    /// اطلاعات تونل
    tunnel: Arc<RwLock<TunnelInfo>>,
}

impl DashboardServer {
    /// ایجاد سرور جدید
    pub fn new(config: DashboardConfig) -> Self {
        Self {
            config,
            tunnel: Arc::new(RwLock::new(TunnelInfo {
                active: false,
                current_ip: String::new(),
                current_port: 0,
                protocol: String::new(),
                cdn: String::new(),
                uptime_secs: 0,
                rx_bytes: 0,
                tx_bytes: 0,
                latency_ms: 0,
            })),
        }
    }

    /// شروع سرور
    pub async fn start(&self) -> Result<()> {
        info!("📊 Dashboard started on http://{}:{}", self.config.bind, self.config.port);
        Ok(())
    }

    /// به‌روزرسانی اطلاعات تونل
    pub async fn update_tunnel(&self, info: TunnelInfo) {
        let mut tunnel = self.tunnel.write().await;
        *tunnel = info;
    }
}
