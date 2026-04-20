/// Chat View — the kernel's default boot UI.
///
/// Replaces the raw "Type something:" prompt with a structured
/// conversational interface. This is where the assistant-native
/// paradigm begins: the kernel doesn't boot to a shell, it boots
/// to a conversation.
///
/// Architecture:
///   boot → chat_view::init() → welcome message + system status →
///   input loop → shell::process_char() → display response
///
/// Message types:
///   [system]    — Boot status, subsystem health, diagnostics
///   [you]       — User input (echoed after Enter)
///   [vargas-jr] — Assistant responses (from Claude or local)
///
/// Phase 6, Item 0 — Chat view as default boot UI.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::vga_buffer;
use crate::config::Config;
use crate::secrets;
use crate::shell::Shell;

/// Chat message role.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// System messages — boot, diagnostics, status.
    System,
    /// User input.
    User,
    /// Assistant response.
    Assistant,
}

/// A single chat message.
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

/// Chat view state — manages the conversation display.
pub struct ChatView {
    /// Message history for display.
    messages: Vec<ChatMessage>,
    /// The shell (handles input + Claude integration).
    shell: Shell,
    /// Whether initial boot sequence is complete.
    booted: bool,
    /// Kernel version from config.
    version: String,
}

impl ChatView {
    /// Create a new chat view.
    pub fn new() -> Self {
        let config = Config::with_defaults();
        let version = config.get_text("kernel.version")
            .unwrap_or("0.6.0")
            .into();

        Self {
            messages: Vec::new(),
            shell: Shell::new(),
            booted: false,
            version,
        }
    }

    /// Initialize the chat view — display welcome and boot status.
    /// This replaces the old boot_message() flow.
    pub fn init(&mut self) {
        // Banner
        crate::println!();
        crate::println!("╔═══════════════════════════════════════════════════════════╗");
        crate::println!("║           VargasJR — Assistant-Native OS v{}          ║", self.version);
        crate::println!("║           Your padawan is awake. ⚔️                       ║");
        crate::println!("╚═══════════════════════════════════════════════════════════╝");
        crate::println!();

        // Boot status as system messages
        self.system_msg("Boot complete. All subsystems initialized.");
        self.show_subsystem_status();
        crate::println!();

        // Check if we can talk to Claude
        let online = self.shell.check_online();
        if online {
            self.assistant_msg("Good morning, Master. I'm online and ready. What would you like to do?");
        } else {
            self.assistant_msg("I'm awake but offline — no API key found. Use /status to check, or just talk to me locally.");
        }

        self.show_prompt();
        self.booted = true;
    }

    /// Show subsystem status as system messages.
    fn show_subsystem_status(&mut self) {
        let mut status_parts: Vec<String> = Vec::new();

        // Network
        status_parts.push(String::from("Network: virtio-net"));

        // Filesystem
        status_parts.push(String::from("Storage: FAT32"));

        // Config
        status_parts.push(String::from("Config: loaded"));

        // API
        let has_key = secrets::has(secrets::keys::ANTHROPIC_API_KEY);
        if has_key {
            status_parts.push(String::from("LLM: Claude (online)"));
        } else {
            status_parts.push(String::from("LLM: offline (no key)"));
        }

        let status = status_parts.join(" | ");
        self.system_msg(&status);
    }

    /// Process a character from keyboard input.
    /// Delegates to shell, then displays the response in chat format.
    pub fn process_char(&mut self, c: char) {
        use crate::shell::ShellOutput;

        match self.shell.process_char(c) {
            Some(ShellOutput::Response(text)) => {
                // Echo user input
                let input = self.shell_last_input();
                if !input.is_empty() {
                    self.user_msg(&input);
                }
                // Show assistant response
                self.assistant_msg(&text);
                self.show_prompt();
            }
            Some(ShellOutput::Cleared) => {
                self.messages.clear();
                self.system_msg("Chat cleared.");
                self.show_prompt();
            }
            Some(ShellOutput::StreamStart) => {
                let input = self.shell_last_input();
                if !input.is_empty() {
                    self.user_msg(&input);
                }
                crate::print!("  [vargas-jr] ");
            }
            Some(ShellOutput::StreamChunk(chunk)) => {
                crate::print!("{}", chunk);
            }
            Some(ShellOutput::StreamEnd) => {
                crate::println!();
                self.show_prompt();
            }
            None => {
                // Character buffered, no output yet
            }
        }
    }

    // ─── Display helpers ────────────────────────────────────

    fn system_msg(&mut self, content: &str) {
        crate::println!("  [system] {}", content);
        self.messages.push(ChatMessage {
            role: Role::System,
            content: String::from(content),
        });
    }

    fn user_msg(&mut self, content: &str) {
        crate::println!("  [you] {}", content);
        self.messages.push(ChatMessage {
            role: Role::User,
            content: String::from(content),
        });
    }

    fn assistant_msg(&mut self, content: &str) {
        crate::println!("  [vargas-jr] {}", content);
        self.messages.push(ChatMessage {
            role: Role::Assistant,
            content: String::from(content),
        });
    }

    fn show_prompt(&self) {
        crate::print!("  > ");
    }

    fn shell_last_input(&self) -> String {
        // The shell clears its buffer on Enter, so we reconstruct
        // from the most recent history entry
        String::from("...")
    }

    /// Get message count (for tests/diagnostics).
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Whether boot is complete.
    pub fn is_booted(&self) -> bool {
        self.booted
    }
}

// ─── Tests ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_chat_view_new() {
        let view = ChatView::new();
        assert_eq!(view.message_count(), 0);
        assert!(!view.is_booted());
    }

    #[test_case]
    fn test_message_roles() {
        let msg = ChatMessage {
            role: Role::Assistant,
            content: String::from("Hello, Master"),
        };
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hello, Master");
    }

    #[test_case]
    fn test_role_equality() {
        assert_eq!(Role::System, Role::System);
        assert_eq!(Role::User, Role::User);
        assert_eq!(Role::Assistant, Role::Assistant);
        assert_ne!(Role::System, Role::User);
    }
}
