/// OS-Level Awareness — the kernel's self-knowledge system.
///
/// This module gives the agent deep awareness of the operating
/// system's state — not just conversation context, but system
/// context. The assistant-native paradigm means the kernel doesn't
/// just run conversations; it understands itself.
///
/// Awareness covers:
/// - Subsystem status (which components are initialized)
/// - Resource tracking (memory usage, uptime, task count)
/// - Environment facts (boot time, kernel version, capabilities)
/// - Session history (commands run, intents processed)
///
/// This feeds into Claude's system prompt so the assistant can
/// answer "how much memory am I using?" or "when did I boot?"
/// without guessing.
///
/// Phase 7, Item 0 — Context system: OS-level awareness.
/// Phase 7 (Assistant-Native Paradigm) begins!

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

/// Kernel subsystem identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subsystem {
    VgaDisplay,
    Interrupts,
    Memory,
    Heap,
    Keyboard,
    Async,
    Network,
    Dns,
    Tls,
    Http,
    Llm,
    FileSystem,
    Config,
    Shell,
    ChatView,
    IntentParser,
    Executor,
    DisplayControl,
    AgentLoop,
}

impl Subsystem {
    /// Human-readable name for system prompt injection.
    pub fn name(&self) -> &'static str {
        match self {
            Self::VgaDisplay => "VGA Display",
            Self::Interrupts => "Interrupts",
            Self::Memory => "Memory Management",
            Self::Heap => "Heap Allocator",
            Self::Keyboard => "Keyboard Input",
            Self::Async => "Async Runtime",
            Self::Network => "Networking (virtio-net)",
            Self::Dns => "DNS Resolver",
            Self::Tls => "TLS 1.3",
            Self::Http => "HTTP Client",
            Self::Llm => "Claude LLM",
            Self::FileSystem => "FAT32 Filesystem",
            Self::Config => "Config Persistence",
            Self::Shell => "NL Shell",
            Self::ChatView => "Chat View",
            Self::IntentParser => "Intent Parser",
            Self::Executor => "Intent Executor",
            Self::DisplayControl => "Display Control",
            Self::AgentLoop => "Agent Loop",
        }
    }
}

/// Status of a subsystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsystemStatus {
    /// Not yet initialized.
    Offline,
    /// Initializing.
    Booting,
    /// Running normally.
    Online,
    /// Running with degraded functionality.
    Degraded,
    /// Failed to initialize or crashed.
    Failed,
}

impl SubsystemStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Offline => "⬛",
            Self::Booting => "🟡",
            Self::Online => "🟢",
            Self::Degraded => "🟠",
            Self::Failed => "🔴",
        }
    }
}

/// A snapshot of kernel resource usage.
#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    /// Heap bytes allocated.
    pub heap_used: usize,
    /// Heap capacity.
    pub heap_capacity: usize,
    /// Number of active async tasks.
    pub active_tasks: usize,
    /// Files in the filesystem.
    pub file_count: usize,
    /// Config entries loaded.
    pub config_entries: usize,
    /// Agent cycles completed.
    pub agent_cycles: u64,
    /// Intents processed.
    pub intents_processed: u64,
}

/// The kernel's awareness state — everything it knows about itself.
pub struct Awareness {
    /// Boot timestamp (ticks since start).
    boot_tick: u64,
    /// Current tick counter.
    current_tick: u64,
    /// Kernel version string.
    version: &'static str,
    /// Subsystem status map.
    subsystems: Vec<(Subsystem, SubsystemStatus)>,
    /// Latest resource snapshot.
    resources: ResourceSnapshot,
    /// Session event log (recent events only).
    event_log: Vec<AwarenessEvent>,
    /// Maximum events to retain.
    max_events: usize,
}

/// An event in the kernel's awareness log.
#[derive(Debug, Clone)]
pub struct AwarenessEvent {
    /// Tick when event occurred.
    pub tick: u64,
    /// Event category.
    pub kind: EventKind,
    /// Human-readable description.
    pub description: String,
}

/// Categories of awareness events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Boot,
    SubsystemChange,
    IntentExecuted,
    FileOperation,
    ConfigChange,
    Error,
    UserInteraction,
}

impl Awareness {
    /// Create a new awareness state at boot.
    pub fn new(version: &'static str) -> Self {
        let mut awareness = Self {
            boot_tick: 0,
            current_tick: 0,
            version,
            subsystems: Vec::new(),
            resources: ResourceSnapshot {
                heap_used: 0,
                heap_capacity: 0,
                active_tasks: 0,
                file_count: 0,
                config_entries: 0,
                agent_cycles: 0,
                intents_processed: 0,
            },
            event_log: Vec::new(),
            max_events: 100,
        };

        awareness.log_event(EventKind::Boot, "Kernel booted — awareness system online");
        awareness
    }

    /// Register a subsystem with its current status.
    pub fn register_subsystem(&mut self, subsystem: Subsystem, status: SubsystemStatus) {
        // Update existing or add new
        for (s, st) in self.subsystems.iter_mut() {
            if *s == subsystem {
                let old = *st;
                *st = status;
                if old != status {
                    self.log_event(
                        EventKind::SubsystemChange,
                        &format!("{}: {} → {}", subsystem.name(), old.symbol(), status.symbol()),
                    );
                }
                return;
            }
        }
        self.subsystems.push((subsystem, status));
        self.log_event(
            EventKind::SubsystemChange,
            &format!("{} registered: {}", subsystem.name(), status.symbol()),
        );
    }

    /// Update the resource snapshot.
    pub fn update_resources(&mut self, resources: ResourceSnapshot) {
        self.resources = resources;
    }

    /// Advance the tick counter.
    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    /// Log an awareness event.
    pub fn log_event(&mut self, kind: EventKind, description: &str) {
        self.event_log.push(AwarenessEvent {
            tick: self.current_tick,
            kind,
            description: String::from(description),
        });

        // Trim old events
        while self.event_log.len() > self.max_events {
            self.event_log.remove(0);
        }
    }

    /// Generate the OS awareness block for Claude's system prompt.
    /// This is injected so the assistant knows its own state.
    pub fn system_prompt_block(&self) -> String {
        let mut prompt = String::from("## Kernel Awareness\n\n");

        // Version + uptime
        let uptime = self.current_tick - self.boot_tick;
        prompt.push_str(&format!("**Version:** {}\n", self.version));
        prompt.push_str(&format!("**Uptime:** {} ticks\n\n", uptime));

        // Subsystem status
        prompt.push_str("### Subsystems\n");
        let online = self.subsystems.iter().filter(|(_, s)| *s == SubsystemStatus::Online).count();
        let total = self.subsystems.len();
        prompt.push_str(&format!("{}/{} online\n", online, total));

        for (subsystem, status) in &self.subsystems {
            prompt.push_str(&format!("  {} {}\n", status.symbol(), subsystem.name()));
        }

        // Resources
        prompt.push_str("\n### Resources\n");
        let heap_pct = if self.resources.heap_capacity > 0 {
            (self.resources.heap_used * 100) / self.resources.heap_capacity
        } else {
            0
        };
        prompt.push_str(&format!(
            "  Heap: {}/{} bytes ({}%)\n",
            self.resources.heap_used, self.resources.heap_capacity, heap_pct
        ));
        prompt.push_str(&format!("  Tasks: {}\n", self.resources.active_tasks));
        prompt.push_str(&format!("  Files: {}\n", self.resources.file_count));
        prompt.push_str(&format!("  Config entries: {}\n", self.resources.config_entries));
        prompt.push_str(&format!("  Agent cycles: {}\n", self.resources.agent_cycles));
        prompt.push_str(&format!("  Intents processed: {}\n", self.resources.intents_processed));

        // Recent events
        let recent: Vec<_> = self.event_log.iter().rev().take(5).collect();
        if !recent.is_empty() {
            prompt.push_str("\n### Recent Events\n");
            for event in recent.iter().rev() {
                prompt.push_str(&format!("  [tick {}] {}\n", event.tick, event.description));
            }
        }

        prompt
    }

    /// Get subsystem status.
    pub fn subsystem_status(&self, subsystem: Subsystem) -> SubsystemStatus {
        self.subsystems
            .iter()
            .find(|(s, _)| *s == subsystem)
            .map(|(_, st)| *st)
            .unwrap_or(SubsystemStatus::Offline)
    }

    /// Get overall health: all subsystems online?
    pub fn is_healthy(&self) -> bool {
        self.subsystems.iter().all(|(_, s)| *s == SubsystemStatus::Online)
    }

    /// Get stats summary.
    pub fn stats(&self) -> AwarenessStats {
        let online = self.subsystems.iter().filter(|(_, s)| *s == SubsystemStatus::Online).count();
        AwarenessStats {
            version: self.version,
            uptime_ticks: self.current_tick - self.boot_tick,
            subsystems_online: online,
            subsystems_total: self.subsystems.len(),
            event_count: self.event_log.len(),
            healthy: self.is_healthy(),
        }
    }
}

/// Summary stats for quick access.
#[derive(Debug)]
pub struct AwarenessStats {
    pub version: &'static str,
    pub uptime_ticks: u64,
    pub subsystems_online: usize,
    pub subsystems_total: usize,
    pub event_count: usize,
    pub healthy: bool,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_awareness_new() {
        let aw = Awareness::new("0.7.0");
        assert_eq!(aw.version, "0.7.0");
        assert_eq!(aw.event_log.len(), 1); // Boot event
        assert_eq!(aw.subsystems.len(), 0);
    }

    #[test_case]
    fn test_register_subsystem() {
        let mut aw = Awareness::new("0.7.0");
        aw.register_subsystem(Subsystem::VgaDisplay, SubsystemStatus::Online);
        assert_eq!(aw.subsystem_status(Subsystem::VgaDisplay), SubsystemStatus::Online);
        assert_eq!(aw.subsystems.len(), 1);
    }

    #[test_case]
    fn test_subsystem_transition() {
        let mut aw = Awareness::new("0.7.0");
        aw.register_subsystem(Subsystem::Network, SubsystemStatus::Booting);
        aw.register_subsystem(Subsystem::Network, SubsystemStatus::Online);
        assert_eq!(aw.subsystem_status(Subsystem::Network), SubsystemStatus::Online);
        // Boot + register + transition = 3 events
        assert_eq!(aw.event_log.len(), 3);
    }

    #[test_case]
    fn test_health_check() {
        let mut aw = Awareness::new("0.7.0");
        assert!(aw.is_healthy()); // No subsystems = vacuously healthy
        aw.register_subsystem(Subsystem::Llm, SubsystemStatus::Online);
        assert!(aw.is_healthy());
        aw.register_subsystem(Subsystem::Network, SubsystemStatus::Failed);
        assert!(!aw.is_healthy());
    }

    #[test_case]
    fn test_system_prompt_block() {
        let mut aw = Awareness::new("0.7.0");
        aw.register_subsystem(Subsystem::VgaDisplay, SubsystemStatus::Online);
        aw.register_subsystem(Subsystem::Llm, SubsystemStatus::Online);
        let prompt = aw.system_prompt_block();
        assert!(prompt.contains("0.7.0"));
        assert!(prompt.contains("VGA Display"));
        assert!(prompt.contains("Claude LLM"));
        assert!(prompt.contains("2/2 online"));
    }

    #[test_case]
    fn test_tick_and_uptime() {
        let mut aw = Awareness::new("0.7.0");
        aw.tick();
        aw.tick();
        aw.tick();
        let stats = aw.stats();
        assert_eq!(stats.uptime_ticks, 3);
    }

    #[test_case]
    fn test_event_trimming() {
        let mut aw = Awareness::new("0.7.0");
        aw.max_events = 5;
        for i in 0..10 {
            aw.log_event(EventKind::UserInteraction, &format!("event {}", i));
        }
        assert_eq!(aw.event_log.len(), 5);
    }
}
