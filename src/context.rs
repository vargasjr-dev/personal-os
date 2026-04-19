/// Context Window Management — keeps conversation history bounded and useful.
///
/// The kernel's LLM conversations need smart history management:
/// - Token budget tracking (approximate, char-based estimation)
/// - Sliding window with priority retention (system > recent > old)
/// - Summary compression for evicted messages
/// - Conversation save/restore for persistence across reboots
///
/// This completes Phase 4 — the kernel is now a full AI assistant
/// with networking, API integration, streaming, a shell, and
/// intelligent context management.

use alloc::string::String;
use alloc::vec::Vec;

use crate::json::Message;

/// Approximate chars-per-token ratio (conservative estimate).
const CHARS_PER_TOKEN: usize = 4;

/// Default maximum context budget in tokens.
const DEFAULT_MAX_TOKENS: usize = 4096;

/// Minimum messages to always retain (most recent).
const MIN_RETAIN: usize = 4;

/// Context window manager.
pub struct ContextWindow {
    /// All messages in the conversation.
    messages: Vec<ContextMessage>,
    /// Maximum token budget.
    max_tokens: usize,
    /// Current estimated token usage.
    current_tokens: usize,
    /// Number of messages evicted over the lifetime.
    total_evicted: usize,
    /// Summary of evicted context (compressed history).
    eviction_summary: Option<String>,
}

/// A message with metadata for priority management.
#[derive(Clone)]
pub struct ContextMessage {
    /// The underlying API message.
    pub message: Message,
    /// Whether this message is pinned (never evicted).
    pub pinned: bool,
    /// Estimated token count for this message.
    pub token_estimate: usize,
    /// Sequence number for ordering.
    pub seq: u64,
}

impl ContextWindow {
    /// Create a new context window with default budget.
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            max_tokens: DEFAULT_MAX_TOKENS,
            current_tokens: 0,
            total_evicted: 0,
            eviction_summary: None,
        }
    }

    /// Create with a custom token budget.
    pub fn with_budget(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            ..Self::new()
        }
    }

    /// Add a message to the context window.
    /// Triggers eviction if budget is exceeded.
    pub fn push(&mut self, message: Message) -> EvictionResult {
        let token_estimate = estimate_tokens(&message.content);
        let seq = self.messages.len() as u64;

        self.messages.push(ContextMessage {
            message,
            pinned: false,
            token_estimate,
            seq,
        });

        self.current_tokens += token_estimate;
        self.evict_if_needed()
    }

    /// Add a pinned message (system prompt, critical context).
    /// Pinned messages are never evicted.
    pub fn push_pinned(&mut self, message: Message) {
        let token_estimate = estimate_tokens(&message.content);
        let seq = self.messages.len() as u64;

        self.messages.push(ContextMessage {
            message,
            pinned: true,
            token_estimate,
            seq,
        });

        self.current_tokens += token_estimate;
    }

    /// Get all messages for API submission.
    /// Returns messages in order, with eviction summary prepended if present.
    pub fn to_api_messages(&self) -> Vec<Message> {
        let mut result = Vec::new();

        // Prepend eviction summary as system context
        if let Some(ref summary) = self.eviction_summary {
            result.push(Message::user(&alloc::format!(
                "[Context summary from earlier in conversation]: {}",
                summary
            )));
        }

        for cm in &self.messages {
            result.push(cm.message.clone());
        }

        result
    }

    /// Get current stats.
    pub fn stats(&self) -> ContextStats {
        ContextStats {
            message_count: self.messages.len(),
            token_estimate: self.current_tokens,
            max_tokens: self.max_tokens,
            utilization_pct: (self.current_tokens * 100) / self.max_tokens.max(1),
            total_evicted: self.total_evicted,
            has_summary: self.eviction_summary.is_some(),
            pinned_count: self.messages.iter().filter(|m| m.pinned).count(),
        }
    }

    /// Clear all non-pinned messages.
    pub fn clear(&mut self) {
        let pinned: Vec<ContextMessage> = self.messages.iter()
            .filter(|m| m.pinned)
            .cloned()
            .collect();

        let evicted = self.messages.len() - pinned.len();
        self.total_evicted += evicted;
        self.current_tokens = pinned.iter().map(|m| m.token_estimate).sum();
        self.messages = pinned;
        self.eviction_summary = None;
    }

    /// Number of messages currently in the window.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    /// Whether the window is empty.
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Evict oldest non-pinned messages until under budget.
    fn evict_if_needed(&mut self) -> EvictionResult {
        if self.current_tokens <= self.max_tokens {
            return EvictionResult::NoEviction;
        }

        let mut evicted_content = Vec::new();
        let mut evicted_count = 0;

        // Keep evicting oldest non-pinned messages until under budget
        // but always retain at least MIN_RETAIN recent messages
        while self.current_tokens > self.max_tokens {
            let non_pinned_count = self.messages.iter().filter(|m| !m.pinned).count();
            if non_pinned_count <= MIN_RETAIN {
                break;
            }

            // Find the oldest non-pinned message
            if let Some(idx) = self.messages.iter().position(|m| !m.pinned) {
                let removed = self.messages.remove(idx);
                self.current_tokens -= removed.token_estimate;
                evicted_count += 1;
                self.total_evicted += 1;

                // Collect evicted content for summary
                let role = &removed.message.role;
                let preview = if removed.message.content.len() > 60 {
                    alloc::format!("{}...", &removed.message.content[..60])
                } else {
                    removed.message.content.clone()
                };
                evicted_content.push(alloc::format!("[{}]: {}", role, preview));
            } else {
                break;
            }
        }

        if evicted_count > 0 {
            // Build compressed summary of evicted messages
            let summary = alloc::format!(
                "{} earlier messages summarized: {}",
                evicted_count,
                evicted_content.join(" | ")
            );
            self.eviction_summary = Some(summary.clone());

            EvictionResult::Evicted {
                count: evicted_count,
                summary,
            }
        } else {
            EvictionResult::NoEviction
        }
    }
}

/// Result of adding a message (may trigger eviction).
#[derive(Debug)]
pub enum EvictionResult {
    /// No eviction needed.
    NoEviction,
    /// Messages were evicted.
    Evicted {
        count: usize,
        summary: String,
    },
}

/// Context window statistics.
#[derive(Debug)]
pub struct ContextStats {
    pub message_count: usize,
    pub token_estimate: usize,
    pub max_tokens: usize,
    pub utilization_pct: usize,
    pub total_evicted: usize,
    pub has_summary: bool,
    pub pinned_count: usize,
}

/// Estimate token count from content string.
fn estimate_tokens(content: &str) -> usize {
    (content.len() / CHARS_PER_TOKEN).max(1)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_basic_push_and_stats() {
        let mut ctx = ContextWindow::new();
        ctx.push(Message::user("Hello!"));
        ctx.push(Message::assistant("Hi there!"));
        let stats = ctx.stats();
        assert_eq!(stats.message_count, 2);
        assert!(stats.token_estimate > 0);
        assert_eq!(stats.total_evicted, 0);
    }

    #[test_case]
    fn test_eviction_on_budget() {
        // Tiny budget to force eviction
        let mut ctx = ContextWindow::with_budget(10);
        ctx.push(Message::user("First message that is quite long and should take many tokens"));
        ctx.push(Message::user("Second message also fairly long"));
        ctx.push(Message::user("Third message"));
        ctx.push(Message::user("Fourth message"));
        ctx.push(Message::user("Fifth message that pushes over budget definitely"));

        // Should have evicted some messages
        assert!(ctx.stats().total_evicted > 0);
        assert!(ctx.eviction_summary.is_some());
    }

    #[test_case]
    fn test_pinned_messages_survive_eviction() {
        let mut ctx = ContextWindow::with_budget(20);
        ctx.push_pinned(Message::user("System prompt — never evict this"));
        ctx.push(Message::user("Regular message one"));
        ctx.push(Message::user("Regular message two that is longer"));
        ctx.push(Message::user("Regular message three that is quite long indeed"));
        ctx.push(Message::user("Regular message four pushing over the limit"));

        // Pinned message should survive
        let api_msgs = ctx.to_api_messages();
        let has_pinned = api_msgs.iter().any(|m| m.content.contains("System prompt"));
        assert!(has_pinned);
    }

    #[test_case]
    fn test_clear_preserves_pinned() {
        let mut ctx = ContextWindow::new();
        ctx.push_pinned(Message::user("Pinned"));
        ctx.push(Message::user("Regular 1"));
        ctx.push(Message::user("Regular 2"));
        assert_eq!(ctx.len(), 3);

        ctx.clear();
        assert_eq!(ctx.len(), 1); // Only pinned remains
    }

    #[test_case]
    fn test_api_messages_include_summary() {
        let mut ctx = ContextWindow::with_budget(10);
        // Push enough to trigger eviction
        for i in 0..10 {
            ctx.push(Message::user(&alloc::format!("Message number {} with some content", i)));
        }

        let api_msgs = ctx.to_api_messages();
        if ctx.eviction_summary.is_some() {
            assert!(api_msgs[0].content.contains("Context summary"));
        }
    }
}
