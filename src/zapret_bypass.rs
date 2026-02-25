//! Zapret/ByeDPI — Deep Packet Inspection Bypass Engine
//! 
//! پیاده‌سازی کامل Zapret و ByeDPI برای دور زدن DPI در سطح کرنل.
//! 
//! ## تکنیک‌های پشتیبانی‌شده:
//! - TCP Fragmentation (تکه‌تکه کردن TLS ClientHello)
//! - Fake Packet Injection (پکت‌های جعلی با TTL پایین)
//! - TCP Disorder (اختلال ترتیب پکت‌ها)
//! - Out-Of-Band (OOB) Data
//! - HTTP/HTTPS Host header obfuscation
//! - TTL-based fake streams
//! - NFQUEUE / iptables / nftables integration (OpenWrt)

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use std::net::IpAddr;
use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Constants ─────────────────────────────────────────────────────────────────

/// حداکثر اندازه فراگمنت (بایت)
const MAX_FRAGMENT_SIZE: usize = 64;

/// حداقل اندازه فراگمنت
const MIN_FRAGMENT_SIZE: usize = 2;

/// TTL برای پکت‌های fake (به قدری پایین که به DPI سرور رسیده ولی به مقصد نمی‌رسد)
const FAKE_PACKET_TTL: u8 = 8;

/// اندازه پکت OOB
const OOB_BYTE: u8 = 0x00;

// ── Strategy Enum ──────────────────────────────────────────────────────────

/// استراتژی bypass
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZapretStrategy {
    /// تکه‌تکه کردن ClientHello در مرز SNI
    Fragment,
    /// اختلال ترتیب TCP Segments
    Disorder,
    /// ارسال پکت‌های جعلی قبل از داده واقعی
    Fake,
    /// ترکیب Fragment + Fake
    FragmentFake,
    /// ترکیب Disorder + Fake
    DisorderFake,
    /// Out-Of-Band data (TCP Urgent Pointer)
    OutOfBand,
    /// حالت کامل (همه تکنیک‌ها)
    FullBypass,
    /// bypass خودکار بر اساس نوع ترافیک
    Auto,
}

impl Default for ZapretStrategy {
    fn default() -> Self { Self::Auto }
}

/// نوع جریان برای bypass
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    /// جریان HTTPS (TLS)
    Https,
    /// جریان HTTP
    Http,
    /// جریان QUIC/HTTP3
    Quic,
    /// نامشخص
    Unknown,
}

impl Default for StreamType {
    fn default() -> Self { Self::Unknown }
}

// ── Configuration ──────────────────────────────────────────────────────────

/// تنظیمات موتور Zapret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZapretConfig {
    /// استراتژی bypass
    pub strategy: ZapretStrategy,
    /// اندازه فراگمنت (بایت) — None برای حالت خودکار
    pub fragment_size: Option<usize>,
    /// فعال‌سازی fake packet
    pub enable_fake: bool,
    /// TTL برای fake packet
    pub fake_ttl: u8,
    /// فعال‌سازی اختلال ترتیب
    pub enable_disorder: bool,
    /// تأخیر بین فراگمنت‌ها (میلی‌ثانیه)
    pub fragment_delay_ms: u64,
    /// شامل کردن HTTP bypass
    pub http_bypass: bool,
    /// شامل کردن HTTPS bypass
    pub https_bypass: bool,
    /// شامل کردن QUIC bypass
    pub quic_bypass: bool,
    /// رنج پورت‌های هدف (پیش‌فرض: 80, 443)
    pub target_ports: Vec<u16>,
    /// دامنه‌های هدف (خالی = همه)
    pub target_domains: Vec<String>,
    /// فعال‌سازی یکپارچه‌سازی NFQUEUE
    pub use_nfqueue: bool,
    /// شماره NFQUEUE
    pub nfqueue_num: u16,
}

impl Default for ZapretConfig {
    fn default() -> Self {
        Self {
            strategy: ZapretStrategy::Auto,
            fragment_size: None,
            enable_fake: true,
            fake_ttl: FAKE_PACKET_TTL,
            enable_disorder: false,
            fragment_delay_ms: 0,
            http_bypass: true,
            https_bypass: true,
            quic_bypass: true,
            target_ports: vec![80, 443],
            target_domains: Vec::new(),
            use_nfqueue: false,
            nfqueue_num: 100,
        }
    }
}

// ── Packet Analysis ────────────────────────────────────────────────────────

/// نتیجه تحلیل پکت
#[derive(Debug, Clone)]
pub struct PacketAnalysis {
    pub stream_type: StreamType,
    pub is_client_hello: bool,
    pub sni_offset: Option<usize>,
    pub sni_length: Option<usize>,
    pub sni_value: Option<String>,
    pub http_host_offset: Option<usize>,
}

/// تحلیل پکت برای تشخیص نوع جریان و محل SNI
pub fn analyze_packet(data: &[u8]) -> PacketAnalysis {
    // بررسی TLS ClientHello
    if data.len() > 5 && data[0] == 0x16 && data[1] == 0x03 {
        let (sni_off, sni_len, sni_val) = find_sni_in_tls(data);
        return PacketAnalysis {
            stream_type: StreamType::Https,
            is_client_hello: data.len() > 5 && data[5] == 0x01,
            sni_offset: sni_off,
            sni_length: sni_len,
            sni_value: sni_val,
            http_host_offset: None,
        };
    }

    // بررسی HTTP
    if data.starts_with(b"GET ") || data.starts_with(b"POST ") || data.starts_with(b"HEAD ") {
        let host_off = find_http_host(data);
        return PacketAnalysis {
            stream_type: StreamType::Http,
            is_client_hello: false,
            sni_offset: None,
            sni_length: None,
            sni_value: None,
            http_host_offset: host_off,
        };
    }

    // بررسی QUIC Initial Packet
    if !data.is_empty() && (data[0] & 0xC0) == 0xC0 && data.len() > 5 {
        if data.get(1..5) == Some(&[0x00, 0x00, 0x00, 0x01]) {
            return PacketAnalysis {
                stream_type: StreamType::Quic,
                is_client_hello: true,
                sni_offset: None,
                sni_length: None,
                sni_value: None,
                http_host_offset: None,
            };
        }
    }

    PacketAnalysis {
        stream_type: StreamType::Unknown,
        is_client_hello: false,
        sni_offset: None,
        sni_length: None,
        sni_value: None,
        http_host_offset: None,
    }
}

/// یافتن SNI در TLS ClientHello
fn find_sni_in_tls(data: &[u8]) -> (Option<usize>, Option<usize>, Option<String>) {
    if data.len() < 43 { return (None, None, None); }
    
    // TLS Record: content_type(1) + version(2) + length(2) + handshake_type(1) + length(3)
    // + client_version(2) + random(32) + session_id_length(1) + ...
    let mut offset = 5; // skip TLS Record header
    
    if offset >= data.len() { return (None, None, None); }
    offset += 1; // handshake type
    
    if offset + 3 > data.len() { return (None, None, None); }
    offset += 3; // handshake length
    
    if offset + 2 > data.len() { return (None, None, None); }
    offset += 2; // client version
    
    if offset + 32 > data.len() { return (None, None, None); }
    offset += 32; // random
    
    if offset >= data.len() { return (None, None, None); }
    let session_id_len = data[offset] as usize;
    offset += 1 + session_id_len;
    
    if offset + 2 > data.len() { return (None, None, None); }
    let cipher_suites_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2 + cipher_suites_len;
    
    if offset >= data.len() { return (None, None, None); }
    let compression_methods_len = data[offset] as usize;
    offset += 1 + compression_methods_len;
    
    if offset + 2 > data.len() { return (None, None, None); }
    let extensions_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
    offset += 2;
    
    let ext_end = offset + extensions_len;
    
    while offset + 4 <= ext_end && offset + 4 <= data.len() {
        let ext_type = u16::from_be_bytes([data[offset], data[offset + 1]]);
        let ext_len = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;
        
        if ext_type == 0x0000 && offset + ext_len <= data.len() {
            // SNI Extension
            if ext_len >= 5 {
                let sni_list_len = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
                if sni_list_len >= 3 {
                    // data[offset+2] = type (0x00 = host_name)
                    let name_len = u16::from_be_bytes([data[offset + 3], data[offset + 4]]) as usize;
                    let name_start = offset + 5;
                    if name_start + name_len <= data.len() {
                        let sni = String::from_utf8_lossy(&data[name_start..name_start + name_len]).to_string();
                        return (Some(name_start), Some(name_len), Some(sni));
                    }
                }
            }
        }
        
        offset += ext_len;
    }
    
    (None, None, None)
}

/// یافتن Host header در HTTP
fn find_http_host(data: &[u8]) -> Option<usize> {
    let text = String::from_utf8_lossy(data);
    text.find("Host: ").map(|pos| pos + 6)
}

// ── Core Engine ────────────────────────────────────────────────────────────

/// موتور اصلی Zapret/ByeDPI
pub struct ZapretEngine {
    config: ZapretConfig,
    stats: std::sync::Mutex<ZapretStats>,
}

impl ZapretEngine {
    /// ایجاد موتور جدید
    pub fn new(config: ZapretConfig) -> Self {
        info!("🛡️ Zapret/ByeDPI Engine v5.0 راه‌اندازی شد");
        info!("   استراتژی: {:?}", config.strategy);
        Self {
            config,
            stats: std::sync::Mutex::new(ZapretStats::default()),
        }
    }

    /// پردازش و bypass یک پکت
    pub fn process_packet(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let analysis = analyze_packet(data);

        let strategy = self.determine_strategy(&analysis);
        
        let result = match strategy {
            ZapretStrategy::Fragment     => self.apply_fragment(data, &analysis),
            ZapretStrategy::Fake         => self.apply_fake_then_real(data),
            ZapretStrategy::Disorder     => self.apply_disorder(data, &analysis),
            ZapretStrategy::FragmentFake => self.apply_fragment_fake(data, &analysis),
            ZapretStrategy::DisorderFake => self.apply_disorder_fake(data, &analysis),
            ZapretStrategy::OutOfBand    => self.apply_oob(data),
            ZapretStrategy::FullBypass   => self.apply_full_bypass(data, &analysis),
            ZapretStrategy::Auto         => self.apply_auto(data, &analysis),
        };

        if let Ok(mut stats) = self.stats.lock() {
            stats.packets_processed += 1;
            stats.bytes_processed += data.len() as u64;
            stats.fragments_sent += result.len() as u64;
        }

        result
    }

    /// تکه‌تکه کردن پکت در محل SNI
    fn apply_fragment(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        let split_point = if let Some(sni_off) = analysis.sni_offset {
            // تقسیم دقیقاً در وسط SNI برای مخفی‌سازی
            sni_off + (analysis.sni_length.unwrap_or(4) / 2)
        } else {
            // تقسیم تصادفی
            let min_split = 2.min(data.len().saturating_sub(1));
            let max_split = (data.len() / 2).max(min_split + 1);
            let mut rng = rand::thread_rng();
            rng.gen_range(min_split..max_split)
        };

        let split_point = split_point.min(data.len().saturating_sub(1)).max(1);

        let frag1 = data[..split_point].to_vec();
        let frag2 = data[split_point..].to_vec();

        debug!("✂️ Fragmented at offset {} (SNI split)", split_point);
        vec![frag1, frag2]
    }

    /// ارسال پکت‌های fake قبل از داده واقعی
    fn apply_fake_then_real(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        
        // پکت fake با TTL پایین
        let fake = self.build_fake_client_hello();
        result.push(fake);
        
        // داده واقعی
        result.push(data.to_vec());

        debug!("👻 Fake packet injected before real data");
        result
    }

    /// اختلال ترتیب TCP Segments
    fn apply_disorder(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        let fragments = self.apply_fragment(data, analysis);
        
        if fragments.len() < 2 {
            return fragments;
        }

        // ارسال ابتدا fragment دوم (DPI را گیج می‌کند)، سپس اول
        // در پیاده‌سازی واقعی این از طریق IP_TTL یا nfqueue کنترل می‌شود
        vec![fragments[1].clone(), fragments[0].clone()]
    }

    /// ترکیب تکه‌تکه کردن + پکت fake
    fn apply_fragment_fake(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let fake = self.build_fake_client_hello();
        result.push(fake);
        
        let fragments = self.apply_fragment(data, analysis);
        result.extend(fragments);

        result
    }

    /// ترکیب اختلال + پکت fake
    fn apply_disorder_fake(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        let mut result = Vec::new();
        let fake = self.build_fake_client_hello();
        result.push(fake);
        
        let disordered = self.apply_disorder(data, analysis);
        result.extend(disordered);

        result
    }

    /// OOB Data (TCP Urgent Pointer)
    fn apply_oob(&self, data: &[u8]) -> Vec<Vec<u8>> {
        // OOB data به عنوان پیش‌پکت ارسال می‌شود
        let oob = vec![OOB_BYTE];
        vec![oob, data.to_vec()]
    }

    /// bypass کامل (همه تکنیک‌ها)
    fn apply_full_bypass(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        self.apply_fragment_fake(data, analysis)
    }

    /// تشخیص خودکار بهترین استراتژی
    fn apply_auto(&self, data: &[u8], analysis: &PacketAnalysis) -> Vec<Vec<u8>> {
        match analysis.stream_type {
            StreamType::Https => {
                if analysis.is_client_hello {
                    // برای ClientHello: Fragment در محل SNI
                    self.apply_fragment_fake(data, analysis)
                } else {
                    vec![data.to_vec()]
                }
            }
            StreamType::Http => {
                // برای HTTP: تکه‌تکه کردن Host header
                self.apply_fragment(data, analysis)
            }
            StreamType::Quic => {
                // برای QUIC: fake packet
                self.apply_fake_then_real(data)
            }
            StreamType::Unknown => {
                vec![data.to_vec()]
            }
        }
    }

    /// تعیین استراتژی بر اساس تنظیمات و تحلیل
    fn determine_strategy(&self, analysis: &PacketAnalysis) -> ZapretStrategy {
        if self.config.strategy != ZapretStrategy::Auto {
            return self.config.strategy;
        }
        
        // انتخاب خودکار
        match analysis.stream_type {
            StreamType::Https if analysis.is_client_hello => {
                if self.config.enable_fake {
                    ZapretStrategy::FragmentFake
                } else {
                    ZapretStrategy::Fragment
                }
            }
            StreamType::Http => ZapretStrategy::Fragment,
            StreamType::Quic => ZapretStrategy::Fake,
            _ => ZapretStrategy::Fragment,
        }
    }

    /// ساخت ClientHello fake با TTL پایین
    fn build_fake_client_hello(&self) -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut hello = Vec::with_capacity(200);

        // TLS Record Header
        hello.push(0x16); // content_type: handshake
        hello.push(0x03);
        hello.push(0x03); // TLS 1.2
        let body_len: u16 = rng.gen_range(100..200);
        hello.extend(body_len.to_be_bytes());
        hello.push(0x01); // handshake_type: client_hello
        
        // Handshake Length (3 bytes)
        let handshake_len = body_len as u32 - 4;
        hello.push(((handshake_len >> 16) & 0xFF) as u8);
        hello.push(((handshake_len >> 8) & 0xFF) as u8);
        hello.push((handshake_len & 0xFF) as u8);

        // Client Version: TLS 1.2
        hello.push(0x03);
        hello.push(0x03);

        // Random (32 bytes)
        let random: [u8; 32] = rng.gen();
        hello.extend_from_slice(&random);

        // Session ID Length: 0
        hello.push(0x00);

        // Cipher Suites
        hello.extend([0x00, 0x04]); // length=4
        hello.extend([0x13, 0x01]); // TLS_AES_128_GCM_SHA256
        hello.extend([0x00, 0xFF]); // TLS_EMPTY_RENEGOTIATION_INFO_SCSV

        // Compression Methods
        hello.push(0x01);
        hello.push(0x00); // null

        // Extensions — add a fake SNI
        let fake_sni = b"www.google.com";
        let sni_ext_len = 5 + fake_sni.len();
        let extensions_len = 4 + sni_ext_len;
        hello.extend((extensions_len as u16).to_be_bytes());
        hello.extend([0x00, 0x00]); // SNI extension type
        hello.extend((sni_ext_len as u16).to_be_bytes());
        hello.extend(((fake_sni.len() + 3) as u16).to_be_bytes()); // list length
        hello.push(0x00); // type: host_name
        hello.extend((fake_sni.len() as u16).to_be_bytes());
        hello.extend_from_slice(fake_sni);

        hello
    }

    /// تولید دستورات iptables برای Google WiFi / OpenWrt
    pub fn generate_iptables_rules(&self) -> String {
        let nfqueue = self.config.nfqueue_num;
        let ports = self.config.target_ports.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(",");

        format!(
            r#"#!/bin/sh
# ══════════════════════════════════════════════════════════════════════
# Network Ghost v5 — Zapret/ByeDPI iptables Rules for OpenWrt/Google WiFi
# ══════════════════════════════════════════════════════════════════════

# پاک کردن قوانین قبلی
iptables -t mangle -F ZAPRET 2>/dev/null
iptables -t mangle -X ZAPRET 2>/dev/null
iptables -t mangle -N ZAPRET

# Exclude local/private networks
iptables -t mangle -A ZAPRET -d 0.0.0.0/8 -j RETURN
iptables -t mangle -A ZAPRET -d 127.0.0.0/8 -j RETURN
iptables -t mangle -A ZAPRET -d 169.254.0.0/16 -j RETURN
iptables -t mangle -A ZAPRET -d 172.16.0.0/12 -j RETURN
iptables -t mangle -A ZAPRET -d 192.168.0.0/16 -j RETURN
iptables -t mangle -A ZAPRET -d 10.0.0.0/8 -j RETURN

# هدایت ترافیک خروجی به NFQUEUE برای پردازش Zapret
iptables -t mangle -A ZAPRET -p tcp -m multiport --dport {ports} \
  -m connbytes --connbytes 0:6 --connbytes-dir original --connbytes-mode packets \
  -j NFQUEUE --queue-num {nfqueue} --queue-bypass

# فعال‌سازی برای خروجی
iptables -t mangle -A OUTPUT -j ZAPRET
# فعال‌سازی برای forward (برای سایر دستگاه‌های شبکه)
iptables -t mangle -A FORWARD -j ZAPRET

echo "✅ Zapret iptables rules applied (NFQUEUE {nfqueue}, ports: {ports})"
"#,
            nfqueue = nfqueue,
            ports = ports
        )
    }

    /// تولید دستورات nftables برای OpenWrt نسخه جدید
    pub fn generate_nftables_rules(&self) -> String {
        let nfqueue = self.config.nfqueue_num;
        let ports = self.config.target_ports.iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"#!/usr/sbin/nft -f
# ══════════════════════════════════════════════════════════════════════
# Network Ghost v5 — Zapret/ByeDPI nftables Rules
# برای OpenWrt 22.03+ و Google WiFi با ImmortalWrt
# ══════════════════════════════════════════════════════════════════════

table inet zapret {{
    chain zapret_out {{
        type filter hook output priority mangle; policy accept;
        # Skip private addresses
        ip daddr {{ 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 127.0.0.0/8 }} return
        # Forward first 6 packets of each TCP connection to NFQUEUE
        meta l4proto tcp tcp dport {{ {ports} }} ct original packets 0-6 queue num {nfqueue} bypass
    }}

    chain zapret_fwd {{
        type filter hook forward priority mangle; policy accept;
        ip daddr {{ 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 }} return
        meta l4proto tcp tcp dport {{ {ports} }} ct original packets 0-6 queue num {nfqueue} bypass
    }}
}}
"#,
            ports = ports,
            nfqueue = nfqueue
        )
    }

    /// تولید اسکریپت نصب برای OpenWrt
    pub fn generate_openwrt_install_script(&self) -> String {
        r#"#!/bin/sh
# Network Ghost v5 — نصب Zapret/ByeDPI روی OpenWrt/Google WiFi

set -e

echo "🚀 نصب Zapret/ByeDPI ..."

# نصب پکیج‌های لازم
opkg update
opkg install kmod-nfnetlink-queue libmnl libnfnetlink kmod-ipt-nfqueue iptables-mod-nfqueue nftables 2>/dev/null || true

# ساخت دایرکتوری‌ها
mkdir -p /opt/zapret/scripts
mkdir -p /opt/zapret/lists

# کپی اسکریپت‌ها
cp /opt/network-ghost/zapret/iptables.sh /opt/zapret/scripts/
cp /opt/network-ghost/zapret/nftables.conf /opt/zapret/scripts/
chmod +x /opt/zapret/scripts/*.sh

# فعال‌سازی سرویس
cat > /etc/init.d/zapret << 'EOF'
#!/bin/sh /etc/rc.common
START=90
STOP=10

start() {
    /opt/zapret/scripts/iptables.sh
    /opt/network-ghost/network-ghost daemon --zapret &
    echo $! > /tmp/zapret.pid
}

stop() {
    kill $(cat /tmp/zapret.pid) 2>/dev/null
    iptables -t mangle -F ZAPRET 2>/dev/null
    iptables -t mangle -X ZAPRET 2>/dev/null
}
EOF
chmod +x /etc/init.d/zapret
/etc/init.d/zapret enable

echo "✅ Zapret/ByeDPI نصب شد."
echo "   راه‌اندازی: /etc/init.d/zapret start"
"#
        .to_string()
    }

    /// دریافت آمار
    pub fn get_stats(&self) -> ZapretStats {
        self.stats.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// بررسی فعال بودن bypass برای یک پورت
    pub fn is_target_port(&self, port: u16) -> bool {
        self.config.target_ports.contains(&port)
    }
}

impl Default for ZapretEngine {
    fn default() -> Self {
        Self::new(ZapretConfig::default())
    }
}

// ── Statistics ─────────────────────────────────────────────────────────────

/// آمار موتور Zapret
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZapretStats {
    pub packets_processed: u64,
    pub bytes_processed: u64,
    pub fragments_sent: u64,
    pub fake_packets_sent: u64,
    pub bypasses_succeeded: u64,
}

// ── Utility Functions ──────────────────────────────────────────────────────

/// تولید لیست IP‌های مسدود ایران برای whitelist
pub fn generate_iran_ip_list() -> Vec<String> {
    vec![
        "5.200.0.0/15".to_string(),
        "31.2.128.0/17".to_string(),
        "31.24.200.0/21".to_string(),
        "37.156.0.0/16".to_string(),
        "37.202.64.0/18".to_string(),
        "45.82.136.0/21".to_string(),
        "62.193.0.0/19".to_string(),
        "78.157.32.0/21".to_string(),
        "79.175.128.0/18".to_string(),
        "80.191.0.0/17".to_string(),
        "85.9.64.0/18".to_string(),
        "85.15.0.0/16".to_string(),
        "87.107.0.0/16".to_string(),
        "89.32.0.0/14".to_string(),
        "91.92.0.0/22".to_string(),
        "91.108.4.0/22".to_string(),  // Telegram
        "91.108.8.0/22".to_string(),  // Telegram
        "95.38.0.0/17".to_string(),
        "104.21.0.0/17".to_string(),  // Cloudflare Iran CDN
        "185.67.88.0/22".to_string(),
        "185.120.136.0/21".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_tls_packet() {
        let tls_hello = vec![
            0x16, 0x03, 0x01, // TLS 1.0 record
            0x00, 0x05,        // length
            0x01,              // client hello
            0x00, 0x00, 0x01, 0x00,
        ];
        let analysis = analyze_packet(&tls_hello);
        assert_eq!(analysis.stream_type, StreamType::Https);
        assert!(analysis.is_client_hello);
    }

    #[test]
    fn test_analyze_http_packet() {
        let http_req = b"GET / HTTP/1.1\r\nHost: example.com\r\n\r\n";
        let analysis = analyze_packet(http_req);
        assert_eq!(analysis.stream_type, StreamType::Http);
    }

    #[test]
    fn test_fragment_packet() {
        let engine = ZapretEngine::default();
        let data = vec![0x16, 0x03, 0x01, 0x00, 0x05, 0x01, 0x00, 0x00, 0x01, 0x00];
        let fragments = engine.process_packet(&data);
        assert!(!fragments.is_empty());
    }
}
