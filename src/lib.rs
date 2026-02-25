//! Network Ghost v5.0 - Zero-Knowledge Phantom Network Tunnel
//!
//! Advanced anti-filtering system for bypassing AI-based DPI.
//! پشتیبانی از: DAE (eBPF), Zapret, GoodbyeDPI, WARP, Hysteria2, TUIC, Reality و بیشتر.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
// ── Core ────────────────────────────────────────────────────────────────────
pub mod types;
pub mod engine;
pub mod config;

// ── Anti-DPI & Bypass ────────────────────────────────────────────────────────
pub mod anti_ai_dpi;
pub mod zapret_bypass;
pub mod goodbyedpi;
pub mod packet_padding;
pub mod smart_detector;

// ── Protocols ────────────────────────────────────────────────────────────────
pub mod shadowtls;
pub mod reality;
pub mod hysteria2;
pub mod tuic;
pub mod masque;
pub mod xhttp;
pub mod smux;
pub mod matryoshka;
pub mod ip_relay;
pub mod warp_client;
pub mod websocket_transport;

// ── Infrastructure ───────────────────────────────────────────────────────────
pub mod scanner;
pub mod fingerprint;
pub mod port_hopper;
pub mod circuit_breaker;
pub mod dns_over_quic;
pub mod multicdn;
pub mod dae_generator;
pub mod ipq40xx_offload;

// ── Config Generators ────────────────────────────────────────────────────────
pub mod singbox_generator;
pub mod router_manager;

// ── Utilities ────────────────────────────────────────────────────────────────
pub mod dashboard;
pub mod proxy_dialer;
pub mod utils;

// ── Re-exports ────────────────────────────────────────────────────────────────

/// Shared types for use across all modules
pub use types::{
    ALTERNATIVE_PORTS, CLOUDFLARE_RANGES, IRANIAN_SNI_WHITELIST,
    CdnType, EngineEvent, ProtocolType, ProxyConfig, ScanResult,
    TunnelState, TrafficStats,
};

// Core engine
pub use engine::NetworkGhostEngine;

// Anti-DPI
pub use anti_ai_dpi::AntiAiDpi;
pub use zapret_bypass::ZapretEngine;
pub use goodbyedpi::GoodbyeDpiEngine;

// Infrastructure
pub use circuit_breaker::CircuitBreaker;
pub use dae_generator::DaeGenerator;
pub use dashboard::{DashboardConfig, DashboardServer};
pub use dns_over_quic::DnsOverQuic;
pub use fingerprint::FingerprintManager;
pub use port_hopper::PortHopper;
pub use scanner::TlsScanner;
pub use smart_detector::SmartDetector;

// New modules
pub use warp_client::WarpClient;
pub use singbox_generator::SingboxGenerator;
pub use router_manager::TproxyManager;
