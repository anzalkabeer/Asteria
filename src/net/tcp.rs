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
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use crate::net::tls::TlsConnection;

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
    /// TLS connection error.
    TlsError(String),
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
            NetworkError::Io(msg) => write!(f, "I/O error: {}", msg),
            NetworkError::Other(msg) => write!(f, "Network error: {}", msg),
            NetworkError::TlsError(msg) => write!(f, "TLS error: {}", msg),
        }
    }
}

impl From<io::Error> for NetworkError {
    fn from(error: io::Error) -> Self {
        NetworkError::IoError(error)
    }
}

// ─── Stream Enum ───────────

/// Represents either a plain TCP stream or a TLS-encrypted stream.
pub enum Stream {
    Plain(TcpStream),
    Tls(Box<TlsConnection>),
}

impl Stream {
    pub fn peek(&self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.peek(buf),
            Stream::Tls(t) => t.get_ref().sock.peek(buf),
        }
    }

    pub fn peek_nonblocking(&self, buf: &mut [u8]) -> io::Result<usize> {
        let sock = match self {
            Stream::Plain(s) => s,
            Stream::Tls(t) => &t.get_ref().sock,
        };
        sock.set_nonblocking(true)?;
        let res = sock.peek(buf);
        let _ = sock.set_nonblocking(false);
        res
    }

    pub fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.set_read_timeout(dur),
            Stream::Tls(t) => t.get_ref().sock.set_read_timeout(dur),
        }
    }

    pub fn set_write_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.set_write_timeout(dur),
            Stream::Tls(t) => t.get_ref().sock.set_write_timeout(dur),
        }
    }

    pub fn set_nodelay(&self, nodelay: bool) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.set_nodelay(nodelay),
            Stream::Tls(t) => t.get_ref().sock.set_nodelay(nodelay),
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.read(buf),
            Stream::Tls(t) => t.get_mut().read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Stream::Plain(s) => s.write(buf),
            Stream::Tls(t) => t.get_mut().write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Stream::Plain(s) => s.flush(),
            Stream::Tls(t) => t.get_mut().flush(),
        }
    }
}

// ─── TcpConnection Struct ───────────

/// Represents a single active network connection (plain or TLS).
pub struct TcpConnection {
    /// The remote address this connection is bound to.
    pub remote_addr: SocketAddr,
    /// The underlying stream (plain TCP or TLS).
    pub stream: Stream,
    /// When the connection was established.
    pub connected_at: Instant,
    /// When the connection was last accessed.
    pub last_used_at: Instant,
    /// The unique identifier in the pool, typically `host:port` or `ip:port`.
    pub key: String,
}

impl fmt::Debug for TcpConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TcpConnection")
            .field("remote_addr", &self.remote_addr)
            .field("key", &self.key)
            .field("last_used_at", &self.last_used_at)
            .finish()
    }
}

impl TcpConnection {
    /// Checks if the connection is still alive by performing a non-blocking
    /// 1-byte peek. If the remote end has closed the connection (Ok(0)) or
    /// an unexpected error occurs, the connection is considered dead.
    pub fn is_alive(&self) -> bool {
        let mut buf = [0u8; 1];
        match self.stream.peek_nonblocking(&mut buf) {
            Ok(0) => false,
            Ok(_) => true,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => true,
            Err(_) => false,
        }
    }
}

// ─── ConnectionPool Struct ───────────

/// Manages a pool of active TCP connections to reduce connection overhead
/// for subsequent requests to the same host with idle timeout and capacity management.
#[derive(Debug)]
pub struct ConnectionPool {
    /// Active connections keyed by their host and port.
    connections: HashMap<String, TcpConnection>,
    /// Timeout for establishing new connections.
    connect_timeout: Duration,
    /// Timeout for reading data from established connections.
    read_timeout: Duration,
    /// Maximum duration an unused connection remains in the pool before eviction.
    idle_timeout: Duration,
    /// Maximum total concurrent connections across all hosts.
    max_total_connections: usize,
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionPool {
    /// Creates a new connection pool with default timeouts and limits.
    /// Default connect timeout: 10 seconds.
    /// Default read timeout: 30 seconds.
    /// Default idle timeout: 60 seconds.
    /// Default max total connections: 32.
    pub fn new() -> Self {
        Self::with_timeouts_and_limits(
            Duration::from_secs(10),
            Duration::from_secs(30),
            Duration::from_secs(60),
            32,
        )
    }

    /// Creates a new connection pool with custom timeouts.
    pub fn with_timeouts(connect: Duration, read: Duration) -> Self {
        Self::with_timeouts_and_limits(connect, read, Duration::from_secs(60), 32)
    }

    /// Creates a new connection pool with custom timeouts, idle expiration, and capacity limits.
    /// A `max_total` of 0 indicates unlimited connections.
    pub fn with_timeouts_and_limits(
        connect: Duration,
        read: Duration,
        idle: Duration,
        max_total: usize,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            connect_timeout: connect,
            read_timeout: read,
            idle_timeout: idle,
            max_total_connections: max_total,
        }
    }

    /// Connects to a specific IP address, reusing an existing connection if alive and not expired.
    /// If a connection exists but is dead or expired, it is replaced.
    pub fn connect(&mut self, addr: SocketAddr) -> Result<&mut Stream, NetworkError> {
        let key = addr.to_string();

        self.prune_idle();

        if let Some(conn) = self.connections.get_mut(&key)
            && conn.is_alive()
            && conn.last_used_at.elapsed() <= self.idle_timeout
        {
            conn.last_used_at = Instant::now();
            return Ok(&mut self.connections.get_mut(&key).unwrap().stream);
        }

        self.ensure_capacity();

        let stream = TcpStream::connect_timeout(&addr, self.connect_timeout).map_err(|_| {
            NetworkError::ConnectionTimeout {
                addr: key.clone(),
                timeout: self.connect_timeout,
            }
        })?;

        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        stream.set_nodelay(true)?;

        let now = Instant::now();
        let conn = TcpConnection {
            remote_addr: addr,
            stream: Stream::Plain(stream),
            connected_at: now,
            last_used_at: now,
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
    ) -> Result<&mut Stream, NetworkError> {
        let key = format!("{}:{}", host, port);

        self.prune_idle();

        if let Some(conn) = self.connections.get_mut(&key)
            && conn.is_alive()
            && conn.last_used_at.elapsed() <= self.idle_timeout
        {
            conn.last_used_at = Instant::now();
            return Ok(&mut self.connections.get_mut(&key).unwrap().stream);
        }

        self.ensure_capacity();

        let stream = TcpStream::connect_timeout(&addr, self.connect_timeout).map_err(|_| {
            NetworkError::ConnectionTimeout {
                addr: key.clone(),
                timeout: self.connect_timeout,
            }
        })?;

        stream.set_read_timeout(Some(self.read_timeout))?;
        stream.set_write_timeout(Some(self.connect_timeout))?;
        stream.set_nodelay(true)?;

        let now = Instant::now();
        let conn = TcpConnection {
            remote_addr: addr,
            stream: Stream::Plain(stream),
            connected_at: now,
            last_used_at: now,
            key: key.clone(),
        };

        self.connections.insert(key.clone(), conn);
        Ok(&mut self.connections.get_mut(&key).unwrap().stream)
    }

    /// Inserts an externally created connection (e.g., a TLS upgraded stream) into the pool.
    pub fn insert(&mut self, key: String, mut conn: TcpConnection) -> &mut Stream {
        self.prune_idle();
        self.ensure_capacity();
        conn.last_used_at = Instant::now();
        self.connections.insert(key.clone(), conn);
        &mut self.connections.get_mut(&key).unwrap().stream
    }

    /// Gets an existing connection if alive and not expired, otherwise returns None.
    pub fn get(&mut self, key: &str) -> Option<&mut Stream> {
        if let Some(conn) = self.connections.get(key)
            && (!conn.is_alive() || conn.last_used_at.elapsed() > self.idle_timeout)
        {
            self.connections.remove(key);
            return None;
        }
        if let Some(conn) = self.connections.get_mut(key) {
            conn.last_used_at = Instant::now();
            return Some(&mut conn.stream);
        }
        None
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

    /// Evicts expired idle connections and dead connections. Returns the count of removed connections.
    pub fn prune_idle(&mut self) -> usize {
        let initial_len = self.connections.len();
        let idle_timeout = self.idle_timeout;
        self.connections
            .retain(|_, conn| conn.is_alive() && conn.last_used_at.elapsed() <= idle_timeout);
        initial_len - self.connections.len()
    }

    /// Ensures that pool size is strictly below `max_total_connections`.
    /// Evicts the connection with the oldest `last_used_at` timestamp if capacity is reached.
    /// If `max_total_connections` is 0, capacity is treated as unlimited.
    fn ensure_capacity(&mut self) {
        if self.max_total_connections == 0 {
            return;
        }
        while self.connections.len() >= self.max_total_connections {
            if let Some(oldest_key) = self
                .connections
                .iter()
                .min_by_key(|(_, conn)| conn.last_used_at)
                .map(|(k, _)| k.clone())
            {
                self.connections.remove(&oldest_key);
            } else {
                break;
            }
        }
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

    #[test]
    fn test_pool_idle_timeout_settings() {
        let pool = ConnectionPool::with_timeouts_and_limits(
            Duration::from_secs(5),
            Duration::from_secs(10),
            Duration::from_secs(1),
            10,
        );
        assert_eq!(pool.idle_timeout, Duration::from_secs(1));
        assert_eq!(pool.max_total_connections, 10);
    }

    #[test]
    fn test_closed_peer_not_reused() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            // Close connection immediately by dropping stream
            drop(stream);
        });

        let mut pool = ConnectionPool::new();
        let _ = pool.connect(addr);
        handle.join().unwrap();

        // Give the OS network stack a tiny moment to complete FIN handshake
        thread::sleep(Duration::from_millis(50));

        // Attempting to get the connection from the pool should recognize it as closed (dead)
        assert!(pool.get(&addr.to_string()).is_none());
        assert_eq!(pool.pool_size(), 0);
    }

    #[test]
    fn test_pool_zero_capacity_unlimited() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let handle = thread::spawn(move || {
            let mut streams = Vec::new();
            for _ in 0..2 {
                let (stream, _) = listener.accept().unwrap();
                streams.push(stream);
            }
            // Keep streams open until thread is done
            thread::sleep(Duration::from_millis(100));
        });

        let mut pool = ConnectionPool::with_timeouts_and_limits(
            Duration::from_secs(5),
            Duration::from_secs(5),
            Duration::from_secs(60),
            0, // zero capacity means unlimited
        );

        let tcp1 = TcpStream::connect(addr).unwrap();
        let tcp2 = TcpStream::connect(addr).unwrap();

        let now = Instant::now();
        pool.insert(
            "k1".into(),
            TcpConnection {
                remote_addr: addr,
                stream: Stream::Plain(tcp1),
                connected_at: now,
                last_used_at: now,
                key: "k1".into(),
            },
        );
        pool.insert(
            "k2".into(),
            TcpConnection {
                remote_addr: addr,
                stream: Stream::Plain(tcp2),
                connected_at: now,
                last_used_at: now,
                key: "k2".into(),
            },
        );

        assert_eq!(pool.pool_size(), 2);
        handle.join().unwrap();
    }
}
