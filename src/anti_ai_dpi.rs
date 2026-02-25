//! سیستم ضد هوش مصنوعی DPI ایران - نسخه Ultra-Ghost v5.0
//!
//! تمامی قابلیت‌های قبلی حفظ شده و لایه‌های جدید فریب AI اضافه شده است.
//! ## تکنیک‌ها:
//! - Traffic Pattern Randomization
//! - Packet Padding with Entropy Manipulation
//! - Human-like Timing Jitter
//! - ML Model Evasion
//! - TCP Smart Fragmentation (جدید)
//! - Fake TLS Decoy Traffic (جدید)
//! - Social Media Traffic Simulation (جدید)
//! - Protocol Signature Rotation / Ghost Mode (جدید)

use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const COMMON_PACKET_SIZES: &[usize] = &[64, 128, 256, 512, 1024, 1280, 1400, 1448, 1500];
const MIN_INTER_PACKET_DELAY_MS: u64 = 5;
const MAX_INTER_PACKET_DELAY_MS: u64 = 50;
const STATS_WINDOW_MS: u64 = 5000;
const HISTORY_CAPACITY: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntiAiMode {
    Normal,
    Aggressive,
    Stealth,
    Adaptive,
    Ghost,
}

impl Default for AntiAiMode {
    fn default() -> Self { Self::Adaptive }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrafficProfile {
    WebBrowsing,
    VideoStreaming,
    FileDownload,
    ApiCalls,
    Gaming,
    SocialMedia,
    VoiceCall,
}

impl Default for TrafficProfile {
    fn default() -> Self { Self::WebBrowsing }
}

pub struct AntiAiDpi {
    mode: std::sync::Mutex<AntiAiMode>,
    profile: std::sync::Mutex<TrafficProfile>,
    stats: std::sync::Mutex<DpiStats>,
    packet_size_history: std::sync::Mutex<VecDeque<usize>>,
    rng: std::sync::Mutex<ChaCha20Rng>,
    packet_counter: AtomicU64,
    start_time: Instant,
    signature_rotation_counter: AtomicU64,
}

impl AntiAiDpi {
    pub fn new() -> Self {
        info!("🛡️ Anti-AI DPI Ghost v5.0 فعال شد");
        Self {
            mode: std::sync::Mutex::new(AntiAiMode::Adaptive),
            profile: std::sync::Mutex::new(TrafficProfile::WebBrowsing),
            stats: std::sync::Mutex::new(DpiStats::default()),
            packet_size_history: std::sync::Mutex::new(VecDeque::with_capacity(HISTORY_CAPACITY)),
            rng: std::sync::Mutex::new(ChaCha20Rng::from_entropy()),
            packet_counter: AtomicU64::new(0),
            start_time: Instant::now(),
            signature_rotation_counter: AtomicU64::new(0),
        }
    }

    pub fn set_mode(&self, mode: AntiAiMode) {
        if let Ok(mut m) = self.mode.lock() {
            info!("🔄 تغییر حالت Anti-AI: {:?}", mode);
            *m = mode;
        }
    }

    pub fn set_profile(&self, profile: TrafficProfile) {
        if let Ok(mut p) = self.profile.lock() {
            *p = profile;
        }
    }

    pub fn current_mode(&self) -> AntiAiMode {
        *self.mode.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn stats(&self) -> DpiStats {
        self.stats.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// اضافه کردن Padding با فریب آنتروپی (همه حالت‌ها پشتیبانی می‌شوند)
    pub fn add_random_padding(&self, data: &[u8]) -> Vec<u8> {
        let mode = *self.mode.lock().unwrap_or_else(|e| e.into_inner());
        let profile = *self.profile.lock().unwrap_or_else(|e| e.into_inner());

        let padding_size = match mode {
            AntiAiMode::Normal     => self.calculate_normal_padding(data.len()),
            AntiAiMode::Aggressive => self.calculate_aggressive_padding(data.len()),
            AntiAiMode::Stealth    => self.calculate_stealth_padding(data.len()),
            AntiAiMode::Adaptive   => self.calculate_adaptive_padding(data.len(), &profile),
            AntiAiMode::Ghost      => self.calculate_ghost_padding(data.len()),
        };

        let mut result = Vec::with_capacity(data.len() + padding_size);
        result.extend_from_slice(data);
        result.extend(self.generate_mimic_padding(padding_size));
        self.record_packet(result.len());
        result
    }

    /// شبیه‌سازی زمان‌بندی (Human-like Jitter)
    pub fn calculate_inter_packet_delay(&self) -> Duration {
        let mode = *self.mode.lock().unwrap_or_else(|e| e.into_inner());
        let profile = *self.profile.lock().unwrap_or_else(|e| e.into_inner());
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());

        let ms = match mode {
            AntiAiMode::Stealth | AntiAiMode::Ghost => {
                self.simulate_network_jitter(&mut rng, &profile)
            }
            AntiAiMode::Adaptive => {
                let counter = self.packet_counter.load(Ordering::Relaxed);
                if counter % 5 == 0 {
                    self.simulate_network_jitter(&mut rng, &profile)
                } else {
                    rng.gen_range(MIN_INTER_PACKET_DELAY_MS..MAX_INTER_PACKET_DELAY_MS)
                }
            }
            _ => rng.gen_range(MIN_INTER_PACKET_DELAY_MS..MAX_INTER_PACKET_DELAY_MS),
        };
        Duration::from_millis(ms)
    }

    /// تطبیق خودکار حالت بر اساس شرایط شبکه
    pub fn auto_adapt(&self, latency_ms: u64, packet_loss: f32) {
        let new_mode = if packet_loss > 0.05 || latency_ms > 500 {
            AntiAiMode::Stealth
        } else if packet_loss > 0.02 || latency_ms > 300 {
            AntiAiMode::Adaptive
        } else {
            AntiAiMode::Ghost
        };
        self.set_mode(new_mode);
    }

    /// تکه‌تکه کردن هوشمند پکت (Smart TCP Fragmentation) - جدید
    /// DPI نمی‌تواند در پکت اول امضای پروتکل را تشخیص دهد
    pub fn smart_fragment(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());
        let mut fragments = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let max_chunk = (data.len() - offset).min(140);
            let chunk_size = if max_chunk > 20 { rng.gen_range(20..max_chunk) } else { max_chunk };
            fragments.push(data[offset..offset + chunk_size].to_vec());
            offset += chunk_size;
        }
        fragments
    }

    /// تولید ترافیک TLS Client Hello فریبنده (Decoy) - جدید
    pub fn generate_fake_tls_traffic(&self) -> Vec<u8> {
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());
        let mut packet = vec![0x16, 0x03, 0x03]; // TLS 1.2 Record Header
        let body_len = rng.gen_range(150u16..300u16);
        packet.extend(body_len.to_be_bytes());
        packet.push(0x01); // Handshake Type: Client Hello
        let mut body = vec![0u8; body_len as usize];
        rng.fill_bytes(&mut body);
        if body_len > 40 {
            body[35] = rng.gen_range(0u8..32u8); // Session ID Length
        }
        packet.extend(body);
        packet
    }

    /// تولید Initial Packet فریبنده QUIC/HTTP3 - جدید
    pub fn generate_fake_quic_initial(&self) -> Vec<u8> {
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());
        let mut packet = Vec::with_capacity(1200);
        packet.push(0xc0);                              // Long Header
        packet.extend([0x00, 0x00, 0x00, 0x01]);       // QUIC v1
        packet.push(8u8);                               // DCIL
        let mut dcid = [0u8; 8]; rng.fill_bytes(&mut dcid);
        packet.extend_from_slice(&dcid);
        packet.push(8u8);                               // SCIL
        let mut scid = [0u8; 8]; rng.fill_bytes(&mut scid);
        packet.extend_from_slice(&scid);
        packet.push(0x00);                              // Token Length = 0
        let remaining = 1200usize.saturating_sub(packet.len());
        let mut payload = vec![0u8; remaining];
        rng.fill_bytes(&mut payload);
        packet.extend(payload);
        packet
    }

    /// تولید STUN Binding Request برای شبیه‌سازی WebRTC - جدید
    pub fn generate_stun_binding_request(&self) -> Vec<u8> {
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());
        let mut packet = Vec::with_capacity(28);
        packet.extend([0x00, 0x01]);                    // Binding Request
        packet.extend([0x00, 0x00]);                    // Length = 0
        packet.extend([0x21, 0x12, 0xa4, 0x42]);        // Magic Cookie
        let mut tid = [0u8; 12]; rng.fill_bytes(&mut tid);
        packet.extend_from_slice(&tid);
        packet
    }

    /// پیش‌پردازش کامل داده خروجی (ترکیب همه تکنیک‌ها) - جدید
    pub fn preprocess_outgoing(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mode = *self.mode.lock().unwrap_or_else(|e| e.into_inner());
        match mode {
            AntiAiMode::Ghost => {
                let fragments = self.smart_fragment(data);
                fragments.into_iter().map(|f| self.add_random_padding(&f)).collect()
            }
            AntiAiMode::Aggressive => vec![self.add_random_padding(data)],
            AntiAiMode::Stealth    => self.smart_fragment(data),
            _                      => vec![self.add_random_padding(data)],
        }
    }

    /// چرخش پروفایل بر اساس ساعت روز (Temporal Fingerprinting Defense) - جدید
    pub fn rotate_profile_by_time(&self) {
        use chrono::Timelike;
        let hour = chrono::Local::now().hour();
        let new_profile = match hour {
            6..=9   => TrafficProfile::SocialMedia,
            10..=12 => TrafficProfile::ApiCalls,
            13..=14 => TrafficProfile::WebBrowsing,
            15..=18 => TrafficProfile::ApiCalls,
            19..=22 => TrafficProfile::VideoStreaming,
            _       => TrafficProfile::Gaming,
        };
        self.set_profile(new_profile);
        debug!("⏰ پروفایل ساعت {}: {:?}", hour, new_profile);
    }

    /// چرخش امضای پروتکل در Ghost mode - جدید
    pub fn rotate_ghost_signature(&self) {
        let count = self.signature_rotation_counter.fetch_add(1, Ordering::Relaxed);
        debug!("👻 Ghost چرخش امضا #{}", count + 1);
    }

    // ==================== توابع کمکی داخلی ====================

    fn generate_mimic_padding(&self, size: usize) -> Vec<u8> {
        if size == 0 { return vec![]; }
        let mut rng = self.rng.lock().unwrap_or_else(|e| e.into_inner());
        let mut padding = vec![0u8; size];
        for byte in padding.iter_mut() {
            *byte = if rng.gen_bool(0.4) { rng.gen_range(32u8..126u8) } else { rng.gen() };
        }
        padding
    }

    fn simulate_network_jitter(&self, rng: &mut ChaCha20Rng, profile: &TrafficProfile) -> u64 {
        match profile {
            TrafficProfile::SocialMedia => {
                if rng.gen_bool(0.3) { rng.gen_range(5..20) } else { rng.gen_range(80..200) }
            }
            TrafficProfile::VideoStreaming => rng.gen_range(8..25),
            TrafficProfile::Gaming        => rng.gen_range(2..15),
            TrafficProfile::VoiceCall     => 20 + rng.gen_range(0..5),
            TrafficProfile::FileDownload  => rng.gen_range(3..12),
            _                             => rng.gen_range(10..60),
        }
    }

    fn calculate_normal_padding(&self, len: usize) -> usize {
        let rem = len % 16;
        if rem == 0 { 16 } else { 16 - rem }
    }

    fn calculate_aggressive_padding(&self, _len: usize) -> usize {
        self.rng.lock().unwrap_or_else(|e| e.into_inner()).gen_range(100..500)
    }

    fn calculate_stealth_padding(&self, len: usize) -> usize {
        let rem = len % 1448;
        if rem == 0 { 0 } else { 1448 - rem }
    }

    fn calculate_adaptive_padding(&self, len: usize, profile: &TrafficProfile) -> usize {
        match profile {
            TrafficProfile::VideoStreaming | TrafficProfile::FileDownload => self.calculate_normal_padding(len),
            TrafficProfile::Gaming | TrafficProfile::VoiceCall => 0,
            _ => self.calculate_stealth_padding(len),
        }
    }

    fn calculate_ghost_padding(&self, _len: usize) -> usize {
        self.rng.lock().unwrap_or_else(|e| e.into_inner()).gen_range(200..1000)
    }

    fn record_packet(&self, size: usize) {
        self.packet_counter.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut stats) = self.stats.lock() {
            stats.packets_processed += 1;
            stats.bytes_processed += size as u64;
        }
        if let Ok(mut history) = self.packet_size_history.lock() {
            if history.len() >= HISTORY_CAPACITY { history.pop_front(); }
            history.push_back(size);
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DpiStats {
    pub packets_processed: u64,
    pub bytes_processed: u64,
    pub signature_rotations: u64,
    pub fragmentations: u64,
}

impl Default for AntiAiDpi {
    fn default() -> Self { Self::new() }
}

// ─── Detection Analysis (used by engine.rs monitoring loop) ─────────────────

/// نتیجه تحلیل ریسک تشخیص AI
#[derive(Debug, Clone, Default)]
pub struct DetectionAnalysis {
    pub is_detected: bool,
    pub confidence: f32,
    pub detection_reason: Option<String>,
    pub recommended_mode: Option<AntiAiMode>,
}

impl AntiAiDpi {
    /// تحلیل ریسک تشخیص توسط AI-DPI
    pub fn analyze_detection_risk(&self) -> DetectionAnalysis {
        let stats = self.stats();
        let mode = self.current_mode();

        // اگر تعداد پکت‌های پردازش‌شده کم است، ریسک پایین
        if stats.packets_processed < 100 {
            return DetectionAnalysis::default();
        }

        // محاسبه میانگین اندازه پکت
        let history = self.packet_size_history.lock()
            .unwrap_or_else(|e| e.into_inner());
        let avg_size: f64 = if history.is_empty() {
            0.0
        } else {
            history.iter().map(|&s| s as f64).sum::<f64>() / history.len() as f64
        };

        // اگر اندازه‌ها بیش از حد یکنواخت باشند، ریسک بالا
        let variance: f64 = if history.len() > 1 {
            let mean = avg_size;
            history.iter().map(|&s| {
                let diff = s as f64 - mean;
                diff * diff
            }).sum::<f64>() / (history.len() - 1) as f64
        } else { 1000.0 };
        drop(history);

        let is_detected = variance < 100.0 && mode != AntiAiMode::Ghost;
        let confidence = if is_detected {
            (100.0 - variance.sqrt()).max(0.0).min(100.0) as f32 / 100.0
        } else { 0.1 };

        DetectionAnalysis {
            is_detected,
            confidence,
            detection_reason: if is_detected {
                Some(format!("Low packet size variance: {:.1}", variance.sqrt()))
            } else { None },
            recommended_mode: if is_detected { Some(AntiAiMode::Ghost) } else { None },
        }
    }

    /// تطبیق با نتیجه تشخیص (switch به Ghost mode اگر detected)
    pub fn adapt_to_detection(&self, analysis: &DetectionAnalysis) {
        if analysis.is_detected {
            if let Some(mode) = analysis.recommended_mode {
                self.set_mode(mode);
                tracing::warn!(
                    "🚨 AI detection risk {:.0}% → switching to {:?}",
                    analysis.confidence * 100.0, mode
                );
            }
        }
    }
}
