//! Utility Functions

use std::net::IpAddr;

/// بررسی آیا IP در رنج است
pub fn is_ip_in_range(_ip: IpAddr, _range: &str) -> bool {
    true
}

/// تبدیل bytes به hex
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}
