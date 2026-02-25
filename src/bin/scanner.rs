//! Network Ghost Scanner — اسکنر IP مستقل

use anyhow::Result;
use clap::Parser;
use tracing::info;
use network_ghost_v5::{NetworkGhostEngine, types::{ProxyConfig, CdnType}};

#[derive(Debug, Parser)]
#[command(name = "scanner", version = "5.0.0")]
struct Cli {
    #[arg(long, default_value = "cloudflare")]
    cdn: String,
    #[arg(long, default_value = "100")]
    max_ips: usize,
    #[arg(long)]
    output: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    let cli = Cli::parse();
    info!("🔍 شروع اسکن IP — CDN: {}", cli.cdn);

    let config = ProxyConfig::default();
    let engine = NetworkGhostEngine::new(config).await?;
    let ips = engine.get_clean_ips().await;

    info!("✅ {} IP تمیز یافت شد", ips.len());

    if let Some(path) = &cli.output {
        let lines: Vec<String> = ips.iter()
            .map(|r| format!("{}:{}", r.ip, r.port))
            .collect();
        tokio::fs::write(path, lines.join("\n")).await?;
        info!("💾 ذخیره در: {}", path.display());
    } else {
        for ip in &ips {
            println!("{}:{} — {}ms", ip.ip, ip.port, ip.latency_ms);
        }
    }
    Ok(())
}
