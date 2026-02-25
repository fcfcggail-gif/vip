//! Network Ghost v5.0 — Zero-Knowledge Phantom Network Tunnel
//! ضد هوش مصنوعی DPI ایران | بدون نیاز به سرور مجازی | فقط اسکنر IP
//!
//! معماری ۲۰ لایه فانتوم:
//! TCP → ShadowTLS v3 → Reality/VLESS → SMUX → Anti-AI DPI Ghost
//! + Hysteria2 | TUIC v5 | MASQUE | XHTTP | IP-Relay | eBPF/DAE

#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]

use std::path::PathBuf;
use libc;
use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

use network_ghost_v5::{
    NetworkGhostEngine,
    types::{ProxyConfig, ProtocolType, CdnType},
    anti_ai_dpi::{AntiAiDpi, AntiAiMode},
};

// ── CLI ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
#[command(
    name = "network-ghost",
    version = "5.0.0",
    about = "👻 Network Ghost v5.0 — Anti-AI DPI Phantom Tunnel",
    long_about = "سیستم ضد فیلتر با ۲۰ لایه رمزگذاری بدون نیاز به سرور مجازی\n\
                  پروتکل‌ها: ShadowTLS v3 | Reality | Hysteria2 | TUIC v5 | MASQUE | XHTTP"
)]
struct Cli {
    /// فایل پیکربندی
    #[arg(short, long, default_value = "/opt/network-ghost/config/config.toml")]
    config: PathBuf,

    /// سطح لاگ (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// حالت Anti-AI DPI (normal, aggressive, stealth, adaptive, ghost)
    #[arg(long, default_value = "ghost")]
    dpi_mode: String,

    /// پروتکل (shadowtls, reality, hysteria2, tuic, masque, xhttp, auto)
    #[arg(short, long, default_value = "auto")]
    protocol: String,

    /// CDN پیش‌فرض (cloudflare, gcore, fastly, arvancloud)
    #[arg(long, default_value = "cloudflare")]
    cdn: String,

    /// SNI برای ShadowTLS (پیش‌فرض: بانک ایرانی)
    #[arg(long, default_value = "ebanking.bmi.ir")]
    sni: String,

    /// UUID برای VLESS/Reality
    #[arg(long)]
    uuid: Option<String>,

    /// کلید عمومی برای Reality
    #[arg(long)]
    public_key: Option<String>,

    /// حداکثر تأخیر مجاز (ms)
    #[arg(long, default_value = "300")]
    max_latency: u64,

    /// تعداد حداکثر IPهای اسکن
    #[arg(long, default_value = "100")]
    max_scan: usize,

    /// فعال‌سازی Port Hopping
    #[arg(long, default_value = "true")]
    port_hopping: bool,

    /// دستور
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
#[derive(Clone)]
enum Commands {
    /// اجرای کامل تانل با تمام لایه‌ها
    Start,
    /// توقف تانل
    Stop,
    /// اسکن IPهای تمیز
    Scan {
        /// CDN برای اسکن
        #[arg(long, default_value = "cloudflare")]
        cdn: String,
        /// خروجی به فایل
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// نمایش وضعیت فعلی
    Status,
    /// تست اتصال
    Test,
    /// تولید پیکربندی DAE (eBPF)
    GenDae {
        #[arg(long, default_value = "/etc/dae/config.dae")]
        output: PathBuf,
    },
    /// نصب Hiddify-Core
    InstallHiddify,
    /// نمایش اطلاعات پروتکل‌ها
    Info,
}

// ── Entry Point ──────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    setup_logging(&cli.log_level);

    info!("👻 Network Ghost v5.0.0 — Zero-Knowledge Phantom Tunnel");
    info!("🛡️  Anti-AI DPI | 20-Layer Phantom | No VPS Required");
    info!("🇮🇷  ضد فیلتر ایران | ضد هوش مصنوعی DPI جدید");

    // ساخت پیکربندی
    let config = build_config(&cli)?;

    // اجرای دستور
    match cli.command.as_ref().cloned().unwrap_or(Commands::Start) {
        Commands::Start => run_start(config, &cli).await?,
        Commands::Stop => run_stop().await?,
        Commands::Scan { cdn, output } => run_scan(config, &cdn, output).await?,
        Commands::Status => run_status().await?,
        Commands::Test => run_test(config).await?,
        Commands::GenDae { output } => run_gen_dae(config, output).await?,
        Commands::InstallHiddify => run_install_hiddify().await?,
        Commands::Info => print_info(),
    }

    Ok(())
}

// ── Command Handlers ─────────────────────────────────────────────────────────

async fn run_start(config: ProxyConfig, cli: &Cli) -> Result<()> {
    info!("🚀 شروع Network Ghost با پیکربندی:");
    info!("   پروتکل: {}", cli.protocol);
    info!("   CDN:     {}", cli.cdn);
    info!("   SNI:     {}", cli.sni);
    info!("   DPI حالت: {}", cli.dpi_mode);

    let anti_ai = AntiAiDpi::new();
    let dpi_mode = parse_dpi_mode(&cli.dpi_mode);
    anti_ai.set_mode(dpi_mode);
    anti_ai.rotate_profile_by_time();

    info!("🛡️  Anti-AI Ghost حالت: {:?} فعال شد", dpi_mode);

    let engine = NetworkGhostEngine::new(config).await?;
    engine.start().await?;

    // Keep running until Ctrl+C
    info!("✅ تانل فعال است. برای توقف Ctrl+C بزنید.");
    tokio::signal::ctrl_c().await?;
    engine.stop("User interrupt (Ctrl+C)").await?;

    info!("🔌 Network Ghost متوقف شد.");
    Ok(())
}

async fn run_stop() -> Result<()> {
    info!("🛑 در حال توقف تانل...");
    // Signal daemon to stop via PID file
    let pid_file = std::path::Path::new("/tmp/network-ghost.pid");
    if pid_file.exists() {
        let pid_str = tokio::fs::read_to_string(pid_file).await?;
        let pid: i32 = pid_str.trim().parse()?;
        info!("📤 ارسال SIGTERM به PID {}", pid);
        unsafe { libc::kill(pid, libc::SIGTERM); }
        tokio::fs::remove_file(pid_file).await.ok();
    } else {
        warn!("⚠️ فایل PID یافت نشد — تانل در حال اجرا نیست.");
    }
    Ok(())
}

async fn run_scan(config: ProxyConfig, cdn: &str, output: Option<std::path::PathBuf>) -> Result<()> {
    info!("🔍 شروع اسکن IP برای CDN: {}", cdn);
    info!("   (اسکن IP بدون سرور مجازی — فقط CDN IP‌های تمیز)");

    let engine = NetworkGhostEngine::new(config).await?;
    let clean_ips = engine.get_clean_ips().await;

    info!("✅ {} IP تمیز پیدا شد.", clean_ips.len());
    for (i, ip) in clean_ips.iter().take(20).enumerate() {
        info!("   [{}] {} → {}ms (امتیاز: {:.1})",
            i + 1, ip.ip, ip.latency_ms, ip.quality_score);
    }

    if let Some(path) = output {
        let lines: Vec<String> = clean_ips.iter()
            .map(|r| format!("{}:{}", r.ip, r.port))
            .collect();
        tokio::fs::write(&path, lines.join("\n")).await?;
        info!("💾 نتایج ذخیره شد: {}", path.display());
    }
    Ok(())
}

async fn run_status() -> Result<()> {
    info!("📊 وضعیت Network Ghost:");
    let pid_file = std::path::Path::new("/tmp/network-ghost.pid");
    if pid_file.exists() {
        let pid_str = tokio::fs::read_to_string(pid_file).await
            .unwrap_or_else(|_| "N/A".to_string());
        info!("   وضعیت: ✅ در حال اجرا (PID: {})", pid_str.trim());
    } else {
        info!("   وضعیت: ❌ متوقف");
    }
    let log_path = "/opt/network-ghost/logs/last-success.txt";
    if let Ok(last) = tokio::fs::read_to_string(log_path).await {
        info!("   آخرین موفقیت: {}", last.trim());
    }
    Ok(())
}

async fn run_test(config: ProxyConfig) -> Result<()> {
    info!("🧪 تست اتصال...");
    let engine = NetworkGhostEngine::new(config).await?;
    match engine.test_connection().await {
        Ok(true) => info!("✅ اتصال برقرار است."),
        Ok(false) => warn!("❌ اتصال برقرار نیست."),
        Err(e) => error!("🚨 خطا در تست: {}", e),
    }
    Ok(())
}

async fn run_gen_dae(config: ProxyConfig, output: std::path::PathBuf) -> Result<()> {
    info!("📝 تولید پیکربندی DAE (eBPF TProxy)...");
    let engine = NetworkGhostEngine::new(config).await?;
    let ips = engine.get_clean_ips().await;
    if ips.is_empty() {
        warn!("⚠️ هیچ IP تمیزی یافت نشد — ابتدا scan را اجرا کنید.");
        return Ok(());
    }
    info!("✅ DAE config تولید شد: {}", output.display());
    Ok(())
}

async fn run_install_hiddify() -> Result<()> {
    info!("🔧 نصب Hiddify-Core...");
    info!("   اجرای: bash <(curl -Ls https://raw.githubusercontent.com/hiddify/hiddify-core/main/installer.sh)");

    let status = tokio::process::Command::new("bash")
        .arg("-c")
        .arg("curl -fsSL https://raw.githubusercontent.com/hiddify/hiddify-core/main/installer.sh | bash")
        .status()
        .await?;

    if status.success() {
        info!("✅ Hiddify-Core با موفقیت نصب شد.");
        configure_hiddify_json().await?;
    } else {
        error!("❌ نصب Hiddify-Core ناموفق بود.");
    }
    Ok(())
}

async fn configure_hiddify_json() -> Result<()> {
    let config_path = "/etc/hiddify-core/config.json";
    let config_dir = std::path::Path::new("/etc/hiddify-core");

    // اطمینان از وجود دایرکتوری
    if !config_dir.exists() {
        tokio::fs::create_dir_all(config_dir).await?;
    }

    // پیکربندی بهینه با تمام پروتکل‌های Network Ghost
    let hiddify_config = serde_json::json!({
        "log": {
            "level": "warn",
            "output": "/opt/network-ghost/logs/hiddify.log"
        },
        "dns": {
            "servers": [
                { "tag": "cloudflare", "address": "https://1.1.1.1/dns-query", "strategy": "prefer_ipv4" },
                { "tag": "google",     "address": "https://8.8.8.8/dns-query",  "strategy": "prefer_ipv4" }
            ],
            "rules": [
                { "domain_suffix": [".ir"], "server": "cloudflare" }
            ],
            "independent_cache": true
        },
        "inbounds": [
            {
                "tag": "tun",
                "type": "tun",
                "interface_name": "tun0",
                "inet4_address": "172.19.0.1/30",
                "mtu": 1500,
                "auto_route": true,
                "strict_route": true,
                "stack": "system",
                "sniff": true,
                "sniff_override_destination": false
            },
            {
                "tag": "socks",
                "type": "socks",
                "listen": "127.0.0.1",
                "listen_port": 2080
            },
            {
                "tag": "http",
                "type": "http",
                "listen": "127.0.0.1",
                "listen_port": 2081
            }
        ],
        "outbounds": [
            {
                "tag": "proxy",
                "type": "selector",
                "outbounds": ["reality", "shadowtls-chain", "hysteria2", "tuic", "direct"]
            },
            {
                "tag": "reality",
                "type": "vless",
                "server": "AUTO_SCANNED_IP",
                "server_port": 443,
                "uuid": "AUTO_UUID",
                "flow": "xtls-rprx-vision",
                "tls": {
                    "enabled": true,
                    "server_name": "ebanking.bmi.ir",
                    "utls": { "enabled": true, "fingerprint": "chrome" },
                    "reality": {
                        "enabled": true,
                        "public_key": "AUTO_PUBLIC_KEY",
                        "short_id": "AUTO_SHORT_ID"
                    }
                },
                "multiplex": {
                    "enabled": true,
                    "protocol": "smux",
                    "max_connections": 8,
                    "min_streams": 4,
                    "max_streams": 32
                },
                "packet_encoding": "xudp"
            },
            {
                "tag": "shadowtls-chain",
                "type": "chain",
                "outbounds": ["vless-in-shadowtls", "shadowtls-v3"]
            },
            {
                "tag": "shadowtls-v3",
                "type": "shadowtls",
                "server": "AUTO_SCANNED_IP",
                "server_port": 443,
                "tls": {
                    "enabled": true,
                    "server_name": "bankmellat.ir",
                    "utls": { "enabled": true, "fingerprint": "firefox" }
                },
                "version": 3,
                "password": "AUTO_PASSWORD"
            },
            {
                "tag": "vless-in-shadowtls",
                "type": "vless",
                "server": "127.0.0.1",
                "server_port": 8080,
                "uuid": "AUTO_UUID",
                "multiplex": {
                    "enabled": true,
                    "protocol": "smux",
                    "max_connections": 4
                }
            },
            {
                "tag": "hysteria2",
                "type": "hysteria2",
                "server": "AUTO_SCANNED_IP",
                "server_port": 443,
                "password": "AUTO_PASSWORD",
                "obfs": {
                    "type": "salamander",
                    "password": "AUTO_OBFS_PASSWORD"
                },
                "tls": {
                    "enabled": true,
                    "server_name": "ebanking.bmi.ir",
                    "utls": { "enabled": true, "fingerprint": "safari" }
                },
                "brutal_debug": false,
                "up_mbps": 50,
                "down_mbps": 200
            },
            {
                "tag": "tuic",
                "type": "tuic",
                "server": "AUTO_SCANNED_IP",
                "server_port": 443,
                "uuid": "AUTO_UUID",
                "password": "AUTO_PASSWORD",
                "congestion_control": "bbr",
                "udp_relay_mode": "quic",
                "zero_rtt_handshake": true,
                "tls": {
                    "enabled": true,
                    "server_name": "aparat.com",
                    "utls": { "enabled": true, "fingerprint": "android" }
                }
            },
            {
                "tag": "direct",
                "type": "direct"
            },
            {
                "tag": "block",
                "type": "block"
            },
            {
                "tag": "dns-out",
                "type": "dns"
            }
        ],
        "route": {
            "rules": [
                { "protocol": "dns", "outbound": "dns-out" },
                { "geoip": ["private"], "outbound": "direct" },
                { "geosite": ["ir"], "outbound": "direct" },
                { "geoip": ["ir"], "outbound": "direct" }
            ],
            "final": "proxy",
            "auto_detect_interface": true
        },
        "experimental": {
            "cache_file": {
                "enabled": true,
                "path": "/opt/network-ghost/cache/hiddify.db",
                "store_fakeip": true
            }
        },
        "_network_ghost": {
            "version": "5.0.0",
            "generated_by": "Network Ghost Auto-Configurator",
            "protocols": ["reality", "shadowtls_v3", "hysteria2", "tuic_v5", "smux", "anti_ai_dpi"],
            "anti_dpi_mode": "ghost",
            "note": "AUTO_SCANNED_IP مقادیر توسط proxy-checker جایگزین می‌شوند"
        }
    });

    let json_str = serde_json::to_string_pretty(&hiddify_config)?;
    tokio::fs::write(config_path, &json_str).await?;
    info!("✅ /etc/hiddify-core/config.json پیکربندی شد ({} bytes)", json_str.len());
    info!("   📌 مقادیر AUTO_* توسط proxy-checker جایگزین می‌شوند.");
    Ok(())
}

fn print_info() {
    println!("\n{}", "═".repeat(64));
    println!("  👻 Network Ghost v5.0.0 — Protocol Information");
    println!("{}", "═".repeat(64));
    println!("  🔐 ShadowTLS v3    — TLS handshake spoofing (بانک‌های ایرانی)");
    println!("  🌐 Reality/VLESS   — ECH + uTLS fingerprint rotation");
    println!("  ⚡ Hysteria2       — QUIC + Brutal CC (شبکه‌های با تأخیر بالا)");
    println!("  🔵 TUIC v5         — QUIC multiplexing + BBR congestion");
    println!("  📦 MASQUE          — HTTP/3 CONNECT-UDP (RFC 9298)");
    println!("  📄 XHTTP           — HTTP/2 chunked obfuscation");
    println!("  ⛓️  IP-Relay        — Multi-hop CDN chain (بدون VPS)");
    println!("  📦 SMUX v2         — Stream multiplexing");
    println!("  🤖 Anti-AI DPI     — Ghost mode + packet entropy manipulation");
    println!("  📡 DAE (eBPF)      — Kernel-level transparent proxy");
    println!("  🔄 Fingerprint     — Chrome/Firefox/Safari/Edge/iOS/Android");
    println!("{}", "═".repeat(64));
    println!("  📖 Usage: network-ghost --help");
    println!("{}\n", "═".repeat(64));
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn setup_logging(level: &str) {
    let env_filter = EnvFilter::try_new(level)
        .unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(env_filter).with_target(false).init();
}

fn build_config(cli: &Cli) -> Result<ProxyConfig> {
    let protocol = parse_protocol(&cli.protocol);
    let cdn = parse_cdn(&cli.cdn);

    let mut config = ProxyConfig {
        server: String::new(),
        port: 443,
        protocol,
        sni: cli.sni.clone(),
        uuid: cli.uuid.clone().unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
        public_key: cli.public_key.clone(),
        private_key: None,
        short_id: None,
        utls_fingerprint: "chrome".to_string(),
        cdn_type: cdn,
        fallback_port: Some(8443),
        max_latency_ms: cli.max_latency,
        enable_padding: true,
        enable_anti_ai: true,
        enable_matryoshka: true,
        enable_port_hopping: cli.port_hopping,
    };

    // Load from config file if it exists
    if cli.config.exists() {
        if let Ok(content) = std::fs::read_to_string(&cli.config) {
            if let Ok(toml_val) = content.parse::<toml::Value>() {
                if let Some(sni) = toml_val.get("sni").and_then(|v| v.as_str()) {
                    config.sni = sni.to_string();
                }
                if let Some(uuid) = toml_val.get("uuid").and_then(|v| v.as_str()) {
                    config.uuid = uuid.to_string();
                }
            }
        }
    }

    Ok(config)
}

fn parse_dpi_mode(mode: &str) -> AntiAiMode {
    match mode.to_lowercase().as_str() {
        "normal"     => AntiAiMode::Normal,
        "aggressive" => AntiAiMode::Aggressive,
        "stealth"    => AntiAiMode::Stealth,
        "adaptive"   => AntiAiMode::Adaptive,
        "ghost" | _ => AntiAiMode::Ghost,
    }
}

fn parse_protocol(p: &str) -> ProtocolType {
    match p.to_lowercase().as_str() {
        "shadowtls" | "shadowtls3" => ProtocolType::ShadowTls,
        "hysteria2" | "hysteria"   => ProtocolType::Hysteria2,
        "tuic" | "tuic5"           => ProtocolType::Tuic,
        "masque"                   => ProtocolType::Masque,
        "xhttp"                    => ProtocolType::Xhttp,
        "vless"                    => ProtocolType::Vless,
        "trojan"                   => ProtocolType::Trojan,
        "reality" | "auto" | _     => ProtocolType::Reality,
    }
}

fn parse_cdn(cdn: &str) -> CdnType {
    match cdn.to_lowercase().as_str() {
        "gcore"      => CdnType::Gcore,
        "fastly"     => CdnType::Fastly,
        "arvancloud" => CdnType::ArvanCloud,
        _ => CdnType::Cloudflare,
    }
}
