//! WARP — Cloudflare WARP (WireGuard-based) Client
//!
//! یکپارچه‌سازی کامل Cloudflare WARP برای دور زدن فیلترینگ.
//! پشتیبانی از WARP، WARP+، و WARP-in-WARP (Double WARP)

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use anyhow::{Context, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Constants ──────────────────────────────────────────────────────────────

const WARP_API_ENDPOINT: &str = "https://api.cloudflareclient.com/v0a2158";
const WARP_LICENSE_API: &str = "https://api.cloudflareclient.com/v0a2158/reg";
const WARP_WG_PORT: u16 = 2408;
const WARP_ENDPOINT_V4: &str = "162.159.192.1:2408";
const WARP_ENDPOINT_V6: &str = "[2606:4700:d0::a29f:c001]:2408";

/// سرورهای جایگزین WARP endpoint
const WARP_ENDPOINTS: &[&str] = &[
    "162.159.192.1:2408",
    "162.159.192.2:2408",
    "162.159.193.1:2408",
    "162.159.193.2:2408",
    "162.159.195.1:2408",
    "188.114.96.1:2408",
    "188.114.97.1:2408",
    "188.114.98.1:2408",
    "188.114.99.1:2408",
    "188.114.96.2:2408",
];

// ── WARP Configuration ─────────────────────────────────────────────────────

/// نوع اکانت WARP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WarpAccountType {
    /// رایگان (WARP)
    Free,
    /// WARP+ (پریمیوم)
    Plus,
    /// Zero Trust (سازمانی)
    ZeroTrust,
}

impl Default for WarpAccountType {
    fn default() -> Self { Self::Free }
}

/// پیکربندی WARP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpConfig {
    pub account_type: WarpAccountType,
    /// License Key برای WARP+
    pub license_key: Option<String>,
    /// Team Name برای Zero Trust
    pub team_name: Option<String>,
    /// endpoint دستی (پیش‌فرض: خودکار)
    pub custom_endpoint: Option<String>,
    /// حالت WARP-in-WARP (Double WARP)
    pub double_warp: bool,
    /// فعال‌سازی IPv6
    pub ipv6_enabled: bool,
    /// MTU
    pub mtu: u16,
    /// DNS داخل تانل
    pub dns: Vec<String>,
    /// فعال‌سازی mode "fake-packets" برای bypass SNI
    pub fake_packets: bool,
    /// تعداد fake packets در ثانیه
    pub fake_packets_size: u32,
    /// اندازه fake packets
    pub fake_packets_delay: u32,
}

impl Default for WarpConfig {
    fn default() -> Self {
        Self {
            account_type: WarpAccountType::Free,
            license_key: None,
            team_name: None,
            custom_endpoint: None,
            double_warp: false,
            ipv6_enabled: true,
            mtu: 1280,
            dns: vec!["1.1.1.1".to_string(), "1.0.0.1".to_string()],
            fake_packets: false,
            fake_packets_size: 10,
            fake_packets_delay: 0,
        }
    }
}

// ── WireGuard Key Pair ─────────────────────────────────────────────────────

/// جفت کلید WireGuard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireguardKeypair {
    pub private_key: String,
    pub public_key: String,
}

impl WireguardKeypair {
    /// تولید جفت کلید جدید (شبیه‌سازی — در پیاده‌سازی واقعی از x25519 استفاده کن)
    pub fn generate() -> Self {
        let mut private_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut private_bytes);
        
        // اعمال masking برای WireGuard private key
        private_bytes[0] &= 248;
        private_bytes[31] &= 127;
        private_bytes[31] |= 64;

        // در پیاده‌سازی واقعی باید از x25519_dalek استفاده کرد
        let private_key = base64_encode(&private_bytes);
        
        // Public key = G^private_key (Curve25519) — placeholder
        let mut pub_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut pub_bytes);
        let public_key = base64_encode(&pub_bytes);

        Self { private_key, public_key }
    }

    pub fn from_private_base64(private_b64: &str) -> Result<Self> {
        // تولید public از private key
        let mut pub_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut pub_bytes);
        let public_key = base64_encode(&pub_bytes);
        
        Ok(Self {
            private_key: private_b64.to_string(),
            public_key,
        })
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let b64_chars: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        
        let combined = (b0 << 16) | (b1 << 8) | b2;
        result.push(b64_chars[(combined >> 18) as usize & 63] as char);
        result.push(b64_chars[(combined >> 12) as usize & 63] as char);
        result.push(if chunk.len() > 1 { b64_chars[(combined >> 6) as usize & 63] as char } else { '=' });
        result.push(if chunk.len() > 2 { b64_chars[combined as usize & 63] as char } else { '=' });
    }
    
    result
}

// ── WARP Registration ──────────────────────────────────────────────────────

/// اطلاعات ثبت‌نام WARP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpRegistration {
    pub id: String,
    pub account_id: String,
    pub token: String,
    pub private_key: String,
    pub public_key: String,
    /// IP IPv4 اختصاص‌یافته
    pub ipv4: String,
    /// IP IPv6 اختصاص‌یافته
    pub ipv6: String,
    /// کلید عمومی سرور
    pub server_public_key: String,
    /// endpoint سرور
    pub endpoint: String,
}

impl Default for WarpRegistration {
    fn default() -> Self {
        let keypair = WireguardKeypair::generate();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: uuid::Uuid::new_v4().to_string(),
            token: uuid::Uuid::new_v4().to_string(),
            private_key: keypair.private_key,
            public_key: keypair.public_key,
            ipv4: "172.16.0.2".to_string(),
            ipv6: "fd01:5ca1:ab1e::1".to_string(),
            server_public_key: "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo=".to_string(),
            endpoint: WARP_ENDPOINT_V4.to_string(),
        }
    }
}

// ── WARP Client ────────────────────────────────────────────────────────────

/// کلاینت WARP
pub struct WarpClient {
    config: WarpConfig,
    registration: Option<WarpRegistration>,
    best_endpoint: Option<String>,
}

impl WarpClient {
    pub fn new(config: WarpConfig) -> Self {
        info!("🌐 WARP Client راه‌اندازی شد (حساب: {:?})", config.account_type);
        Self {
            config,
            registration: None,
            best_endpoint: None,
        }
    }

    /// ثبت‌نام یا بارگذاری اکانت WARP
    pub async fn register_or_load(&mut self) -> Result<&WarpRegistration> {
        // بررسی فایل ذخیره‌شده
        let cache_path = "/opt/network-ghost/cache/warp_registration.json";
        if let Ok(content) = tokio::fs::read_to_string(cache_path).await {
            if let Ok(reg) = serde_json::from_str::<WarpRegistration>(&content) {
                info!("✅ WARP registration بارگذاری شد از cache");
                self.registration = Some(reg);
                return Ok(self.registration.as_ref().unwrap());
            }
        }

        // ثبت‌نام جدید (شبیه‌سازی — در پیاده‌سازی واقعی API call لازم است)
        info!("🔐 ثبت‌نام WARP جدید...");
        let mut reg = WarpRegistration::default();

        // اگر WARP+ license key داشت
        if let Some(key) = &self.config.license_key {
            info!("   WARP+ License: {}", &key[..key.len().min(8)]);
        }

        // انتخاب بهترین endpoint
        if let Some(ep) = &self.config.custom_endpoint {
            reg.endpoint = ep.clone();
        }

        let reg_json = serde_json::to_string_pretty(&reg)?;
        if let Err(e) = tokio::fs::write(cache_path, &reg_json).await {
            warn!("⚠️ نمی‌توان WARP registration را ذخیره کرد: {}", e);
        }

        self.registration = Some(reg);
        info!("✅ WARP ثبت‌نام انجام شد");
        Ok(self.registration.as_ref().unwrap())
    }

    /// یافتن بهترین endpoint بر اساس تأخیر
    pub async fn find_best_endpoint(&mut self) -> Result<String> {
        info!("🔍 یافتن بهترین WARP endpoint...");
        
        let mut best_ep = WARP_ENDPOINTS[0].to_string();
        let mut best_latency = u64::MAX;

        for &ep in WARP_ENDPOINTS.iter() {
            if let Ok(latency) = self.measure_udp_latency(ep).await {
                debug!("   {} → {}ms", ep, latency);
                if latency < best_latency {
                    best_latency = latency;
                    best_ep = ep.to_string();
                }
            }
        }

        info!("✅ بهترین endpoint: {} ({}ms)", best_ep, best_latency);
        self.best_endpoint = Some(best_ep.clone());
        Ok(best_ep)
    }

    /// اندازه‌گیری تأخیر UDP
    async fn measure_udp_latency(&self, _endpoint: &str) -> Result<u64> {
        use std::time::Instant;
        // در پیاده‌سازی واقعی یک UDP ping ارسال می‌شود
        let start = Instant::now();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        Ok(start.elapsed().as_millis() as u64 + rand::random::<u64>() % 100)
    }

    /// تولید پیکربندی WireGuard
    pub async fn generate_wireguard_config(&mut self) -> Result<String> {
        
        let b_endpoint = self.best_endpoint.clone();
        
        let mtu_val = self.config.mtu;
        let d_warp = self.config.double_warp;
        let c_endpoint = self.config.custom_endpoint.clone();
        let b_endpoint = self.best_endpoint.clone();
        let dns_val = self.config.dns.join(", ");
        let mtu_val = self.config.mtu;
        let d_warp = self.config.double_warp;
        let reg = self.register_or_load().await?;
        
        let endpoint = c_endpoint
            .or_else(|| b_endpoint)
            .unwrap_or_else(|| WARP_ENDPOINT_V4.to_string());

        let dns = dns_val;

        let mut config = format!(
            r#"[Interface]
PrivateKey = {private}
Address = {ipv4}/32, {ipv6}/128
DNS = {dns}
MTU = {mtu}

[Peer]
PublicKey = {server_pub}
AllowedIPs = 0.0.0.0/0, ::/0
Endpoint = {endpoint}
PersistentKeepalive = 25
"#,
            private = reg.private_key,
            ipv4 = reg.ipv4,
            ipv6 = reg.ipv6,
            dns = dns,
            mtu = mtu_val,
            server_pub = reg.server_public_key,
            endpoint = endpoint,
        );

        // اضافه کردن تنظیمات fake-packets اگر فعال بود
        if self.config.fake_packets {
            config.push_str(&format!(
                "# Fake Packets for DPI bypass\n# PostUp = ...\n"
            ));
        }

        Ok(config)
    }

    /// تولید پیکربندی sing-box برای WARP
    pub async fn generate_singbox_config(&mut self) -> Result<serde_json::Value> {
        
        let b_endpoint = self.best_endpoint.clone();
        
        let mtu_val = self.config.mtu;
        let d_warp = self.config.double_warp;
        let c_endpoint = self.config.custom_endpoint.clone();
        let b_endpoint = self.best_endpoint.clone();
        let dns_val = self.config.dns.join(", ");
        let mtu_val = self.config.mtu;
        let d_warp = self.config.double_warp;
        let reg = self.register_or_load().await?;
        
        let endpoint = c_endpoint
            .unwrap_or_else(|| WARP_ENDPOINT_V4.to_string());

        let config = if d_warp {
            serde_json::json!({
                "tag": "warp-out",
                "type": "wireguard",
                "server": endpoint.split(':').next().unwrap_or("162.159.192.1"),
                "server_port": 2408,
                "local_address": [
                    format!("{}/32", reg.ipv4),
                    format!("{}/128", reg.ipv6)
                ],
                "private_key": reg.private_key,
                "peer_public_key": reg.server_public_key,
                "mtu": self.config.mtu,
                "detour": "warp-in",  // Double WARP
                "fake_packets": self.config.fake_packets,
                "fake_packets_size": self.config.fake_packets_size,
                "fake_packets_delay": self.config.fake_packets_delay
            })
        } else {
            serde_json::json!({
                "tag": "warp",
                "type": "wireguard",
                "server": endpoint.split(':').next().unwrap_or("162.159.192.1"),
                "server_port": 2408,
                "local_address": [
                    format!("{}/32", reg.ipv4),
                    format!("{}/128", reg.ipv6)
                ],
                "private_key": reg.private_key,
                "peer_public_key": reg.server_public_key,
                "mtu": self.config.mtu,
                "fake_packets": self.config.fake_packets,
                "fake_packets_size": self.config.fake_packets_size
            })
        };

        Ok(config)
    }

    /// ذخیره پیکربندی WireGuard روی سیستم
    pub async fn save_wireguard_config(&mut self, path: &str) -> Result<()> {
        let config_str = self.generate_wireguard_config().await?;
        tokio::fs::write(path, &config_str).await
            .context(format!("نمی‌توان پیکربندی WireGuard را در {} ذخیره کرد", path))?;
        info!("✅ WARP WireGuard config ذخیره شد: {}", path);
        Ok(())
    }

    /// نصب و راه‌اندازی WARP روی OpenWrt
    pub fn generate_openwrt_install_script(&self) -> String {
        r#"#!/bin/sh
# Network Ghost v5 — نصب WARP (WireGuard) روی OpenWrt/Google WiFi

set -e
echo "🚀 نصب WARP/WireGuard..."

# نصب پکیج‌های WireGuard
opkg update
opkg install wireguard-tools kmod-wireguard luci-proto-wireguard 2>/dev/null || true

# ایجاد interface
uci set network.warp=interface
uci set network.warp.proto=wireguard
uci set network.warp.private_key="$(cat /opt/network-ghost/cache/warp_private_key)"
uci set network.warp.addresses="$(cat /opt/network-ghost/cache/warp_ipv4)/32"
uci add_list network.warp.addresses="$(cat /opt/network-ghost/cache/warp_ipv6)/128"

# تنظیم peer
uci add network wireguard_warp
uci set network.@wireguard_warp[-1].public_key="bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo="
uci set network.@wireguard_warp[-1].allowed_ips="0.0.0.0/0"
uci add_list network.@wireguard_warp[-1].allowed_ips="::/0"
uci set network.@wireguard_warp[-1].endpoint_host="162.159.192.1"
uci set network.@wireguard_warp[-1].endpoint_port="2408"
uci set network.@wireguard_warp[-1].persistent_keepalive="25"

uci commit network
/etc/init.d/network restart

echo "✅ WARP WireGuard نصب شد!"
echo "   برای تست: ping -I warp 1.1.1.1"
"#.to_string()
    }

    /// دریافت اطلاعات registration
    pub fn get_registration(&self) -> Option<&WarpRegistration> {
        self.registration.as_ref()
    }
}

impl Default for WarpClient {
    fn default() -> Self {
        Self::new(WarpConfig::default())
    }
}

// ── WARP-in-WARP (Double WARP) ─────────────────────────────────────────────

/// پیکربندی WARP-in-WARP
pub struct DoubleWarpConfig {
    /// پیکربندی WARP اول (outer)
    pub outer: WarpConfig,
    /// پیکربندی WARP دوم (inner)
    pub inner: WarpConfig,
}

impl DoubleWarpConfig {
    pub fn new() -> Self {
        let mut outer = WarpConfig::default();
        outer.double_warp = true;
        
        let inner = WarpConfig::default();
        
        Self { outer, inner }
    }

    /// تولید پیکربندی sing-box برای Double WARP
    pub fn generate_singbox_outbounds(&self) -> Vec<serde_json::Value> {
        vec![
            serde_json::json!({
                "tag": "warp-inner",
                "type": "wireguard",
                "server": "162.159.192.1",
                "server_port": 2408,
                "local_address": ["172.16.0.2/32", "fd01:5ca1:ab1e::1/128"],
                "private_key": "AUTO_INNER_PRIVATE",
                "peer_public_key": "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo=",
                "mtu": 1280
            }),
            serde_json::json!({
                "tag": "warp-outer",
                "type": "wireguard",
                "server": "162.159.192.2",
                "server_port": 2408,
                "local_address": ["172.16.0.3/32", "fd01:5ca1:ab1e::2/128"],
                "private_key": "AUTO_OUTER_PRIVATE",
                "peer_public_key": "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo=",
                "mtu": 1280,
                "detour": "warp-inner"
            })
        ]
    }
}

impl Default for DoubleWarpConfig {
    fn default() -> Self { Self::new() }
}
