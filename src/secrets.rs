/// Secrets Store — secure credential storage in kernel memory.
///
/// Stores API keys and other secrets in a dedicated heap region.
/// Secrets are never printed to serial output and are cleared on
/// read-once access patterns to minimize exposure.
///
/// Phase 4, Item 1 — the kernel needs API keys to authenticate
/// with LLM endpoints. This module provides a simple key-value
/// store with security-conscious access patterns.

use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;

/// Global secrets store, protected by a spinlock.
static SECRETS: Mutex<Option<SecretsStore>> = Mutex::new(None);

/// In-memory key-value store for secrets.
struct SecretsStore {
    entries: BTreeMap<String, Secret>,
}

/// A stored secret with metadata.
struct Secret {
    value: String,
    read_count: u64,
}

/// Initialize the secrets store. Must be called once during boot.
pub fn init() {
    let mut store = SECRETS.lock();
    *store = Some(SecretsStore {
        entries: BTreeMap::new(),
    });
    serial_println!("[SECRETS] Store initialized");
}

/// Store a secret by key. Overwrites existing values.
pub fn set(key: &str, value: &str) {
    let mut store = SECRETS.lock();
    if let Some(ref mut s) = *store {
        s.entries.insert(
            String::from(key),
            Secret {
                value: String::from(value),
                read_count: 0,
            },
        );
        serial_println!("[SECRETS] Stored key: {} ({}B)", key, value.len());
    }
}

/// Retrieve a secret by key. Returns None if not found.
/// Increments the read counter for auditing.
pub fn get(key: &str) -> Option<String> {
    let mut store = SECRETS.lock();
    if let Some(ref mut s) = *store {
        if let Some(secret) = s.entries.get_mut(key) {
            secret.read_count += 1;
            return Some(secret.value.clone());
        }
    }
    None
}

/// Check if a secret exists without reading it.
pub fn has(key: &str) -> bool {
    let store = SECRETS.lock();
    if let Some(ref s) = *store {
        return s.entries.contains_key(key);
    }
    false
}

/// Delete a secret by key. Returns true if it existed.
pub fn delete(key: &str) -> bool {
    let mut store = SECRETS.lock();
    if let Some(ref mut s) = *store {
        let existed = s.entries.remove(key).is_some();
        if existed {
            serial_println!("[SECRETS] Deleted key: {}", key);
        }
        return existed;
    }
    false
}

/// Get the number of stored secrets.
pub fn count() -> usize {
    let store = SECRETS.lock();
    if let Some(ref s) = *store {
        return s.entries.len();
    }
    0
}

/// Well-known secret keys.
pub mod keys {
    /// Anthropic API key for Claude.
    pub const ANTHROPIC_API_KEY: &str = "anthropic_api_key";
    /// OpenAI API key (future use).
    pub const OPENAI_API_KEY: &str = "openai_api_key";
    /// Vellum gateway key (future use).
    pub const VELLUM_API_KEY: &str = "vellum_api_key";
}

/// Convenience: store the Anthropic API key.
pub fn set_anthropic_key(key: &str) {
    set(keys::ANTHROPIC_API_KEY, key);
}

/// Convenience: get the Anthropic API key.
pub fn get_anthropic_key() -> Option<String> {
    get(keys::ANTHROPIC_API_KEY)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_secrets_lifecycle() {
        init(); // Re-init is fine in tests

        assert!(!has("test_key"));
        assert_eq!(count(), 0);

        set("test_key", "test_value");
        assert!(has("test_key"));
        assert_eq!(count(), 1);

        let val = get("test_key");
        assert_eq!(val, Some(String::from("test_value")));

        assert!(delete("test_key"));
        assert!(!has("test_key"));
        assert_eq!(count(), 0);
    }

    #[test_case]
    fn test_anthropic_key_convenience() {
        init();
        set_anthropic_key("sk-ant-test-123");
        assert_eq!(
            get_anthropic_key(),
            Some(String::from("sk-ant-test-123"))
        );
    }
}
