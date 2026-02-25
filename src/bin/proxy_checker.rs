//! Proxy Checker — بررسی و فیلتر ISP پروکسی‌ها

#![allow(unused_imports)]

use std::sync::Arc;
use anyhow::Result;
use clap::Parser;
use tokio::sync::Semaphore;
use tracing::{info, warn, error};
use colored::Colorize;

const DEFAULT_MAX_CONCURRENT: usize = 50;
const DEFAULT_TIMEOUT_SECONDS: u64 = 6;
const CHECK_URL: &str = "https://ipp.nscl.ir";
const MAX_PING_DEFAULT: u64 = 300;

const GOOD_ISPS: &[&str] = &[
    "Google", "Amazon", "Cloudflare", "Microsoft", "Fastly",
    "Akamai", "DigitalOcean", "Linode", "Vultr", "OVH",
    "Hetzner", "Leaseweb", "Contabo", "Scaleway", "UpCloud",
    "Zscaler", "Imperva", "Sucuri", "Radware", "F5",
    "GTT", "HE", "NTT", "Telia", "Level 3",
    "Cogent", "Hurricane Electric", "Lumen", "Zayo", "Equinix",
    "GCore", "CDN77", "BunnyCDN", "KeyCDN", "StackPath",
    "Fly.io", "Railway", "Render", "Vercel", "Netlify",
    "Oracle Cloud", "IBM Cloud", "Alibaba Cloud", "Tencent",
    "Quad9", "NextDNS", "Control D", "AdGuard", "Mullvad",
    "Datacenter", "Data Center",
];

#[derive(Debug, Parser)]
#[command(name = "proxy-checker", version = "5.0.0",
    about = "🔍 Proxy Checker with ISP Filtering & CDN Detection")]
struct Cli {
    #[arg(long, default_value = "/opt/network-ghost/sub/proxies.txt")]
    proxy_file: std::path::PathBuf,

    #[arg(long)]
    output_file: Option<std::path::PathBuf>,

    #[arg(long, action)]
    json_output: bool,

    #[arg(long, default_value_t = DEFAULT_MAX_CONCURRENT)]
    max_concurrent: usize,

    #[arg(long, default_value_t = DEFAULT_TIMEOUT_SECONDS)]
    timeout: u64,

    #[arg(long, action)]
    filter_isp: bool,

    #[arg(long, default_value_t = MAX_PING_DEFAULT)]
    max_ping_ms: u64,
}

#[derive(Debug, Clone)]
struct ProxyResult {
    address: String,
    alive: bool,
    ping_ms: u64,
    isp: String,
    country: String,
    is_good_isp: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"))
        )
        .init();

    let cli = Cli::parse();

    println!("{}", "═".repeat(60).cyan());
    println!("{}", "  🔍 Network Ghost Proxy Checker v5.0".cyan().bold());
    println!("{}", "═".repeat(60).cyan());

    // خواندن فایل پروکسی
    let proxies = if cli.proxy_file.exists() {
        let content = tokio::fs::read_to_string(&cli.proxy_file).await?;
        content.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .map(|l| l.trim().to_string())
            .collect::<Vec<_>>()
    } else {
        warn!("⚠️ فایل پروکسی یافت نشد: {}", cli.proxy_file.display());
        Vec::new()
    };

    info!("📋 {} پروکسی بارگذاری شد", proxies.len());
    println!("  📋 {} proxies loaded", proxies.len());

    if proxies.is_empty() {
        println!("  ❌ هیچ پروکسی‌ای یافت نشد.");
        return Ok(());
    }

    let semaphore = Arc::new(Semaphore::new(cli.max_concurrent));
    let timeout_secs = cli.timeout;
    let max_ping = cli.max_ping_ms;
    let filter_isp = cli.filter_isp;

    // بررسی همزمان
    let mut handles = Vec::new();
    for proxy in proxies.clone() {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            check_proxy(&proxy, timeout_secs, max_ping).await
        });
        handles.push(handle);
    }

    let mut results: Vec<ProxyResult> = Vec::new();
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }

    // فیلتر ISP
    let final_results: Vec<&ProxyResult> = if filter_isp {
        results.iter().filter(|r| r.alive && r.is_good_isp).collect()
    } else {
        results.iter().filter(|r| r.alive).collect()
    };

    let total = results.len();
    let alive = results.iter().filter(|r| r.alive).count();
    let good_isp = results.iter().filter(|r| r.alive && r.is_good_isp).count();

    println!("\n  📊 نتایج:");
    println!("  ├── کل: {}", total);
    println!("  ├── فعال: {} {}", alive, format!("({:.0}%)", alive as f32 / total as f32 * 100.0).green());
    println!("  └── ISP تأیید شده: {} {}", good_isp, "✅".green());

    // نمایش بهترین پروکسی‌ها
    let mut sorted = final_results.clone();
    sorted.sort_by_key(|r| r.ping_ms);

    println!("\n  🏆 برترین پروکسی‌ها:");
    for (i, r) in sorted.iter().take(10).enumerate() {
        let ping_str = if r.ping_ms < 100 {
            format!("{}ms ⚡", r.ping_ms).green().to_string()
        } else if r.ping_ms < 200 {
            format!("{}ms 🐇", r.ping_ms).yellow().to_string()
        } else {
            format!("{}ms 🐌", r.ping_ms).red().to_string()
        };
        println!("  {:2}. {} | {} | {} | {}",
            i + 1,
            r.address.cyan(),
            ping_str,
            r.isp.bright_white(),
            r.country.yellow()
        );
    }

    // ذخیره خروجی
    if let Some(output_path) = &cli.output_file {
        let report = generate_markdown_report(&results, &sorted);
        tokio::fs::write(output_path, &report).await?;
        println!("\n  💾 گزارش ذخیره شد: {}", output_path.display());
    }

    if cli.json_output {
        let json_data: Vec<serde_json::Value> = results.iter().map(|r| serde_json::json!({
            "address": r.address,
            "alive": r.alive,
            "ping_ms": r.ping_ms,
            "isp": r.isp,
            "country": r.country,
            "good_isp": r.is_good_isp,
        })).collect();
        println!("\n{}", serde_json::to_string_pretty(&json_data)?);
    }

    println!("\n{}", "═".repeat(60).cyan());
    Ok(())
}

async fn check_proxy(address: &str, timeout_secs: u64, max_ping_ms: u64) -> ProxyResult {
    use std::time::Instant;
    use tokio::time::timeout;
    use tokio::net::TcpStream;

    let start = Instant::now();

    // جداسازی host:port
    let (host, port) = if let Some(idx) = address.rfind(':') {
        let p: u16 = address[idx+1..].parse().unwrap_or(443);
        (&address[..idx], p)
    } else {
        (address, 443u16)
    };

    let addr_str = format!("{}:{}", host, port);

    let connect_result = timeout(
        std::time::Duration::from_secs(timeout_secs),
        TcpStream::connect(&addr_str)
    ).await;

    let ping_ms = start.elapsed().as_millis() as u64;
    let alive = connect_result.is_ok() && ping_ms <= max_ping_ms;

    // شبیه‌سازی ISP detection (در پیاده‌سازی واقعی از API استفاده می‌شود)
    let isp = detect_isp_by_ip(host);
    let country = "N/A".to_string();
    let is_good_isp = GOOD_ISPS.iter().any(|&g|
        isp.to_lowercase().contains(&g.to_lowercase())
    );

    ProxyResult { address: address.to_string(), alive, ping_ms, isp, country, is_good_isp }
}

fn detect_isp_by_ip(host: &str) -> String {
    // تشخیص CDN از روی رنج IP
    if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
        let octets = ip.octets();
        return match octets {
            [104, 16..=31, _, _] => "Cloudflare".to_string(),
            [172, 64..=71, _, _] => "Cloudflare".to_string(),
            [104, 21..=31, _, _] => "Cloudflare".to_string(),
            [8, 8, _, _]         => "Google".to_string(),
            [34, ..] | [35, ..] | [142, ..] => "Google Cloud".to_string(),
            [52, ..] | [54, ..] | [18, ..]  => "Amazon AWS".to_string(),
            [13, ..] | [3, ..]              => "Amazon AWS".to_string(),
            [23, ..] | [151, 101, _, _]     => "Fastly".to_string(),
            [185, 60, ..] | [31, 13, _, _]  => "Facebook".to_string(),
            _                               => "Unknown CDN".to_string(),
        };
    }
    "Unknown".to_string()
}

fn generate_markdown_report(results: &[ProxyResult], sorted: &[&ProxyResult]) -> String {
    let alive = results.iter().filter(|r| r.alive).count();
    let good = results.iter().filter(|r| r.alive && r.is_good_isp).count();

    let mut md = String::new();
    md.push_str("# 👻 Network Ghost Proxy Report\n\n");
    md.push_str(&format!(
        "![alive](https://img.shields.io/badge/alive-{}-green) \
         ![total](https://img.shields.io/badge/total-{}-blue) \
         ![isp](https://img.shields.io/badge/good_isp-{}-brightgreen)\n\n",
        alive, results.len(), good
    ));
    md.push_str("## 🏆 برترین پروکسی‌ها\n\n");
    md.push_str("| # | آدرس | پینگ | ISP | کشور |\n");
    md.push_str("|---|------|------|-----|------|\n");
    for (i, r) in sorted.iter().take(30).enumerate() {
        let emoji = if r.ping_ms < 100 { "⚡" } else if r.ping_ms < 200 { "🐇" } else { "🐌" };
        md.push_str(&format!("| {} | `{}` | {}{}ms | {} | {} |\n",
            i+1, r.address, emoji, r.ping_ms, r.isp, r.country));
    }
    md
}
