use std::net::TcpStream;
use std::sync::Arc;

use rustls::client::Resumption;
use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use crate::net::tcp::NetworkError;

// ─── TlsConnection ──────────────────────────────────────────────────

/// Wraps a Rustls connection and a TCP stream.
pub struct TlsConnection {
    stream: StreamOwned<ClientConnection, TcpStream>,
}

impl TlsConnection {
    pub fn new(stream: StreamOwned<ClientConnection, TcpStream>) -> Self {
        Self { stream }
    }

    pub fn into_inner(self) -> StreamOwned<ClientConnection, TcpStream> {
        self.stream
    }

    pub fn get_mut(&mut self) -> &mut StreamOwned<ClientConnection, TcpStream> {
        &mut self.stream
    }

    pub fn get_ref(&self) -> &StreamOwned<ClientConnection, TcpStream> {
        &self.stream
    }
}

// ─── TlsConnector ───────────────────────────────────────────────────

/// Handles TLS configuration and creating connections.
#[derive(Clone)]
pub struct TlsConnector {
    config: Arc<ClientConfig>,
}

impl TlsConnector {
    pub fn new() -> Self {
        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let mut config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        // LRU Cache with 512 capacity for TLS session resumption.
        // Rustls naturally manages time limits based on TLS session ticket lifetimes internally.
        config.resumption = Resumption::in_memory_sessions(512);

        Self {
            config: Arc::new(config),
        }
    }

    /// Parses and sanitizes a domain string into a valid Rustls `ServerName`.
    /// Strips any accidental schemes, ports, or trailing dots.
    pub fn parse_server_name(domain: &str) -> Result<ServerName<'static>, NetworkError> {
        let clean = domain
            .trim()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or(domain)
            .split(':')
            .next()
            .unwrap_or(domain)
            .trim_end_matches('.');

        if clean.is_empty() {
            return Err(NetworkError::TlsError("Domain name is empty".to_string()));
        }

        ServerName::try_from(clean.to_string()).map_err(|e| {
            NetworkError::TlsError(format!("Invalid TLS server name '{}': {}", clean, e))
        })
    }

    pub fn connect(&self, domain: &str, stream: TcpStream) -> Result<TlsConnection, NetworkError> {
        let server_name = Self::parse_server_name(domain)?;

        let conn = ClientConnection::new(self.config.clone(), server_name)
            .map_err(|e| NetworkError::TlsError(format!("TLS connect error: {}", e)))?;

        let stream_owned = StreamOwned::new(conn, stream);

        Ok(TlsConnection::new(stream_owned))
    }
}

impl Default for TlsConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_name_valid() {
        assert!(TlsConnector::parse_server_name("example.com").is_ok());
        assert!(TlsConnector::parse_server_name("sub.domain.org").is_ok());
        assert!(TlsConnector::parse_server_name("https://example.com:443/path").is_ok());
        assert!(TlsConnector::parse_server_name("example.com.").is_ok());
        assert!(TlsConnector::parse_server_name("127.0.0.1").is_ok());
    }

    #[test]
    fn test_parse_server_name_invalid() {
        assert!(TlsConnector::parse_server_name("").is_err());
        assert!(TlsConnector::parse_server_name("   ").is_err());
        assert!(TlsConnector::parse_server_name("invalid..domain").is_err());
        assert!(TlsConnector::parse_server_name("domain with spaces.com").is_err());
    }
}
