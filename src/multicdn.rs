//! Multi-CDN Failover

use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

use super::{CdnType, ScanResult};

/// وضعیت CDN
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnStatus {
    /// نوع CDN
    pub cdn: CdnType,
    /// فعال
    pub active: bool,
    /// تعداد IPهای سالم
    pub healthy_ips: usize,
    /// میانگین تأخیر
    pub avg_latency_ms: f64,
    /// امتیاز
    pub score: f32,
}

impl Default for CdnStatus {
    fn default() -> Self {
        Self {
            cdn: CdnType::Cloudflare,
            active: true,
            healthy_ips: 0,
            avg_latency_ms: 0.0,
            score: 1.0,
        }
    }
}

/// مدیر Multi-CDN
pub struct MultiCdnManager {
    /// وضعیت CDNها
    statuses: Arc<Mutex<Vec<CdnStatus>>>,
    /// CDN فعال
    active_cdn: Arc<Mutex<CdnType>>,
}

impl MultiCdnManager {
    /// ایجاد Manager جدید
    pub fn new() -> Self {
        let statuses = vec![
            CdnStatus { cdn: CdnType::Cloudflare, active: true, score: 1.0, ..Default::default() },
            CdnStatus { cdn: CdnType::Gcore, active: true, score: 0.8, ..Default::default() },
            CdnStatus { cdn: CdnType::Fastly, active: true, score: 0.7, ..Default::default() },
        ];

        Self {
            statuses: Arc::new(Mutex::new(statuses)),
            active_cdn: Arc::new(Mutex::new(CdnType::Cloudflare)),
        }
    }

    /// به‌روزرسانی وضعیت
    pub async fn update_from_scan(&self, results: &[ScanResult]) {
        let mut statuses = self.statuses.lock().await;

        for result in results {
            if let Some(status) = statuses.iter_mut().find(|s| s.cdn == result.cdn_type) {
                if result.is_clean {
                    status.healthy_ips += 1;
                }
            }
        }

        debug!("📊 CDN statuses updated");
    }

    /// دریافت بهترین CDN
    pub async fn get_best_cdn(&self) -> CdnType {
        let statuses = self.statuses.lock().await;

        statuses
            .iter()
            .filter(|s| s.active)
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
            .map(|s| s.cdn)
            .unwrap_or(CdnType::Cloudflare)
    }

    /// دریافت CDN فعال
    pub async fn get_active_cdn(&self) -> CdnType {
        *self.active_cdn.lock().await
    }
}

impl Default for MultiCdnManager {
    fn default() -> Self {
        Self::new()
    }
}
