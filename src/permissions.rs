/// Permission Model — governs what the assistant can do autonomously.
///
/// In an assistant-native OS, the agent has real power: it can
/// read/write files, change config, control the display, and
/// eventually interact with hardware. That power needs boundaries.
///
/// The permission model defines:
/// - Action categories (file, config, display, network, system)
/// - Permission levels (Allowed, AskOnce, AskAlways, Denied)
/// - Policy evaluation: given an action, can the agent proceed?
/// - Audit trail: every permission check is logged
///
/// This isn't security theater — it's the contract between the
/// assistant and the user. The agent can do anything it's permitted
/// to do, and it knows what it can't do without asking.
///
/// Phase 7, Item 1 — Permission model for assistant actions.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

/// Categories of actions the assistant can take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    /// Reading files from the filesystem.
    FileRead,
    /// Writing/creating/deleting files.
    FileWrite,
    /// Reading configuration values.
    ConfigRead,
    /// Changing configuration values.
    ConfigWrite,
    /// Manipulating the display (colors, banners).
    Display,
    /// Making network requests (HTTP, DNS).
    Network,
    /// System operations (reboot, shutdown).
    System,
    /// Executing arbitrary shell commands.
    Execute,
}

impl ActionCategory {
    /// Human-readable name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::FileRead => "File Read",
            Self::FileWrite => "File Write",
            Self::ConfigRead => "Config Read",
            Self::ConfigWrite => "Config Write",
            Self::Display => "Display Control",
            Self::Network => "Network Access",
            Self::System => "System Operations",
            Self::Execute => "Command Execution",
        }
    }

    /// Risk level (for display and default policy).
    pub fn risk(&self) -> RiskLevel {
        match self {
            Self::FileRead | Self::ConfigRead | Self::Display => RiskLevel::Low,
            Self::FileWrite | Self::ConfigWrite | Self::Network => RiskLevel::Medium,
            Self::System | Self::Execute => RiskLevel::High,
        }
    }
}

/// Risk levels for categorizing actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

impl RiskLevel {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Low => "🟢",
            Self::Medium => "🟡",
            Self::High => "🔴",
        }
    }
}

/// What happens when the agent tries an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionLevel {
    /// Action is always allowed — no prompt.
    Allowed,
    /// Ask the user once, then remember the answer.
    AskOnce,
    /// Ask the user every time.
    AskAlways,
    /// Action is always denied.
    Denied,
}

/// A permission policy entry.
#[derive(Debug, Clone)]
pub struct PolicyEntry {
    /// The action category.
    pub category: ActionCategory,
    /// The permission level.
    pub level: PermissionLevel,
    /// Optional path/resource pattern (e.g., "/config.toml" for FileWrite).
    pub resource: Option<String>,
}

/// Result of a permission check.
#[derive(Debug, Clone)]
pub struct PermissionCheck {
    /// Whether the action is allowed right now.
    pub allowed: bool,
    /// Whether user confirmation is needed.
    pub needs_confirmation: bool,
    /// The category checked.
    pub category: ActionCategory,
    /// The resource involved (if any).
    pub resource: Option<String>,
    /// Reason for the decision.
    pub reason: String,
}

/// Audit log entry for permission checks.
#[derive(Debug, Clone)]
pub struct AuditEntry {
    /// Tick when the check occurred.
    pub tick: u64,
    /// The action category.
    pub category: ActionCategory,
    /// The resource involved.
    pub resource: Option<String>,
    /// Whether it was allowed.
    pub allowed: bool,
    /// The decision reason.
    pub reason: String,
}

/// The permission manager.
pub struct PermissionManager {
    /// Policy entries (checked in order, first match wins).
    policies: Vec<PolicyEntry>,
    /// Remembered one-time decisions.
    remembered: Vec<(ActionCategory, Option<String>, bool)>,
    /// Audit trail.
    audit_log: Vec<AuditEntry>,
    /// Maximum audit entries.
    max_audit: usize,
    /// Current tick (for audit timestamps).
    current_tick: u64,
}

impl PermissionManager {
    /// Create with sensible defaults.
    ///
    /// Default policy:
    /// - Low risk (file read, config read, display): Allowed
    /// - Medium risk (file write, config write, network): AskOnce
    /// - High risk (system, execute): AskAlways
    pub fn with_defaults() -> Self {
        let mut mgr = Self {
            policies: Vec::new(),
            remembered: Vec::new(),
            audit_log: Vec::new(),
            max_audit: 200,
            current_tick: 0,
        };

        // Default policies by risk level
        mgr.add_policy(ActionCategory::FileRead, PermissionLevel::Allowed, None);
        mgr.add_policy(ActionCategory::ConfigRead, PermissionLevel::Allowed, None);
        mgr.add_policy(ActionCategory::Display, PermissionLevel::Allowed, None);
        mgr.add_policy(ActionCategory::FileWrite, PermissionLevel::AskOnce, None);
        mgr.add_policy(ActionCategory::ConfigWrite, PermissionLevel::AskOnce, None);
        mgr.add_policy(ActionCategory::Network, PermissionLevel::AskOnce, None);
        mgr.add_policy(ActionCategory::System, PermissionLevel::AskAlways, None);
        mgr.add_policy(ActionCategory::Execute, PermissionLevel::AskAlways, None);

        // Special: protect /config.toml from deletion
        mgr.add_policy(
            ActionCategory::FileWrite,
            PermissionLevel::AskAlways,
            Some(String::from("/config.toml")),
        );

        mgr
    }

    /// Add a policy entry. Resource-specific policies should be added
    /// before category-wide ones (first match wins).
    pub fn add_policy(
        &mut self,
        category: ActionCategory,
        level: PermissionLevel,
        resource: Option<String>,
    ) {
        // Resource-specific goes to front, general to back
        let entry = PolicyEntry {
            category,
            level,
            resource,
        };
        if entry.resource.is_some() {
            self.policies.insert(0, entry);
        } else {
            self.policies.push(entry);
        }
    }

    /// Check whether an action is permitted.
    pub fn check(
        &mut self,
        category: ActionCategory,
        resource: Option<&str>,
    ) -> PermissionCheck {
        // Find matching policy (first match wins)
        let policy = self.policies.iter().find(|p| {
            p.category == category
                && match (&p.resource, resource) {
                    (Some(pattern), Some(res)) => res == pattern.as_str(),
                    (None, _) => true,
                    _ => false,
                }
        });

        let (allowed, needs_confirmation, reason) = match policy {
            Some(p) => match p.level {
                PermissionLevel::Allowed => (true, false, String::from("Policy: allowed")),
                PermissionLevel::Denied => (false, false, String::from("Policy: denied")),
                PermissionLevel::AskOnce => {
                    // Check if we already have a remembered decision
                    let res_str = resource.map(String::from);
                    if let Some((_, _, decision)) = self.remembered.iter().find(|(c, r, _)| {
                        *c == category && *r == res_str
                    }) {
                        (*decision, false, String::from("Remembered decision"))
                    } else {
                        (false, true, String::from("Needs one-time confirmation"))
                    }
                }
                PermissionLevel::AskAlways => {
                    (false, true, String::from("Always requires confirmation"))
                }
            },
            None => {
                // No policy found — default by risk level
                let risk = category.risk();
                match risk {
                    RiskLevel::Low => (true, false, String::from("Default: low risk")),
                    RiskLevel::Medium => (false, true, String::from("Default: medium risk")),
                    RiskLevel::High => (false, true, String::from("Default: high risk")),
                }
            }
        };

        // Log the check
        let entry = AuditEntry {
            tick: self.current_tick,
            category,
            resource: resource.map(String::from),
            allowed,
            reason: reason.clone(),
        };
        self.audit_log.push(entry);
        while self.audit_log.len() > self.max_audit {
            self.audit_log.remove(0);
        }

        PermissionCheck {
            allowed,
            needs_confirmation,
            category,
            resource: resource.map(String::from),
            reason,
        }
    }

    /// Record a user's decision for AskOnce permissions.
    pub fn remember_decision(
        &mut self,
        category: ActionCategory,
        resource: Option<String>,
        allowed: bool,
    ) {
        self.remembered.push((category, resource, allowed));
    }

    /// Set the current tick for audit timestamps.
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Get audit log stats.
    pub fn stats(&self) -> PermissionStats {
        let allowed = self.audit_log.iter().filter(|e| e.allowed).count();
        let denied = self.audit_log.len() - allowed;
        PermissionStats {
            total_checks: self.audit_log.len(),
            allowed,
            denied,
            policies: self.policies.len(),
            remembered: self.remembered.len(),
        }
    }

    /// Generate a summary for the awareness system prompt.
    pub fn prompt_block(&self) -> String {
        let mut s = String::from("### Permissions\n");
        for policy in &self.policies {
            let res = policy
                .resource
                .as_deref()
                .unwrap_or("*");
            s.push_str(&format!(
                "  {} {} [{}]: {:?}\n",
                policy.category.risk().symbol(),
                policy.category.name(),
                res,
                policy.level,
            ));
        }
        let stats = self.stats();
        s.push_str(&format!(
            "  Checks: {} ({} allowed, {} denied)\n",
            stats.total_checks, stats.allowed, stats.denied,
        ));
        s
    }
}

/// Permission statistics.
#[derive(Debug)]
pub struct PermissionStats {
    pub total_checks: usize,
    pub allowed: usize,
    pub denied: usize,
    pub policies: usize,
    pub remembered: usize,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_default_policies() {
        let mut mgr = PermissionManager::with_defaults();
        // Low risk — auto-allowed
        let check = mgr.check(ActionCategory::FileRead, None);
        assert!(check.allowed);
        assert!(!check.needs_confirmation);
    }

    #[test_case]
    fn test_medium_risk_asks() {
        let mut mgr = PermissionManager::with_defaults();
        let check = mgr.check(ActionCategory::FileWrite, Some("/test.txt"));
        assert!(!check.allowed);
        assert!(check.needs_confirmation);
    }

    #[test_case]
    fn test_high_risk_always_asks() {
        let mut mgr = PermissionManager::with_defaults();
        let check = mgr.check(ActionCategory::System, None);
        assert!(!check.allowed);
        assert!(check.needs_confirmation);
    }

    #[test_case]
    fn test_remember_decision() {
        let mut mgr = PermissionManager::with_defaults();
        // First check — needs confirmation
        let check1 = mgr.check(ActionCategory::FileWrite, Some("/test.txt"));
        assert!(check1.needs_confirmation);

        // Remember "yes"
        mgr.remember_decision(ActionCategory::FileWrite, Some(String::from("/test.txt")), true);

        // Second check — remembered
        let check2 = mgr.check(ActionCategory::FileWrite, Some("/test.txt"));
        assert!(check2.allowed);
        assert!(!check2.needs_confirmation);
    }

    #[test_case]
    fn test_resource_specific_override() {
        let mut mgr = PermissionManager::with_defaults();
        // config.toml always asks, even though FileWrite is AskOnce
        let check = mgr.check(ActionCategory::FileWrite, Some("/config.toml"));
        assert!(check.needs_confirmation);
        assert!(check.reason.contains("confirmation"));
    }

    #[test_case]
    fn test_audit_trail() {
        let mut mgr = PermissionManager::with_defaults();
        mgr.check(ActionCategory::FileRead, None);
        mgr.check(ActionCategory::System, None);
        let stats = mgr.stats();
        assert_eq!(stats.total_checks, 2);
        assert_eq!(stats.allowed, 1);
        assert_eq!(stats.denied, 1);
    }

    #[test_case]
    fn test_prompt_block() {
        let mgr = PermissionManager::with_defaults();
        let block = mgr.prompt_block();
        assert!(block.contains("Permissions"));
        assert!(block.contains("File Read"));
        assert!(block.contains("System Operations"));
    }
}
