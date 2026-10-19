//! Identifiers, tokens and timestamps.

use std::time::{SystemTime, UNIX_EPOCH};

/// Unix time in milliseconds.
#[must_use]
pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Short random id with a type prefix, e.g. `thr_1a2b3c4d5e6f`.
#[must_use]
pub fn new_id(prefix: &str) -> String {
    format!("{prefix}_{}", &uuid::Uuid::new_v4().simple().to_string()[..12])
}

/// 128-bit random bearer token (hex).
#[must_use]
pub fn new_token() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_prefixed_and_unique() {
        let (a, b) = (new_id("thr"), new_id("thr"));
        assert!(a.starts_with("thr_") && a.len() == 16);
        assert_ne!(a, b);
    }

    #[test]
    fn tokens_are_32_hex_chars() {
        let t = new_token();
        assert_eq!(t.len(), 32);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
