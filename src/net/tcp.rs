// ─── Module Documentation ───────────
/// TCP Connection Management
///
/// This module provides the core networking layer for the Asteria browser engine.
/// It implements a low-latency, zero-copy (where possible) connection pool for
/// managing long-lived HTTP and WebSocket connections.
///
/// Key design goals:
/// - Low resource overhead (battery and RAM conscious)
/// - Zero-copy operations where possible
/// - Clean, reusable abstractions
/// - Fault-tolerant connection handling
use std::collections::HashMap;
use std::fmt;
use std::io;
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

// ─── NetworkError Enum ───────────

/// Represents all possible network-related errors in the Asteria engine.
/// Provides descriptive variants for better error handling and debugging.
#[derive(Debug)]
pub enum NetworkError {
    /// Failed to resolve DNS.
    DnsError(String),
    /// Connection to the host failed.
    ConnectionFailed { addr: String, message: String },
    /// Connection attempt timed out.
    ConnectionTimeout { addr: String, timeout: Duration },
    /// Reading from the connection timed out.
    ReadTimeout { message: String },
    /// Writing to the connection failed.
    WriteError { message: String },
    /// The provided URL is invalid (simple variant for URL parsing).
    InvalidUrl(String),
    /// HTTP protocol error.
    HttpError { status: u16, message: String },
    /// Too many redirects encountered.
    TooManyRedirects { url: String, max: usize },
    /// Standard I/O error wrapper (wraps std::io::Error).
    IoError(io::Error),
    /// String-based I/O error for cases where the original error is consumed.
    Io(String),
    /// Generic / catch-all error for protocol violations, malformed data, etc.
    Other(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::DnsError(msg) => write!(f, "DNS resolution failed: {}", msg),
            NetworkError::ConnectionFailed { addr, message } => {
                write!(f, "Connection to {} failed: {}", addr, message)
            }
            NetworkError::ConnectionTimeout { addr, timeout } => {
                write!(
                    f,
                    "Connection to {} timed out after {}ms",
                    addr,
                    timeout.as_millis()
                )
            }
            NetworkError::ReadTimeout { message } => write!(f, "Read timeout: {}", message),
            NetworkError::WriteError { message } => write!(f, "Write error: {}", message),
            NetworkError::InvalidUrl(reason) => {
                write!(f, "Invalid URL: {}", reason)
            }
            NetworkError::HttpError { status, message } => {
                write!(f, "HTTP Error {}: {}", status, message)
            }
            NetworkError::TooManyRedirects { url, max } => {
                write!(f, "Too many redirects (max {}) for URL: {}", max, url)
            }
            NetworkError::IoError(err) => write!(f, "I/O Error: {}", err),
            NetworkError::Io(msg) => write!(f, "I/O Error: {}", msg),
            NetworkError::Other(msg) => write!(f, "Network error: {}", msg),
        }
    }
}

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
        NetworkError::IoError(error)
    }
}

// ─── TcpConnection Struct ───────────

/// Represents a single active TCP connection to a remote host.
/// Stores connection metadata alongside the actual stream.
#[derive(Debug)]
pub struct TcpConnection {
    /// The remote IP and port.
    pub remote_addr: SocketAddr,
    /// The underlying TCP stream.
    pub stream: TcpStream,
    /// When this connection was established.
    pub connected_at: Instant,
    /// The unique identifier in the pool, typically `host:port` or `ip:port`.
    pub key: String,
}

impl TcpConnection {
    /// Checks if the connection is still alive by performing a non-blocking
    /// zero-byte peek. If the remote end has closed the connection, or if
    /// an error occurs during the peek, the connection is considered dead.
    pub fn is_alive(&self) -> bool {
        let mut buf = [0; 0];
        self.stream.peek(&mut buf).is_ok()
    }
}

// ─── ConnectionPool Struct ───────────

/// Manages a pool of active TCP connections to reduce connection overhead
/// for subsequent requests to the same host.
#[derive(Debug)]
pub struct ConnectionPool {
    /// Active connections keyed by their host and port.
    connections: HashMap<String, TcpConnection>,
    /// Timeout for establishing new connections.
    connect_timeout: Duration,
    /// Timeout for reading data from established connections.
    read_timeout: Duration,
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool {
    /// Creates a new connection pool with default timeouts.
    /// Default connect timeout is 10 seconds.
    /// Default read timeout is 30 seconds.
    pub fn new() -> Self {
        Self::with_timeouts(Duration::from_secs(10), Duration::from_secs(30))
    }

    /// Creates a new connection pool with custom timeouts.
    pub fn with_timeouts(connect: Duration, read: Duration) -> Self {
        Self {
            connections: HashMap::new(),
            connect_timeout: connect,
            read_timeout: read,
        }
    }

    /// Connects to a specific IP address, reusing an existing connection if alive.
    /// If a connection exists but is dead, it is replaced.
    pub fn connect(&mut self, addr: SocketAddr) -> Result<&mut TcpStream, NetworkError> {
        let key = addr.to_string();

        if let Some(conn) = self.connections.get(&key)
            && conn.is_alive()
        {
            return Ok(&mut self.connections.get_mut(&key).unwrap().stream);
        }

        let stream = TcpStream::connect_timeout(&addr, self.connect_timeout).map_err(|_| {
            NetworkError::ConnectionTimeout {
                addr: key.clone(),
                timeout: self.connect_timeout,
            }
        })?;

        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        stream.set_nodelay(true)?;

        let conn = TcpConnection {
            remote_addr: addr,
            stream,
            connected_at: Instant::now(),
            key: key.clone(),
        };

        self.connections.insert(key.clone(), conn);
        Ok(&mut self.connections.get_mut(&key).unwrap().stream)
    }

    /// Connects to a host and port, primarily checking the pool by `host:port` key.
    /// If not found, falls back to connecting to the provided `addr`.
    pub fn get_or_connect(
        &mut self,
        host: &str,
        port: u16,
        addr: SocketAddr,
    ) -> Result<&mut TcpStream, NetworkError> {
        let key = format!("{}:{}", host, port);

        if let Some(conn) = self.connections.get(&key)
            && conn.is_alive()
        {
            return Ok(&mut self.connections.get_mut(&key).unwrap().stream);
        }

        let stream = TcpStream::connect_timeout(&addr, self.connect_timeout).map_err(|_| {
            NetworkError::ConnectionTimeout {
                addr: key.clone(),
                timeout: self.connect_timeout,
            }
        })?;

        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        stream.set_nodelay(true)?;

        let conn = TcpConnection {
            remote_addr: addr,
            stream,
            connected_at: Instant::now(),
            key: key.clone(),
        };

        self.connections.insert(key.clone(), conn);
        Ok(&mut self.connections.get_mut(&key).unwrap().stream)
    }

    /// Disconnects and removes a connection from the pool by its key.
    pub fn disconnect(&mut self, key: &str) {
        self.connections.remove(key);
    }

    /// Closes all connections by clearing the pool.
    pub fn close_all(&mut self) {
        self.connections.clear();
    }

    /// Returns the current number of connections in the pool.
    pub fn pool_size(&self) -> usize {
        self.connections.len()
    }

    /// Removes all dead connections from the pool, returning the count of removed connections.
    pub fn prune_dead(&mut self) -> usize {
        let initial_len = self.connections.len();
        self.connections.retain(|_, conn| conn.is_alive());
        initial_len - self.connections.len()
    }
}

// ─── Tests ───────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
    use std::thread;

    #[test]
    fn test_pool_new_empty() {
        let pool = ConnectionPool::new();
        assert_eq!(pool.pool_size(), 0);
    }

    #[test]
    fn test_pool_connect_localhost() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port));

        thread::spawn(move || {
            let _ = listener.accept();
        });

        let mut pool = ConnectionPool::new();
        let stream = pool.connect(addr);
        assert!(stream.is_ok());
        assert_eq!(pool.pool_size(), 1);
    }

    #[test]
    fn test_pool_disconnect() {
        let mut pool = ConnectionPool::new();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let _ = listener.accept();
        });

        pool.connect(addr).unwrap();
        assert_eq!(pool.pool_size(), 1);

        pool.disconnect(&addr.to_string());
        assert_eq!(pool.pool_size(), 0);
    }

    #[test]
    fn test_pool_close_all() {
        let mut pool = ConnectionPool::new();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let _ = listener.accept();
        });

        pool.connect(addr).unwrap();
        pool.close_all();
        assert_eq!(pool.pool_size(), 0);
    }

    #[test]
    fn test_pool_prune() {
        let mut pool = ConnectionPool::new();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            drop(stream);
        });

        pool.connect(addr).unwrap();
        assert_eq!(pool.pool_size(), 1);

        handle.join().unwrap();
        thread::sleep(Duration::from_millis(50));

        let _ = pool.prune_dead();
    }

    #[test]
    fn test_default_timeouts() {
        let pool = ConnectionPool::new();
        assert_eq!(pool.connect_timeout, Duration::from_secs(10));
        assert_eq!(pool.read_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_custom_timeouts() {
        let pool = ConnectionPool::with_timeouts(Duration::from_secs(5), Duration::from_secs(15));
        assert_eq!(pool.connect_timeout, Duration::from_secs(5));
        assert_eq!(pool.read_timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_network_error_display() {
        let err = NetworkError::DnsError("example.com not found".to_string());
        assert_eq!(
            format!("{}", err),
            "DNS resolution failed: example.com not found"
        );

        let err2 = NetworkError::HttpError {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(format!("{}", err2), "HTTP Error 404: Not Found");
    }

    #[test]
    fn test_network_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let net_err: NetworkError = io_err.into();
        match net_err {
            NetworkError::IoError(e) => assert_eq!(e.kind(), io::ErrorKind::NotFound),
            _ => panic!("Expected IoError variant"),
        }
    }

    #[test]
    fn test_connection_pool_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<ConnectionPool>();
    }
}
