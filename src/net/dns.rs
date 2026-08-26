// ─── DNS Resolution Module ───────────
//! DNS Resolution Module for the Asteria Browser Engine
//!
//! This module provides a caching DNS resolver designed for
//! low overhead and ease of use within a semi-multithreaded architecture.
//! It relies on the system's `ToSocketAddrs` for actual resolution.

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

// ─── Types and Errors ───────────

/// Represents an error that can occur during DNS resolution.
#[derive(Debug, Clone)]
pub enum DnsError {
    /// The hostname could not be resolved.
    ResolutionFailed { hostname: String, message: String },
    /// The provided hostname was invalid (e.g., empty or contained a scheme).
    InvalidHostname { hostname: String },
    /// A potential DNS rebinding attack was detected (public domain resolved to private/loopback IP).
    RebindingDetected {
        hostname: String,
        attempted_ip: IpAddr,
    },
}

impl fmt::Display for DnsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnsError::ResolutionFailed { hostname, message } => {
                write!(f, "Failed to resolve hostname '{}': {}", hostname, message)
            }
            DnsError::InvalidHostname { hostname } => {
                write!(f, "Invalid hostname provided: '{}'", hostname)
            }
            DnsError::RebindingDetected {
                hostname,
                attempted_ip,
            } => {
                write!(
                    f,
                    "DNS rebinding attempt blocked for '{}' resolving to private/loopback IP '{}'",
                    hostname, attempted_ip
                )
            }
        }
    }
}

impl std::error::Error for DnsError {}

/// Helper to check if an IP address is loopback or private.
pub fn is_private_or_loopback_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_unspecified()
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // Unique local address (fc00::/7)
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // Link local unicast (fe80::/10)
        }
    }
}

/// Helper to check if a hostname is an intrinsically local hostname (e.g. localhost, .local, .internal, or loopback IP).
pub fn is_local_hostname(hostname: &str) -> bool {
    let lower = hostname.to_ascii_lowercase();
    lower == "localhost"
        || lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower == "127.0.0.1"
        || lower == "::1"
}

/// A single DNS cache entry containing resolved IP addresses and TTL information.
#[derive(Debug, Clone)]
pub struct DnsEntry {
    /// The resolved hostname.
    pub hostname: String,
    /// The list of resolved IP addresses.
    pub ip_addresses: Vec<IpAddr>,
    /// The time at which the resolution was performed.
    pub resolved_at: Instant,
    /// The time-to-live for this cache entry.
    pub ttl: Duration,
}

impl DnsEntry {
    /// Checks if the DNS entry has expired based on its TTL.
    pub fn is_expired(&self) -> bool {
        self.resolved_at.elapsed() > self.ttl
    }
}

// ─── DNS Resolver ───────────

/// A caching DNS resolver with DNS rebinding protection and host-pinning support.
///
/// Designed to be `Send` so it can be moved across threads. It maintains an internal
/// cache of DNS resolutions and pinned hosts to protect against DNS rebinding attacks.
#[derive(Debug, Clone)]
pub struct DnsResolver {
    /// Internal cache mapping hostnames to their DNS entries.
    cache: HashMap<String, DnsEntry>,
    /// Pinned hostnames mapping to specific verified IP addresses.
    pinned_hosts: HashMap<String, Vec<IpAddr>>,
    /// The default time-to-live for cached entries.
    default_ttl: Duration,
    /// Whether DNS rebinding mitigation is enabled.
    rebinding_protection: bool,
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsResolver {
    /// Creates a new `DnsResolver` with a default TTL of 5 minutes and rebinding protection enabled.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            pinned_hosts: HashMap::new(),
            default_ttl: Duration::from_secs(5 * 60), // 5 minutes
            rebinding_protection: true,
        }
    }

    /// Creates a new `DnsResolver` with a custom default TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            pinned_hosts: HashMap::new(),
            default_ttl: ttl,
            rebinding_protection: true,
        }
    }

    /// Enable or disable DNS rebinding protection (enabled by default).
    pub fn set_rebinding_protection(&mut self, enabled: bool) {
        self.rebinding_protection = enabled;
    }

    /// Pins a hostname to a specific list of IP addresses for the duration of a session or origin lifecycle.
    pub fn pin_host(&mut self, hostname: &str, ip_addresses: Vec<IpAddr>) {
        self.pinned_hosts.insert(hostname.to_string(), ip_addresses);
    }

    /// Checks if a hostname is currently pinned.
    pub fn is_pinned(&self, hostname: &str) -> bool {
        self.pinned_hosts.contains_key(hostname)
    }

    /// Unpins a hostname.
    pub fn unpin_host(&mut self, hostname: &str) {
        self.pinned_hosts.remove(hostname);
    }

    /// Resolves a hostname to a `DnsEntry`.
    ///
    /// Checks pinned hosts first, then cache. If a valid (non-expired) entry is found, it is returned.
    /// Otherwise, performs a system DNS lookup, enforces rebinding validation, caches the result, and returns it.
    pub fn resolve(&mut self, hostname: &str) -> Result<DnsEntry, DnsError> {
        self.validate_hostname(hostname)?;

        // Check if the hostname has been pinned for this origin / session
        if let Some(pinned_ips) = self.pinned_hosts.get(hostname) {
            return Ok(DnsEntry {
                hostname: hostname.to_string(),
                ip_addresses: pinned_ips.clone(),
                resolved_at: Instant::now(),
                ttl: self.default_ttl,
            });
        }

        // Check cache for a valid entry
        if let Some(entry) = self.cache.get(hostname)
            && !entry.is_expired()
        {
            return Ok(entry.clone());
        }

        // Perform a fresh resolution and update the cache
        self.resolve_and_cache(hostname)
    }

    /// Resolves a hostname to a `DnsEntry`, unconditionally bypassing the cache.
    ///
    /// The result will be stored in the cache, overwriting any previous entry.
    pub fn resolve_fresh(&mut self, hostname: &str) -> Result<DnsEntry, DnsError> {
        self.validate_hostname(hostname)?;
        if let Some(pinned_ips) = self.pinned_hosts.get(hostname) {
            return Ok(DnsEntry {
                hostname: hostname.to_string(),
                ip_addresses: pinned_ips.clone(),
                resolved_at: Instant::now(),
                ttl: self.default_ttl,
            });
        }
        self.resolve_and_cache(hostname)
    }

    /// Clears the entire DNS cache and unpins all hosts.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.pinned_hosts.clear();
    }

    /// Returns cache statistics as a tuple: `(total_entries, expired_entries)`.
    pub fn cache_stats(&self) -> (usize, usize) {
        let total = self.cache.len();
        let expired = self
            .cache
            .values()
            .filter(|entry| entry.is_expired())
            .count();
        (total, expired)
    }

    // ─── Helper Methods ───────────

    /// Validates the hostname structure.
    fn validate_hostname(&self, hostname: &str) -> Result<(), DnsError> {
        if hostname.is_empty() {
            return Err(DnsError::InvalidHostname {
                hostname: hostname.to_string(),
            });
        }
        if hostname.contains("://") {
            return Err(DnsError::InvalidHostname {
                hostname: hostname.to_string(),
            });
        }
        Ok(())
    }

    /// Performs the actual system lookup, validates rebinding constraints, and caches the result.
    fn resolve_and_cache(&mut self, hostname: &str) -> Result<DnsEntry, DnsError> {
        let lookup_addr = (hostname, 0u16);
        match lookup_addr.to_socket_addrs() {
            Ok(addrs) => {
                let mut ip_addresses = Vec::new();
                for addr in addrs {
                    let ip = addr.ip();
                    if !ip_addresses.contains(&ip) {
                        ip_addresses.push(ip);
                    }
                }

                if ip_addresses.is_empty() {
                    return Err(DnsError::ResolutionFailed {
                        hostname: hostname.to_string(),
                        message: "No IP addresses found".to_string(),
                    });
                }

                // DNS Rebinding Mitigation:
                // If rebinding protection is enabled and the hostname is not an explicitly local name,
                // reject any resolution that points to private/loopback/link-local address spaces.
                if self.rebinding_protection && !is_local_hostname(hostname) {
                    for ip in &ip_addresses {
                        if is_private_or_loopback_ip(ip) {
                            return Err(DnsError::RebindingDetected {
                                hostname: hostname.to_string(),
                                attempted_ip: *ip,
                            });
                        }
                    }
                }

                let entry = DnsEntry {
                    hostname: hostname.to_string(),
                    ip_addresses,
                    resolved_at: Instant::now(),
                    ttl: self.default_ttl,
                };

                self.cache.insert(hostname.to_string(), entry.clone());
                Ok(entry)
            }
            Err(e) => Err(DnsError::ResolutionFailed {
                hostname: hostname.to_string(),
                message: e.to_string(),
            }),
        }
    }
}

// ─── Tests ───────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_localhost() {
        let mut resolver = DnsResolver::new();
        let entry = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");
        assert!(!entry.ip_addresses.is_empty());
        assert_eq!(entry.hostname, "localhost");
    }

    #[test]
    fn test_cache_hit() {
        let mut resolver = DnsResolver::new();
        let entry1 = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");
        let entry2 = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");
        // Due to extremely fast execution, Instant::now() might be very close,
        // but they should be exactly the same object if from cache.
        // We can verify by checking the exact resolved_at time.
        assert_eq!(entry1.resolved_at, entry2.resolved_at);
    }

    #[test]
    fn test_cache_expired() {
        // Create resolver with 0 TTL
        let mut resolver = DnsResolver::with_ttl(Duration::from_secs(0));
        let entry1 = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");

        // Slight delay to ensure time progresses
        std::thread::sleep(Duration::from_millis(1));

        let entry2 = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");

        // Since TTL is 0, the first entry should be expired immediately,
        // causing a fresh resolution and thus a different resolved_at.
        assert_ne!(entry1.resolved_at, entry2.resolved_at);
    }

    #[test]
    fn test_invalid_hostname_empty() {
        let mut resolver = DnsResolver::new();
        let result = resolver.resolve("");
        assert!(matches!(result, Err(DnsError::InvalidHostname { .. })));
    }

    #[test]
    fn test_invalid_hostname_with_scheme() {
        let mut resolver = DnsResolver::new();
        let result = resolver.resolve("http://example.com");
        assert!(matches!(result, Err(DnsError::InvalidHostname { .. })));
    }

    #[test]
    fn test_clear_cache() {
        let mut resolver = DnsResolver::new();
        resolver
            .resolve("localhost")
            .expect("Should resolve localhost");
        assert_eq!(resolver.cache_stats().0, 1);

        resolver.clear_cache();
        assert_eq!(resolver.cache_stats().0, 0);
    }

    #[test]
    fn test_cache_stats() {
        let mut resolver = DnsResolver::with_ttl(Duration::from_secs(0));
        resolver
            .resolve("localhost")
            .expect("Should resolve localhost");

        std::thread::sleep(Duration::from_millis(1));

        let (total, expired) = resolver.cache_stats();
        assert_eq!(total, 1);
        assert_eq!(expired, 1);
    }

    #[test]
    fn test_resolve_fresh_bypasses_cache() {
        let mut resolver = DnsResolver::new();
        let entry1 = resolver
            .resolve("localhost")
            .expect("Should resolve localhost");

        std::thread::sleep(Duration::from_millis(1));

        let entry2 = resolver
            .resolve_fresh("localhost")
            .expect("Should resolve fresh");

        // Even though TTL is 5 min, resolve_fresh bypasses cache
        assert_ne!(entry1.resolved_at, entry2.resolved_at);
    }

    #[test]
    fn test_host_pinning() {
        let mut resolver = DnsResolver::new();
        let test_ip: IpAddr = "93.184.216.34".parse().unwrap();
        resolver.pin_host("example.com", vec![test_ip]);

        assert!(resolver.is_pinned("example.com"));
        let entry = resolver
            .resolve("example.com")
            .expect("Should resolve pinned host");
        assert_eq!(entry.ip_addresses, vec![test_ip]);

        resolver.unpin_host("example.com");
        assert!(!resolver.is_pinned("example.com"));
    }

    #[test]
    fn test_private_and_local_ip_checks() {
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        let private_v4: IpAddr = "192.168.1.1".parse().unwrap();
        let public_v4: IpAddr = "93.184.216.34".parse().unwrap();
        let loopback_v6: IpAddr = "::1".parse().unwrap();

        assert!(is_private_or_loopback_ip(&loopback));
        assert!(is_private_or_loopback_ip(&private_v4));
        assert!(!is_private_or_loopback_ip(&public_v4));
        assert!(is_private_or_loopback_ip(&loopback_v6));

        assert!(is_local_hostname("localhost"));
        assert!(is_local_hostname("test.local"));
        assert!(is_local_hostname("service.internal"));
        assert!(!is_local_hostname("example.com"));
        assert!(!is_local_hostname("attacker.com"));
    }
}
