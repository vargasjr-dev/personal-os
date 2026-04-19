/// HTTP Client — minimal HTTPS request layer for the kernel.
///
/// Ties together the full networking stack:
///   dns::resolve() → net (smoltcp TCP) → tls (rustls) → HTTP/1.1
///
/// This is how the kernel will call LLM APIs (api.anthropic.com).
/// Deliberately minimal — only GET and POST with JSON bodies.
/// No redirects, no cookies, no chunked encoding (yet).

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::dns;
use crate::tls;

/// HTTP methods supported by the kernel.
#[derive(Debug, Clone, Copy)]
pub enum Method {
    Get,
    Post,
}

impl Method {
    fn as_str(&self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
        }
    }
}

/// A minimal HTTP request.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub host: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl Request {
    /// Build a GET request.
    pub fn get(host: &str, path: &str) -> Self {
        Self {
            method: Method::Get,
            host: String::from(host),
            path: String::from(path),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Build a POST request with a JSON body.
    pub fn post(host: &str, path: &str, body: &str) -> Self {
        let mut req = Self {
            method: Method::Post,
            host: String::from(host),
            path: String::from(path),
            headers: Vec::new(),
            body: Some(String::from(body)),
        };
        req.header("Content-Type", "application/json");
        req
    }

    /// Add a header.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((String::from(key), String::from(value)));
        self
    }

    /// Serialize to HTTP/1.1 wire format.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = format!(
            "{} {} HTTP/1.1\r\nHost: {}\r\n",
            self.method.as_str(),
            self.path,
            self.host
        );

        for (key, value) in &self.headers {
            buf.push_str(&format!("{}: {}\r\n", key, value));
        }

        if let Some(body) = &self.body {
            buf.push_str(&format!("Content-Length: {}\r\n", body.len()));
            buf.push_str("\r\n");
            buf.push_str(body);
        } else {
            buf.push_str("\r\n");
        }

        buf.into_bytes()
    }
}

/// A parsed HTTP response.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    /// Parse an HTTP/1.1 response from raw bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, HttpError> {
        let text = core::str::from_utf8(data)
            .map_err(|_| HttpError::InvalidResponse)?;

        // Split headers from body
        let (header_section, body) = text
            .split_once("\r\n\r\n")
            .ok_or(HttpError::InvalidResponse)?;

        let mut lines = header_section.lines();

        // Parse status line: "HTTP/1.1 200 OK"
        let status_line = lines.next().ok_or(HttpError::InvalidResponse)?;
        let status = status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or(HttpError::InvalidResponse)?;

        // Parse headers
        let mut headers = Vec::new();
        for line in lines {
            if let Some((key, value)) = line.split_once(": ") {
                headers.push((String::from(key), String::from(value)));
            }
        }

        Ok(Response {
            status,
            headers,
            body: String::from(body),
        })
    }

    /// Check if the response indicates success (2xx).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status)
    }

    /// Get a header value by name (case-insensitive).
    pub fn header(&self, name: &str) -> Option<&str> {
        let lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == lower)
            .map(|(_, v)| v.as_str())
    }
}

/// HTTP errors.
#[derive(Debug)]
pub enum HttpError {
    /// DNS resolution failed.
    DnsError(String),
    /// TLS is not available for this host.
    TlsUnavailable,
    /// Could not parse the response.
    InvalidResponse,
    /// Connection or transport error.
    ConnectionError(String),
}

/// Pre-flight check: can we make an HTTPS request to this host?
///
/// Verifies both DNS resolution and TLS endpoint availability.
pub fn can_reach(host: &str) -> bool {
    dns::can_resolve(host) && tls::is_known_endpoint(host)
}

/// Build an Anthropic API request.
///
/// This is the primary use case — calling Claude from the kernel.
/// Requires an API key to be set (will be stored in kernel config).
pub fn anthropic_request(api_key: &str, prompt: &str) -> Request {
    let body = format!(
        r#"{{"model":"claude-sonnet-4-20250514","max_tokens":1024,"messages":[{{"role":"user","content":"{}"}}]}}"#,
        prompt
    );

    Request::post("api.anthropic.com", "/v1/messages", &body)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_get_request_serialization() {
        let req = Request::get("api.anthropic.com", "/v1/models");
        let bytes = req.to_bytes();
        let text = core::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("GET /v1/models HTTP/1.1\r\n"));
        assert!(text.contains("Host: api.anthropic.com"));
    }

    #[test_case]
    fn test_post_request_with_body() {
        let req = Request::post("api.anthropic.com", "/v1/messages", r#"{"test":true}"#);
        let bytes = req.to_bytes();
        let text = core::str::from_utf8(&bytes).unwrap();
        assert!(text.starts_with("POST /v1/messages HTTP/1.1\r\n"));
        assert!(text.contains("Content-Type: application/json"));
        assert!(text.contains("Content-Length: 13"));
        assert!(text.ends_with(r#"{"test":true}"#));
    }

    #[test_case]
    fn test_response_parsing() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true}";
        let resp = Response::from_bytes(raw).unwrap();
        assert_eq!(resp.status, 200);
        assert!(resp.is_success());
        assert_eq!(resp.body, "{\"ok\":true}");
        assert_eq!(resp.header("content-type"), Some("application/json"));
    }

    #[test_case]
    fn test_can_reach() {
        assert!(can_reach("api.anthropic.com"));
        assert!(!can_reach("unknown.example.com"));
    }

    #[test_case]
    fn test_anthropic_request() {
        let req = anthropic_request("sk-test", "hello");
        assert_eq!(req.host, "api.anthropic.com");
        assert_eq!(req.path, "/v1/messages");
        let bytes = req.to_bytes();
        let text = core::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("x-api-key: sk-test"));
        assert!(text.contains("anthropic-version: 2023-06-01"));
    }
}
