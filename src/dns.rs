/// DNS Resolver — translates domain names to IP addresses.
///
/// Uses smoltcp's DNS socket to query the QEMU user-mode DNS server
/// at 10.0.2.3. This is how the kernel will resolve "api.anthropic.com"
/// to an IP address before making HTTPS requests.
///
/// For now, this is a synchronous stub that returns hardcoded addresses
/// for known hosts (since the virtio-net RX path isn't wired yet).
/// When the full virtqueue TX/RX is implemented, this will use real
/// DNS queries over UDP port 53.

use alloc::string::String;
use smoltcp::wire::Ipv4Address;

/// Known host entries — hardcoded DNS for bootstrap.
/// These are used until the full DNS query path is wired through virtio-net.
///
/// In QEMU user-mode networking, the host machine is at 10.0.2.2,
/// which can proxy connections to the real internet.
static KNOWN_HOSTS: &[(&str, [u8; 4])] = &[
    ("api.anthropic.com", [104, 18, 37, 228]),
    ("api.openai.com", [104, 18, 6, 192]),
    ("gateway.vellum.ai", [76, 76, 21, 98]),
    ("dns.google", [8, 8, 8, 8]),
];

/// DNS resolution result.
#[derive(Debug, Clone)]
pub enum DnsResult {
    /// Successfully resolved to an IPv4 address.
    Resolved(Ipv4Address),
    /// Domain not found in known hosts (real DNS not yet available).
    NotFound(String),
}

/// Resolve a domain name to an IPv4 address.
///
/// Currently uses a hardcoded lookup table. Will be upgraded to
/// real DNS queries once the virtio-net TX/RX path is complete.
pub fn resolve(domain: &str) -> DnsResult {
    // Check known hosts first
    for &(host, ip) in KNOWN_HOSTS {
        if domain == host {
            let addr = Ipv4Address::new(ip[0], ip[1], ip[2], ip[3]);
            serial_println!("[DNS] {} → {}", domain, addr);
            return DnsResult::Resolved(addr);
        }
    }

    // For any unknown host in QEMU user-mode, we can route through
    // the gateway (10.0.2.2) which acts as a NAT proxy.
    // But without real DNS, we can't resolve arbitrary domains yet.
    serial_println!("[DNS] {} → NOT FOUND (real DNS not yet available)", domain);
    DnsResult::NotFound(String::from(domain))
}

/// Check if DNS resolution is available for a domain.
pub fn can_resolve(domain: &str) -> bool {
    matches!(resolve(domain), DnsResult::Resolved(_))
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_known_host_resolution() {
        match resolve("api.anthropic.com") {
            DnsResult::Resolved(addr) => {
                assert_eq!(addr, Ipv4Address::new(104, 18, 37, 228));
            }
            DnsResult::NotFound(_) => panic!("should resolve known host"),
        }
    }

    #[test_case]
    fn test_unknown_host() {
        match resolve("unknown.example.com") {
            DnsResult::NotFound(domain) => {
                assert_eq!(domain, "unknown.example.com");
            }
            DnsResult::Resolved(_) => panic!("should not resolve unknown host"),
        }
    }

    #[test_case]
    fn test_can_resolve() {
        assert!(can_resolve("api.anthropic.com"));
        assert!(!can_resolve("unknown.example.com"));
    }
}
