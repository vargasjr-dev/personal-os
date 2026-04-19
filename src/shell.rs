/// Natural Language Shell — the kernel's conversational interface.
///
/// Replaces the basic input loop with an AI-powered shell where
/// user input is sent to Claude and responses stream back.
/// This is the moment the kernel becomes an actual AI assistant.
///
/// Architecture:
///   keyboard input → shell::process() → anthropic::Client →
///   http request → (future: virtio-net TX/RX) → streaming response →
///   VGA/serial display
///
/// Until the network TX/RX path is complete, the shell operates
/// in "offline mode" with local-only commands. Once networking
/// is wired, it switches to live Claude conversations.

use alloc::string::String;
use alloc::vec::Vec;

use crate::anthropic;
use crate::json::Message;
use crate::secrets;

/// Shell state — tracks conversation history and mode.
pub struct Shell {
    /// Conversation history for context window.
    history: Vec<Message>,
    /// Current input buffer.
    input_buffer: String,
    /// Whether the network path is available.
    online: bool,
    /// Maximum messages to keep in context.
    max_context: usize,
}

impl Shell {
    /// Create a new shell instance.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            input_buffer: String::new(),
            online: false,
            max_context: 20,
        }
    }

    /// Check if we can make API calls.
    pub fn check_online(&mut self) -> bool {
        self.online = secrets::has(secrets::keys::ANTHROPIC_API_KEY)
            && anthropic::Client::new("").can_reach();
        self.online
    }

    /// Process a character of keyboard input.
    /// Returns Some(response) when Enter is pressed.
    pub fn process_char(&mut self, c: char) -> Option<ShellOutput> {
        match c {
            '\n' | '\r' => {
                let input = self.input_buffer.clone();
                self.input_buffer.clear();

                if input.is_empty() {
                    return Some(ShellOutput::Empty);
                }

                Some(self.process_command(&input))
            }
            '\x08' | '\x7f' => {
                // Backspace
                self.input_buffer.pop();
                Some(ShellOutput::Backspace)
            }
            c if c.is_ascii_graphic() || c == ' ' => {
                self.input_buffer.push(c);
                Some(ShellOutput::Echo(c))
            }
            _ => None,
        }
    }

    /// Process a complete command/message.
    fn process_command(&mut self, input: &str) -> ShellOutput {
        // Built-in commands
        match input.trim() {
            "/help" => return ShellOutput::Response(self.help_text()),
            "/clear" => {
                self.history.clear();
                return ShellOutput::Response(String::from("Conversation cleared."));
            }
            "/status" => return ShellOutput::Response(self.status_text()),
            "/history" => {
                let count = self.history.len();
                return ShellOutput::Response(
                    alloc::format!("{} messages in context.", count),
                );
            }
            _ => {}
        }

        // Add user message to history
        self.history.push(Message::user(input));

        // Trim context window
        while self.history.len() > self.max_context {
            self.history.remove(0);
        }

        if self.online {
            // Build the request (actual sending happens when network is wired)
            if let Some(api_key) = secrets::get_anthropic_key() {
                let client = anthropic::Client::new(&api_key);
                match client.build_request(&self.history) {
                    Ok(req) => {
                        let bytes = req.to_bytes();
                        ShellOutput::PendingRequest {
                            bytes_ready: bytes.len(),
                            prompt: String::from(input),
                        }
                    }
                    Err(_) => ShellOutput::Response(String::from(
                        "[Shell] Failed to build request.",
                    )),
                }
            } else {
                ShellOutput::Response(String::from(
                    "[Shell] No API key set. Use secrets::set_anthropic_key().",
                ))
            }
        } else {
            // Offline mode — acknowledge the message
            ShellOutput::Response(alloc::format!(
                "[Offline] Message queued: \"{}\"\n\
                 Network not yet available. Type /help for commands.",
                input
            ))
        }
    }

    fn help_text(&self) -> String {
        String::from(
            "VargasJR Shell — Natural Language Interface\n\
             \n\
             Commands:\n\
             /help     — Show this help\n\
             /clear    — Clear conversation history\n\
             /status   — Show shell status\n\
             /history  — Show context window size\n\
             \n\
             Or just type naturally — your message goes to Claude.\n\
             (Network connection required for live responses)",
        )
    }

    fn status_text(&self) -> String {
        let api_key = if secrets::has(secrets::keys::ANTHROPIC_API_KEY) {
            "set"
        } else {
            "not set"
        };

        alloc::format!(
            "Shell Status:\n\
             Mode: {}\n\
             API Key: {}\n\
             Context: {}/{} messages\n\
             Input buffer: {} chars",
            if self.online { "online" } else { "offline" },
            api_key,
            self.history.len(),
            self.max_context,
            self.input_buffer.len(),
        )
    }
}

/// Shell output types.
#[derive(Debug)]
pub enum ShellOutput {
    /// Empty input (just pressed Enter).
    Empty,
    /// Echo a character back to display.
    Echo(char),
    /// Backspace — remove last character from display.
    Backspace,
    /// A response to display.
    Response(String),
    /// Request built and ready to send when network is wired.
    PendingRequest {
        bytes_ready: usize,
        prompt: String,
    },
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_shell_help() {
        let mut shell = Shell::new();
        for c in "/help".chars() {
            shell.process_char(c);
        }
        let output = shell.process_char('\n');
        match output {
            Some(ShellOutput::Response(text)) => {
                assert!(text.contains("VargasJR Shell"));
                assert!(text.contains("/help"));
            }
            _ => panic!("Expected help response"),
        }
    }

    #[test_case]
    fn test_shell_offline_message() {
        let mut shell = Shell::new();
        for c in "hello claude".chars() {
            shell.process_char(c);
        }
        let output = shell.process_char('\n');
        match output {
            Some(ShellOutput::Response(text)) => {
                assert!(text.contains("Offline"));
                assert!(text.contains("hello claude"));
            }
            _ => panic!("Expected offline response"),
        }
    }

    #[test_case]
    fn test_shell_history_tracking() {
        let mut shell = Shell::new();
        for c in "test message".chars() {
            shell.process_char(c);
        }
        shell.process_char('\n');
        assert_eq!(shell.history.len(), 1);
        assert_eq!(shell.history[0].content, "test message");
    }
}
