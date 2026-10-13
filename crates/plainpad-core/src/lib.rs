//! Plainpad core — pure Rust, no shell dependencies (shared by desktop, CLI and,
//! later, the Flutter mobile shells via FFI).
//!
//! Storage is plain files: one `.txt` per note, JSON sidecars for state.
//! There is no database anywhere in this crate.

pub mod currency;
pub mod export;
pub mod intent;
pub mod mathwrap;
pub mod model;
pub mod store;
pub mod sync;
pub mod timers;
pub mod trigger_config;
pub mod units;

pub use model::{
    now_iso, title_from_text, Antilink, Attachment, ChecklistItem, Note, NoteMeta, SyncState,
};
pub use trigger_config::{TriggerCategory, TriggerConfig};

use serde::{Deserialize, Serialize};

/// One entry in the recoverable trash (30-day retention).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TrashEntry {
    pub id: String,
    pub title: String,
    pub deleted_at_ms: u64,
}

/// Wall-clock milliseconds since the Unix epoch.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Time-sortable note id: hex ms since epoch plus a tiny time-derived suffix.
pub fn new_note_id(ms: u64) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    let mut salt = (nanos ^ (ms << 7)) % 1_296_000; // 36^4
    let mut suffix = [0u8; 4];
    for c in suffix.iter_mut() {
        const DIGITS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        *c = DIGITS[(salt % 36) as usize];
        salt /= 36;
    }
    format!("{:x}-{}", ms, std::str::from_utf8(&suffix).unwrap_or("zzzz"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique_and_sortable() {
        let ms = now_ms();
        let a = new_note_id(ms);
        let b = new_note_id(ms + 1);
        assert_ne!(a, b);
        let (num, _) = a.split_once('-').unwrap();
        assert!(u64::from_str_radix(num, 16).unwrap() >= ms - 1);
    }

    #[test]
    fn titles_derive_from_first_line() {
        assert_eq!(title_from_text("\n  hello world  \nmore"), "hello world");
        let long = "x".repeat(100);
        let t = title_from_text(&long);
        assert!(t.chars().count() == 58 && t.ends_with('…')); // 57 + ellipsis, under the 60 cap
    }
}
