//! Network Ghost v5.0 - Smart Packet Padding & Obfuscation
//! لایه‌ی محافظتی پیشرفته برای دور زدن آنالیز سایز پکت (DPI)

use rand::{Rng, RngCore, thread_rng};
use tracing::debug;

/// الگوی Padding پیشرفته
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaddingPattern {
    Random,        // کاملاً تصادفی
    Fixed,         // مضرب‌های ثابت (مثل ۱۶ بایت)
    TlsSimulated,  // شبیه‌سازی دقیق TLS
    Adaptive,      // تطبیقی بر اساس حجم دیتا
    GhostUltra,    // 👻 حالت ویژه Ghost برای ایران (فوق امن)
}

/// موتور هوشمند پدینگ
pub struct PacketPadding {
    enabled: bool,
    min_size: usize,
    max_size: usize,
    pattern: PaddingPattern,
}

impl Default for PacketPadding {
    fn default() -> Self {
        Self {
            enabled: true,
            min_size: 16,
            max_size: 1460,
            pattern: PaddingPattern::GhostUltra, // پیش‌فرض روی حالت هوشمند
        }
    }
}

impl PacketPadding {
    pub fn new() -> Self {
        Self::default()
    }

    /// اضافه کردن پدینگ به صورت خودکار و هوشمند
    pub fn apply_smart(&self, buffer: &mut Vec<u8>) {
        if !self.enabled {
            return;
        }

        let original_size = buffer.len();
        let target_size = self.calculate_target_size(original_size);
        
        if target_size > original_size {
            let padding_size = target_size - original_size;
            let padding = self.generate_padding(padding_size);
            buffer.extend_from_slice(&padding);
            
            debug!("🛡️ Ghost Padding applied: {} -> {} bytes", original_size, buffer.len());
        }
    }

    /// محاسبه اندازه هدف با متدهای پیشرفته
    fn calculate_target_size(&self, current: usize) -> usize {
        let mut rng = thread_rng();
        
        match self.pattern {
            PaddingPattern::GhostUltra => {
                // 🚀 ترکیب متد تطبیقی و تصادفی برای خنثی کردن AI
                let base_padding = if current < 500 {
                    rng.gen_range(64..256)
                } else {
                    rng.gen_range(16..128)
                };
                (current + base_padding).min(self.max_size)
            }
            PaddingPattern::TlsSimulated => {
                self.simulate_tls_size(current)
            }
            PaddingPattern::Random => {
                rng.gen_range(current..=self.max_size)
            }
            PaddingPattern::Fixed => {
                let remainder = current % 16;
                if remainder == 0 { current } else { current + 16 - remainder }
            }
            PaddingPattern::Adaptive => {
                if current < 128 { (current + 128).min(self.max_size) } 
                else { (current + 64).min(self.max_size) }
            }
        }
    }

    /// شبیه‌سازی اثر انگشت پکت‌های TLS واقعی
    fn simulate_tls_size(&self, current: usize) -> usize {
        let common_sizes = [64, 128, 512, 1024, 1280, 1448, 1460];
        let mut closest = 1460;
        
        for &size in &common_sizes {
            if size > current {
                closest = size;
                break;
            }
        }
        
        let jitter = thread_rng().gen_range(0..12);
        (closest + jitter).min(self.max_size)
    }

    /// تولید دیتای زباله (Junk Data) با آنتروپی بالا
    fn generate_padding(&self, size: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        let mut padding = vec![0u8; size];
        
        // ایجاد آنتروپی بالا برای عبور از فیلترهای حساس به فشرده‌سازی
        rng.fill_bytes(&mut padding);
        padding
    }
}

/// 👻 لایه‌ی استاتیک برای فراخوانی سریع (GhostPadding)
pub struct GhostPadding;

impl GhostPadding {
    pub fn apply(buffer: &mut Vec<u8>) {
        let protector = PacketPadding::new();
        protector.apply_smart(buffer);
    }
}
