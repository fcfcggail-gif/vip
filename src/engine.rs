//! Network Ghost Engine - Core orchestration engine

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result};
use tokio::{
    sync::{Mutex, RwLock, broadcast},
    time::{interval, timeout},
};
use tracing::{error, info, warn};

use crate::{
    anti_ai_dpi::AntiAiDpi,
    circuit_breaker::CircuitBreaker,
    dae_generator::DaeGenerator,
    dashboard::{DashboardConfig, DashboardServer, TunnelInfo},
    dns_over_quic::DnsOverQuic,
    fingerprint::FingerprintManager,
    matryoshka::MatryoshkaDialer,
    port_hopper::PortHopper,
    scanner::TlsScanner,
    types::{
        CdnType, EngineEvent, ProtocolType, ProxyConfig, ScanResult, TunnelState,
        ALTERNATIVE_PORTS,
    },
};

/// Core Network Ghost engine
pub struct NetworkGhostEngine {
    /// Configuration
    config: Arc<RwLock<ProxyConfig>>,
    /// Tunnel state
    state: Arc<RwLock<TunnelState>>,
    /// Clean IP list
    clean_ips: Arc<Mutex<Vec<ScanResult>>>,
    /// Circuit Breaker
    circuit_breaker: Arc<CircuitBreaker>,
    /// Anti-AI DPI system
    anti_ai: Arc<AntiAiDpi>,
    /// DNS manager
    dns_manager: Arc<DnsOverQuic>,
    /// DAE generator
    dae_gen: Arc<DaeGenerator>,
    /// Port Hopper
    port_hopper: Arc<tokio::sync::Mutex<PortHopper>>,
    /// Dashboard
    dashboard: Arc<DashboardServer>,
    /// Event broadcast channel
    event_tx: broadcast::Sender<EngineEvent>,
    /// Fingerprint manager
    fp_manager: Arc<FingerprintManager>,
}

impl NetworkGhostEngine {
    /// Create a new engine instance
    pub async fn new(config: ProxyConfig) -> Result<Self> {
        let (event_tx, _) = broadcast::channel(1024);

        let dashboard = DashboardServer::new(DashboardConfig::default());
        let port_hopper = PortHopper::new();

        let engine = Self {
            config: Arc::new(RwLock::new(config)),
            state: Arc::new(RwLock::new(TunnelState::default())),
            clean_ips: Arc::new(Mutex::new(Vec::new())),
            circuit_breaker: Arc::new(CircuitBreaker::new(
                Duration::from_secs(30),
                3,
                Duration::from_millis(300),
            )),
            anti_ai: Arc::new(AntiAiDpi::new()),
            dns_manager: Arc::new(DnsOverQuic::new("9.9.9.9:53").await?),
            dae_gen: Arc::new(DaeGenerator::new()),
            port_hopper: Arc::new(tokio::sync::Mutex::new(port_hopper)),
            dashboard: Arc::new(dashboard),
            event_tx,
            fp_manager: Arc::new(FingerprintManager::new()),
        };

        Ok(engine)
    }

    /// Start the tunnel
    pub async fn start(&self) -> Result<()> {
        info!("🚀 Starting Network Ghost v5.0 (Kernel-Level Infrastructure)...");

        // Stage 0: Start Dashboard
        self.dashboard.start().await?;

        // Stage 1: Initialize Fingerprint Rotation
        let fp = self.fp_manager.current();
        info!("🔑 uTLS Fingerprint: {}", fp.fp_type);

        // Stage 2: Scan clean IPs with TLS Fragmentation Detection
        let clean_ips = self.scan_clean_ips().await?;

        if clean_ips.is_empty() {
            error!("No clean IPs found!");
            return Err(anyhow::anyhow!("No clean IPs found"));
        }

        // Store clean IPs
        {
            let mut ips = self.clean_ips.lock().await;
            *ips = clean_ips.clone();
        }

        // Stage 3: Select best IP
        let best_ip = self.select_best_ip(&clean_ips)?;

        // Stage 4: Build Matryoshka chain (up to 20 layers)
        let matryoshka = self.build_matryoshka_chain(&best_ip).await?;

        // Stage 5: Connect via Matryoshka
        self.connect_with_matryoshka(matryoshka).await?;

        // Stage 6: Start Port Hopping
        if self.config.read().await.enable_port_hopping {
            self.start_port_hopping().await;
        }

        // Stage 7: Start monitoring
        self.start_monitoring().await;

        // Stage 8: Generate DAE config for eBPF
        self.generate_dae_config().await?;

        info!("✅ Infrastructure is LIVE. Dashboard at http://ROUTER_IP:9090");
        Ok(())
    }

    /// Scan clean IPs with Fragmentation Detection
    async fn scan_clean_ips(&self) -> Result<Vec<ScanResult>> {
        info!("🔍 Starting clean IP scan (TLS Fragmentation Detection)...");

        let scanner = TlsScanner::new(self.dns_manager.clone(), self.anti_ai.clone());

        let config = self.config.read().await;
        let results = scanner
            .scan_all_cdns(config.cdn_type, ALTERNATIVE_PORTS, Some(50))
            .await?;

        // Filter IPs that support Fragmentation or are clean
        let fragmentation_ok: Vec<ScanResult> = results
            .into_iter()
            .filter(|r| r.supports_fragmentation || r.is_clean)
            .collect();

        info!(
            "✅ {} clean IPs found (Fragmentation OK)",
            fragmentation_ok.len()
        );

        let _ = self.event_tx.send(EngineEvent::ScanCompleted {
            count: fragmentation_ok.len(),
        });

        Ok(fragmentation_ok)
    }

    /// Select the best IP from scan results
    fn select_best_ip(&self, results: &[ScanResult]) -> Result<ScanResult> {
        let mut sorted = results.to_vec();
        sorted.sort_by(|a, b| {
            // Prioritise IPs with fragmentation support
            let frag_cmp = b.supports_fragmentation.cmp(&a.supports_fragmentation);
            if frag_cmp != std::cmp::Ordering::Equal {
                return frag_cmp;
            }
            b.quality_score
                .partial_cmp(&a.quality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        sorted.into_iter().next().context("No valid IP available")
    }

    /// Build Matryoshka chain
    async fn build_matryoshka_chain(
        &self,
        scan_result: &ScanResult,
    ) -> Result<MatryoshkaDialer> {
        let config = self.config.read().await;

        info!("🧅 Building Matryoshka chain...");

        let mut dialer =
            MatryoshkaDialer::from_ip(scan_result.ip, scan_result.port);

        // Layer 1: ShadowTLS with Iranian SNI
        dialer = dialer.wrap_with_shadowtls(&config.sni);
        let _ = self
            .event_tx
            .send(EngineEvent::LayerAdded { layer: "ShadowTLS v3".to_string() });

        // Layer 2: Reality/VLESS
        if matches!(config.protocol, ProtocolType::Reality) {
            dialer = dialer.wrap_with_reality(
                &config.uuid,
                &config.public_key.clone().unwrap_or_default(),
            );
            let _ = self
                .event_tx
                .send(EngineEvent::LayerAdded { layer: "Reality (VLESS)".to_string() });
        }

        // Layer 3: SMUX Multiplexing
        dialer = dialer.enable_smux();
        let _ = self
            .event_tx
            .send(EngineEvent::LayerAdded { layer: "SMUX Mux".to_string() });

        let layer_count = dialer.layer_count();
        info!("🧅 Matryoshka chain: {} layers", layer_count);

        let _ = self
            .event_tx
            .send(EngineEvent::MatryoshkaChainComplete { layers: layer_count });

        Ok(dialer)
    }

    /// Connect via Matryoshka
    async fn connect_with_matryoshka(
        &self,
        mut matryoshka: MatryoshkaDialer,
    ) -> Result<()> {
        let config = self.config.read().await.clone();

        info!("🔐 Establishing Matryoshka connection...");

        matryoshka.start().await?;

        {
            let mut state = self.state.write().await;
            state.active = true;
            state.current_ip = Some(matryoshka.target_addr().ip());
            state.current_port = matryoshka.target_addr().port();
            state.protocol = config.protocol;
            state.active_layers = matryoshka.layer_count();
            state.started_at = Some(chrono::Utc::now());
        }

        let _ = self.event_tx.send(EngineEvent::TunnelStarted {
            ip: matryoshka.target_addr().ip(),
            port: matryoshka.target_addr().port(),
        });

        info!("✅ Connection established with {} layers", matryoshka.layer_count());
        Ok(())
    }

    /// Start Port Hopping background task
    async fn start_port_hopping(&self) {
        info!("🔄 Starting Port Hopping...");

        let hopper = self.port_hopper.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(60));

            loop {
                tick.tick().await;

                let hopper_guard = hopper.lock().await;
                if hopper_guard.should_hop().await {
                    let old_port = hopper_guard.current_port();
                    if let Ok(new_port) = hopper_guard.hop().await {
                        let _ = event_tx.send(EngineEvent::PortSwitched {
                            old: old_port,
                            new: new_port,
                        });
                    }
                }
            }
        });
    }

    /// Start monitoring background task
    async fn start_monitoring(&self) {
        let state = self.state.clone();
        let clean_ips = self.clean_ips.clone();
        let circuit_breaker = self.circuit_breaker.clone();
        let event_tx = self.event_tx.clone();
        let anti_ai = self.anti_ai.clone();
        let dashboard = self.dashboard.clone();

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(10));

            loop {
                tick.tick().await;

                let current_state = state.read().await.clone();

                if !current_state.active {
                    continue;
                }

                if let Some(ip) = current_state.current_ip {
                    let latency =
                        Self::measure_latency(ip).await.unwrap_or(u64::MAX);

                    // Circuit Breaker check
                    if circuit_breaker.should_trip(latency) {
                        warn!(
                            "⚠️ Circuit Breaker triggered! Latency: {}ms",
                            latency
                        );

                        let _ = event_tx.send(EngineEvent::CircuitBreakerTriggered {
                            ip,
                            latency_ms: latency,
                        });

                        if let Err(e) =
                            Self::switch_to_new_ip(&state, &clean_ips, &event_tx)
                                .await
                        {
                            error!("IP switch error: {}", e);
                        }
                    }

                    // Update Dashboard
                    dashboard
                        .update_tunnel(TunnelInfo {
                            active: current_state.active,
                            current_ip: ip.to_string(),
                            current_port: current_state.current_port,
                            protocol: format!("{:?}", current_state.protocol),
                            cdn: format!("{:?}", current_state.cdn),
                            uptime_secs: current_state
                                .started_at
                                .map(|t| {
                                    (chrono::Utc::now() - t).num_seconds() as u64
                                })
                                .unwrap_or(0),
                            rx_bytes: current_state.stats.rx_bytes,
                            tx_bytes: current_state.stats.tx_bytes,
                            latency_ms: latency,
                        })
                        .await;
                }

                // Check AI detection risk
                let detection = anti_ai.analyze_detection_risk();
                if detection.is_detected {
                    warn!(
                        "⚠️ Suspicious AI detection: {:?}",
                        detection.detection_reason
                    );
                    anti_ai.adapt_to_detection(&detection);
                }
            }
        });
    }

    /// Measure TCP latency to an IP
    async fn measure_latency(ip: IpAddr) -> Result<u64> {
        let start = Instant::now();
        let addr = SocketAddr::new(ip, 443);
        let stream = timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect(addr),
        )
        .await??;
        drop(stream);
        Ok(start.elapsed().as_millis() as u64)
    }

    /// Switch to a new clean IP
    async fn switch_to_new_ip(
        state: &Arc<RwLock<TunnelState>>,
        clean_ips: &Arc<Mutex<Vec<ScanResult>>>,
        event_tx: &broadcast::Sender<EngineEvent>,
    ) -> Result<()> {
        let ips = clean_ips.lock().await;

        if ips.len() < 2 {
            return Err(anyhow::anyhow!("No alternative IPs available"));
        }

        let old_ip = state.read().await.current_ip;
        let new_ip = ips[1].ip;

        {
            let mut s = state.write().await;
            s.current_ip = Some(new_ip);
            s.switch_count += 1;
        }

        if let Some(old) = old_ip {
            let _ = event_tx.send(EngineEvent::IpSwitched { old, new: new_ip });
        }

        info!("🔄 Switched to new IP: {}", new_ip);
        Ok(())
    }

    /// Generate DAE config for eBPF kernel-level routing
    async fn generate_dae_config(&self) -> Result<()> {
        info!("📝 Generating DAE config (eBPF Kernel-Level)...");

        let config = self.config.read().await;
        let state = self.state.read().await;
        let clean_ips = self.clean_ips.lock().await;

        let nodes: Vec<_> = clean_ips
            .iter()
            .map(|r| {
                (
                    r.ip,
                    r.port,
                    config.protocol.clone(),
                    config.sni.clone(),
                    config.uuid.clone(),
                )
            })
            .collect();

        let dae_config = if nodes.is_empty() {
            self.dae_gen.generate(
                state.current_ip,
                state.current_port,
                config.protocol.clone(),
                config.sni.clone(),
                config.uuid.clone(),
            )?
        } else {
            self.dae_gen.generate_multi(nodes)?
        };

        tokio::fs::write("/etc/dae/config.dae", &dae_config).await?;

        info!("✅ DAE config saved (eBPF TProxy)");
        Ok(())
    }

    /// Stop the tunnel
    pub async fn stop(&self, reason: &str) -> Result<()> {
        info!("🛑 Stopping tunnel: {}", reason);

        {
            let mut state = self.state.write().await;
            state.active = false;
            state.last_error = Some(reason.to_string());
        }

        let _ = self.event_tx.send(EngineEvent::TunnelStopped {
            reason: reason.to_string(),
        });

        Ok(())
    }

    /// Get current tunnel state
    pub async fn get_state(&self) -> TunnelState {
        self.state.read().await.clone()
    }

    /// Subscribe to engine events
    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.event_tx.subscribe()
    }

    /// Get list of clean IPs
    pub async fn get_clean_ips(&self) -> Vec<ScanResult> {
        self.clean_ips.lock().await.clone()
    }

    /// Test the current connection
    pub async fn test_connection(&self) -> Result<bool> {
        let state = self.state.read().await;

        if !state.active {
            return Ok(false);
        }

        if let Some(ip) = state.current_ip {
            let latency = Self::measure_latency(ip).await?;
            Ok(latency < 1000)
        } else {
            Ok(false)
        }
    }
}
