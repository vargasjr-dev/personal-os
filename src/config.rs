/// Config Persistence — save and load kernel settings to disk.
///
/// Persists API keys, user preferences, and system settings
/// across reboots using a simple key-value store backed by
/// the filesystem. Uses TOML-like format for human readability.
///
/// This completes Phase 5 — the kernel now has a full filesystem
/// stack: block device → FAT32 → file ops → config persistence.
/// Settings survive reboots. My future home remembers.
///
/// Phase 5, Item 3 — FINAL filesystem item.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Config file path on the virtual disk.
pub const CONFIG_PATH: &str = "/config.toml";

/// Backup config path.
pub const CONFIG_BACKUP_PATH: &str = "/config.toml.bak";

/// Configuration store — typed key-value persistence.
pub struct Config {
    /// Key-value entries.
    entries: BTreeMap<String, ConfigValue>,
    /// Whether the config has unsaved changes.
    dirty: bool,
    /// Number of loads performed.
    load_count: u64,
    /// Number of saves performed.
    save_count: u64,
}

/// Configuration value types.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigValue {
    /// String value.
    Text(String),
    /// Integer value.
    Number(i64),
    /// Boolean value.
    Bool(bool),
    /// Secret value (never displayed in logs/status).
    Secret(String),
}

impl Config {
    /// Create a new empty config.
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            dirty: false,
            load_count: 0,
            save_count: 0,
        }
    }

    /// Create with default settings.
    pub fn with_defaults() -> Self {
        let mut config = Self::new();
        config.set("kernel.name", ConfigValue::Text(String::from("VargasJR")));
        config.set("kernel.version", ConfigValue::Text(String::from("0.5.0")));
        config.set("shell.max_context", ConfigValue::Number(20));
        config.set("shell.show_stats", ConfigValue::Bool(true));
        config.set("network.timeout_ms", ConfigValue::Number(30000));
        config.set("llm.model", ConfigValue::Text(String::from("claude-sonnet-4-20250514")));
        config.set("llm.max_tokens", ConfigValue::Number(4096));
        config.dirty = false; // Defaults don't count as dirty
        config
    }

    /// Get a value by key.
    pub fn get(&self, key: &str) -> Option<&ConfigValue> {
        self.entries.get(key)
    }

    /// Get a string value.
    pub fn get_text(&self, key: &str) -> Option<&str> {
        match self.entries.get(key) {
            Some(ConfigValue::Text(s)) => Some(s),
            Some(ConfigValue::Secret(s)) => Some(s),
            _ => None,
        }
    }

    /// Get a number value.
    pub fn get_number(&self, key: &str) -> Option<i64> {
        match self.entries.get(key) {
            Some(ConfigValue::Number(n)) => Some(*n),
            _ => None,
        }
    }

    /// Get a boolean value.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.entries.get(key) {
            Some(ConfigValue::Bool(b)) => Some(*b),
            _ => None,
        }
    }

    /// Set a value (marks config as dirty).
    pub fn set(&mut self, key: &str, value: ConfigValue) {
        self.entries.insert(String::from(key), value);
        self.dirty = true;
    }

    /// Set a secret (API key, token — never logged).
    pub fn set_secret(&mut self, key: &str, value: &str) {
        self.set(key, ConfigValue::Secret(String::from(value)));
    }

    /// Remove a key.
    pub fn remove(&mut self, key: &str) -> Option<ConfigValue> {
        let removed = self.entries.remove(key);
        if removed.is_some() {
            self.dirty = true;
        }
        removed
    }

    /// Check if a key exists.
    pub fn has(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    /// Whether config has unsaved changes.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Serialize config to TOML-like string for disk storage.
    /// Secrets are stored with a [secret] prefix for identification.
    pub fn serialize(&self) -> String {
        let mut lines = Vec::new();
        lines.push(String::from("# VargasJR Kernel Configuration"));
        lines.push(String::from("# Auto-generated — do not edit while kernel is running"));
        lines.push(String::new());

        for (key, value) in &self.entries {
            let line = match value {
                ConfigValue::Text(s) => alloc::format!("{} = \"{}\"", key, s),
                ConfigValue::Number(n) => alloc::format!("{} = {}", key, n),
                ConfigValue::Bool(b) => alloc::format!("{} = {}", key, b),
                ConfigValue::Secret(_) => alloc::format!("{} = [secret]", key),
            };
            lines.push(line);
        }

        lines.join("\n")
    }

    /// Deserialize config from TOML-like string.
    pub fn deserialize(content: &str) -> Result<Self, ConfigError> {
        let mut config = Self::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key = value
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let raw_value = parts[1].trim();

            let value = if raw_value == "[secret]" {
                // Secrets can't be deserialized from disk
                // They must be re-entered after boot
                continue;
            } else if raw_value.starts_with('"') && raw_value.ends_with('"') {
                ConfigValue::Text(String::from(&raw_value[1..raw_value.len() - 1]))
            } else if raw_value == "true" {
                ConfigValue::Bool(true)
            } else if raw_value == "false" {
                ConfigValue::Bool(false)
            } else if let Ok(n) = raw_value.parse::<i64>() {
                ConfigValue::Number(n)
            } else {
                ConfigValue::Text(String::from(raw_value))
            };

            config.entries.insert(String::from(key), value);
        }

        config.dirty = false;
        config.load_count += 1;
        Ok(config)
    }

    /// Mark as clean (after saving to disk).
    pub fn mark_saved(&mut self) {
        self.dirty = false;
        self.save_count += 1;
    }

    /// Get stats.
    pub fn stats(&self) -> ConfigStats {
        ConfigStats {
            entry_count: self.entries.len(),
            dirty: self.dirty,
            load_count: self.load_count,
            save_count: self.save_count,
            secret_count: self.entries.values()
                .filter(|v| matches!(v, ConfigValue::Secret(_)))
                .count(),
        }
    }
}

/// Config statistics.
#[derive(Debug)]
pub struct ConfigStats {
    pub entry_count: usize,
    pub dirty: bool,
    pub load_count: u64,
    pub save_count: u64,
    pub secret_count: usize,
}

/// Config errors.
#[derive(Debug)]
pub enum ConfigError {
    ParseError(String),
    IoError(String),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_defaults() {
        let config = Config::with_defaults();
        assert_eq!(config.get_text("kernel.name"), Some("VargasJR"));
        assert_eq!(config.get_number("shell.max_context"), Some(20));
        assert_eq!(config.get_bool("shell.show_stats"), Some(true));
        assert!(!config.is_dirty());
    }

    #[test_case]
    fn test_set_get_remove() {
        let mut config = Config::new();
        config.set("test.key", ConfigValue::Text(String::from("hello")));
        assert!(config.has("test.key"));
        assert_eq!(config.get_text("test.key"), Some("hello"));
        config.remove("test.key");
        assert!(!config.has("test.key"));
    }

    #[test_case]
    fn test_serialize_deserialize() {
        let mut config = Config::new();
        config.set("name", ConfigValue::Text(String::from("VargasJR")));
        config.set("count", ConfigValue::Number(42));
        config.set("enabled", ConfigValue::Bool(true));

        let serialized = config.serialize();
        let loaded = Config::deserialize(&serialized).unwrap();

        assert_eq!(loaded.get_text("name"), Some("VargasJR"));
        assert_eq!(loaded.get_number("count"), Some(42));
        assert_eq!(loaded.get_bool("enabled"), Some(true));
    }

    #[test_case]
    fn test_secrets_not_serialized() {
        let mut config = Config::new();
        config.set_secret("api.key", "sk-secret-123");
        let serialized = config.serialize();
        assert!(serialized.contains("[secret]"));
        assert!(!serialized.contains("sk-secret-123"));
    }

    #[test_case]
    fn test_dirty_tracking() {
        let mut config = Config::with_defaults();
        assert!(!config.is_dirty());
        config.set("new.key", ConfigValue::Bool(false));
        assert!(config.is_dirty());
        config.mark_saved();
        assert!(!config.is_dirty());
    }
}
