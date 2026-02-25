//! Network Ghost Daemon — پروسه پس‌زمینه

use std::time::Duration;
use anyhow::Result;
use tokio::time::interval;
use tracing::{error, info};
use network_ghost_v5::{NetworkGhostEngine, types::ProxyConfig};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("🌙 Network Ghost Daemon v5.0 شروع شد");

    // ذخیره PID
    let pid = std::process::id();
    tokio::fs::write("/tmp/network-ghost.pid", pid.to_string()).await?;

    let config = ProxyConfig::default();
    let engine = NetworkGhostEngine::new(config).await?;

    // شروع تانل
    if let Err(e) = engine.start().await {
        error!("❌ خطا در شروع تانل: {}", e);
    }

    // حلقه نگهداری (watchdog)
    let mut tick = interval(Duration::from_secs(30));
    loop {
        tick.tick().await;
        let state = engine.get_state().await;
        if !state.active {
            info!("🔄 تانل غیر فعال — تلاش برای راه‌اندازی مجدد...");
            if let Err(e) = engine.start().await {
                error!("❌ راه‌اندازی مجدد ناموفق: {}", e);
            }
        }
    }
}
