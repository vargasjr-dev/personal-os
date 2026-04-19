/// JSON Support — serialize/deserialize for API communication.
///
/// Re-exports serde + serde_json with kernel-specific types for
/// Anthropic API messages. This completes the networking stack:
///   virtio-net → smoltcp → dns → tls → http → json (this!)
///
/// After this, the kernel has everything it needs to:
///   1. Resolve api.anthropic.com (dns)
///   2. Open a TCP connection (net/smoltcp)
///   3. Establish TLS (rustls)
///   4. Send HTTP requests (http)
///   5. Parse JSON responses (this module)

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

// Re-export core serde_json functions for kernel use
pub use serde_json::{from_str, to_string, Value};

// ─── Anthropic API Types ────────────────────────────────────────────────────

/// A message in the Anthropic conversation format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn user(content: &str) -> Self {
        Self {
            role: String::from("user"),
            content: String::from(content),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: String::from("assistant"),
            content: String::from(content),
        }
    }
}

/// Request body for the Anthropic Messages API.
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<Message>,
}

impl AnthropicRequest {
    /// Create a simple single-turn request.
    pub fn simple(prompt: &str) -> Self {
        Self {
            model: String::from("claude-sonnet-4-20250514"),
            max_tokens: 1024,
            messages: vec![Message::user(prompt)],
        }
    }
}

/// A content block in the Anthropic response.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(default)]
    pub text: String,
}

/// Response from the Anthropic Messages API.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub response_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
}

impl AnthropicResponse {
    /// Extract the text content from the first content block.
    pub fn text(&self) -> Option<&str> {
        self.content
            .iter()
            .find(|b| b.block_type == "text")
            .map(|b| b.text.as_str())
    }
}

/// Error response from the Anthropic API.
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicError {
    #[serde(rename = "type")]
    pub error_type: String,
    pub error: ErrorDetail,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ErrorDetail {
    #[serde(rename = "type")]
    pub detail_type: String,
    pub message: String,
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_serialize_request() {
        let req = AnthropicRequest::simple("Hello from the kernel!");
        let json = to_string(&req).unwrap();
        assert!(json.contains("claude-sonnet-4-20250514"));
        assert!(json.contains("Hello from the kernel!"));
        assert!(json.contains("\"role\":\"user\""));
    }

    #[test_case]
    fn test_deserialize_response() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "Hello from Claude!"}],
            "model": "claude-sonnet-4-20250514",
            "stop_reason": "end_turn"
        }"#;
        let resp: AnthropicResponse = from_str(json).unwrap();
        assert_eq!(resp.id, "msg_123");
        assert_eq!(resp.text(), Some("Hello from Claude!"));
        assert_eq!(resp.stop_reason, Some(String::from("end_turn")));
    }

    #[test_case]
    fn test_deserialize_error() {
        let json = r#"{
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "messages: Required"
            }
        }"#;
        let err: AnthropicError = from_str(json).unwrap();
        assert_eq!(err.error.detail_type, "invalid_request_error");
        assert_eq!(err.error.message, "messages: Required");
    }

    #[test_case]
    fn test_message_constructors() {
        let user = Message::user("test");
        assert_eq!(user.role, "user");
        let asst = Message::assistant("reply");
        assert_eq!(asst.role, "assistant");
    }

    #[test_case]
    fn test_generic_json_parsing() {
        let val: Value = from_str(r#"{"key": 42}"#).unwrap();
        assert_eq!(val["key"], 42);
    }
}
