/// Anthropic Client — unified interface for calling Claude from the kernel.
///
/// Combines the HTTP client + JSON types into a high-level API:
///   anthropic::Client::new(api_key) → client.send(prompt) → Response
///
/// This is Phase 4 Item 0 — the first module that treats the networking
/// stack as a complete tool rather than individual layers. When the
/// virtio-net TX/RX path is fully wired, this becomes the kernel's
/// primary way to think (by calling Claude).

use alloc::string::String;
use alloc::vec::Vec;

use crate::dns;
use crate::http::{self, Request, Response, HttpError};
use crate::json::{self, AnthropicRequest, AnthropicResponse, AnthropicError, Message};

const API_HOST: &str = "api.anthropic.com";
const API_PATH: &str = "/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
const DEFAULT_MAX_TOKENS: u32 = 1024;

/// High-level Anthropic API client.
#[derive(Debug, Clone)]
pub struct Client {
    api_key: String,
    model: String,
    max_tokens: u32,
}

impl Client {
    /// Create a new client with an API key.
    pub fn new(api_key: &str) -> Self {
        Self {
            api_key: String::from(api_key),
            model: String::from(DEFAULT_MODEL),
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }

    /// Set the model (default: claude-sonnet-4-20250514).
    pub fn with_model(mut self, model: &str) -> Self {
        self.model = String::from(model);
        self
    }

    /// Set max tokens (default: 1024).
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Check if the API endpoint is reachable (DNS + TLS pre-flight).
    pub fn can_reach(&self) -> bool {
        http::can_reach(API_HOST)
    }

    /// Build an HTTP request for the Anthropic Messages API.
    ///
    /// This creates the fully-formed HTTP request with headers and
    /// JSON body. The caller is responsible for sending it over the
    /// network (once virtio-net TX/RX is wired).
    pub fn build_request(&self, messages: &[Message]) -> Result<Request, ClientError> {
        if !self.can_reach() {
            return Err(ClientError::Unreachable);
        }

        let request_body = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: self.max_tokens,
            messages: messages.to_vec(),
        };

        let body_json = json::to_string(&request_body)
            .map_err(|_| ClientError::SerializationError)?;

        let req = Request::post(API_HOST, API_PATH, &body_json)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .header("Accept", "application/json");

        Ok(req)
    }

    /// Build a simple single-turn request from a prompt string.
    pub fn build_simple(&self, prompt: &str) -> Result<Request, ClientError> {
        self.build_request(&[Message::user(prompt)])
    }

    /// Parse a raw HTTP response into an Anthropic response.
    pub fn parse_response(response: &Response) -> Result<AnthropicResponse, ClientError> {
        if !response.is_success() {
            // Try to parse as error response
            if let Ok(err) = json::from_str::<AnthropicError>(&response.body) {
                return Err(ClientError::ApiError {
                    status: response.status,
                    error_type: err.error.detail_type,
                    message: err.error.message,
                });
            }
            return Err(ClientError::HttpError(response.status));
        }

        json::from_str::<AnthropicResponse>(&response.body)
            .map_err(|_| ClientError::DeserializationError)
    }

    /// Extract just the text from a parsed response.
    pub fn extract_text(response: &AnthropicResponse) -> Option<String> {
        response.text().map(String::from)
    }
}

/// Client errors.
#[derive(Debug)]
pub enum ClientError {
    /// DNS or TLS pre-flight failed.
    Unreachable,
    /// Could not serialize request body.
    SerializationError,
    /// Could not deserialize response.
    DeserializationError,
    /// API returned a non-2xx status.
    HttpError(u16),
    /// API returned a structured error.
    ApiError {
        status: u16,
        error_type: String,
        message: String,
    },
    /// Network transport error.
    TransportError(HttpError),
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_client_creation() {
        let client = Client::new("sk-ant-test123");
        assert_eq!(client.model, DEFAULT_MODEL);
        assert_eq!(client.max_tokens, DEFAULT_MAX_TOKENS);
        assert!(client.can_reach()); // api.anthropic.com is in known hosts
    }

    #[test_case]
    fn test_build_simple_request() {
        let client = Client::new("sk-ant-test123");
        let req = client.build_simple("Hello from the kernel!").unwrap();
        let bytes = req.to_bytes();
        let text = core::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("POST /v1/messages HTTP/1.1"));
        assert!(text.contains("x-api-key: sk-ant-test123"));
        assert!(text.contains("anthropic-version: 2023-06-01"));
        assert!(text.contains("Hello from the kernel!"));
    }

    #[test_case]
    fn test_parse_success_response() {
        let raw = br#"HTTP/1.1 200 OK
Content-Type: application/json

{"id":"msg_123","type":"message","role":"assistant","content":[{"type":"text","text":"Hello!"}],"model":"claude-sonnet-4-20250514","stop_reason":"end_turn"}"#;
        let http_resp = Response::from_bytes(raw).unwrap();
        let parsed = Client::parse_response(&http_resp).unwrap();
        assert_eq!(Client::extract_text(&parsed), Some(String::from("Hello!")));
    }

    #[test_case]
    fn test_parse_error_response() {
        let raw = br#"HTTP/1.1 401 Unauthorized
Content-Type: application/json

{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;
        let http_resp = Response::from_bytes(raw).unwrap();
        let err = Client::parse_response(&http_resp).unwrap_err();
        match err {
            ClientError::ApiError { status, error_type, message } => {
                assert_eq!(status, 401);
                assert_eq!(error_type, "authentication_error");
                assert_eq!(message, "Invalid API key");
            }
            _ => panic!("Expected ApiError"),
        }
    }

    #[test_case]
    fn test_with_model() {
        let client = Client::new("sk-test").with_model("claude-3-haiku-20240307");
        assert_eq!(client.model, "claude-3-haiku-20240307");
    }
}
