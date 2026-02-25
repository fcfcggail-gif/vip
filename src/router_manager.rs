//! OpenWrt / ImmortalWrt Router Manager
//!
//! مدیریت کامل روتر Google WiFi با ImmortalWrt:
//! - پیکربندی TProxy / TPROXY kernel-level
//! - eBPF/DAE یکپارچه‌سازی
//! - UCI و firewall management
//! - Hardware offload برای IPQ40xx
//! - نصب خودکار ابزارها

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

// ── Router Profile ─────────────────────────────────────────────────────────

/// پروفایل روتر Google WiFi
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterProfile {
    /// نام مدل
    pub model: String,
    /// معماری CPU
    pub arch: String,
    /// هسته Linux
    pub kernel: String,
    /// رام (MB)
    pub ram_mb: u32,
    /// فلش (MB)
    pub flash_mb: u32,
    /// interface WAN
    pub wan_interface: String,
    /// interface LAN
    pub lan_interface: String,
    /// پشتیبانی از HW offload
    pub hw_offload: bool,
    /// نسخه OpenWrt/ImmortalWrt
    pub openwrt_version: String,
}

impl Default for RouterProfile {
    fn default() -> Self {
        // Google WiFi (AC-1304) با ImmortalWrt
        Self {
            model: "Google WiFi (AC-1304)".to_string(),
            arch: "arm_cortex-a7_neon-vfpv4".to_string(),
            kernel: "5.15.167".to_string(),
            ram_mb: 512,
            flash_mb: 4096,
            wan_interface: "eth0".to_string(),
            lan_interface: "br-lan".to_string(),
            hw_offload: true,
            openwrt_version: "ImmortalWrt-23.05".to_string(),
        }
    }
}

// ── TProxy Setup ───────────────────────────────────────────────────────────

/// تنظیمات TPROXY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TproxyConfig {
    pub listen_port: u16,
    pub dns_port: u16,
    pub mark: u32,
    pub table_id: u32,
    pub bypass_uid: u32,
    pub enable_ipv6: bool,
    pub bypass_private: bool,
    pub bypass_iran_geoip: bool,
}

impl Default for TproxyConfig {
    fn default() -> Self {
        Self {
            listen_port: 7892,
            dns_port: 7874,
            mark: 1,
            table_id: 100,
            bypass_uid: 65534,
            enable_ipv6: true,
            bypass_private: true,
            bypass_iran_geoip: true,
        }
    }
}

/// مدیر TPROXY
pub struct TproxyManager {
    config: TproxyConfig,
    profile: RouterProfile,
}

impl TproxyManager {
    pub fn new(config: TproxyConfig, profile: RouterProfile) -> Self {
        Self { config, profile }
    }

    /// تولید اسکریپت راه‌اندازی TPROXY کامل
    pub fn generate_setup_script(&self) -> String {
        let port = self.config.listen_port;
        let dns_port = self.config.dns_port;
        let mark = self.config.mark;
        let table = self.config.table_id;
        let wan = &self.profile.wan_interface;

        format!(
            r#"#!/bin/sh
# ══════════════════════════════════════════════════════════════════════
# Network Ghost v5 — TPROXY Setup for Google WiFi / ImmortalWrt
# پیکربندی TPROXY kernel-level برای تمام دستگاه‌های شبکه
# ══════════════════════════════════════════════════════════════════════

set -e
PROXY_PORT={port}
DNS_PORT={dns_port}
MARK={mark}
TABLE={table}
WAN={wan}

# ── پاک‌سازی قوانین قبلی ──────────────────────────────────────────
ip route flush table $TABLE 2>/dev/null || true
ip rule del fwmark $MARK table $TABLE 2>/dev/null || true
iptables -t mangle -F GHOST_TP 2>/dev/null || true
iptables -t mangle -X GHOST_TP 2>/dev/null || true
iptables -t nat -F GHOST_NAT 2>/dev/null || true
iptables -t nat -X GHOST_NAT 2>/dev/null || true

# ── راه‌اندازی Routing Table ──────────────────────────────────────
ip route add local default dev lo table $TABLE
ip rule add fwmark $MARK table $TABLE

# ── tproxy chain — ترافیک TCP/UDP به Ghost هدایت می‌شود ──────────
iptables -t mangle -N GHOST_TP

# Bypass: شبکه‌های خصوصی
iptables -t mangle -A GHOST_TP -d 0.0.0.0/8 -j RETURN
iptables -t mangle -A GHOST_TP -d 127.0.0.0/8 -j RETURN
iptables -t mangle -A GHOST_TP -d 169.254.0.0/16 -j RETURN
iptables -t mangle -A GHOST_TP -d 172.16.0.0/12 -j RETURN
iptables -t mangle -A GHOST_TP -d 192.168.0.0/16 -j RETURN
iptables -t mangle -A GHOST_TP -d 10.0.0.0/8 -j RETURN
iptables -t mangle -A GHOST_TP -d 224.0.0.0/4 -j RETURN
iptables -t mangle -A GHOST_TP -d 240.0.0.0/4 -j RETURN

# Bypass: ترافیک از پروسه Ghost خودش (جلوگیری از loop)
iptables -t mangle -A GHOST_TP -m mark --mark $MARK -j RETURN

# TPROXY: TCP
iptables -t mangle -A GHOST_TP -p tcp -j TPROXY \
  --tproxy-mark $MARK --on-port $PROXY_PORT

# TPROXY: UDP
iptables -t mangle -A GHOST_TP -p udp -j TPROXY \
  --tproxy-mark $MARK --on-port $PROXY_PORT

# اتصال chain به PREROUTING (برای forward — دستگاه‌های شبکه)
iptables -t mangle -A PREROUTING -j GHOST_TP

# ── DNS Redirect ────────────────────────────────────────────────────
iptables -t nat -N GHOST_NAT
iptables -t nat -A GHOST_NAT -p udp --dport 53 -j REDIRECT --to-port $DNS_PORT
iptables -t nat -A GHOST_NAT -p tcp --dport 53 -j REDIRECT --to-port $DNS_PORT
iptables -t nat -A PREROUTING -j GHOST_NAT

# ── ترافیک LOCAL (خود روتر) ──────────────────────────────────────
iptables -t mangle -N GHOST_LOCAL
iptables -t mangle -A GHOST_LOCAL -d 127.0.0.0/8 -j RETURN
iptables -t mangle -A GHOST_LOCAL -d 10.0.0.0/8 -j RETURN
iptables -t mangle -A GHOST_LOCAL -d 172.16.0.0/12 -j RETURN
iptables -t mangle -A GHOST_LOCAL -d 192.168.0.0/16 -j RETURN
iptables -t mangle -A GHOST_LOCAL -p tcp -j MARK --set-mark $MARK
iptables -t mangle -A GHOST_LOCAL -p udp -j MARK --set-mark $MARK
iptables -t mangle -A OUTPUT -j GHOST_LOCAL

# ── IPv6 TPROXY ─────────────────────────────────────────────────────
if [ -n "$(which ip6tables)" ]; then
    ip6tables -t mangle -N GHOST_TP6 2>/dev/null || true
    ip6tables -t mangle -A GHOST_TP6 -d ::1/128 -j RETURN
    ip6tables -t mangle -A GHOST_TP6 -d fc00::/7 -j RETURN
    ip6tables -t mangle -A GHOST_TP6 -p tcp -j TPROXY \
      --tproxy-mark $MARK --on-port $PROXY_PORT
    ip6tables -t mangle -A GHOST_TP6 -p udp -j TPROXY \
      --tproxy-mark $MARK --on-port $PROXY_PORT
    ip6tables -t mangle -A PREROUTING -j GHOST_TP6
    
    ip -6 route add local default dev lo table $TABLE 2>/dev/null || true
    ip -6 rule add fwmark $MARK table $TABLE 2>/dev/null || true
fi

# ── بهینه‌سازی kernel برای IPQ40xx ──────────────────────────────
sysctl -w net.core.rmem_max=26214400 >/dev/null
sysctl -w net.core.wmem_max=26214400 >/dev/null
sysctl -w net.ipv4.tcp_rmem="4096 87380 26214400" >/dev/null
sysctl -w net.ipv4.tcp_wmem="4096 65536 26214400" >/dev/null
sysctl -w net.ipv4.tcp_fastopen=3 >/dev/null
sysctl -w net.ipv4.tcp_bbr=1 >/dev/null 2>&1 || true

echo "✅ TPROXY راه‌اندازی شد (پورت: $PROXY_PORT, DNS: $DNS_PORT)"
"#,
            port = port,
            dns_port = dns_port,
            mark = mark,
            table = table,
            wan = wan,
        )
    }

    /// تولید اسکریپت پاک‌سازی TPROXY
    pub fn generate_cleanup_script(&self) -> String {
        let mark = self.config.mark;
        let table = self.config.table_id;

        format!(
            r#"#!/bin/sh
# Network Ghost v5 — TPROXY Cleanup

ip rule del fwmark {mark} table {table} 2>/dev/null || true
ip route flush table {table} 2>/dev/null || true
ip -6 rule del fwmark {mark} table {table} 2>/dev/null || true
ip -6 route flush table {table} 2>/dev/null || true

for chain in GHOST_TP GHOST_LOCAL GHOST_NAT GHOST_TP6; do
    iptables -t mangle -F $chain 2>/dev/null
    iptables -t mangle -X $chain 2>/dev/null
    iptables -t nat -F $chain 2>/dev/null
    iptables -t nat -X $chain 2>/dev/null
    ip6tables -t mangle -F $chain 2>/dev/null
    ip6tables -t mangle -X $chain 2>/dev/null
done

echo "✅ TPROXY قوانین پاک شد"
"#,
            mark = mark,
            table = table,
        )
    }
}

// ── Hardware Offload ───────────────────────────────────────────────────────

/// پیکربندی Hardware Offload برای IPQ40xx/Google WiFi
pub struct HardwareOffloadManager {
    profile: RouterProfile,
}

impl HardwareOffloadManager {
    pub fn new(profile: RouterProfile) -> Self {
        Self { profile }
    }

    /// فعال‌سازی Hardware Offload برای عملکرد بهتر
    pub fn generate_hwoffload_script(&self) -> String {
        r#"#!/bin/sh
# Network Ghost v5 — Hardware Offload برای Google WiFi (IPQ40xx)
# این اسکریپت CPU load روتر را به شدت کاهش می‌دهد

# فعال‌سازی Flow Offload (kernel-level)
echo "1" > /sys/kernel/debug/ecm/front_end_ipv4_stop 2>/dev/null || true
echo "1" > /sys/kernel/debug/ecm/front_end_ipv6_stop 2>/dev/null || true

# UCI Hardware Flow Offload
uci set firewall.@defaults[0].flow_offloading='1'
uci set firewall.@defaults[0].flow_offloading_hw='1'
uci commit firewall
/etc/init.d/firewall restart

# بهینه‌سازی IRQ برای IPQ40xx (4 هسته)
for i in $(ls /proc/irq/ | grep -E "^[0-9]+$"); do
    echo 4 > /proc/irq/$i/smp_affinity 2>/dev/null || true
done

# تنظیم CPU frequency governor
for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
    echo "performance" > $cpu 2>/dev/null || true
done

# بهینه‌سازی network buffers
sysctl -w net.core.netdev_max_backlog=5000
sysctl -w net.ipv4.tcp_congestion_control=bbr 2>/dev/null || true

# فعال‌سازی eBPF JIT (برای DAE)
echo 1 > /proc/sys/net/core/bpf_jit_enable 2>/dev/null || true
echo 1 > /proc/sys/net/core/bpf_jit_harden 2>/dev/null || true

echo "✅ Hardware offload فعال شد"
echo "   برای تأیید: cat /proc/net/nf_conntrack | wc -l"
"#.to_string()
    }

    /// تولید پیکربندی OpenWrt UCI کامل
    pub fn generate_uci_config(&self) -> String {
        r#"# Network Ghost v5 — OpenWrt UCI Configuration
# این فایل در /etc/uci-defaults/99-network-ghost قرار می‌گیرد

# فعال‌سازی IPv6
uci set network.globals.ula_prefix='fd00::/48'
uci set network.globals.packet_steering='1'

# بهینه‌سازی LAN
uci set network.lan.force_link='1'

# بهینه‌سازی WAN
uci set network.wan.peerdns='0'
uci set network.wan.dns='1.1.1.1 8.8.8.8'

# فعال‌سازی BBR
echo 'net.core.default_qdisc=fq' >> /etc/sysctl.d/10-bbr.conf
echo 'net.ipv4.tcp_congestion_control=bbr' >> /etc/sysctl.d/10-bbr.conf

# حافظه بافر
echo 'net.core.rmem_max=26214400' >> /etc/sysctl.d/10-network.conf
echo 'net.core.wmem_max=26214400' >> /etc/sysctl.d/10-network.conf
echo 'net.ipv4.tcp_fastopen=3' >> /etc/sysctl.d/10-network.conf

uci commit network

echo "✅ UCI configuration applied"
"#.to_string()
    }
}

// ── Auto-Install Script Generator ─────────────────────────────────────────

/// تولید اسکریپت نصب کامل برای Google WiFi
pub fn generate_full_install_script() -> String {
    r#"#!/bin/sh
# ══════════════════════════════════════════════════════════════════════
# Network Ghost v5.0 — نصب کامل روی Google WiFi / ImmortalWrt
# ══════════════════════════════════════════════════════════════════════

set -e
NG_DIR="/opt/network-ghost"
NG_VERSION="5.0.0"

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  👻 Network Ghost v${NG_VERSION} — نصب روی Google WiFi  ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""

# بررسی اتصال
ping -c1 -W2 1.1.1.1 >/dev/null 2>&1 || { echo "❌ خطا: اینترنت ندارید!"; exit 1; }

# نصب پیش‌نیازها
echo "📦 نصب پیش‌نیازها..."
opkg update 2>/dev/null
opkg install kmod-tun kmod-nft-tproxy kmod-nf-tproxy \
    kmod-nfnetlink-queue kmod-ipt-tproxy \
    iptables-mod-tproxy iptables-mod-extra \
    nftables kmod-nft-queue kmod-nft-nat \
    kmod-wireguard wireguard-tools \
    ca-bundle curl wget-ssl \
    ip-full ipset \
    kmod-sched-cake tc-full 2>/dev/null || true

# ساخت دایرکتوری‌ها
echo "📁 ساخت دایرکتوری‌ها..."
mkdir -p $NG_DIR/{bin,config,logs,cache,scripts,zapret,geodata}

# دانلود GeoData
echo "🌍 دانلود GeoData..."
wget -qO $NG_DIR/geodata/geoip.db \
    https://github.com/SagerNet/sing-geoip/releases/latest/download/geoip.db 2>/dev/null || true
wget -qO $NG_DIR/geodata/geosite.db \
    https://github.com/SagerNet/sing-geosite/releases/latest/download/geosite.db 2>/dev/null || true

# پیکربندی اولیه
echo "⚙️ پیکربندی اولیه..."
cat > $NG_DIR/config/config.toml << 'CONF'
# Network Ghost v5 — پیکربندی اصلی
sni = "ebanking.bmi.ir"
protocol = "auto"
cdn = "cloudflare"
dpi_mode = "ghost"
max_latency = 300
port_hopping = true
enable_zapret = true
enable_warp = false
enable_goodbyedpi = true
[server]
server = ""
port = 443
uuid = ""
public_key = ""
CONF

# ایجاد سرویس init.d
cat > /etc/init.d/network-ghost << 'INIT'
#!/bin/sh /etc/rc.common
START=90
STOP=10
PROG=/opt/network-ghost/bin/network-ghost
PID_FILE=/tmp/network-ghost.pid

start() {
    echo "🚀 شروع Network Ghost..."
    $PROG start --config /opt/network-ghost/config/config.toml &
    echo $! > $PID_FILE
    /opt/network-ghost/scripts/tproxy-setup.sh
    echo "✅ Network Ghost فعال است"
}

stop() {
    echo "🛑 توقف Network Ghost..."
    /opt/network-ghost/scripts/tproxy-cleanup.sh
    [ -f $PID_FILE ] && kill $(cat $PID_FILE) 2>/dev/null
    rm -f $PID_FILE
}

restart() { stop; sleep 1; start; }
status() {
    [ -f $PID_FILE ] && echo "✅ در حال اجرا (PID: $(cat $PID_FILE))" || echo "❌ متوقف"
}
INIT
chmod +x /etc/init.d/network-ghost
/etc/init.d/network-ghost enable

echo ""
echo "╔══════════════════════════════════════════════════╗"
echo "║  ✅ نصب کامل شد!                                ║"
echo "║                                                  ║"
echo "║  برای شروع:  /etc/init.d/network-ghost start    ║"
echo "║  وضعیت:      /etc/init.d/network-ghost status   ║"
echo "║  Dashboard:  http://192.168.1.1:9090             ║"
echo "╚══════════════════════════════════════════════════╝"
echo ""
"#.to_string()
}

/// تولید OpenWrt hotplug script برای auto-start
pub fn generate_hotplug_script() -> String {
    r#"#!/bin/sh
# /etc/hotplug.d/iface/99-network-ghost
# راه‌اندازی خودکار Network Ghost هنگام اتصال WAN

[ "$ACTION" = "ifup" ] && [ "$INTERFACE" = "wan" ] && {
    sleep 3
    /etc/init.d/network-ghost restart &
    logger -t network-ghost "WAN came up — restarting tunnel"
}
"#.to_string()
}
