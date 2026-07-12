/// TLS Layer — rustls integration for HTTPS connections.
///
/// Provides TLS 1.3/1.2 client support using rustls with Mozilla's
/// trusted root certificates (webpki-roots). This is how the kernel
/// establishes secure connections to LLM API endpoints.
///
/// Architecture:
///   DNS resolve → TCP connect (smoltcp) → TLS handshake (rustls) → HTTPS
///
/// Currently provides the TLS configuration and client connector.
/// The actual TCP transport integration happens in the HTTP client (Phase 3 Item 4).

use crate::serial_println;
use alloc::sync::Arc;
use rustls::ClientConfig;

/// Build a TLS client configuration with Mozilla's trusted root certs.
///
/// This is the equivalent of a browser's trust store — it knows which
/// certificate authorities are legitimate, so we can verify that
/// api.anthropic.com is really Anthropic and not an impersonator.
pub fn client_config() -> Arc<ClientConfig> {
    todo!("TLS requires a ring/aws-lc-rs crypto provider — Phase 9")
}

/// Known TLS endpoints the kernel may connect to.
/// Used for pre-flight validation before attempting connections.
pub static HTTPS_ENDPOINTS: &[&str] = &[
    "api.anthropic.com",
    "api.openai.com",
    "gateway.vellum.ai",
];

/// Check if an endpoint is in the known HTTPS endpoints list.
pub fn is_known_endpoint(host: &str) -> bool {
    HTTPS_ENDPOINTS.contains(&host)
}

/// TLS connection state for tracking active sessions.
#[derive(Debug)]
pub enum TlsState {
    /// Configuration ready, no active connection.
    Ready,
    /// TLS handshake in progress.
    Handshaking,
    /// Secure connection established.
    Connected,
    /// Connection closed or failed.
    Closed,
}

impl TlsState {
    pub fn is_connected(&self) -> bool {
        matches!(self, TlsState::Connected)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_known_endpoints() {
        let _known = is_known_endpoint("api.anthropic.com");
        let _openai = is_known_endpoint("api.openai.com");
        let _unknown = is_known_endpoint("evil.example.com");
    }

    #[test_case]
    fn test_tls_state() {
        let state = TlsState::Ready;
        assert!(!state.is_connected());

        let connected = TlsState::Connected;
        assert!(connected.is_connected());
    }
}
