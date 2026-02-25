#!/bin/sh
# Network Ghost v5.0 — نصب کامل روی Google WiFi / ImmortalWrt
set -e

NG_DIR="/opt/network-ghost"

echo "╔══════════════════════════════════════════════╗"
echo "║  👻 Network Ghost v5.0 — نصب روی Google WiFi ║"
echo "╚══════════════════════════════════════════════╝"

opkg update 2>/dev/null
opkg install kmod-tun kmod-nft-tproxy kmod-nf-tproxy \
    kmod-nfnetlink-queue kmod-ipt-tproxy iptables-mod-tproxy \
    nftables kmod-wireguard wireguard-tools \
    ca-bundle curl wget-ssl ip-full 2>/dev/null || true

mkdir -p $NG_DIR/{bin,config,logs,cache,scripts,geodata,zapret}

cat > $NG_DIR/config/config.toml << 'CONF'
sni = "ebanking.bmi.ir"
protocol = "auto"
cdn = "cloudflare"
dpi_mode = "ghost"
max_latency = 300
port_hopping = true
enable_zapret = true
enable_goodbyedpi = true
enable_warp = false
CONF

# TPROXY setup script
cat > $NG_DIR/scripts/tproxy-setup.sh << 'TPROXY'
#!/bin/sh
PROXY_PORT=7892
DNS_PORT=7874
MARK=1
TABLE=100

ip route add local default dev lo table $TABLE 2>/dev/null || true
ip rule add fwmark $MARK table $TABLE 2>/dev/null || true

iptables -t mangle -N GHOST_TP 2>/dev/null || iptables -t mangle -F GHOST_TP
for net in 0.0.0.0/8 127.0.0.0/8 169.254.0.0/16 172.16.0.0/12 192.168.0.0/16 10.0.0.0/8; do
    iptables -t mangle -A GHOST_TP -d $net -j RETURN
done
iptables -t mangle -A GHOST_TP -m mark --mark $MARK -j RETURN
iptables -t mangle -A GHOST_TP -p tcp -j TPROXY --tproxy-mark $MARK --on-port $PROXY_PORT
iptables -t mangle -A GHOST_TP -p udp -j TPROXY --tproxy-mark $MARK --on-port $PROXY_PORT
iptables -t mangle -A PREROUTING -j GHOST_TP

iptables -t nat -N GHOST_NAT 2>/dev/null || iptables -t nat -F GHOST_NAT
iptables -t nat -A GHOST_NAT -p udp --dport 53 -j REDIRECT --to-port $DNS_PORT
iptables -t nat -A PREROUTING -j GHOST_NAT

sysctl -w net.ipv4.tcp_fastopen=3 >/dev/null 2>&1 || true
sysctl -w net.core.rmem_max=26214400 >/dev/null 2>&1 || true
echo 1 > /proc/sys/net/core/bpf_jit_enable 2>/dev/null || true
echo "✅ TPROXY active (port $PROXY_PORT)"
TPROXY

cat > $NG_DIR/scripts/tproxy-cleanup.sh << 'CLN'
#!/bin/sh
ip rule del fwmark 1 table 100 2>/dev/null || true
ip route flush table 100 2>/dev/null || true
for chain in GHOST_TP GHOST_NAT GHOST_LOCAL; do
    iptables -t mangle -F $chain 2>/dev/null; iptables -t mangle -X $chain 2>/dev/null
    iptables -t nat -F $chain 2>/dev/null; iptables -t nat -X $chain 2>/dev/null
done
echo "✅ TPROXY cleaned"
CLN

cat > $NG_DIR/scripts/zapret-iptables.sh << 'ZAPR'
#!/bin/sh
iptables -t mangle -N ZAPRET 2>/dev/null || iptables -t mangle -F ZAPRET
for net in 0.0.0.0/8 127.0.0.0/8 169.254.0.0/16 172.16.0.0/12 192.168.0.0/16 10.0.0.0/8; do
    iptables -t mangle -A ZAPRET -d $net -j RETURN
done
iptables -t mangle -A ZAPRET -p tcp -m multiport --dport 80,443 \
  -m connbytes --connbytes 0:6 --connbytes-dir original --connbytes-mode packets \
  -j NFQUEUE --queue-num 100 --queue-bypass
iptables -t mangle -A OUTPUT -j ZAPRET
iptables -t mangle -A FORWARD -j ZAPRET
echo "✅ Zapret iptables applied"
ZAPR

chmod +x $NG_DIR/scripts/*.sh

cat > /etc/init.d/network-ghost << 'INIT'
#!/bin/sh /etc/rc.common
START=90
STOP=10
PROG=/opt/network-ghost/bin/network-ghost
PID=/tmp/network-ghost.pid
start() {
    $PROG start --config /opt/network-ghost/config/config.toml &
    echo $! > $PID
    /opt/network-ghost/scripts/tproxy-setup.sh
    /opt/network-ghost/scripts/zapret-iptables.sh
}
stop() {
    /opt/network-ghost/scripts/tproxy-cleanup.sh
    [ -f $PID ] && kill $(cat $PID) 2>/dev/null; rm -f $PID
}
restart() { stop; sleep 1; start; }
INIT
chmod +x /etc/init.d/network-ghost
/etc/init.d/network-ghost enable

cat > /etc/hotplug.d/iface/99-network-ghost << 'HOT'
#!/bin/sh
[ "$ACTION" = "ifup" ] && [ "$INTERFACE" = "wan" ] && {
    sleep 5; /etc/init.d/network-ghost restart &
}
HOT

echo "✅ نصب کامل شد!"
echo "   راه‌اندازی: /etc/init.d/network-ghost start"
echo "   Dashboard:  http://192.168.1.1:9090"
