/// Proactive Hooks — context-driven suggestions without being asked.
///
/// The assistant-native paradigm means the OS doesn't just respond
/// to commands — it notices things and speaks up. Proactive hooks
/// watch system state and fire when conditions are met, generating
/// suggestions the agent can surface to the user.
///
/// Examples:
/// - High heap usage → "Memory is getting tight, want me to check?"
/// - Many failed intents → "I'm having trouble understanding, try rephrasing?"
/// - Long uptime → "Been running a while, want a status check?"
/// - Config changes → "Config was modified, want me to summarize?"
/// - File system full → "Running low on disk space"
///
/// Hooks are checked periodically by the agent loop and produce
/// Suggestion values that can be injected into the conversation.
///
/// Phase 7, Item 2 — Proactive hooks: context-based suggestions.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::awareness::{Awareness, AwarenessStats, ResourceSnapshot, SubsystemStatus};

/// A proactive suggestion the agent can surface.
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Unique identifier for deduplication.
    pub id: &'static str,
    /// Priority (higher = more important).
    pub priority: u8,
    /// The suggestion message.
    pub message: String,
    /// Category for grouping.
    pub category: SuggestionCategory,
    /// Whether this was already surfaced (prevents repeats).
    pub surfaced: bool,
}

/// Categories of proactive suggestions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuggestionCategory {
    /// System health warnings.
    Health,
    /// Performance observations.
    Performance,
    /// Usage patterns.
    Usage,
    /// Helpful tips.
    Tip,
    /// Maintenance reminders.
    Maintenance,
}

impl SuggestionCategory {
    pub fn emoji(&self) -> &'static str {
        match self {
            Self::Health => "⚠️",
            Self::Performance => "📊",
            Self::Usage => "💡",
            Self::Tip => "✨",
            Self::Maintenance => "🔧",
        }
    }
}

/// A proactive hook — a condition + suggestion generator.
pub struct Hook {
    /// Hook identifier.
    pub id: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Minimum ticks between firings (cooldown).
    pub cooldown: u64,
    /// Last tick this hook fired.
    pub last_fired: u64,
    /// Whether this hook is enabled.
    pub enabled: bool,
    /// The check function.
    checker: fn(&HookContext) -> Option<Suggestion>,
}

/// Context passed to hook checkers.
pub struct HookContext {
    /// Current awareness stats.
    pub stats: AwarenessStats,
    /// Current resource snapshot.
    pub resources: ResourceSnapshot,
    /// Current tick.
    pub tick: u64,
    /// Agent cycles completed.
    pub agent_cycles: u64,
    /// Intents processed.
    pub intents_processed: u64,
}

/// The proactive engine — runs hooks and collects suggestions.
pub struct ProactiveEngine {
    /// Registered hooks.
    hooks: Vec<Hook>,
    /// Pending suggestions (not yet surfaced).
    pending: Vec<Suggestion>,
    /// Maximum pending suggestions.
    max_pending: usize,
}

impl ProactiveEngine {
    /// Create with default hooks.
    pub fn new() -> Self {
        let mut engine = Self {
            hooks: Vec::new(),
            pending: Vec::new(),
            max_pending: 10,
        };

        // Register default hooks
        engine.register(Hook {
            id: "high_heap",
            description: "Warns when heap usage exceeds 80%",
            cooldown: 50,
            last_fired: 0,
            enabled: true,
            checker: check_high_heap,
        });

        engine.register(Hook {
            id: "long_uptime",
            description: "Suggests status check after 1000 ticks",
            cooldown: 500,
            last_fired: 0,
            enabled: true,
            checker: check_long_uptime,
        });

        engine.register(Hook {
            id: "subsystem_degraded",
            description: "Alerts on degraded or failed subsystems",
            cooldown: 20,
            last_fired: 0,
            enabled: true,
            checker: check_subsystem_health,
        });

        engine.register(Hook {
            id: "idle_agent",
            description: "Notices when agent hasn't processed intents",
            cooldown: 100,
            last_fired: 0,
            enabled: true,
            checker: check_idle_agent,
        });

        engine.register(Hook {
            id: "many_files",
            description: "Suggests cleanup when file count is high",
            cooldown: 200,
            last_fired: 0,
            enabled: true,
            checker: check_many_files,
        });

        engine
    }

    /// Register a new hook.
    pub fn register(&mut self, hook: Hook) {
        self.hooks.push(hook);
    }

    /// Run all hooks against current context.
    /// Returns new suggestions generated this tick.
    pub fn tick(&mut self, ctx: &HookContext) -> Vec<Suggestion> {
        let mut new_suggestions = Vec::new();

        for hook in self.hooks.iter_mut() {
            if !hook.enabled {
                continue;
            }

            // Check cooldown
            if ctx.tick - hook.last_fired < hook.cooldown {
                continue;
            }

            // Run the checker
            if let Some(suggestion) = (hook.checker)(ctx) {
                hook.last_fired = ctx.tick;
                new_suggestions.push(suggestion.clone());
                self.pending.push(suggestion);
            }
        }

        // Trim pending
        while self.pending.len() > self.max_pending {
            self.pending.remove(0);
        }

        new_suggestions
    }

    /// Get and clear pending suggestions (for the agent to surface).
    pub fn drain_suggestions(&mut self) -> Vec<Suggestion> {
        let suggestions: Vec<Suggestion> = self.pending.drain(..).collect();
        suggestions
    }

    /// Get count of pending suggestions.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get hook count.
    pub fn hook_count(&self) -> usize {
        self.hooks.len()
    }

    /// Enable/disable a hook by ID.
    pub fn set_hook_enabled(&mut self, id: &str, enabled: bool) -> bool {
        for hook in self.hooks.iter_mut() {
            if hook.id == id {
                hook.enabled = enabled;
                return true;
            }
        }
        false
    }

    /// Generate a summary for the awareness prompt.
    pub fn prompt_block(&self) -> String {
        let mut s = String::from("### Proactive Hooks\n");
        s.push_str(&format!(
            "  {} hooks active, {} pending suggestions\n",
            self.hooks.iter().filter(|h| h.enabled).count(),
            self.pending.len(),
        ));
        for suggestion in &self.pending {
            s.push_str(&format!(
                "  {} {}\n",
                suggestion.category.emoji(),
                suggestion.message,
            ));
        }
        s
    }
}

// ─── Default Hook Checkers ──────────────────────────────────────────────────

fn check_high_heap(ctx: &HookContext) -> Option<Suggestion> {
    if ctx.resources.heap_capacity == 0 {
        return None;
    }
    let usage_pct = (ctx.resources.heap_used * 100) / ctx.resources.heap_capacity;
    if usage_pct >= 80 {
        Some(Suggestion {
            id: "high_heap",
            priority: 8,
            message: format!(
                "Heap usage at {}% ({}/{} bytes). Consider freeing resources.",
                usage_pct, ctx.resources.heap_used, ctx.resources.heap_capacity,
            ),
            category: SuggestionCategory::Health,
            surfaced: false,
        })
    } else {
        None
    }
}

fn check_long_uptime(ctx: &HookContext) -> Option<Suggestion> {
    if ctx.stats.uptime_ticks >= 1000 {
        Some(Suggestion {
            id: "long_uptime",
            priority: 3,
            message: format!(
                "Running for {} ticks. Want a full status check?",
                ctx.stats.uptime_ticks,
            ),
            category: SuggestionCategory::Maintenance,
            surfaced: false,
        })
    } else {
        None
    }
}

fn check_subsystem_health(ctx: &HookContext) -> Option<Suggestion> {
    if !ctx.stats.healthy && ctx.stats.subsystems_total > 0 {
        let degraded = ctx.stats.subsystems_total - ctx.stats.subsystems_online;
        Some(Suggestion {
            id: "subsystem_degraded",
            priority: 9,
            message: format!(
                "{} subsystem(s) not fully online ({}/{} healthy).",
                degraded, ctx.stats.subsystems_online, ctx.stats.subsystems_total,
            ),
            category: SuggestionCategory::Health,
            surfaced: false,
        })
    } else {
        None
    }
}

fn check_idle_agent(ctx: &HookContext) -> Option<Suggestion> {
    if ctx.agent_cycles > 10 && ctx.intents_processed == 0 {
        Some(Suggestion {
            id: "idle_agent",
            priority: 2,
            message: String::from(
                "Agent has been running but no intents processed. Try asking me to do something!",
            ),
            category: SuggestionCategory::Tip,
            surfaced: false,
        })
    } else {
        None
    }
}

fn check_many_files(ctx: &HookContext) -> Option<Suggestion> {
    if ctx.resources.file_count >= 50 {
        Some(Suggestion {
            id: "many_files",
            priority: 4,
            message: format!(
                "{} files in the filesystem. Want me to list them for cleanup?",
                ctx.resources.file_count,
            ),
            category: SuggestionCategory::Maintenance,
            surfaced: false,
        })
    } else {
        None
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx(tick: u64) -> HookContext {
        HookContext {
            stats: AwarenessStats {
                version: "0.7.0",
                uptime_ticks: tick,
                subsystems_online: 5,
                subsystems_total: 5,
                event_count: 0,
                healthy: true,
            },
            resources: ResourceSnapshot {
                heap_used: 1000,
                heap_capacity: 10000,
                active_tasks: 1,
                file_count: 3,
                config_entries: 5,
                agent_cycles: 5,
                intents_processed: 3,
            },
            tick,
            agent_cycles: 5,
            intents_processed: 3,
        }
    }

    #[test_case]
    fn test_engine_new() {
        let engine = ProactiveEngine::new();
        assert_eq!(engine.hook_count(), 5);
        assert_eq!(engine.pending_count(), 0);
    }

    #[test_case]
    fn test_no_suggestions_normal() {
        let mut engine = ProactiveEngine::new();
        let ctx = test_ctx(10);
        let suggestions = engine.tick(&ctx);
        assert_eq!(suggestions.len(), 0);
    }

    #[test_case]
    fn test_high_heap_fires() {
        let mut engine = ProactiveEngine::new();
        let mut ctx = test_ctx(100);
        ctx.resources.heap_used = 9000; // 90%
        let suggestions = engine.tick(&ctx);
        assert!(suggestions.iter().any(|s| s.id == "high_heap"));
    }

    #[test_case]
    fn test_long_uptime_fires() {
        let mut engine = ProactiveEngine::new();
        let ctx = test_ctx(1500);
        let suggestions = engine.tick(&ctx);
        assert!(suggestions.iter().any(|s| s.id == "long_uptime"));
    }

    #[test_case]
    fn test_cooldown_prevents_repeat() {
        let mut engine = ProactiveEngine::new();
        let mut ctx = test_ctx(100);
        ctx.resources.heap_used = 9000;
        engine.tick(&ctx); // fires
        let suggestions = engine.tick(&ctx); // cooldown blocks
        assert_eq!(suggestions.len(), 0);
    }

    #[test_case]
    fn test_drain_clears_pending() {
        let mut engine = ProactiveEngine::new();
        let mut ctx = test_ctx(1500);
        ctx.resources.heap_used = 9000;
        engine.tick(&ctx);
        assert!(engine.pending_count() > 0);
        let drained = engine.drain_suggestions();
        assert!(!drained.is_empty());
        assert_eq!(engine.pending_count(), 0);
    }

    #[test_case]
    fn test_disable_hook() {
        let mut engine = ProactiveEngine::new();
        engine.set_hook_enabled("high_heap", false);
        let mut ctx = test_ctx(100);
        ctx.resources.heap_used = 9000;
        let suggestions = engine.tick(&ctx);
        assert!(!suggestions.iter().any(|s| s.id == "high_heap"));
    }
}
