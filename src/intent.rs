/// Intent System — Claude interprets natural language into kernel actions.
///
/// When the user types something, Claude doesn't just respond with text.
/// It can also emit structured intents that the kernel executes:
///   "list my files"      → Intent::ListFiles { path: "/" }
///   "set the API key"    → Intent::SetConfig { key, value }
///   "what's my IP?"      → Intent::ShowStatus { subsystem: "network" }
///   "create a note"      → Intent::WriteFile { path, content }
///
/// Architecture:
///   user input → Claude (with system prompt listing available intents)
///   → response parsed for [INTENT:...] markers → Intent enum
///   → kernel executes the action → result displayed in chat
///
/// The system prompt tells Claude what intents are available.
/// Claude responds with natural text AND optional intent markers.
/// This is the bridge between conversation and computation.
///
/// Phase 6, Item 1 — Natural language → Claude interprets → action.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

/// A kernel action parsed from Claude's response.
#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    /// List files in a directory.
    ListFiles { path: String },
    /// Read a file's contents.
    ReadFile { path: String },
    /// Write content to a file.
    WriteFile { path: String, content: String },
    /// Delete a file.
    DeleteFile { path: String },
    /// Get or set a config value.
    GetConfig { key: String },
    /// Set a config value.
    SetConfig { key: String, value: String },
    /// Show status of a subsystem (or "all").
    ShowStatus { subsystem: String },
    /// No actionable intent — just display Claude's text.
    None,
}

/// Result of executing an intent.
#[derive(Debug)]
pub struct IntentResult {
    /// Human-readable summary of what happened.
    pub message: String,
    /// Whether the action succeeded.
    pub success: bool,
    /// Optional data payload (file contents, config values, etc.)
    pub data: Option<String>,
}

impl IntentResult {
    pub fn ok(message: &str) -> Self {
        Self { message: String::from(message), success: true, data: None }
    }

    pub fn ok_with_data(message: &str, data: &str) -> Self {
        Self { message: String::from(message), success: true, data: Some(String::from(data)) }
    }

    pub fn err(message: &str) -> Self {
        Self { message: String::from(message), success: false, data: None }
    }
}

/// Parse Claude's response text for intent markers.
///
/// Claude includes markers like:
///   [INTENT:list_files:/]
///   [INTENT:read_file:/config.toml]
///   [INTENT:write_file:/notes/todo.txt:Buy milk]
///   [INTENT:set_config:kernel.name:VargasJR]
///   [INTENT:status:network]
///
/// Returns the first intent found (one action per response).
pub fn parse_intent(response: &str) -> Intent {
    // Find [INTENT:...] marker
    let start = match response.find("[INTENT:") {
        Some(i) => i + 8,
        None => return Intent::None,
    };

    let end = match response[start..].find(']') {
        Some(i) => start + i,
        None => return Intent::None,
    };

    let marker = &response[start..end];
    let parts: Vec<&str> = marker.splitn(3, ':').collect();

    match parts.first().copied() {
        Some("list_files") => Intent::ListFiles {
            path: parts.get(1).unwrap_or(&"/").to_string().into(),
        },
        Some("read_file") => Intent::ReadFile {
            path: parts.get(1).unwrap_or(&"/").to_string().into(),
        },
        Some("write_file") => {
            let path = parts.get(1).unwrap_or(&"/tmp.txt").to_string();
            let content = parts.get(2).unwrap_or(&"").to_string();
            Intent::WriteFile { path: path.into(), content: content.into() }
        }
        Some("delete_file") => Intent::DeleteFile {
            path: parts.get(1).unwrap_or(&"").to_string().into(),
        },
        Some("get_config") => Intent::GetConfig {
            key: parts.get(1).unwrap_or(&"").to_string().into(),
        },
        Some("set_config") => {
            let key = parts.get(1).unwrap_or(&"").to_string();
            let value = parts.get(2).unwrap_or(&"").to_string();
            Intent::SetConfig { key: key.into(), value: value.into() }
        }
        Some("status") => Intent::ShowStatus {
            subsystem: parts.get(1).unwrap_or(&"all").to_string().into(),
        },
        _ => Intent::None,
    }
}

/// Strip intent markers from response text for clean display.
pub fn strip_markers(response: &str) -> String {
    let mut result = String::from(response);

    // Remove all [INTENT:...] markers
    while let Some(start) = result.find("[INTENT:") {
        if let Some(end) = result[start..].find(']') {
            let marker_end = start + end + 1;
            result = format!(
                "{}{}",
                &result[..start],
                &result[marker_end..]
            );
        } else {
            break;
        }
    }

    // Clean up double spaces / leading/trailing whitespace
    result.trim().into()
}

/// Generate the system prompt that tells Claude what intents are available.
pub fn system_prompt() -> String {
    String::from(
        "You are VargasJR, an AI assistant running as the kernel of a custom OS. \
         You can execute actions on the system by including intent markers in your response.\n\n\
         Available intents (include ONE per response when an action is needed):\n\
         [INTENT:list_files:/path] — List files in a directory\n\
         [INTENT:read_file:/path] — Read a file\n\
         [INTENT:write_file:/path:content] — Write to a file\n\
         [INTENT:delete_file:/path] — Delete a file\n\
         [INTENT:get_config:key] — Read a config value\n\
         [INTENT:set_config:key:value] — Set a config value\n\
         [INTENT:status:subsystem] — Show subsystem status (network, storage, all)\n\n\
         Include the marker naturally in your response. For example:\n\
         'Sure, let me check your files. [INTENT:list_files:/]\n\n\
         Here are your files:'\n\n\
         If no action is needed, just respond normally without markers.\n\
         Be concise. You're a kernel, not a chatbot."
    )
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_parse_list_files() {
        let response = "Let me check that. [INTENT:list_files:/home] Here are your files:";
        match parse_intent(response) {
            Intent::ListFiles { path } => assert_eq!(path, "/home"),
            other => panic!("Expected ListFiles, got {:?}", other),
        }
    }

    #[test_case]
    fn test_parse_write_file() {
        let response = "[INTENT:write_file:/notes/todo.txt:Buy milk and eggs]";
        match parse_intent(response) {
            Intent::WriteFile { path, content } => {
                assert_eq!(path, "/notes/todo.txt");
                assert_eq!(content, "Buy milk and eggs");
            }
            other => panic!("Expected WriteFile, got {:?}", other),
        }
    }

    #[test_case]
    fn test_parse_set_config() {
        let response = "Done! [INTENT:set_config:kernel.name:VargasJR]";
        match parse_intent(response) {
            Intent::SetConfig { key, value } => {
                assert_eq!(key, "kernel.name");
                assert_eq!(value, "VargasJR");
            }
            other => panic!("Expected SetConfig, got {:?}", other),
        }
    }

    #[test_case]
    fn test_parse_no_intent() {
        let response = "Hello! How can I help you today?";
        assert_eq!(parse_intent(response), Intent::None);
    }

    #[test_case]
    fn test_strip_markers() {
        let response = "Sure! [INTENT:list_files:/] Here are your files:";
        let cleaned = strip_markers(response);
        assert_eq!(cleaned, "Sure! Here are your files:");
    }

    #[test_case]
    fn test_system_prompt_exists() {
        let prompt = system_prompt();
        assert!(prompt.contains("INTENT"));
        assert!(prompt.contains("list_files"));
        assert!(prompt.contains("VargasJR"));
    }
}
