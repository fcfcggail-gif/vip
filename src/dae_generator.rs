//! DAE Config Generator

use std::net::IpAddr;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::{CdnType, ProtocolType};

/// تنظیمات DAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaeConfig {
    /// Global
    pub global: DaeGlobal,
    /// DNS
    pub dns: DaeDns,
    /// Nodes
    pub nodes: Vec<DaeNode>,
    /// Groups
    pub groups: Vec<DaeGroup>,
    /// Rules
    pub rules: Vec<String>,
}

/// تنظیمات Global
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaeGlobal {
    /// Log Level
    pub log_level: String,
    /// WAN Interface
    pub wan_interface: String,
}

impl Default for DaeGlobal {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            wan_interface: "eth0".to_string(),
        }
    }
}

/// تنظیمات DNS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaeDns {
    /// Upstream
    pub upstream: Vec<String>,
}

impl Default for DaeDns {
    fn default() -> Self {
        Self {
            upstream: vec![
                "https://dns.google.com/dns-query".to_string(),
            ],
        }
    }
}

/// Node DAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaeNode {
    /// نام
    pub name: String,
    /// نوع پروتکل
    pub protocol: String,
    /// آدرس
    pub address: String,
    /// پورت
    pub port: u16,
}

/// Group DAE
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaeGroup {
    /// نام
    pub name: String,
    /// Nodes
    pub nodes: Vec<String>,
}

/// Generator DAE
pub struct DaeGenerator;

impl DaeGenerator {
    /// ایجاد Generator جدید
    pub fn new() -> Self {
        Self
    }

    /// تولید Config
    pub fn generate(
        &self,
        ip: Option<IpAddr>,
        port: u16,
        protocol: ProtocolType,
        sni: String,
        uuid: String,
    ) -> Result<String> {
        let address = ip.map(|i| i.to_string()).unwrap_or_default();
        
        let protocol_str = match protocol {
            ProtocolType::ShadowTls => "shadowtls",
            ProtocolType::Reality => "vless",
            ProtocolType::Tuic => "tuic",
            ProtocolType::Hysteria2 => "hysteria2",
            ProtocolType::Masque => "masque",
            ProtocolType::Xhttp => "http2",
            ProtocolType::Trojan => "trojan",
            ProtocolType::Vless => "vless",
            ProtocolType::Warp => "warp",
            ProtocolType::Cascade { .. } => "vless",
        };

        let config = format!(
            r#"global {{
  log_level: {}
  wan_interface: eth0
}}

dns {{
  upstream {{
    https://dns.google.com/dns-query
  }}
}}

node(node_0) {{
  protocol: {}
  address: {}
  port: {}
  sni: {}
  uuid: {}
}}

group(proxy) {{
  node: node_0
  policy: min_last_latency
}}

routing {{
  dns(geoip:private) -> direct
  domain(geosite:ir) -> direct
  ip(geoip:ir) -> direct
  fallback -> proxy
}}
"#,
            "info", protocol_str, address, port, sni, uuid
        );

        debug!("📝 DAE config generated");
        Ok(config)
    }

    /// تولید Config برای Multi-Node
    pub fn generate_multi(
        &self,
        nodes: Vec<(IpAddr, u16, ProtocolType, String, String)>,
    ) -> Result<String> {
        let mut config = String::new();
        
        config.push_str("global {\n  log_level: info\n  wan_interface: eth0\n}\n\n");
        config.push_str("dns {\n  upstream {\n    https://dns.google.com/dns-query\n  }\n}\n\n");

        let mut node_names = Vec::new();
        
        for (i, (ip, port, protocol, sni, uuid)) in nodes.iter().enumerate() {
            let protocol_str = match protocol {
                ProtocolType::ShadowTls => "shadowtls",
                ProtocolType::Reality => "vless",
                ProtocolType::Tuic => "tuic",
                ProtocolType::Hysteria2 => "hysteria2",
                ProtocolType::Trojan => "trojan",
                ProtocolType::Vless => "vless",
                ProtocolType::Warp => "warp",
                _ => "vless",
            };
            
            let name = format!("node_{}", i);
            node_names.push(name.clone());
            
            config.push_str(&format!(
                "node({}) {{\n  protocol: {}\n  address: {}\n  port: {}\n  sni: {}\n  uuid: {}\n}}\n\n",
                name, protocol_str, ip, port, sni, uuid
            ));
        }

        config.push_str(&format!(
            "group(proxy) {{\n  node: {}\n  policy: min_last_latency\n}}\n\n",
            node_names.join(", ")
        ));

        config.push_str("routing {\n  dns(geoip:private) -> direct\n  domain(geosite:ir) -> direct\n  ip(geoip:ir) -> direct\n  fallback -> proxy\n}\n");

        Ok(config)
    }
}

impl Default for DaeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
