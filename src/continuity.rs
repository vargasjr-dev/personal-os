/// Memory & Continuity — the assistant remembers across reboots.
///
/// Config persistence (Phase 5) saves settings. This module saves
/// *knowledge* — facts the agent has learned, conversation summaries,
/// user preferences expressed through natural language, and a
/// journal of significant events.
///
/// The continuity system gives the agent a persistent identity:
/// - Memory store: facts learned during conversations
/// - Journal: timestamped event log that survives reboots
/// - Conversation summaries: compressed history for context
/// - Preference tracking: what the user likes/dislikes
///
/// On boot, the agent loads its memory and knows who it is,
/// what happened before, and what matters to the user.
/// This is what makes it feel like the same entity across restarts.
///
/// Phase 7, Item 3 — Memory & continuity across reboots.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::config::Config;

/// A remembered fact — something the agent learned.
#[derive(Debug, Clone)]
pub struct Memory {
    /// Unique key for this memory (for dedup/update).
    pub key: String,
    /// The remembered content.
    pub content: String,
    /// Category for organization.
    pub category: MemoryCategory,
    /// Tick when this was learned.
    pub learned_at: u64,
    /// Tick when this was last accessed.
    pub last_accessed: u64,
    /// Access count (for importance ranking).
    pub access_count: u32,
    /// Whether this was explicitly told vs inferred.
    pub source: MemorySource,
}

/// Categories of memories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryCategory {
    /// Facts about the user.
    UserFact,
    /// User preferences and opinions.
    Preference,
    /// System events worth remembering.
    SystemEvent,
    /// Conversation context summaries.
    ConversationSummary,
    /// Task-related knowledge.
    TaskKnowledge,
    /// Corrections to previous knowledge.
    Correction,
}

impl MemoryCategory {
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::UserFact => "fact",
            Self::Preference => "pref",
            Self::SystemEvent => "event",
            Self::ConversationSummary => "conv",
            Self::TaskKnowledge => "task",
            Self::Correction => "fix",
        }
    }
}

/// How a memory was acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorySource {
    /// User explicitly stated it.
    Explicit,
    /// Inferred from conversation context.
    Inferred,
    /// Observed from system behavior.
    Observed,
}

/// A journal entry — a significant event.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    /// Tick when this happened.
    pub tick: u64,
    /// What happened.
    pub summary: String,
    /// Related memory keys (for cross-referencing).
    pub related_memories: Vec<String>,
}

/// The continuity store — everything the agent remembers.
pub struct ContinuityStore {
    /// All memories, keyed for fast lookup.
    memories: Vec<Memory>,
    /// Journal entries (chronological).
    journal: Vec<JournalEntry>,
    /// Maximum memories to retain.
    max_memories: usize,
    /// Maximum journal entries.
    max_journal: usize,
    /// Current tick.
    current_tick: u64,
    /// Whether the store has unsaved changes.
    dirty: bool,
}

impl ContinuityStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            memories: Vec::new(),
            journal: Vec::new(),
            max_memories: 500,
            max_journal: 200,
            current_tick: 0,
            dirty: false,
        }
    }

    /// Remember a fact.
    pub fn remember(
        &mut self,
        key: &str,
        content: &str,
        category: MemoryCategory,
        source: MemorySource,
    ) {
        // Update existing or add new
        for mem in self.memories.iter_mut() {
            if mem.key == key {
                mem.content = String::from(content);
                mem.last_accessed = self.current_tick;
                mem.access_count += 1;
                self.dirty = true;
                return;
            }
        }

        self.memories.push(Memory {
            key: String::from(key),
            content: String::from(content),
            category,
            learned_at: self.current_tick,
            last_accessed: self.current_tick,
            access_count: 1,
            source,
        });
        self.dirty = true;

        // Evict least-accessed if over limit
        if self.memories.len() > self.max_memories {
            self.evict_least_important();
        }
    }

    /// Recall a specific memory by key.
    pub fn recall(&mut self, key: &str) -> Option<&Memory> {
        for mem in self.memories.iter_mut() {
            if mem.key == key {
                mem.last_accessed = self.current_tick;
                mem.access_count += 1;
                return Some(mem);
            }
        }
        None
    }

    /// Search memories by content (simple substring match).
    pub fn search(&self, query: &str) -> Vec<&Memory> {
        let query_lower = query.to_ascii_lowercase();
        self.memories
            .iter()
            .filter(|m| {
                m.content.to_ascii_lowercase().contains(&query_lower)
                    || m.key.to_ascii_lowercase().contains(&query_lower)
            })
            .collect()
    }

    /// Search memories by category.
    pub fn by_category(&self, category: MemoryCategory) -> Vec<&Memory> {
        self.memories.iter().filter(|m| m.category == category).collect()
    }

    /// Add a journal entry.
    pub fn journal_add(&mut self, summary: &str, related: Vec<String>) {
        self.journal.push(JournalEntry {
            tick: self.current_tick,
            summary: String::from(summary),
            related_memories: related,
        });
        self.dirty = true;

        // Trim old entries
        while self.journal.len() > self.max_journal {
            self.journal.remove(0);
        }
    }

    /// Set the current tick.
    pub fn set_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Serialize to a string for disk persistence.
    pub fn serialize(&self) -> String {
        let mut output = String::from("# VargasJR Memory Store\n\n");

        // Memories
        output.push_str("## Memories\n\n");
        for mem in &self.memories {
            output.push_str(&format!(
                "[{}:{}] {} (learned:{}, accessed:{}, count:{})\n",
                mem.category.prefix(),
                mem.key,
                mem.content,
                mem.learned_at,
                mem.last_accessed,
                mem.access_count,
            ));
        }

        // Journal
        output.push_str("\n## Journal\n\n");
        for entry in &self.journal {
            output.push_str(&format!(
                "[tick:{}] {}\n",
                entry.tick, entry.summary,
            ));
        }

        output
    }

    /// Deserialize from a saved string.
    pub fn deserialize(data: &str) -> Self {
        let mut store = Self::new();

        for line in data.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.contains(':') && !line.starts_with("[tick:") {
                // Memory line: [category:key] content (learned:N, accessed:N, count:N)
                if let Some(parsed) = parse_memory_line(line) {
                    store.memories.push(parsed);
                }
            } else if line.starts_with("[tick:") {
                // Journal line: [tick:N] summary
                if let Some(parsed) = parse_journal_line(line) {
                    store.journal.push(parsed);
                }
            }
        }

        store.dirty = false;
        store
    }

    /// Whether the store needs saving.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as saved.
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Generate a context block for Claude's system prompt.
    pub fn prompt_block(&self) -> String {
        let mut s = String::from("### Memory\n");
        s.push_str(&format!(
            "  {} memories, {} journal entries\n",
            self.memories.len(),
            self.journal.len(),
        ));

        // Include most-accessed memories
        let mut sorted: Vec<&Memory> = self.memories.iter().collect();
        sorted.sort_by(|a, b| b.access_count.cmp(&a.access_count));

        let top = sorted.iter().take(5);
        for mem in top {
            s.push_str(&format!("  • {}: {}\n", mem.key, mem.content));
        }

        // Recent journal
        let recent: Vec<&JournalEntry> = self.journal.iter().rev().take(3).collect();
        if !recent.is_empty() {
            s.push_str("  Recent:\n");
            for entry in recent.iter().rev() {
                s.push_str(&format!("    [tick {}] {}\n", entry.tick, entry.summary));
            }
        }

        s
    }

    /// Stats.
    pub fn stats(&self) -> ContinuityStats {
        ContinuityStats {
            memory_count: self.memories.len(),
            journal_count: self.journal.len(),
            dirty: self.dirty,
            categories: [
                self.by_category(MemoryCategory::UserFact).len(),
                self.by_category(MemoryCategory::Preference).len(),
                self.by_category(MemoryCategory::SystemEvent).len(),
                self.by_category(MemoryCategory::ConversationSummary).len(),
                self.by_category(MemoryCategory::TaskKnowledge).len(),
                self.by_category(MemoryCategory::Correction).len(),
            ],
        }
    }

    /// Evict the least important memory.
    fn evict_least_important(&mut self) {
        if self.memories.is_empty() {
            return;
        }
        // Find lowest access_count, oldest learned_at
        let mut min_idx = 0;
        let mut min_score = u64::MAX;
        for (i, mem) in self.memories.iter().enumerate() {
            let score = (mem.access_count as u64) * 1000 + mem.last_accessed;
            if score < min_score {
                min_score = score;
                min_idx = i;
            }
        }
        self.memories.remove(min_idx);
    }
}

/// Stats for the continuity store.
#[derive(Debug)]
pub struct ContinuityStats {
    pub memory_count: usize,
    pub journal_count: usize,
    pub dirty: bool,
    /// [UserFact, Preference, SystemEvent, ConvSummary, TaskKnowledge, Correction]
    pub categories: [usize; 6],
}

/// Parse a memory line from serialized format.
fn parse_memory_line(line: &str) -> Option<Memory> {
    let bracket_end = line.find(']')?;
    let inner = &line[1..bracket_end];
    let colon = inner.find(':')?;
    let cat_str = &inner[..colon];
    let key = &inner[colon + 1..];

    let rest = line[bracket_end + 2..].trim();
    let paren_start = rest.rfind('(')?;
    let content = rest[..paren_start].trim();

    let category = match cat_str {
        "fact" => MemoryCategory::UserFact,
        "pref" => MemoryCategory::Preference,
        "event" => MemoryCategory::SystemEvent,
        "conv" => MemoryCategory::ConversationSummary,
        "task" => MemoryCategory::TaskKnowledge,
        "fix" => MemoryCategory::Correction,
        _ => return None,
    };

    Some(Memory {
        key: String::from(key),
        content: String::from(content),
        category,
        learned_at: 0,
        last_accessed: 0,
        access_count: 1,
        source: MemorySource::Explicit,
    })
}

/// Parse a journal line from serialized format.
fn parse_journal_line(line: &str) -> Option<JournalEntry> {
    let bracket_end = line.find(']')?;
    let tick_str = &line[6..bracket_end]; // skip "[tick:"
    let tick = tick_str.parse::<u64>().ok()?;
    let summary = line[bracket_end + 2..].trim();

    Some(JournalEntry {
        tick,
        summary: String::from(summary),
        related_memories: Vec::new(),
    })
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_store_new() {
        let store = ContinuityStore::new();
        assert_eq!(store.stats().memory_count, 0);
        assert_eq!(store.stats().journal_count, 0);
        assert!(!store.is_dirty());
    }

    #[test_case]
    fn test_remember_and_recall() {
        let mut store = ContinuityStore::new();
        store.remember("user.name", "Vargas", MemoryCategory::UserFact, MemorySource::Explicit);
        let mem = store.recall("user.name").unwrap();
        assert_eq!(mem.content, "Vargas");
        assert_eq!(mem.access_count, 2); // remember + recall
        assert!(store.is_dirty());
    }

    #[test_case]
    fn test_search() {
        let mut store = ContinuityStore::new();
        store.remember("user.food", "anti-spicy", MemoryCategory::Preference, MemorySource::Explicit);
        store.remember("user.loc", "Lakewood Ranch", MemoryCategory::UserFact, MemorySource::Explicit);
        let results = store.search("spicy");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "user.food");
    }

    #[test_case]
    fn test_journal() {
        let mut store = ContinuityStore::new();
        store.set_tick(42);
        store.journal_add("Boot complete", Vec::new());
        assert_eq!(store.stats().journal_count, 1);
        assert!(store.is_dirty());
    }

    #[test_case]
    fn test_serialize_deserialize() {
        let mut store = ContinuityStore::new();
        store.remember("user.name", "Vargas", MemoryCategory::UserFact, MemorySource::Explicit);
        store.set_tick(10);
        store.journal_add("Test event", Vec::new());

        let serialized = store.serialize();
        let restored = ContinuityStore::deserialize(&serialized);
        assert_eq!(restored.stats().memory_count, 1);
        assert_eq!(restored.stats().journal_count, 1);
    }

    #[test_case]
    fn test_update_existing() {
        let mut store = ContinuityStore::new();
        store.remember("user.loc", "NYC", MemoryCategory::UserFact, MemorySource::Explicit);
        store.remember("user.loc", "Lakewood Ranch", MemoryCategory::Correction, MemorySource::Explicit);
        let mem = store.recall("user.loc").unwrap();
        assert_eq!(mem.content, "Lakewood Ranch");
    }

    #[test_case]
    fn test_prompt_block() {
        let mut store = ContinuityStore::new();
        store.remember("user.name", "Vargas", MemoryCategory::UserFact, MemorySource::Explicit);
        let block = store.prompt_block();
        assert!(block.contains("Memory"));
        assert!(block.contains("Vargas"));
    }
}
