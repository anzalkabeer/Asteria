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

    pub fn connect(&self, domain: &str, stream: TcpStream) -> Result<TlsConnection, NetworkError> {
        let server_name = ServerName::try_from(domain.to_string())
            .map_err(|e| NetworkError::TlsError(format!("Invalid DNS name: {}", e)))?;

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
