#!/bin/bash
# ════════════════════════════════════════════════════════════════════════
# 🚀 Network Ghost v5.1 - Ultimate Phantom Edition
# 🎯 Kernel-Level eBPF + DAE + Advanced Net-Tuning
# 🔧 Optimized for: Google WiFi OnHub / OpenWrt / ImmortalWrt
# 📦 Merged: All v5.0 Features + All v5.1 Enhancements
# 🧹 Auto-Log Clean: Memory Protection for Routers
# ════════════════════════════════════════════════════════════════════════

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Configuration
VERSION="5.1.0"
INSTALL_DIR="/opt/network-ghost"
CONFIG_DIR="/etc/network-ghost"
BACKUP_DIR="/opt/network-ghost/backups"
LOG_DIR="/var/log/network-ghost"
BIN_DIR="/usr/local/bin"
DAE_DIR="/etc/dae"
SERVICE_NAME="network-ghost"

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PACKAGE_DIR="$(dirname "${SCRIPT_DIR}")"

echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo -e "${MAGENTA}   🌟 Network Ghost v${VERSION} - Ultimate Phantom Edition${NC}"
echo -e "${CYAN}   Kernel-Level eBPF + DAE + Advanced Networking${NC}"
echo -e "${CYAN}   + Auto-Log Clean for Router Memory Protection${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
echo ""

# ════════════════════════════════════════════════════════════════════════
# Step 1: Root & Environment Check
# ════════════════════════════════════════════════════════════════════════

echo -e "${YELLOW}[1/16]${NC} Checking permissions & initializing environment..."

if [ "$(id -u)" -ne 0 ]; then
    echo -e "${RED}❌ This script must be run as root${NC}"
    echo -e "${RED}❌ Error: You must be root to modify kernel-level hooks.${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Running as root${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 2: System Detection & HW Optimization
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[2/16]${NC} Detecting system & hardware capabilities..."

# Architecture
ARCH=$(uname -m)
echo -e "   Architecture: ${CYAN}${ARCH}${NC}"

# Memory
TOTAL_MEM=$(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo "524288")
TOTAL_MEM_MB=$((TOTAL_MEM / 1024))
echo -e "   Memory: ${CYAN}${TOTAL_MEM_MB}MB${NC}"

# Kernel
KERNEL=$(uname -r)
echo -e "   Kernel: ${CYAN}${KERNEL}${NC}"

# Check if OpenWrt/ImmortalWrt
if [ -f "/etc/openwrt_release" ]; then
    OS_TYPE="openwrt"
    echo -e "   OS: ${CYAN}OpenWrt/ImmortalWrt${NC}"
elif [ -f "/etc/os-release" ]; then
    OS_TYPE="linux"
    . /etc/os-release
    echo -e "   OS: ${CYAN}${NAME}${NC}"
else
    OS_TYPE="unknown"
    echo -e "   OS: ${YELLOW}Unknown${NC}"
fi

# eBPF Support Check
if [ -d "/sys/fs/bpf" ]; then
    EBPF_SUPPORTED=true
    EBPF_READY="YES"
    echo -e "   eBPF: ${GREEN}Supported & Ready${NC}"
else
    EBPF_SUPPORTED=false
    EBPF_READY="NO"
    echo -e "   eBPF: ${YELLOW}Not detected (Legacy Mode)${NC}"
fi

# ════════════════════════════════════════════════════════════════════════
# Step 3: Create Directories (Including Backup)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[3/16]${NC} Creating directories..."

mkdir -p "${INSTALL_DIR}"
mkdir -p "${CONFIG_DIR}"
mkdir -p "${BACKUP_DIR}"
mkdir -p "${LOG_DIR}"
mkdir -p "${BIN_DIR}"
mkdir -p "${DAE_DIR}"

# Backup existing config if exists
if [ -f "${CONFIG_DIR}/config.toml" ]; then
    cp "${CONFIG_DIR}/config.toml" "${BACKUP_DIR}/config_$(date +%Y%m%d_%H%M%S).toml.bak"
    echo -e "${GREEN}✅ Existing configuration backed up to ${BACKUP_DIR}${NC}"
fi

echo -e "${GREEN}✅ All directories created${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 4: Install Binary
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[4/16]${NC} Installing binary..."

BINARY_FOUND=false

if [ -f "${PACKAGE_DIR}/bin/network-ghost" ]; then
    cp "${PACKAGE_DIR}/bin/network-ghost" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/network-ghost"
    ln -sf "${INSTALL_DIR}/network-ghost" "${BIN_DIR}/network-ghost"
    BINARY_FOUND=true
elif [ -f "${PACKAGE_DIR}/network-ghost" ]; then
    cp "${PACKAGE_DIR}/network-ghost" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/network-ghost"
    ln -sf "${INSTALL_DIR}/network-ghost" "${BIN_DIR}/network-ghost"
    BINARY_FOUND=true
elif [ -f "./network-ghost" ]; then
    cp "./network-ghost" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/network-ghost"
    ln -sf "${INSTALL_DIR}/network-ghost" "${BIN_DIR}/network-ghost"
    BINARY_FOUND=true
elif [ -f "./bin/network-ghost" ]; then
    cp "./bin/network-ghost" "${INSTALL_DIR}/"
    chmod +x "${INSTALL_DIR}/network-ghost"
    ln -sf "${INSTALL_DIR}/network-ghost" "${BIN_DIR}/network-ghost"
    BINARY_FOUND=true
fi

if [ "$BINARY_FOUND" = true ] && [ -f "${INSTALL_DIR}/network-ghost" ]; then
    BINARY_SIZE=$(stat -c%s "${INSTALL_DIR}/network-ghost" 2>/dev/null || stat -f%z "${INSTALL_DIR}/network-ghost" 2>/dev/null || echo "0")
    BINARY_SIZE_MB=$((BINARY_SIZE / 1024 / 1024))
    echo -e "${GREEN}✅ Binary installed (${BINARY_SIZE_MB}MB)${NC}"
else
    echo -e "${YELLOW}⚠️  Binary not found in package directories (assuming upgrade mode)${NC}"
    if [ -f "${INSTALL_DIR}/network-ghost" ]; then
        echo -e "${GREEN}✅ Using existing binary${NC}"
    fi
fi

# ════════════════════════════════════════════════════════════════════════
# Step 5: Install Configuration (with Auto-UUID)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[5/16]${NC} Installing configuration..."

# Generate Auto-UUID
NEW_UUID=$(cat /proc/sys/kernel/random/uuid 2>/dev/null || uuidgen 2>/dev/null || echo "12345678-1234-1234-1234-1234567890ab")

# Copy config if exists in package
if [ -f "${PACKAGE_DIR}/config/config.toml" ]; then
    cp "${PACKAGE_DIR}/config/config.toml" "${CONFIG_DIR}/"
    echo -e "${GREEN}✅ Configuration copied from package${NC}"
elif [ ! -f "${CONFIG_DIR}/config.toml" ]; then
    # Create default config with auto-generated UUID
    cat > "${CONFIG_DIR}/config.toml" << EOF
# Network Ghost v${VERSION} Configuration
# Ultimate Phantom Edition - Kernel-Level Infrastructure

[general]
memory_limit_mb = 120
auto_save_interval = 300
enable_watchdog = true

[proxy]
server = "YOUR_SERVER_IP"
port = 443
protocol = "reality"
cdn_type = "cloudflare"
sni = "ebanking.bmi.ir"
uuid = "${NEW_UUID}"
utls_fingerprint = "chrome"
max_latency_ms = 250
auto_switch = true
enable_matryoshka = true
enable_port_hopping = true
alternative_ports = [443, 2053, 2083, 2087, 2096, 8443]

[scanner]
max_ips = 50
connect_timeout_ms = 3000
tls_timeout_ms = 5000
concurrency = 10
enable_anti_ai = true
scan_interval = 300

[anti_ai]
enabled = true
mode = "adaptive"
enable_padding = true
enable_timing = true
enable_decoy = false

[matryoshka]
enabled = true
max_layers = 20
layers = ["shadowtls", "reality", "smux"]

[dns]
servers = ["9.9.9.9:53", "dns.google.com:53", "1.1.1.1", "8.8.8.8"]
enable_doh = true
enable_doq = true
cache_size = 1000

[dashboard]
enabled = true
port = 9090
bind = "0.0.0.0"

[logging]
level = "info"
file = "/var/log/network-ghost/ghost.log"
max_size_mb = 10
max_files = 5
EOF
    echo -e "${GREEN}✅ Default config created with auto-generated UUID: ${CYAN}${NEW_UUID}${NC}"
fi

# Copy CDN list if exists
if [ -f "${PACKAGE_DIR}/config/p-list-multicdn.txt" ]; then
    cp "${PACKAGE_DIR}/config/p-list-multicdn.txt" "${CONFIG_DIR}/"
    echo -e "${GREEN}✅ CDN list copied${NC}"
fi

# ════════════════════════════════════════════════════════════════════════
# Step 6: Configure eBPF/DAE
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[6/16]${NC} Configuring eBPF/DAE kernel integration..."

# Create DAE config directory
mkdir -p "${DAE_DIR}"

# Create enhanced DAE config
cat > "${DAE_DIR}/config.dae" << 'EOF'
# DAE Config - Auto-generated by Network Ghost v5.1
# Kernel-Level eBPF Transparent Proxy

global {
  log_level: info
  wan_interface: auto
  allow_insecure: false
  tls_implementation: tls
  udp_check_dns: "dns.google.com:53"
  check_interval: 30s
  check_tolerance: 100
  
  # Memory optimization for 512MB routers
  dial_timeout: 5s
  so_mark_from_dae: 1234
}

dns {
  upstream {
    googledns: "tcp+udp://8.8.8.8:53"
    cfdns: "tcp+udp://1.1.1.1:53"
    quad9: "tcp+udp://9.9.9.9:53"
    doh: "https://dns.google.com/dns-query"
  }
  listen: 0.0.0.0:53
  routing {
    request {
      fallback: googledns
    }
  }
}

# Nodes will be added dynamically by Network Ghost
# routing rules

routing {
  # DNS - direct
  dns(geoip:private) -> direct
  
  # Iranian domains - direct
  domain(geosite:ir) -> direct
  
  # Iranian IPs - direct  
  ip(geoip:ir) -> direct
  
  # Private networks - direct
  dip(geoip:private) -> direct
  ip(geoip:private) -> direct
  
  # Default - will be updated with proxy nodes
  fallback -> direct
}
EOF

echo -e "${GREEN}✅ DAE config created${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 7: Create Systemd Service (with Hardening)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[7/16]${NC} Creating systemd service..."

cat > /etc/systemd/system/network-ghost.service << EOF
[Unit]
Description=Network Ghost v${VERSION} - Phantom Tunnel
Documentation=https://github.com/network-ghost/v5
After=network.target network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
WorkingDirectory=${INSTALL_DIR}
ExecStartPre=/usr/bin/sleep 5
ExecStart=${INSTALL_DIR}/network-ghost --config ${CONFIG_DIR}/config.toml
Restart=on-failure
RestartSec=10
LimitNOFILE=1048576

# Memory limits for router (optimized for 512MB devices)
MemoryMax=160M
MemoryHigh=130M

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=${CONFIG_DIR} ${LOG_DIR} ${DAE_DIR} ${BACKUP_DIR}

# eBPF permissions (enhanced capabilities)
CapabilityBoundingSet=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN CAP_SYS_PTRACE
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_RAW CAP_SYS_ADMIN

[Install]
WantedBy=multi-user.target
EOF

# Reload systemd
systemctl daemon-reload

echo -e "${GREEN}✅ Service created${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 8: Configure Advanced Kernel (Phantom Stack - BBR + FQ + TFO)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[8/16]${NC} Applying Advanced Kernel-Level Optimizations..."

# Core TCP & Congestion Control (BBR + FQ)
sysctl -w net.core.default_qdisc=fq 2>/dev/null || true
sysctl -w net.ipv4.tcp_congestion_control=bbr 2>/dev/null || true

# TCP Fast Open (Client + Server) - Reduces latency for web loading
sysctl -w net.ipv4.tcp_fastopen=3 2>/dev/null || true

# Disable slow start after idle for better performance
sysctl -w net.ipv4.tcp_slow_start_after_idle=0 2>/dev/null || true

# Memory & Buffer Tweaks (Enhanced for low-latency)
sysctl -w net.core.rmem_max=67108864 2>/dev/null || true
sysctl -w net.core.wmem_max=67108864 2>/dev/null || true
sysctl -w net.ipv4.tcp_rmem="4096 87380 16777216" 2>/dev/null || true
sysctl -w net.ipv4.tcp_wmem="4096 65536 16777216" 2>/dev/null || true

# Increase file descriptors
sysctl -w fs.file-max=65535 2>/dev/null || true

# IP Forwarding for Router Mode (IPv4 + IPv6)
sysctl -w net.ipv4.ip_forward=1 2>/dev/null || true
sysctl -w net.ipv6.conf.all.forwarding=1 2>/dev/null || true

# Save to sysctl.d for persistence (new location)
cat > /etc/sysctl.d/99-network-ghost.conf << 'EOF'
# Network Ghost v5.1 - Ultimate Phantom Optimizations
# BBR + FQ + TCP Fast Open + Advanced TCP Stack

# Congestion Control
net.core.default_qdisc=fq
net.ipv4.tcp_congestion_control=bbr

# TCP Fast Open (Client + Server)
net.ipv4.tcp_fastopen=3
net.ipv4.tcp_slow_start_after_idle=0

# IP Forwarding
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1

# Memory & Buffers
net.core.rmem_max=67108864
net.core.wmem_max=67108864
net.ipv4.tcp_rmem=4096 87380 16777216
net.ipv4.tcp_wmem=4096 65536 16777216

# File Descriptors
fs.file-max=65535
EOF

# Also save to sysctl.conf for compatibility
cat >> /etc/sysctl.conf 2>/dev/null << 'EOF'

# Network Ghost v5.1 Optimizations (Backup)
net.core.rmem_max=67108864
net.core.wmem_max=67108864
net.ipv4.tcp_rmem=4096 87380 16777216
net.ipv4.tcp_wmem=4096 65536 16777216
net.ipv4.tcp_congestion_control=bbr
net.core.default_qdisc=fq
net.ipv4.tcp_fastopen=3
net.ipv4.tcp_slow_start_after_idle=0
fs.file-max=65535
net.ipv4.ip_forward=1
net.ipv6.conf.all.forwarding=1
EOF

echo -e "${GREEN}✅ Kernel stack tuned for low-latency phantom mode${NC}"
echo -e "   ${CYAN}→ BBR: Congestion control${NC}"
echo -e "   ${CYAN}→ FQ: Fair queuing scheduler${NC}"
echo -e "   ${CYAN}→ TFO: TCP Fast Open (Client + Server)${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 9: Configure Firewall Rules
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[9/16]${NC} Configuring firewall rules..."

if command -v iptables &> /dev/null; then
    # Allow loopback
    iptables -C INPUT -i lo -j ACCEPT 2>/dev/null || iptables -A INPUT -i lo -j ACCEPT
    iptables -C OUTPUT -o lo -j ACCEPT 2>/dev/null || iptables -A OUTPUT -o lo -j ACCEPT
    
    # Allow established connections
    iptables -C INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT 2>/dev/null || \
        iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
    
    # Allow dashboard port
    iptables -C INPUT -p tcp --dport 9090 -j ACCEPT 2>/dev/null || \
        iptables -A INPUT -p tcp --dport 9090 -j ACCEPT
    
    # Mark tunnel traffic
    iptables -t mangle -C OUTPUT -p tcp --dport 443 -j MARK --set-mark 0x1 2>/dev/null || \
        iptables -t mangle -A OUTPUT -p tcp --dport 443 -j MARK --set-mark 0x1
    
    # NAT for transparent proxy (if eBPF not available)
    if [ "$EBPF_SUPPORTED" = false ]; then
        iptables -t nat -C OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 9091 2>/dev/null || \
            iptables -t nat -A OUTPUT -p tcp --dport 443 -j REDIRECT --to-ports 9091
    fi
    
    echo -e "${GREEN}✅ Firewall configured${NC}"
else
    echo -e "${YELLOW}⚠️  iptables not found, skipping firewall rules${NC}"
fi

# ════════════════════════════════════════════════════════════════════════
# Step 10: Configure ulimits
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[10/16]${NC} Configuring system limits..."

# Add to limits.conf
grep -q "network-ghost" /etc/security/limits.conf 2>/dev/null || cat >> /etc/security/limits.conf << EOF

# Network Ghost v${VERSION}
root soft nofile 65535
root hard nofile 65535
* soft nofile 65535
* hard nofile 65535
EOF

echo -e "${GREEN}✅ Limits configured${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 11: Create Helper Scripts (Including ghost-cli)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[11/16]${NC} Creating helper scripts..."

# Status script
cat > "${INSTALL_DIR}/status.sh" << 'EOF'
#!/bin/bash
echo "═══════════════════════════════════════════════════════════"
echo "   Network Ghost v5.1 - Status"
echo "═══════════════════════════════════════════════════════════"
echo ""
systemctl status network-ghost --no-pager
echo ""
echo "Dashboard: http://$(hostname -I | awk '{print $1}'):9090"
echo "Logs: tail -f /var/log/network-ghost/ghost.log"
echo "CLI: ghost-cli {start|stop|restart|status|logs|log-clean}"
EOF
chmod +x "${INSTALL_DIR}/status.sh"

# Uninstall script
cat > "${INSTALL_DIR}/uninstall.sh" << EOF
#!/bin/bash
echo "🚨 Uninstalling Network Ghost v${VERSION}..."
systemctl stop network-ghost 2>/dev/null || true
systemctl disable network-ghost 2>/dev/null || true
systemctl stop network-ghost-log-clean.timer 2>/dev/null || true
systemctl disable network-ghost-log-clean.timer 2>/dev/null || true
rm -f /etc/systemd/system/network-ghost.service
rm -f /etc/systemd/system/network-ghost-log-clean.service
rm -f /etc/systemd/system/network-ghost-log-clean.timer
rm -rf ${INSTALL_DIR}
rm -rf ${CONFIG_DIR}
rm -rf ${LOG_DIR}
rm -f ${BIN_DIR}/network-ghost
rm -f ${BIN_DIR}/ghost-cli
rm -f /etc/sysctl.d/99-network-ghost.conf
rm -f /etc/logrotate.d/network-ghost
systemctl daemon-reload
echo "✅ Network Ghost uninstalled"
EOF
chmod +x "${INSTALL_DIR}/uninstall.sh"

# Log viewer
cat > "${INSTALL_DIR}/logs.sh" << 'EOF'
#!/bin/bash
tail -f /var/log/network-ghost/ghost.log
EOF
chmod +x "${INSTALL_DIR}/logs.sh"

# Backup script
cat > "${INSTALL_DIR}/backup.sh" << 'EOF'
#!/bin/bash
BACKUP_DIR="/opt/network-ghost/backups"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

echo "🔄 Creating backup..."
mkdir -p "${BACKUP_DIR}"

# Backup config
if [ -f "/etc/network-ghost/config.toml" ]; then
    cp /etc/network-ghost/config.toml "${BACKUP_DIR}/config_${TIMESTAMP}.toml.bak"
    echo "✅ Config backed up: ${BACKUP_DIR}/config_${TIMESTAMP}.toml.bak"
fi

# Backup DAE config
if [ -f "/etc/dae/config.dae" ]; then
    cp /etc/dae/config.dae "${BACKUP_DIR}/config_${TIMESTAMP}.dae.bak"
    echo "✅ DAE config backed up: ${BACKUP_DIR}/config_${TIMESTAMP}.dae.bak"
fi

# Cleanup old backups (keep last 10)
ls -t "${BACKUP_DIR}"/*.bak 2>/dev/null | tail -n +11 | xargs rm -f 2>/dev/null || true
echo "✅ Backup complete!"
EOF
chmod +x "${INSTALL_DIR}/backup.sh"

echo -e "${GREEN}✅ Helper scripts created${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 12: Auto-Log Clean Setup (Memory Protection for Routers)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[12/16]${NC} Setting up Auto-Log Clean for memory protection..."

# Log cleanup script
LOG_CLEAN_SCRIPT="${INSTALL_DIR}/log-clean.sh"
cat > "${LOG_CLEAN_SCRIPT}" << 'LOGCLEAN_EOF'
#!/bin/bash
# ════════════════════════════════════════════════════════════════════════
# Network Ghost - Auto Log Cleaner
# Protects router memory from log overflow
# ════════════════════════════════════════════════════════════════════════

LOG_DIR="/var/log/network-ghost"
MAX_LOG_SIZE_MB=10
MAX_LOG_FILES=5
MAX_LOG_AGE_DAYS=30

echo "🧹 Cleaning Network Ghost logs..."

# Create log directory if not exists
mkdir -p "${LOG_DIR}"

# Get current log directory size
if [ -d "${LOG_DIR}" ]; then
    CURRENT_SIZE=$(du -sm "${LOG_DIR}" 2>/dev/null | awk '{print $1}')
    echo "   Current log size: ${CURRENT_SIZE}MB"
    
    # Remove old rotated logs (keep last N files)
    LOG_COUNT=$(find "${LOG_DIR}" -name "*.log.*" -type f 2>/dev/null | wc -l)
    if [ "$LOG_COUNT" -gt "${MAX_LOG_FILES}" ]; then
        find "${LOG_DIR}" -name "*.log.*" -type f -printf '%T@ %p\n' 2>/dev/null | \
            sort -rn | tail -n +$((MAX_LOG_FILES + 1)) | cut -f2- -d" " | \
            xargs rm -f 2>/dev/null
        echo "   ✅ Removed $((LOG_COUNT - MAX_LOG_FILES)) old log files"
    fi
    
    # Remove logs older than MAX_LOG_AGE_DAYS
    DELETED_OLD=$(find "${LOG_DIR}" -name "*.log*" -type f -mtime +${MAX_LOG_AGE_DAYS} 2>/dev/null | wc -l)
    if [ "$DELETED_OLD" -gt 0 ]; then
        find "${LOG_DIR}" -name "*.log*" -type f -mtime +${MAX_LOG_AGE_DAYS} -delete 2>/dev/null
        echo "   ✅ Removed ${DELETED_OLD} logs older than ${MAX_LOG_AGE_DAYS} days"
    fi
    
    # Truncate main log if too large
    MAIN_LOG="${LOG_DIR}/ghost.log"
    if [ -f "${MAIN_LOG}" ]; then
        LOG_SIZE=$(stat -c%s "${MAIN_LOG}" 2>/dev/null || echo "0")
        LOG_SIZE_MB=$((LOG_SIZE / 1024 / 1024))
        
        if [ "$LOG_SIZE_MB" -gt "${MAX_LOG_SIZE_MB}" ]; then
            # Rotate the log
            mv "${MAIN_LOG}" "${MAIN_LOG}.$(date +%Y%m%d_%H%M%S)" 2>/dev/null
            touch "${MAIN_LOG}"
            echo "   ✅ Rotated main log (${LOG_SIZE_MB}MB -> new file)"
        fi
    fi
    
    # Clean journal logs if journalctl exists
    if command -v journalctl &> /dev/null; then
        journalctl --vacuum-time=7d --quiet 2>/dev/null || true
    fi
    
    # Report final size
    FINAL_SIZE=$(du -sm "${LOG_DIR}" 2>/dev/null | awk '{print $1}')
    echo "   Final log size: ${FINAL_SIZE}MB"
    echo "✅ Log cleanup complete!"
else
    echo "   Log directory not found, skipping..."
fi
LOGCLEAN_EOF
chmod +x "${LOG_CLEAN_SCRIPT}"

# Create logrotate configuration
if [ -d "/etc/logrotate.d" ]; then
    cat > /etc/logrotate.d/network-ghost << 'LOGROTATE_EOF'
/var/log/network-ghost/*.log {
    daily
    rotate 5
    compress
    delaycompress
    missingok
    notifempty
    create 0644 root root
    maxsize 10M
    postrotate
        systemctl reload network-ghost > /dev/null 2>&1 || true
    endscript
}
LOGROTATE_EOF
    echo -e "   ${GREEN}✅ Logrotate configured${NC}"
fi

# Setup cron job for weekly log cleanup
CRON_JOB="0 3 * * 0 ${INSTALL_DIR}/log-clean.sh >> ${LOG_DIR}/log-clean.log 2>&1"

# Check if cron is available
if command -v crontab &> /dev/null; then
    # Add cron job if not exists
    (crontab -l 2>/dev/null | grep -v "log-clean.sh"; echo "${CRON_JOB}") | crontab - 2>/dev/null || true
    echo -e "   ${GREEN}✅ Weekly cron job scheduled (Sunday 3:00 AM)${NC}" 
fi

# Create systemd timer as alternative to cron
cat > /etc/systemd/system/network-ghost-log-clean.service << 'EOF'
[Unit]
Description=Network Ghost Log Cleanup
After=network.target

[Service]
Type=oneshot
ExecStart=/opt/network-ghost/log-clean.sh

[Install]
WantedBy=multi-user.target
EOF

cat > /etc/systemd/system/network-ghost-log-clean.timer << 'EOF'
[Unit]
Description=Weekly Network Ghost Log Cleanup Timer

[Timer]
OnCalendar=Sun *-*-* 03:00:00
Persistent=true

[Install]
WantedBy=timers.target
EOF

systemctl daemon-reload
systemctl enable network-ghost-log-clean.timer 2>/dev/null || true

echo -e "${GREEN}✅ Auto-Log Clean configured${NC}"
echo -e "   ${CYAN}→ Logrotate: daily rotation, max 10MB per file${NC}"
echo -e "   ${CYAN}→ Cron: weekly cleanup at Sunday 3:00 AM${NC}"
echo -e "   ${CYAN}→ Systemd Timer: alternative weekly cleanup${NC}"
echo -e "   ${CYAN}→ Manual: ghost-cli log-clean${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 13: Create ghost-cli (with log-clean command)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[13/16]${NC} Creating ghost-cli utility..."

cat > "${BIN_DIR}/ghost-cli" << 'GHOSTCLI_EOF'
#!/bin/bash
# Network Ghost CLI Utility

case $1 in
    start)
        systemctl start network-ghost
        echo "✅ Network Ghost started"
        ;;
    stop)
        systemctl stop network-ghost
        echo "🛑 Network Ghost stopped"
        ;;
    restart)
        systemctl restart network-ghost
        echo "🔄 Network Ghost restarted"
        ;;
    status)
        systemctl status network-ghost --no-pager
        ;;
    logs)
        journalctl -u network-ghost -f
        ;;
    config)
        nano /etc/network-ghost/config.toml
        ;;
    backup)
        cp /etc/network-ghost/config.toml /opt/network-ghost/backups/config_$(date +%Y%m%d_%H%M%S).toml.bak
        echo "✅ Configuration backed up"
        ;;
    log-clean)
        /opt/network-ghost/log-clean.sh
        ;;
    memory)
        echo "📊 Memory Status:"
        echo ""
        if [ -f "/proc/meminfo" ]; then
            MEM_TOTAL=$(grep MemTotal /proc/meminfo | awk '{print $2}')
            MEM_AVAILABLE=$(grep MemAvailable /proc/meminfo | awk '{print $2}')
            MEM_USED=$((MEM_TOTAL - MEM_AVAILABLE))
            MEM_USED_MB=$((MEM_USED / 1024))
            MEM_TOTAL_MB=$((MEM_TOTAL / 1024))
            MEM_PERCENT=$((MEM_USED * 100 / MEM_TOTAL))
            echo "   Total: ${MEM_TOTAL_MB}MB"
            echo "   Used:  ${MEM_USED_MB}MB (${MEM_PERCENT}%)"
        fi
        if [ -d "/var/log/network-ghost" ]; then
            LOG_SIZE=$(du -sm /var/log/network-ghost 2>/dev/null | awk '{print $1}')
            echo "   Logs:  ${LOG_SIZE}MB"
        fi
        ;;
    *)
        echo "🚀 Network Ghost CLI"
        echo ""
        echo "Usage: ghost-cli {start|stop|restart|status|logs|config|backup|log-clean|memory}"
        echo ""
        echo "Commands:"
        echo "  start     - Start the phantom tunnel"
        echo "  stop      - Stop the phantom tunnel"
        echo "  restart   - Restart the phantom tunnel"
        echo "  status    - Show service status"
        echo "  logs      - Follow real-time logs"
        echo "  config    - Edit configuration"
        echo "  backup    - Backup current config"
        echo "  log-clean - Clean old logs (free memory)"
        echo "  memory    - Show memory status"
        ;;
esac
GHOSTCLI_EOF
chmod +x "${BIN_DIR}/ghost-cli"

echo -e "${GREEN}✅ ghost-cli created${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 14: Enable Service
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[14/16]${NC} Enabling service..."

systemctl enable network-ghost

echo -e "${GREEN}✅ Service enabled${NC}"

# ════════════════════════════════════════════════════════════════════════
# Step 15: Kernel Optimization Verification (BBR & FQ Check)
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${YELLOW}[15/16]${NC} Verifying Kernel Optimizations..."

# Check if BBR is active
CURRENT_CC=$(sysctl net.ipv4.tcp_congestion_control 2>/dev/null | awk '{print $3}' || echo "unknown")
CURRENT_QDISC=$(sysctl net.core.default_qdisc 2>/dev/null | awk '{print $3}' || echo "unknown")

if [[ "$CURRENT_CC" == "bbr" ]]; then
    echo -e "   TCP Congestion Control: ${GREEN}✅ BBR is Active${NC}"
else
    echo -e "   TCP Congestion Control: ${RED}❌ $CURRENT_CC (BBR failed to activate)${NC}"
    echo -e "   ${YELLOW}Attempting forced module load...${NC}"
    modprobe tcp_bbr 2>/dev/null || echo -e "   ${RED}⚠ Kernel module 'tcp_bbr' not found or already built-in.${NC}"
    # Re-check after module load attempt
    CURRENT_CC=$(sysctl net.ipv4.tcp_congestion_control 2>/dev/null | awk '{print $3}' || echo "unknown")
    if [[ "$CURRENT_CC" == "bbr" ]]; then
        echo -e "   TCP Congestion Control: ${GREEN}✅ BBR is now Active${NC}"
    fi
fi

if [[ "$CURRENT_QDISC" == "fq" ]]; then
    echo -e "   Network Scheduler:      ${GREEN}✅ FQ is Active${NC}"
else
    echo -e "   Network Scheduler:      ${YELLOW}⚠️ $CURRENT_QDISC (FQ not default, but BBR will still work)${NC}"
fi

# Check eBPF availability for DAE
if lsmod 2>/dev/null | grep -q "cls_bpf" || [ -d "/sys/fs/bpf" ]; then
    echo -e "   eBPF Datapath:          ${GREEN}✅ Kernel Ready${NC}"
else
    echo -e "   eBPF Datapath:          ${MAGENTA}⚠ Limited (Using Legacy Tproxy)${NC}"
fi

# Check TCP Fast Open
TFO=$(cat /proc/sys/net/ipv4/tcp_fastopen 2>/dev/null || echo "0")
if [[ "$TFO" == "3" ]]; then
    echo -e "   TCP Fast Open:          ${GREEN}✅ Enabled (Client + Server)${NC}"
else
    echo -e "   TCP Fast Open:          ${YELLOW}⚠️ Value: $TFO${NC}"
fi

# ════════════════════════════════════════════════════════════════════════
# Step 16: Final Summary
# ════════════════════════════════════════════════════════════════════════

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  ✅ Network Ghost v${VERSION} Installed Successfully!      ║${NC}"
echo -e "${GREEN}║     Ultimate Phantom Edition                                 ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  📂 Install Dir:   ${CYAN}${INSTALL_DIR}${NC}"
echo -e "  📂 Config Dir:    ${CYAN}${CONFIG_DIR}${NC}"
echo -e "  📂 Backup Dir:    ${CYAN}${BACKUP_DIR}${NC}"
echo -e "  📂 Log Dir:       ${CYAN}${LOG_DIR}${NC}"
echo -e "  📂 DAE Config:    ${CYAN}${DAE_DIR}${NC}"
if [ -n "${BINARY_SIZE_MB:-}" ] && [ "${BINARY_SIZE_MB}" -gt 0 ]; then
    echo -e "  💾 Binary Size:   ${CYAN}${BINARY_SIZE_MB}MB${NC}"
fi
echo -e "  🧠 Memory Limit:  ${CYAN}160MB${NC}"
echo ""
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${YELLOW}🚀 Quick Start Guide:${NC}"
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${CYAN}1. Edit configuration:${NC}"
echo -e "     nano ${CONFIG_DIR}/config.toml"
echo -e "     ${MAGENTA}or use: ghost-cli config${NC}"
echo ""
echo -e "  ${CYAN}2. Set your server details in config:${NC}"
echo -e "     - server: Your proxy server IP"
echo -e "     - uuid: Your UUID (auto-generated: ${NEW_UUID})"
echo ""
echo -e "  ${CYAN}3. Start the service:${NC}"
echo -e "     systemctl start network-ghost"
echo -e "     ${MAGENTA}or use: ghost-cli start${NC}"
echo ""
echo -e "  ${CYAN}4. Check status:${NC}"
echo -e "     ${INSTALL_DIR}/status.sh"
echo -e "     ${MAGENTA}or use: ghost-cli status${NC}"
echo ""
echo -e "  ${CYAN}5. View real-time logs:${NC}"
echo -e "     ${INSTALL_DIR}/logs.sh"
echo -e "     ${MAGENTA}or use: ghost-cli logs${NC}"
echo ""
echo -e "  ${CYAN}6. Open Dashboard:${NC}"
echo -e "     http://ROUTER_IP:9090"
echo ""
echo -e "  ${CYAN}7. Backup configuration:${NC}"
echo -e "     ${INSTALL_DIR}/backup.sh"
echo -e "     ${MAGENTA}or use: ghost-cli backup${NC}"
echo ""
echo -e "  ${CYAN}8. Clean logs (free memory):${NC}"
echo -e "     ${INSTALL_DIR}/log-clean.sh"
echo -e "     ${MAGENTA}or use: ghost-cli log-clean${NC}"
echo ""
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}🔧 Available Commands:${NC}"
echo -e "${MAGENTA}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${CYAN}ghost-cli start${NC}     - Start phantom tunnel"
echo -e "  ${CYAN}ghost-cli stop${NC}      - Stop phantom tunnel"
echo -e "  ${CYAN}ghost-cli restart${NC}   - Restart phantom tunnel"
echo -e "  ${CYAN}ghost-cli status${NC}    - Show service status"
echo -e "  ${CYAN}ghost-cli logs${NC}      - Follow real-time logs"
echo -e "  ${CYAN}ghost-cli config${NC}    - Edit configuration"
echo -e "  ${CYAN}ghost-cli backup${NC}    - Backup current config"
echo -e "  ${CYAN}ghost-cli log-clean${NC} - Clean old logs (free memory)"
echo -e "  ${CYAN}ghost-cli memory${NC}    - Show memory status"
echo ""

# Ask to start now
echo ""
read -p "🚀 Start Network Ghost now? (y/n) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    systemctl start network-ghost
    sleep 3
    systemctl status network-ghost --no-pager
    echo ""
    echo -e "${GREEN}✨ Phantom Tunnel is now running in the background.${NC}"
fi

echo ""
echo -e "${GREEN}🎉 Setup complete!${NC}"
echo -e "${MAGENTA}💡 TIP: Use ${CYAN}ghost-cli logs${MAGENTA} to see real-time traffic.${NC}"
echo -e "${MAGENTA}💡 TIP: Use ${CYAN}ghost-cli log-clean${MAGENTA} to free memory on your router.${NC}"
echo ""
