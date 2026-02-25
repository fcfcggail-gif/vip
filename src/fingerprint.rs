//! uTLS Fingerprint Rotation
//!
//! تغییر خودکار Fingerprint برای جلوگیری از تشخیص

use std::collections::HashMap;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

/// Fingerprint Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FingerprintType {
    /// Chrome
    Chrome,
    /// Firefox
    Firefox,
    /// Safari
    Safari,
    /// Edge
    Edge,
    /// iOS
    Ios,
    /// Android
    Android,
}

impl Default for FingerprintType {
    fn default() -> Self {
        Self::Chrome
    }
}

impl std::fmt::Display for FingerprintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FingerprintType::Chrome => write!(f, "chrome"),
            FingerprintType::Firefox => write!(f, "firefox"),
            FingerprintType::Safari => write!(f, "safari"),
            FingerprintType::Edge => write!(f, "edge"),
            FingerprintType::Ios => write!(f, "ios"),
            FingerprintType::Android => write!(f, "android"),
        }
    }
}

/// Fingerprint Data
#[derive(Debug, Clone)]
pub struct Fingerprint {
    /// Type
    pub fp_type: FingerprintType,
    /// Client Hello pattern
    pub client_hello_pattern: Vec<u8>,
    /// Cipher suites
    pub cipher_suites: Vec<u16>,
    /// Extensions order
    pub extensions_order: Vec<u16>,
    /// User Agent
    pub user_agent: String,
    /// Weight for selection
    pub weight: f32,
}

/// Fingerprint Manager
pub struct FingerprintManager {
    /// همه fingerprints
    fingerprints: HashMap<FingerprintType, Fingerprint>,
    /// Fingerprint فعلی
    current: FingerprintType,
    /// Counter برای rotation
    counter: u64,
    /// Interval برای rotation
    rotation_interval: u64,
}

impl FingerprintManager {
    /// ایجاد Manager جدید
    pub fn new() -> Self {
        let mut fingerprints = HashMap::new();

        // Chrome
        fingerprints.insert(
            FingerprintType::Chrome,
            Fingerprint {
                fp_type: FingerprintType::Chrome,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0x1301, 0x1302, 0x1303, // TLS 1.3
                    0xC02F, 0xC030, 0xC02B, 0xC02C, // TLS 1.2
                ],
                extensions_order: vec![
                    0x0000, // server_name
                    0x000B, // ec_point_formats
                    0x000A, // supported_groups
                    0x000D, // signature_algorithms
                    0x0017, // extended_master_secret
                    0xFF01, // renegotiation_info
                    0x0005, // status_request
                    0x002B, // supported_versions
                    0x0012, // signed_certificate_timestamp
                    0x7550, // extension_delegated_credentials
                    0x0015, // padding
                    0x0000, // key_share
                ],
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                weight: 0.4,
            },
        );

        // Firefox
        fingerprints.insert(
            FingerprintType::Firefox,
            Fingerprint {
                fp_type: FingerprintType::Firefox,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0x1301, 0x1303, 0x1302,
                    0xC02B, 0xC02F, 0xC02C, 0xC030,
                ],
                extensions_order: vec![
                    0x0000, 0x000B, 0x000A, 0x000D,
                    0x0017, 0xFF01, 0x002B, 0x0005,
                ],
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/119.0".to_string(),
                weight: 0.2,
            },
        );

        // Safari
        fingerprints.insert(
            FingerprintType::Safari,
            Fingerprint {
                fp_type: FingerprintType::Safari,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0xC02F, 0xC030, 0x1301, 0x1302, 0x1303,
                ],
                extensions_order: vec![
                    0x0000, 0x000B, 0x000A, 0x000D,
                    0x0017, 0xFF01, 0x002B, 0x0005,
                ],
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15".to_string(),
                weight: 0.15,
            },
        );

        // Edge
        fingerprints.insert(
            FingerprintType::Edge,
            Fingerprint {
                fp_type: FingerprintType::Edge,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0x1301, 0x1302, 0x1303,
                    0xC02F, 0xC030,
                ],
                extensions_order: vec![
                    0x0000, 0x000B, 0x000A, 0x000D,
                    0x0017, 0xFF01, 0x002B,
                ],
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/119.0.0.0 Safari/537.36 Edg/119.0.0.0".to_string(),
                weight: 0.1,
            },
        );

        // iOS
        fingerprints.insert(
            FingerprintType::Ios,
            Fingerprint {
                fp_type: FingerprintType::Ios,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0xC02F, 0xC030, 0x1301, 0x1302, 0x1303,
                ],
                extensions_order: vec![
                    0x0000, 0x000B, 0x000A, 0x000D,
                    0x0017, 0xFF01, 0x002B,
                ],
                user_agent: "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15".to_string(),
                weight: 0.1,
            },
        );

        // Android
        fingerprints.insert(
            FingerprintType::Android,
            Fingerprint {
                fp_type: FingerprintType::Android,
                client_hello_pattern: vec![0x16, 0x03, 0x01],
                cipher_suites: vec![
                    0x1301, 0x1302, 0x1303,
                    0xC02F, 0xC030,
                ],
                extensions_order: vec![
                    0x0000, 0x000B, 0x000A, 0x000D,
                    0x0017, 0xFF01, 0x002B,
                ],
                user_agent: "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/119.0.0.0 Mobile".to_string(),
                weight: 0.05,
            },
        );

        Self {
            fingerprints,
            current: FingerprintType::Chrome,
            counter: 0,
            rotation_interval: 100,
        }
    }

    /// دریافت Fingerprint فعلی
    pub fn current(&self) -> &Fingerprint {
        self.fingerprints.get(&self.current).unwrap()
    }

    /// دریافت Fingerprint با نوع خاص
    pub fn get(&self, fp_type: FingerprintType) -> Option<&Fingerprint> {
        self.fingerprints.get(&fp_type)
    }

    /// Rotation خودکار
    pub fn rotate(&mut self) -> FingerprintType {
        self.counter += 1;

        if self.counter >= self.rotation_interval {
            self.counter = 0;
            let new_fp = self.select_weighted_random();
            self.current = new_fp;
            info!("🔄 Fingerprint rotated to: {}", new_fp);
        }

        self.current
    }

    /// انتخاب وزنی تصادفی
    fn select_weighted_random(&self) -> FingerprintType {
        use rand::thread_rng;
        
        let total_weight: f32 = self.fingerprints.values().map(|f| f.weight).sum();
        let mut rng = thread_rng();
        let random = rng.gen::<f32>() * total_weight;

        let mut cumulative = 0.0;
        for (fp_type, fp) in &self.fingerprints {
            cumulative += fp.weight;
            if random < cumulative {
                return *fp_type;
            }
        }

        FingerprintType::Chrome
    }

    /// تنظیم دستی
    pub fn set(&mut self, fp_type: FingerprintType) {
        self.current = fp_type;
        debug!("🎯 Fingerprint set to: {}", fp_type);
    }

    /// تنظیم Interval
    pub fn set_rotation_interval(&mut self, interval: u64) {
        self.rotation_interval = interval;
    }

    /// دریافت Cipher Suites
    pub fn get_cipher_suites(&self) -> Vec<u8> {
        let fp = self.current();
        let mut bytes = Vec::new();

        bytes.push(0x00);
        bytes.push((fp.cipher_suites.len() * 2) as u8);

        for suite in &fp.cipher_suites {
            bytes.extend_from_slice(&suite.to_be_bytes());
        }

        bytes
    }

    /// دریافت User Agent
    pub fn get_user_agent(&self) -> &str {
        &self.current().user_agent
    }
}

impl Default for FingerprintManager {
    fn default() -> Self {
        Self::new()
    }
}
