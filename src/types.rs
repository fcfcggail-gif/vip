//! Shared types and constants for Network Ghost v5

use std::net::IpAddr;

use serde::{Deserialize, Serialize};

// ==================== CONSTANTS ====================

/// Cloudflare IP ranges
pub const CLOUDFLARE_RANGES: &[&str] = &[
    "104.16.0.0/13",
    "104.24.0.0/14",
    "172.64.0.0/13",
    "173.245.48.0/20",
    "188.114.96.0/20",
    "190.93.240.0/20",
    "197.234.240.0/22",
    "198.41.128.0/17",
    "162.158.0.0/15",
];

/// Alternative HTTPS ports for port hopping
pub const ALTERNATIVE_PORTS: &[u16] = &[443, 2053, 2083, 2087, 2096, 8443];

/// Iranian SNI whitelist for ShadowTLS spoofing
pub const IRANIAN_SNI_WHITELIST: &[&str] = &[
    "ebanking.bmi.ir",
    "ibsi.bmi.ir",
    "bpi.ir",
    "bankmellat.ir",
    "banksepah.ir",
    "edbi.ir",
    "bank-maskan.ir",
    "postbank.ir",
    "ttac.ir",
    "aparat.com",
    "digikala.com",
    "iranserver.com",
];

// ==================== PROTOCOL ENUMS ====================

/// Tunnel protocol type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtocolType {
    /// ShadowTLS v3
    ShadowTls,
    /// Reality with VLESS
    Reality,
    /// TUIC v5
    Tuic,
    /// Hysteria2
    Hysteria2,
    /// MASQUE
    Masque,
    /// XHTTP
    Xhttp,
    /// Trojan
    Trojan,
    /// VLESS
    Vless,
    /// Warp
    Warp,
    /// Cascade (layered)
    Cascade {
        /// Outer layer
        outer: Box<ProtocolType>,
        /// Inner layer
        inner: Box<ProtocolType>,
    },
}

impl Default for ProtocolType {
    fn default() -> Self {
        Self::Reality
    }
}

/// CDN type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CdnType {
    /// Cloudflare
    Cloudflare,
    /// GCore
    Gcore,
    /// Fastly
    Fastly,
    /// ArvanCloud
    ArvanCloud,
    /// Direct (no CDN)
    Direct,
}

impl Default for CdnType {
    fn default() -> Self {
        Self::Cloudflare
    }
}

// ==================== CORE STRUCTS ====================

/// IP scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// IP address
    pub ip: IpAddr,
    /// Port
    pub port: u16,
    /// Latency (ms)
    pub latency_ms: u64,
    /// TLS valid
    pub tls_valid: bool,
    /// IP is clean
    pub is_clean: bool,
    /// Fragmentation supported
    pub supports_fragmentation: bool,
    /// CDN type
    pub cdn_type: CdnType,
    /// Quality score
    pub quality_score: f32,
    /// Last tested timestamp
    pub last_tested: chrono::DateTime<chrono::Utc>,
    /// TLS fingerprint
    pub tls_fingerprint: String,
}

/// Full proxy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// Server address
    pub server: String,
    /// Port
    pub port: u16,
    /// Protocol type
    pub protocol: ProtocolType,
    /// SNI for ShadowTLS
    pub sni: String,
    /// UUID for Reality/VLESS
    pub uuid: String,
    /// Private key
    pub private_key: Option<String>,
    /// Public key
    pub public_key: Option<String>,
    /// Short ID for Reality
    pub short_id: Option<String>,
    /// uTLS Fingerprint
    pub utls_fingerprint: String,
    /// CDN type
    pub cdn_type: CdnType,
    /// Fallback port
    pub fallback_port: Option<u16>,
    /// Maximum allowed latency (ms)
    pub max_latency_ms: u64,
    /// Enable packet padding
    pub enable_padding: bool,
    /// Enable Anti-AI DPI
    pub enable_anti_ai: bool,
    /// Enable Matryoshka chaining
    pub enable_matryoshka: bool,
    /// Enable port hopping
    pub enable_port_hopping: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            port: 443,
            protocol: ProtocolType::Reality,
            sni: "ebanking.bmi.ir".to_string(),
            uuid: uuid::Uuid::new_v4().to_string(),
            private_key: None,
            public_key: None,
            short_id: None,
            utls_fingerprint: "chrome".to_string(),
            cdn_type: CdnType::Cloudflare,
            fallback_port: Some(8443),
            max_latency_ms: 300,
            enable_padding: true,
            enable_anti_ai: true,
            enable_matryoshka: true,
            enable_port_hopping: true,
        }
    }
}

/// Tunnel state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelState {
    /// Active
    pub active: bool,
    /// Current IP
    pub current_ip: Option<IpAddr>,
    /// Current port
    pub current_port: u16,
    /// Active protocol
    pub protocol: ProtocolType,
    /// Active CDN
    pub cdn: CdnType,
    /// Number of active layers
    pub active_layers: usize,
    /// Traffic stats
    pub stats: TrafficStats,
    /// Start time
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of switches
    pub switch_count: u64,
    /// Last error
    pub last_error: Option<String>,
}

impl Default for TunnelState {
    fn default() -> Self {
        Self {
            active: false,
            current_ip: None,
            current_port: 443,
            protocol: ProtocolType::Reality,
            cdn: CdnType::Cloudflare,
            active_layers: 0,
            stats: TrafficStats::default(),
            started_at: None,
            switch_count: 0,
            last_error: None,
        }
    }
}

/// Traffic statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrafficStats {
    /// Received bytes
    pub rx_bytes: u64,
    /// Transmitted bytes
    pub tx_bytes: u64,
    /// Connection count
    pub connections: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Error count
    pub errors: u64,
    /// Packet loss percentage
    pub packet_loss_pct: f32,
}

/// Engine events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineEvent {
    /// Tunnel started
    TunnelStarted { ip: IpAddr, port: u16 },
    /// Tunnel stopped
    TunnelStopped { reason: String },
    /// IP switched
    IpSwitched { old: IpAddr, new: IpAddr },
    /// Port switched
    PortSwitched { old: u16, new: u16 },
    /// Error occurred
    Error { message: String },
    /// Scan completed
    ScanCompleted { count: usize },
    /// CDN switched
    CdnSwitched { from: CdnType, to: CdnType },
    /// Circuit Breaker triggered
    CircuitBreakerTriggered { ip: IpAddr, latency_ms: u64 },
    /// Layer added
    LayerAdded { layer: String },
    /// Matryoshka chain complete
    MatryoshkaChainComplete { layers: usize },
}
