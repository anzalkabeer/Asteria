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
        }
    }
}

impl std::error::Error for DnsError {}

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

/// A caching DNS resolver.
///
/// Designed to be `Send` so it can be moved across threads. It maintains an internal
/// cache of DNS resolutions to minimize system calls and improve performance.
#[derive(Debug, Clone)]
pub struct DnsResolver {
    /// Internal cache mapping hostnames to their DNS entries.
    cache: HashMap<String, DnsEntry>,
    /// The default time-to-live for cached entries.
    default_ttl: Duration,
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsResolver {
    /// Creates a new `DnsResolver` with a default TTL of 5 minutes.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl: Duration::from_secs(5 * 60), // 5 minutes
        }
    }

    /// Creates a new `DnsResolver` with a custom default TTL.
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl: ttl,
        }
    }

    /// Resolves a hostname to a `DnsEntry`.
    ///
    /// Checks the cache first. If a valid (non-expired) entry is found, it is returned.
    /// Otherwise, performs a system DNS lookup, caches the result, and returns it.
    pub fn resolve(&mut self, hostname: &str) -> Result<DnsEntry, DnsError> {
        self.validate_hostname(hostname)?;

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
        self.resolve_and_cache(hostname)
    }

    /// Clears the entire DNS cache.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
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

    /// Performs the actual system lookup and caches the result.
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
}
