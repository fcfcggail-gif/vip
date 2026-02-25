# 👻 Network Ghost v5.0 — Zero-Knowledge Phantom Network Tunnel

**سیستم ضد فیلتر نسل پنجم با ۲۰+ لایه رمزگذاری و ضد هوش مصنوعی DPI**  
طراحی‌شده برای Google WiFi با ImmortalWrt / OpenWrt

---

## ✨ قابلیت‌های v5.0 Ultra

### موتورهای DPI Bypass

- **Zapret/ByeDPI** — تکه‌تکه کردن TLS ClientHello در محل دقیق SNI + پکت‌های fake با TTL پایین. استراتژی‌ها: Fragment, Fake, Disorder, FragmentFake, DisorderFake, OOB, FullBypass, Auto
- **GoodbyeDPI** — bypass HTTP/HTTPS + mixed-case Host + DNS redirect. حالت‌ها: Passive, ActiveHttp, ActiveHttps, Complete, Iranian
- **DAE (eBPF)** — مسیریابی kernel-level با eBPF TProxy. کمترین CPU load
- **TPROXY** — transparent proxy کامل برای تمام دستگاه‌های شبکه (IPv4 + IPv6)
- **Anti-AI DPI Ghost Mode** — پنهان‌سازی کامل با آنتروپی تصادفی، fake TLS/QUIC traffic، و rotation پروفایل

### پروتکل‌ها

Reality/VLESS, ShadowTLS v3, Hysteria2, TUIC v5, MASQUE (RFC 9298), XHTTP, WARP (WireGuard), Double WARP, WebSocket Transport, gRPC Transport, IP-Relay (Multi-hop CDN)

### قابلیت‌های روتر (Google WiFi / ImmortalWrt)

- Hardware Flow Offload برای IPQ40xx (کاهش CPU load تا ۸۰٪)
- BBR Congestion Control
- eBPF JIT kernel-level
- UCI configuration generator
- init.d service + hotplug auto-start

---

## نصب

```bash
tar -xzf network-ghost-v5-ultra-final.tar.gz
cd network-ghost-v5
chmod +x setup-router.sh
./setup-router.sh
```

---

## CLI

```bash
network-ghost start --dpi-mode ghost
network-ghost scan --cdn cloudflare
network-ghost gen-dae --output /etc/dae/config.dae
network-ghost info
network-ghost status
```

---

## پیکربندی

```toml
# /opt/network-ghost/config/config.toml
dpi_mode       = "ghost"
enable_zapret  = true
zapret_strategy = "auto"
enable_goodbyedpi = true
goodbyedpi_mode   = "iranian"
enable_warp    = false
port_hopping   = true
```

---

## Dashboard

`http://192.168.1.1:9090` — Clash API سازگار

---

MIT License — Network Ghost Team
