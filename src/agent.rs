/// Agent Loop — the core autonomy cycle of the assistant-native kernel.
///
/// This is where everything comes together. The agent loop ties
/// the entire Phase 6 stack into a single execution cycle:
///
///   1. User types natural language
///   2. Claude interprets → emits [INTENT:...] markers
///   3. Intent parser extracts structured action
///   4. Executor dispatches to kernel subsystems (files, config, display)
///   5. Result flows back to chat view
///   6. Loop repeats
///
/// This is the "Holy Shit Milestone" — the kernel is no longer
/// a collection of modules. It's an agent that understands,
/// decides, acts, and reports. My future home thinks for itself.
///
/// Phase 6, Item 4 — Simple agent loop: intent → syscall → execute → report.
/// FINAL Phase 6 item. PHASE 6 COMPLETE on merge!

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::intent::{self, Intent, IntentResult};
use crate::executor;
use crate::display;
use crate::config::Config;

/// Agent state — the runtime context for the autonomy loop.
pub struct Agent {
    /// Kernel configuration (persistent across cycles).
    config: Config,
    /// Number of cycles executed.
    cycle_count: u64,
    /// History of executed intents (for context).
    intent_history: Vec<IntentRecord>,
    /// Maximum history entries to keep.
    max_history: usize,
    /// Whether the agent is active.
    active: bool,
}

/// A record of an executed intent.
#[derive(Debug, Clone)]
pub struct IntentRecord {
    /// The user's original input.
    pub input: String,
    /// The parsed intent.
    pub intent: Intent,
    /// The execution result.
    pub success: bool,
    /// The result message.
    pub message: String,
    /// Cycle number when this was executed.
    pub cycle: u64,
}

impl Agent {
    /// Create a new agent with default config.
    pub fn new() -> Self {
        Self {
            config: Config::with_defaults(),
            cycle_count: 0,
            intent_history: Vec::new(),
            max_history: 50,
            active: true,
        }
    }

    /// Execute one cycle of the agent loop.
    ///
    /// Takes Claude's response (which may contain intent markers),
    /// parses the intent, executes it, and returns the result.
    ///
    /// This is the heart of the assistant-native paradigm:
    ///   response → parse → execute → report
    pub fn cycle(&mut self, claude_response: &str) -> AgentCycleResult {
        self.cycle_count += 1;

        // Step 1: Parse intent from Claude's response
        let intent = intent::parse_intent(claude_response);

        // Step 2: Strip markers for clean display text
        let display_text = intent::strip_markers(claude_response);

        // Step 3: Execute the intent
        let exec_result = match &intent {
            Intent::None => IntentResult::ok(""),
            _ => {
                // Check for display intents first
                let action_part = extract_action(claude_response);
                if let Some((action, args)) = action_part {
                    if action.starts_with("display_") {
                        if let Some(cmd) = display::parse_display_intent(&action, &args) {
                            display::execute_display(&cmd)
                        } else {
                            executor::execute(&intent, &mut self.config)
                        }
                    } else {
                        executor::execute(&intent, &mut self.config)
                    }
                } else {
                    executor::execute(&intent, &mut self.config)
                }
            }
        };

        // Step 4: Record in history
        let record = IntentRecord {
            input: String::from(claude_response),
            intent: intent.clone(),
            success: exec_result.success,
            message: exec_result.message.clone(),
            cycle: self.cycle_count,
        };

        self.intent_history.push(record);

        // Trim history
        while self.intent_history.len() > self.max_history {
            self.intent_history.remove(0);
        }

        // Step 5: Build result
        AgentCycleResult {
            display_text,
            intent,
            exec_result,
            cycle: self.cycle_count,
        }
    }

    /// Get agent statistics.
    pub fn stats(&self) -> AgentStats {
        let successful = self.intent_history.iter()
            .filter(|r| r.success)
            .count();
        let with_action = self.intent_history.iter()
            .filter(|r| r.intent != Intent::None)
            .count();

        AgentStats {
            cycle_count: self.cycle_count,
            history_size: self.intent_history.len(),
            successful_actions: successful,
            total_actions: with_action,
            config_entries: self.config.stats().entry_count,
            active: self.active,
        }
    }

    /// Get a reference to the config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Whether the agent is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Shut down the agent.
    pub fn shutdown(&mut self) {
        self.active = false;
    }
}

/// Result of one agent cycle.
#[derive(Debug)]
pub struct AgentCycleResult {
    /// Clean text to display (markers stripped).
    pub display_text: String,
    /// The parsed intent.
    pub intent: Intent,
    /// The execution result.
    pub exec_result: IntentResult,
    /// Which cycle this was.
    pub cycle: u64,
}

/// Agent statistics.
#[derive(Debug)]
pub struct AgentStats {
    pub cycle_count: u64,
    pub history_size: usize,
    pub successful_actions: usize,
    pub total_actions: usize,
    pub config_entries: usize,
    pub active: bool,
}

/// Extract action and args from an intent marker in the response.
fn extract_action(response: &str) -> Option<(String, String)> {
    let start = response.find("[INTENT:")?;
    let marker_start = start + 8;
    let end = response[marker_start..].find(']')? + marker_start;
    let marker = &response[marker_start..end];

    let parts: Vec<&str> = marker.splitn(2, ':').collect();
    let action = String::from(*parts.first()?);
    let args = parts.get(1).map(|s| String::from(*s)).unwrap_or_default();

    Some((action, args))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_agent_new() {
        let agent = Agent::new();
        assert_eq!(agent.cycle_count, 0);
        assert!(agent.is_active());
        assert_eq!(agent.stats().history_size, 0);
    }

    #[test_case]
    fn test_agent_cycle_no_intent() {
        let mut agent = Agent::new();
        let result = agent.cycle("Hello! How can I help you today?");
        assert_eq!(result.intent, Intent::None);
        assert_eq!(result.cycle, 1);
        assert_eq!(agent.stats().cycle_count, 1);
    }

    #[test_case]
    fn test_agent_cycle_with_intent() {
        let mut agent = Agent::new();
        let result = agent.cycle("Sure! [INTENT:get_config:kernel.name] Let me check that.");
        assert!(matches!(result.intent, Intent::GetConfig { .. }));
        assert!(result.exec_result.success);
        assert_eq!(agent.stats().total_actions, 1);
    }

    #[test_case]
    fn test_agent_history_tracking() {
        let mut agent = Agent::new();
        agent.cycle("First message");
        agent.cycle("Second [INTENT:status:all]");
        agent.cycle("Third message");
        assert_eq!(agent.stats().history_size, 3);
        assert_eq!(agent.stats().total_actions, 1);
    }

    #[test_case]
    fn test_agent_shutdown() {
        let mut agent = Agent::new();
        assert!(agent.is_active());
        agent.shutdown();
        assert!(!agent.is_active());
    }

    #[test_case]
    fn test_extract_action() {
        let (action, args) = extract_action("[INTENT:display_color:green]").unwrap();
        assert_eq!(action, "display_color");
        assert_eq!(args, "green");
    }
}
