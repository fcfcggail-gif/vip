//! Sing-Box / V2Ray Config Generator — نسخه پیشرفته
//!
//! تولید پیکربندی کامل sing-box و V2Ray برای کلاینت‌های مختلف.
//! پشتیبانی از تمام پروتکل‌ها + Anti-DPI + WARP + Zapret.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::info;

use crate::types::ProxyConfig;

// ── Outbound Types ─────────────────────────────────────────────────────────

/// نوع outbound پشتیبانی‌شده
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboundType {
    Reality,
    ShadowTlsV3,
    Hysteria2,
    TuicV5,
    Masque,
    Xhttp,
    Trojan,
    Warp,
    DoubleWarp,
    Vmess,
    Vless,
    ShadowsocksR,
    Direct,
    Block,
}

/// تنظیمات کامل config generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingboxGeneratorConfig {
    pub primary_outbound: OutboundType,
    pub fallback_outbounds: Vec<OutboundType>,
    pub enable_tun: bool,
    pub tun_stack: String,
    pub listen_socks_port: u16,
    pub listen_http_port: u16,
    pub enable_sniff: bool,
    pub route_bypass_iran: bool,
    pub log_level: String,
    pub enable_clash_api: bool,
    pub clash_api_port: u16,
}

impl Default for SingboxGeneratorConfig {
    fn default() -> Self {
        Self {
            primary_outbound: OutboundType::Reality,
            fallback_outbounds: vec![OutboundType::ShadowTlsV3, OutboundType::Hysteria2],
            enable_tun: true,
            tun_stack: "system".to_string(),
            listen_socks_port: 2080,
            listen_http_port: 2081,
            enable_sniff: true,
            route_bypass_iran: true,
            log_level: "warn".to_string(),
            enable_clash_api: true,
            clash_api_port: 9090,
        }
    }
}

// ── Generator ──────────────────────────────────────────────────────────────

/// موتور تولید پیکربندی
pub struct SingboxGenerator {
    config: SingboxGeneratorConfig,
}

impl SingboxGenerator {
    pub fn new(config: SingboxGeneratorConfig) -> Self {
        Self { config }
    }

    /// تولید پیکربندی کامل sing-box
    pub fn generate_full_config(
        &self,
        proxy: &ProxyConfig,
        server_ip: &str,
        additional_servers: &[(&str, u16)],
    ) -> Result<serde_json::Value> {
        info!("📝 تولید پیکربندی کامل sing-box...");

        let inbounds = self.generate_inbounds();
        let outbounds = self.generate_outbounds(proxy, server_ip, additional_servers);
        let route = self.generate_route();
        let dns = self.generate_dns();
        let experimental = self.generate_experimental();

        let config = json!({
            "log": {
                "level": self.config.log_level,
                "timestamp": true,
                "output": "/opt/network-ghost/logs/singbox.log"
            },
            "dns": dns,
            "inbounds": inbounds,
            "outbounds": outbounds,
            "route": route,
            "experimental": experimental
        });

        info!("✅ sing-box پیکربندی تولید شد");
        Ok(config)
    }

    fn generate_inbounds(&self) -> serde_json::Value {
        let mut inbounds = vec![
            json!({
                "tag": "socks",
                "type": "socks",
                "listen": "0.0.0.0",
                "listen_port": self.config.listen_socks_port,
                "sniff": self.config.enable_sniff,
                "sniff_override_destination": false,
                "users": []
            }),
            json!({
                "tag": "http",
                "type": "http",
                "listen": "0.0.0.0",
                "listen_port": self.config.listen_http_port,
                "sniff": self.config.enable_sniff
            }),
        ];

        if self.config.enable_tun {
            inbounds.push(json!({
                "tag": "tun",
                "type": "tun",
                "interface_name": "tun0",
                "inet4_address": "172.19.0.1/30",
                "inet6_address": "fdfe:dcba:9876::1/126",
                "mtu": 9000,
                "auto_route": true,
                "strict_route": true,
                "stack": self.config.tun_stack,
                "sniff": self.config.enable_sniff,
                "sniff_override_destination": false,
                "domain_strategy": "prefer_ipv4"
            }));
        }

        json!(inbounds)
    }

    fn generate_outbounds(
        &self,
        proxy: &ProxyConfig,
        server_ip: &str,
        additional_servers: &[(&str, u16)],
    ) -> serde_json::Value {
        let mut outbounds = Vec::new();

        // Reality / VLESS
        outbounds.push(self.build_reality_outbound(proxy, server_ip));

        // ShadowTLS v3
        outbounds.push(self.build_shadowtls_outbound(proxy, server_ip));

        // Hysteria2
        outbounds.push(self.build_hysteria2_outbound(proxy, server_ip));

        // TUIC v5
        outbounds.push(self.build_tuic_outbound(proxy, server_ip));

        // MASQUE
        outbounds.push(self.build_masque_outbound(server_ip));

        // XHTTP
        outbounds.push(self.build_xhttp_outbound(proxy, server_ip));

        // WARP
        outbounds.push(self.build_warp_outbound());

        // Selector (اصلی)
        let all_tags: Vec<serde_json::Value> = vec![
            json!("reality"), json!("shadowtls"), json!("hysteria2"),
            json!("tuic"), json!("masque"), json!("xhttp"), json!("warp"),
            json!("direct")
        ];

        outbounds.push(json!({
            "tag": "proxy",
            "type": "selector",
            "outbounds": all_tags,
            "default": "reality"
        }));

        // URLTest (auto-select)
        outbounds.push(json!({
            "tag": "auto",
            "type": "urltest",
            "outbounds": ["reality", "shadowtls", "hysteria2", "tuic", "warp"],
            "url": "https://www.gstatic.com/generate_204",
            "interval": "3m",
            "tolerance": 50
        }));

        // Direct, Block, DNS
        outbounds.push(json!({"tag": "direct", "type": "direct"}));
        outbounds.push(json!({"tag": "block", "type": "block"}));
        outbounds.push(json!({"tag": "dns-out", "type": "dns"}));

        json!(outbounds)
    }

    fn build_reality_outbound(&self, proxy: &ProxyConfig, server: &str) -> serde_json::Value {
        json!({
            "tag": "reality",
            "type": "vless",
            "server": server,
            "server_port": 443,
            "uuid": proxy.uuid,
            "flow": "xtls-rprx-vision",
            "tls": {
                "enabled": true,
                "server_name": proxy.sni,
                "utls": {
                    "enabled": true,
                    "fingerprint": proxy.utls_fingerprint
                },
                "reality": {
                    "enabled": true,
                    "public_key": proxy.public_key.clone().unwrap_or_default(),
                    "short_id": proxy.short_id.clone().unwrap_or_default()
                }
            },
            "multiplex": {
                "enabled": true,
                "protocol": "smux",
                "max_connections": 8,
                "min_streams": 4,
                "max_streams": 32,
                "padding": true
            },
            "packet_encoding": "xudp"
        })
    }

    fn build_shadowtls_outbound(&self, proxy: &ProxyConfig, server: &str) -> serde_json::Value {
        json!({
            "tag": "shadowtls",
            "type": "shadowtls",
            "server": server,
            "server_port": 443,
            "version": 3,
            "password": proxy.uuid,
            "tls": {
                "enabled": true,
                "server_name": proxy.sni,
                "utls": {
                    "enabled": true,
                    "fingerprint": "firefox"
                }
            }
        })
    }

    fn build_hysteria2_outbound(&self, proxy: &ProxyConfig, server: &str) -> serde_json::Value {
        json!({
            "tag": "hysteria2",
            "type": "hysteria2",
            "server": server,
            "server_port": 443,
            "password": proxy.uuid,
            "obfs": {
                "type": "salamander",
                "password": format!("{}-obfs", proxy.uuid)
            },
            "tls": {
                "enabled": true,
                "server_name": proxy.sni,
                "utls": {
                    "enabled": true,
                    "fingerprint": "safari"
                }
            },
            "up_mbps": 50,
            "down_mbps": 200
        })
    }

    fn build_tuic_outbound(&self, proxy: &ProxyConfig, server: &str) -> serde_json::Value {
        json!({
            "tag": "tuic",
            "type": "tuic",
            "server": server,
            "server_port": 443,
            "uuid": proxy.uuid,
            "password": format!("{}-tuic", proxy.uuid),
            "congestion_control": "bbr",
            "udp_relay_mode": "quic",
            "zero_rtt_handshake": true,
            "tls": {
                "enabled": true,
                "server_name": proxy.sni,
                "utls": {
                    "enabled": true,
                    "fingerprint": "chrome"
                }
            }
        })
    }

    fn build_masque_outbound(&self, server: &str) -> serde_json::Value {
        json!({
            "tag": "masque",
            "type": "http",
            "server": server,
            "server_port": 443,
            "path": "/masque",
            "tls": {
                "enabled": true
            }
        })
    }

    fn build_xhttp_outbound(&self, proxy: &ProxyConfig, server: &str) -> serde_json::Value {
        json!({
            "tag": "xhttp",
            "type": "vless",
            "server": server,
            "server_port": 443,
            "uuid": proxy.uuid,
            "transport": {
                "type": "http",
                "host": [proxy.sni.clone()],
                "path": "/xhttp",
                "method": "POST"
            },
            "tls": {
                "enabled": true,
                "server_name": proxy.sni
            }
        })
    }

    fn build_warp_outbound(&self) -> serde_json::Value {
        json!({
            "tag": "warp",
            "type": "wireguard",
            "server": "162.159.192.1",
            "server_port": 2408,
            "local_address": ["172.16.0.2/32", "fd01:5ca1:ab1e::1/128"],
            "private_key": "AUTO_WARP_PRIVATE_KEY",
            "peer_public_key": "bmXOC+F1FxEMF9dyiK2H5/1SUtzH0JuVo51h2wPfgyo=",
            "mtu": 1280,
            "fake_packets": true,
            "fake_packets_size": "10-30",
            "fake_packets_delay": "20-250",
            "fake_packets_mode": "m4"
        })
    }

    fn generate_route(&self) -> serde_json::Value {
        let mut rules = vec![
            json!({"protocol": "dns", "outbound": "dns-out"}),
            json!({"network": "udp", "port": 443, "outbound": "block"}), // QUIC block if needed
        ];

        if self.config.route_bypass_iran {
            rules.push(json!({"geosite": ["ir"], "outbound": "direct"}));
            rules.push(json!({"geoip": ["ir", "private"], "outbound": "direct"}));
        } else {
            rules.push(json!({"geoip": ["private"], "outbound": "direct"}));
        }

        json!({
            "rules": rules,
            "final": "proxy",
            "auto_detect_interface": true,
            "override_android_vpn": true
        })
    }

    fn generate_dns(&self) -> serde_json::Value {
        json!({
            "servers": [
                {
                    "tag": "cf-doh",
                    "address": "https://1.1.1.1/dns-query",
                    "address_resolver": "local",
                    "strategy": "prefer_ipv4",
                    "detour": "proxy"
                },
                {
                    "tag": "google-doh",
                    "address": "https://8.8.8.8/dns-query",
                    "address_resolver": "local",
                    "strategy": "prefer_ipv4",
                    "detour": "proxy"
                },
                {
                    "tag": "ir-dns",
                    "address": "https://dns.403.online/dns-query",
                    "strategy": "prefer_ipv4",
                    "detour": "direct"
                },
                {
                    "tag": "local",
                    "address": "223.5.5.5",
                    "detour": "direct"
                }
            ],
            "rules": [
                {"outbound": "any", "server": "local"},
                {"geosite": ["ir"], "server": "ir-dns"},
                {"geosite": ["google", "cloudflare"], "server": "cf-doh"}
            ],
            "final": "cf-doh",
            "independent_cache": true,
            "reverse_mapping": true
        })
    }

    fn generate_experimental(&self) -> serde_json::Value {
        let mut exp = json!({
            "cache_file": {
                "enabled": true,
                "path": "/opt/network-ghost/cache/singbox.db",
                "cache_id": "network-ghost-v5",
                "store_fakeip": true,
                "store_rdrc": true
            }
        });

        if self.config.enable_clash_api {
            exp["clash_api"] = json!({
                "external_controller": format!("0.0.0.0:{}", self.config.clash_api_port),
                "external_ui": "/opt/network-ghost/dashboard",
                "external_ui_download_url": "https://github.com/MetaCubeX/metacubexd/releases/download/v1.133.0/compressed-dist.tgz",
                "external_ui_download_detour": "direct",
                "secret": "",
                "default_mode": "rule"
            });
        }

        exp
    }

    /// ذخیره پیکربندی روی دیسک
    pub async fn save_config(
        &self,
        proxy: &ProxyConfig,
        server_ip: &str,
        path: &str,
    ) -> Result<()> {
        let config = self.generate_full_config(proxy, server_ip, &[])?;
        let json_str = serde_json::to_string_pretty(&config)?;
        tokio::fs::write(path, &json_str).await?;
        info!("✅ sing-box config ذخیره شد: {} ({} bytes)", path, json_str.len());
        Ok(())
    }
}

impl Default for SingboxGenerator {
    fn default() -> Self {
        Self::new(SingboxGeneratorConfig::default())
    }
}

// ── V2Ray Config ───────────────────────────────────────────────────────────

/// تولید پیکربندی V2Ray/Xray سازگار
pub fn generate_v2ray_config(proxy: &ProxyConfig, server_ip: &str) -> serde_json::Value {
    json!({
        "log": {"loglevel": "warning"},
        "inbounds": [
            {
                "port": 1080,
                "protocol": "socks",
                "settings": {"auth": "noauth", "udp": true}
            },
            {
                "port": 1081,
                "protocol": "http",
                "settings": {}
            }
        ],
        "outbounds": [
            {
                "protocol": "vless",
                "settings": {
                    "vnext": [{
                        "address": server_ip,
                        "port": 443,
                        "users": [{
                            "id": proxy.uuid,
                            "flow": "xtls-rprx-vision",
                            "encryption": "none"
                        }]
                    }]
                },
                "streamSettings": {
                    "network": "tcp",
                    "security": "reality",
                    "realitySettings": {
                        "serverName": proxy.sni,
                        "publicKey": proxy.public_key.clone().unwrap_or_default(),
                        "shortId": proxy.short_id.clone().unwrap_or_default(),
                        "fingerprint": proxy.utls_fingerprint
                    }
                }
            }
        ],
        "routing": {
            "domainStrategy": "IPIfNonMatch",
            "rules": [
                {"type": "field", "ip": ["geoip:private"], "outboundTag": "freedom"},
                {"type": "field", "domain": ["geosite:ir"], "outboundTag": "freedom"}
            ]
        }
    })
}
