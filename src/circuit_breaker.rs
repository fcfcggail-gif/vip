//! Circuit Breaker Pattern
//!
//! جلوگیری از تماس‌های مداوم با سرور خراب

use std::{
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant},
};

/// حالت Circuit Breaker
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// بسته
    Closed,
    /// باز
    Open,
    /// نیمه‌باز
    HalfOpen,
}

/// Circuit Breaker
pub struct CircuitBreaker {
    /// حالت
    state: AtomicBool, // false = Closed, true = Open
    /// تعداد خطاها
    failure_count: AtomicU64,
    /// آستانه خطا
    failure_threshold: u32,
    /// زمان آخرین خطا
    last_failure: std::sync::Mutex<Option<Instant>>,
    /// تایم‌اوت
    timeout: Duration,
    /// حداکثر تأخیر مجاز
    max_latency: Duration,
}

impl CircuitBreaker {
    /// ایجاد Circuit Breaker جدید
    pub fn new(timeout: Duration, failure_threshold: u32, max_latency: Duration) -> Self {
        Self {
            state: AtomicBool::new(false),
            failure_count: AtomicU64::new(0),
            failure_threshold,
            last_failure: std::sync::Mutex::new(None),
            timeout,
            max_latency,
        }
    }

    /// بررسی آیا باید trip کند
    pub fn should_trip(&self, latency: u64) -> bool {
        let latency_dur = Duration::from_millis(latency);
        
        if latency_dur > self.max_latency {
            self.record_failure();
        }

        let failures = self.failure_count.load(Ordering::Relaxed);
        failures >= self.failure_threshold as u64
    }

    /// ثبت خطا
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Relaxed);
        
        if let Ok(mut last) = self.last_failure.lock() {
            *last = Some(Instant::now());
        }

        let failures = self.failure_count.load(Ordering::Relaxed);
        if failures >= self.failure_threshold as u64 {
            self.state.store(true, Ordering::Relaxed);
        }
    }

    /// ثبت موفقیت
    pub fn record_success(&self) {
        self.failure_count.store(0, Ordering::Relaxed);
        self.state.store(false, Ordering::Relaxed);
    }

    /// بررسی آیا باز است
    pub fn is_open(&self) -> bool {
        if !self.state.load(Ordering::Relaxed) {
            return false;
        }

        // بررسی آیا زمان Half-Open رسیده
        if let Ok(last) = self.last_failure.lock() {
            if let Some(time) = *last {
                if time.elapsed() > self.timeout {
                    // Half-Open
                    return false;
                }
            }
        }

        true
    }

    /// دریافت حالت
    pub fn get_state(&self) -> CircuitState {
        if self.is_open() {
            CircuitState::Open
        } else if self.failure_count.load(Ordering::Relaxed) > 0 {
            CircuitState::HalfOpen
        } else {
            CircuitState::Closed
        }
    }
}
