//! Port Hopping داینامیک
//!
//! تغییر خودکار پورت برای جلوگیری از تشخیص الگو

use std::{
    sync::atomic::{AtomicU16, AtomicU64, Ordering},
    time::{Duration, Instant},
};

use anyhow::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// ==================== CONSTANTS ====================

/// پورت‌های استاندارد HTTPS
const HTTPS_PORTS: &[u16] = &[443, 2053, 2083, 2087, 2096, 8443];

/// فاصله زمانی پیش‌فرض برای Hopping (ثانیه)
const DEFAULT_HOP_INTERVAL_SECS: u64 = 300; // 5 دقیقه

// ==================== PORT STRATEGY ====================

/// استراتژی انتخاب پورت
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PortStrategy {
    /// ترتیبی
    Sequential,
    /// تصادفی
    Random,
    /// وزنی
    Weighted,
    /// بر اساس تأخیر
    LatencyBased,
    /// تطبیقی
    Adaptive,
}

impl Default for PortStrategy {
    fn default() -> Self {
        Self::Adaptive
    }
}

// ==================== PORT STATE ====================

/// وضعیت یک پورت
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortState {
    /// شماره پورت
    pub port: u16,
    /// فعال
    pub active: bool,
    /// میانگین تأخیر (ms)
    pub avg_latency_ms: f64,
    /// تعداد موفقیت
    pub success_count: u64,
    /// تعداد خطا
    pub error_count: u64,
    /// امتیاز
    pub score: f32,
    /// آخرین استفاده
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for PortState {
    fn default() -> Self {
        Self {
            port: 443,
            active: true,
            avg_latency_ms: 0.0,
            success_count: 0,
            error_count: 0,
            score: 1.0,
            last_used: None,
        }
    }
}

// ==================== PORT HOPPER ====================

/// مدیریت Port Hopping
pub struct PortHopper {
    /// پورت‌های موجود
    ports: RwLock<Vec<PortState>>,
    /// پورت فعلی
    current_port: AtomicU16,
    /// استراتژی
    strategy: std::sync::Mutex<PortStrategy>,
    /// فاصله زمانی (ثانیه)
    hop_interval: AtomicU64,
    /// آخرین تغییر
    last_hop: std::sync::Mutex<Instant>,
    /// تعداد تغییرات
    hop_count: AtomicU64,
    /// فعال
    enabled: std::sync::Mutex<bool>,
}

impl PortHopper {
    /// ایجاد Port Hopper جدید
    pub fn new() -> Self {
        let ports: Vec<PortState> = HTTPS_PORTS
            .iter()
            .map(|&p| PortState {
                port: p,
                active: true,
                score: if p == 443 { 1.0 } else { 0.8 },
                ..Default::default()
            })
            .collect();

        Self {
            ports: RwLock::new(ports),
            current_port: AtomicU16::new(443),
            strategy: std::sync::Mutex::new(PortStrategy::Adaptive),
            hop_interval: AtomicU64::new(DEFAULT_HOP_INTERVAL_SECS),
            last_hop: std::sync::Mutex::new(Instant::now()),
            hop_count: AtomicU64::new(0),
            enabled: std::sync::Mutex::new(true),
        }
    }

    /// دریافت پورت فعلی
    pub fn current_port(&self) -> u16 {
        self.current_port.load(Ordering::Relaxed)
    }

    /// تنظیم استراتژی
    pub async fn set_strategy(&self, strategy: PortStrategy) {
        let mut s = self.strategy.lock().unwrap();
        *s = strategy;
        debug!("📊 Port strategy: {:?}", strategy);
    }

    /// تنظیم فاصله زمانی
    pub fn set_hop_interval(&self, seconds: u64) {
        self.hop_interval.store(seconds, Ordering::Relaxed);
        debug!("⏱️ Hop interval: {}s", seconds);
    }

    /// فعال/غیرفعال
    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut e) = self.enabled.lock() {
            *e = enabled;
        }
    }

    /// بررسی نیاز به تغییر پورت
    pub async fn should_hop(&self) -> bool {
        let enabled = self.enabled.lock().unwrap();
        if !*enabled {
            return false;
        }
        drop(enabled);

        let interval = self.hop_interval.load(Ordering::Relaxed);
        let last = *self.last_hop.lock().unwrap();

        last.elapsed().as_secs() >= interval
    }

    /// تغییر پورت
    pub async fn hop(&self) -> Result<u16> {
        let strategy = *self.strategy.lock().unwrap();
        let new_port = self.select_port(strategy).await?;

        let old_port = self.current_port.swap(new_port, Ordering::Relaxed);

        // به‌روزرسانی زمان
        *self.last_hop.lock().unwrap() = Instant::now();

        // افزایش شمارنده
        self.hop_count.fetch_add(1, Ordering::Relaxed);

        info!("🔄 Port hopped: {} → {} (#{}", 
            old_port, new_port, self.hop_count.load(Ordering::Relaxed));

        Ok(new_port)
    }

    /// انتخاب پورت جدید
    async fn select_port(&self, strategy: PortStrategy) -> Result<u16> {
        let ports = self.ports.read().await;

        if ports.is_empty() {
            return Ok(443);
        }

        let active_ports: Vec<_> = ports.iter().filter(|p| p.active).collect();

        if active_ports.is_empty() {
            return Ok(443);
        }

        let selected = match strategy {
            PortStrategy::Sequential => {
                let current = self.current_port.load(Ordering::Relaxed);
                let current_idx = active_ports.iter().position(|p| p.port == current);
                
                let next_idx = match current_idx {
                    Some(idx) => (idx + 1) % active_ports.len(),
                    None => 0,
                };

                active_ports[next_idx].port
            }

            PortStrategy::Random => {
                let idx = rand::random::<usize>() % active_ports.len();
                active_ports[idx].port
            }

            PortStrategy::Weighted => {
                self.select_weighted(&active_ports)
            }

            PortStrategy::LatencyBased => {
                self.select_by_latency(&active_ports)
            }

            PortStrategy::Adaptive => {
                self.select_adaptive(&active_ports)
            }
        };

        Ok(selected)
    }

    /// انتخاب وزنی
    fn select_weighted(&self, ports: &[&PortState]) -> u16 {
        let total_score: f32 = ports.iter().map(|p| p.score).sum();
        let mut rng = rand::thread_rng();
        let random = rng.gen::<f32>() * total_score;

        let mut cumulative = 0.0;
        for port in ports {
            cumulative += port.score;
            if random < cumulative {
                return port.port;
            }
        }

        ports[0].port
    }

    /// انتخاب بر اساس تأخیر
    fn select_by_latency(&self, ports: &[&PortState]) -> u16 {
        ports
            .iter()
            .min_by(|a, b| {
                a.avg_latency_ms
                    .partial_cmp(&b.avg_latency_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.port)
            .unwrap_or(443)
    }

    /// انتخاب تطبیقی
    fn select_adaptive(&self, ports: &[&PortState]) -> u16 {
        // ترکیب تأخیر و امتیاز
        let current = self.current_port.load(Ordering::Relaxed);

        // اگر پورت فعلی خوب است، تغییر نده
        if let Some(current_port) = ports.iter().find(|p| p.port == current) {
            if current_port.avg_latency_ms < 200.0 && current_port.error_count < 5 {
                return current;
            }
        }

        // در غیر این صورت، بهترین را انتخاب کن
        ports
            .iter()
            .max_by(|a, b| {
                let score_a = a.score * (1.0_f32 / (1.0_f32 + a.avg_latency_ms as f32 / 100.0_f32));
                let score_b = b.score * (1.0_f32 / (1.0_f32 + b.avg_latency_ms as f32 / 100.0_f32));
                score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|p| p.port)
            .unwrap_or(443)
    }

    /// گزارش موفقیت
    pub async fn report_success(&self, port: u16, latency_ms: u64) {
        let mut ports = self.ports.write().await;

        if let Some(p) = ports.iter_mut().find(|p| p.port == port) {
            p.success_count += 1;
            p.avg_latency_ms = (p.avg_latency_ms + latency_ms as f64) / 2.0;
            p.last_used = Some(chrono::Utc::now());

            // به‌روزرسانی امتیاز
            p.score = self.calculate_score(p);

            debug!("✅ Port {} success: {}ms (score: {:.2})", port, latency_ms, p.score);
        }
    }

    /// گزارش خطا
    pub async fn report_error(&self, port: u16) {
        let mut ports = self.ports.write().await;

        if let Some(p) = ports.iter_mut().find(|p| p.port == port) {
            p.error_count += 1;
            p.score = self.calculate_score(p);

            // غیرفعال کردن اگر خطای زیادی دارد
            if p.error_count > 10 && p.success_count < p.error_count / 2 {
                p.active = false;
                warn!("🚫 Port {} disabled due to errors", port);
            }

            debug!("❌ Port {} error (score: {:.2})", port, p.score);
        }
    }

    /// محاسبه امتیاز
    fn calculate_score(&self, port: &PortState) -> f32 {
        if port.success_count == 0 && port.error_count == 0 {
            return 0.5;
        }

        let success_rate = port.success_count as f32 / (port.success_count + port.error_count) as f32;
        let latency_score = if port.avg_latency_ms < 100.0 {
            1.0
        } else if port.avg_latency_ms < 200.0 {
            0.8
        } else if port.avg_latency_ms < 300.0 {
            0.6
        } else {
            0.4
        };

        success_rate * 0.6 + latency_score * 0.4
    }

    /// دریافت وضعیت همه پورت‌ها
    pub async fn get_all_ports(&self) -> Vec<PortState> {
        self.ports.read().await.clone()
    }

    /// دریافت آمار
    pub fn get_stats(&self) -> PortHopperStats {
        PortHopperStats {
            current_port: self.current_port.load(Ordering::Relaxed),
            hop_count: self.hop_count.load(Ordering::Relaxed),
            enabled: *self.enabled.lock().unwrap(),
        }
    }
}

impl Default for PortHopper {
    fn default() -> Self {
        Self::new()
    }
}

/// آمار Port Hopper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortHopperStats {
    /// پورت فعلی
    pub current_port: u16,
    /// تعداد تغییرات
    pub hop_count: u64,
    /// فعال
    pub enabled: bool,
}
