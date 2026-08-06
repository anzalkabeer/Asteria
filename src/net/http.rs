// ─── Imports ───────────
use std::fmt;
use std::io::Write;
use std::net::TcpStream;

use super::dns::DnsResolver;
use super::tcp::{ConnectionPool, NetworkError, Stream, TcpConnection};
use super::tls::TlsConnector;

// ─── URL Parsing ───────────

/// Represents a parsed URL for the HTTP client.
#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
    pub raw: String,
}

impl Url {
    /// Parses a URL string into a `Url` struct.
    ///
    /// Must handle: `http://host`, `http://host:port`, `http://host/path`, `http://host:port/path`
    /// Defaults to port 80 for HTTP and path "/" if not specified.
    pub fn parse(url: &str) -> Result<Url, NetworkError> {
        let raw = url.to_string();

        let scheme_end = url.find("://").ok_or_else(|| {
            NetworkError::InvalidUrl("Missing scheme (expected 'http://')".into())
        })?;

        let scheme = &url[..scheme_end];
        if scheme != "http" && scheme != "https" {
            return Err(NetworkError::InvalidUrl(format!(
                "Unsupported scheme: {}",
                scheme
            )));
        }

        let rest = &url[scheme_end + 3..];
        if rest.is_empty() {
            return Err(NetworkError::InvalidUrl("Empty host".into()));
        }

        let (host_port, path) = if let Some(path_start) = rest.find('/') {
            (&rest[..path_start], &rest[path_start..])
        } else {
            (rest, "/")
        };

        if host_port.is_empty() {
            return Err(NetworkError::InvalidUrl("Empty host".into()));
        }

        let (host, port) = if let Some(colon_pos) = host_port.find(':') {
            let h = &host_port[..colon_pos];
            if h.is_empty() {
                return Err(NetworkError::InvalidUrl("Empty host".into()));
            }
            let p_str = &host_port[colon_pos + 1..];
            let p = p_str
                .parse::<u16>()
                .map_err(|_| NetworkError::InvalidUrl("Invalid port number".into()))?;
            (h, p)
        } else {
            (host_port, if scheme == "https" { 443 } else { 80 })
        };

        Ok(Url {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
            path: path.to_string(),
            raw,
        })
    }

    /// Returns the host and port in "host:port" format.
    pub fn host_port(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Resolves a relative or absolute URL string against this base URL.
    pub fn resolve_relative(&self, location: &str) -> Result<Url, NetworkError> {
        if location.starts_with("http://") || location.starts_with("https://") {
            Url::parse(location)
        } else if location.starts_with('/') {
            let new_url_str = format!("{}://{}{}", self.scheme, self.host_port(), location);
            Url::parse(&new_url_str)
        } else {
            // Relative path not starting with '/'
            let mut path_base = self.path.clone();
            if let Some(last_slash) = path_base.rfind('/') {
                path_base.truncate(last_slash + 1);
            } else {
                path_base = "/".to_string();
            }
            let new_url_str = format!(
                "{}://{}{}{}",
                self.scheme,
                self.host_port(),
                path_base,
                location
            );
            Url::parse(&new_url_str)
        }
    }
}

// ─── HTTP Method ───────────

/// Represents an HTTP method.
#[derive(Debug, Clone, PartialEq)]
pub enum HttpMethod {
    Get,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
        }
    }
}

// ─── HTTP Request ───────────

/// Represents an outgoing HTTP request.
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: Url,
    pub headers: Vec<(String, String)>,
}

impl HttpRequest {
    /// Serializes the request into HTTP/1.1 wire format bytes.
    pub fn to_request_bytes(&self) -> Vec<u8> {
        let mut req = String::new();
        req.push_str(&format!("{} {} HTTP/1.1\r\n", self.method, self.url.path));

        let mut has_host = false;
        let mut has_connection = false;
        let mut has_user_agent = false;
        let mut has_accept = false;

        for (k, v) in &self.headers {
            req.push_str(&format!("{}: {}\r\n", k, v));
            let lower_k = k.to_lowercase();
            if lower_k == "host" {
                has_host = true;
            }
            if lower_k == "connection" {
                has_connection = true;
            }
            if lower_k == "user-agent" {
                has_user_agent = true;
            }
            if lower_k == "accept" {
                has_accept = true;
            }
        }

        if !has_host {
            let host_header = if self.url.port == 80 {
                self.url.host.clone()
            } else {
                self.url.host_port()
            };
            req.push_str(&format!("Host: {}\r\n", host_header));
        }
        if !has_connection {
            req.push_str("Connection: keep-alive\r\n");
        }
        if !has_user_agent {
            req.push_str("User-Agent: Asteria/0.1\r\n");
        }
        if !has_accept {
            req.push_str("Accept: text/html,text/css,*/*\r\n");
        }

        req.push_str("\r\n");
        req.into_bytes()
    }
}

// ─── HTTP Response ───────────

/// Represents an incoming HTTP response.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status_code: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub url: String,
}

impl HttpResponse {
    /// Performs a case-insensitive lookup for a header.
    pub fn header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers.iter().find_map(|(k, v)| {
            if k.to_lowercase() == name_lower {
                Some(v.as_str())
            } else {
                None
            }
        })
    }

    /// Shortcut to get the Content-Type header.
    pub fn content_type(&self) -> Option<&str> {
        self.header("Content-Type")
    }

    /// Returns true if the status code indicates a redirect (301, 302, 303, 307, 308).
    pub fn is_redirect(&self) -> bool {
        matches!(self.status_code, 301 | 302 | 303 | 307 | 308)
    }

    /// Returns the Location header value if present.
    pub fn redirect_location(&self) -> Option<&str> {
        self.header("Location")
    }

    /// Returns true if the status code is a success (200-299).
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status_code)
    }

    /// Attempts to parse the response body as a UTF-8 string.
    pub fn body_as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.body).ok()
    }
}

// ─── HTTP Client ───────────

/// The main HTTP client for fetching web resources.
pub struct HttpClient {
    pub dns: DnsResolver,
    pub pool: ConnectionPool,
    pub tls: TlsConnector,
    pub max_redirects: usize,
}

impl HttpClient {
    /// Creates a new `HttpClient` with default settings.
    pub fn new() -> Self {
        Self {
            dns: DnsResolver::new(),
            pool: ConnectionPool::new(),
            tls: TlsConnector::new(),
            max_redirects: 5,
        }
    }

    /// Executes an HTTP GET request for the given URL, following redirects.
    pub fn get(&mut self, url: &str) -> Result<HttpResponse, NetworkError> {
        let mut current_url = Url::parse(url)?;
        let mut redirects = 0;

        loop {
            let request = HttpRequest {
                method: HttpMethod::Get,
                url: current_url.clone(),
                headers: Vec::new(),
            };

            let response = self.send_request(&request)?;

            if response.is_redirect() {
                if redirects >= self.max_redirects {
                    return Err(NetworkError::Other("Too many redirects".into()));
                }

                if let Some(location) = response.redirect_location() {
                    current_url = current_url.resolve_relative(location)?;
                    redirects += 1;
                    continue;
                }
            }

            return Ok(response);
        }
    }

    /// Sends a prepared `HttpRequest` and returns the `HttpResponse`.
    pub fn send_request(&mut self, request: &HttpRequest) -> Result<HttpResponse, NetworkError> {
        // 1. DNS resolve
        let dns_entry = self
            .dns
            .resolve(&request.url.host)
            .map_err(|e| NetworkError::DnsError(format!("{}", e)))?;

        // 2. Pick the first resolved IP and build a SocketAddr
        let ip = dns_entry.ip_addresses.first().ok_or_else(|| {
            NetworkError::DnsError(format!(
                "No IP addresses resolved for '{}'",
                request.url.host
            ))
        })?;
        let addr = std::net::SocketAddr::new(*ip, request.url.port);

        let key = format!("{}:{}", request.url.host, request.url.port);

        if self.pool.get(&key).is_none() {
            let tcp = TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(10))
                .map_err(|_| NetworkError::ConnectionTimeout {
                    addr: key.clone(),
                    timeout: std::time::Duration::from_secs(10),
                })?;
            let _ = tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)));
            let _ = tcp.set_write_timeout(Some(std::time::Duration::from_secs(10)));
            let _ = tcp.set_nodelay(true);

            let stream_wrapper = if request.url.scheme == "https" {
                Stream::Tls(Box::new(self.tls.connect(&request.url.host, tcp)?))
            } else {
                Stream::Plain(tcp)
            };

            let conn = TcpConnection {
                remote_addr: addr,
                stream: stream_wrapper,
                connected_at: std::time::Instant::now(),
                key: key.clone(),
            };
            self.pool.insert(key.clone(), conn);
        }

        let stream = self.pool.get(&key).unwrap();

        // 4. Write the HTTP request
        let req_bytes = request.to_request_bytes();
        stream
            .write_all(&req_bytes)
            .map_err(|e| NetworkError::WriteError {
                message: e.to_string(),
            })?;

        // 5. Read the response
        let mut response = read_response(stream)?;
        response.url = request.url.raw.clone();

        Ok(response)
    }
    pub fn stream(
        &mut self,
        url: &str,
        sender: std::sync::mpsc::Sender<crate::net::bus::ResourceBusEvent>,
    ) -> Result<(), NetworkError> {
        let current_url = Url::parse(url)?;
        let request = HttpRequest {
            method: HttpMethod::Get,
            url: current_url.clone(),
            headers: Vec::new(),
        };

        // 1. DNS resolve
        let dns_entry = self
            .dns
            .resolve(&request.url.host)
            .map_err(|e| NetworkError::DnsError(format!("{}", e)))?;

        // 2. Pick the first resolved IP and build a SocketAddr
        let ip = dns_entry.ip_addresses.first().ok_or_else(|| {
            NetworkError::DnsError(format!(
                "No IP addresses resolved for '{}'",
                request.url.host
            ))
        })?;
        let addr = std::net::SocketAddr::new(*ip, request.url.port);

        let key = format!("{}:{}", request.url.host, request.url.port);

        if self.pool.get(&key).is_none() {
            let tcp =
                std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_secs(10))
                    .map_err(|_| NetworkError::ConnectionTimeout {
                        addr: key.clone(),
                        timeout: std::time::Duration::from_secs(10),
                    })?;
            let _ = tcp.set_read_timeout(Some(std::time::Duration::from_secs(30)));
            let _ = tcp.set_write_timeout(Some(std::time::Duration::from_secs(10)));
            let _ = tcp.set_nodelay(true);

            let stream_wrapper = if request.url.scheme == "https" {
                Stream::Tls(Box::new(self.tls.connect(&request.url.host, tcp)?))
            } else {
                Stream::Plain(tcp)
            };

            let conn = TcpConnection {
                remote_addr: addr,
                stream: stream_wrapper,
                connected_at: std::time::Instant::now(),
                key: key.clone(),
            };
            self.pool.insert(key.clone(), conn);
        }

        let stream = self.pool.get(&key).unwrap();

        // Write request
        let req_bytes = request.to_request_bytes();
        stream
            .write_all(&req_bytes)
            .map_err(|e| NetworkError::WriteError {
                message: e.to_string(),
            })?;

        // Read headers
        let (status_code, _, headers) = read_response_headers(stream)?;

        let is_chunked = headers.iter().any(|(k, v)| {
            k.to_lowercase() == "transfer-encoding" && v.to_lowercase().contains("chunked")
        });
        let content_length = headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == "content-length")
            .and_then(|(_, v)| v.parse::<usize>().ok());
        let content_type = headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == "content-type")
            .map(|(_, v)| v.clone());

        let url_str = url.to_string();

        let _ = sender.send(crate::net::bus::ResourceBusEvent::HeaderReceived {
            url: url_str.clone(),
            status_code,
            content_length,
            content_type,
        });

        if is_chunked {
            stream_chunked_body(stream, &url_str, &sender)?;
        } else if let Some(len) = content_length {
            stream_exact_body(stream, len, &url_str, &sender)?;
        } else {
            stream_eof_body(stream, &url_str, &sender)?;
        }

        Ok(())
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Internal Parsing Helpers ───────────

#[allow(clippy::type_complexity)]
fn read_response_headers<R: std::io::Read>(
    stream: &mut R,
) -> Result<(u16, String, Vec<(String, String)>), NetworkError> {
    let mut header_buf = Vec::new();
    let mut byte = [0u8; 1];

    // Read headers byte-by-byte until \r\n\r\n
    loop {
        if stream.read_exact(&mut byte).is_err() {
            return Err(NetworkError::Io("Connection closed unexpectedly".into()));
        }
        header_buf.push(byte[0]);

        if header_buf.ends_with(b"\r\n\r\n") {
            break;
        }
        if header_buf.len() > 1024 * 1024 {
            // 1MB max header size protection
            return Err(NetworkError::Other("Headers too large".into()));
        }
    }

    let header_str = String::from_utf8_lossy(&header_buf);
    let mut lines = header_str.split("\r\n");

    // Parse status line
    let status_line = lines
        .next()
        .ok_or_else(|| NetworkError::Other("Empty response".into()))?;
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(NetworkError::Other("Invalid status line".into()));
    }

    let status_code: u16 = parts[1]
        .parse()
        .map_err(|_| NetworkError::Other("Invalid status code".into()))?;
    let status_text = parts.get(2).unwrap_or(&"").to_string();

    // Parse headers
    let mut headers = Vec::new();
    for line in lines {
        if line.is_empty() {
            break; // End of headers
        }
        if let Some(colon_idx) = line.find(':') {
            let key = line[..colon_idx].trim().to_string();
            let val = line[colon_idx + 1..].trim().to_string();
            headers.push((key, val));
        }
    }

    Ok((status_code, status_text, headers))
}

/// Reads and parses an HTTP response from a TCP stream.
fn read_response<R: std::io::Read>(stream: &mut R) -> Result<HttpResponse, NetworkError> {
    let (status_code, status_text, headers) = read_response_headers(stream)?;

    let is_chunked = headers.iter().any(|(k, v)| {
        k.to_lowercase() == "transfer-encoding" && v.to_lowercase().contains("chunked")
    });

    let content_length = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "content-length")
        .and_then(|(_, v)| v.parse::<usize>().ok());

    let body = if is_chunked {
        read_chunked_body(stream)?
    } else if let Some(len) = content_length {
        read_exact_body(stream, len)?
    } else {
        // No body length specified, read until EOF
        let mut body = Vec::new();
        let _ = stream.read_to_end(&mut body);
        body
    };

    Ok(HttpResponse {
        status_code,
        status_text,
        headers,
        body,
        url: String::new(), // Filled in by caller
    })
}

/// Reads a chunked HTTP response body.
fn read_chunked_body<R: std::io::Read>(stream: &mut R) -> Result<Vec<u8>, NetworkError> {
    let mut body = Vec::new();
    let mut byte = [0u8; 1];

    loop {
        // Read chunk size
        let mut size_str = String::new();
        loop {
            stream
                .read_exact(&mut byte)
                .map_err(|_| NetworkError::Io("Failed to read chunk size".into()))?;
            size_str.push(byte[0] as char);
            if size_str.ends_with("\r\n") {
                break;
            }
        }

        let hex_size = size_str.trim();
        let size = usize::from_str_radix(hex_size, 16)
            .map_err(|_| NetworkError::Other("Invalid chunk size format".into()))?;

        if size == 0 {
            // Read the final \r\n
            stream
                .read_exact(&mut [0u8; 2])
                .map_err(|_| NetworkError::Io("Failed to read chunk trailer".into()))?;
            break;
        }

        // Read chunk data
        let mut chunk = vec![0u8; size];
        stream
            .read_exact(&mut chunk)
            .map_err(|_| NetworkError::Io("Failed to read chunk data".into()))?;
        body.extend_from_slice(&chunk);

        // Read \r\n after chunk data
        stream
            .read_exact(&mut [0u8; 2])
            .map_err(|_| NetworkError::Io("Failed to read chunk newline".into()))?;
    }

    Ok(body)
}

/// Reads an exact number of bytes for the HTTP response body.
fn read_exact_body<R: std::io::Read>(
    stream: &mut R,
    length: usize,
) -> Result<Vec<u8>, NetworkError> {
    let mut body = vec![0u8; length];
    stream
        .read_exact(&mut body)
        .map_err(|_| NetworkError::Io("Failed to read exact body".into()))?;
    Ok(body)
}

fn stream_chunked_body<R: std::io::Read>(
    stream: &mut R,
    url: &str,
    sender: &std::sync::mpsc::Sender<crate::net::bus::ResourceBusEvent>,
) -> Result<(), NetworkError> {
    let mut offset = 0;
    loop {
        let mut size_buf = Vec::new();
        let mut byte = [0u8; 1];

        // Read chunk size
        loop {
            if stream.read_exact(&mut byte).is_err() {
                return Err(NetworkError::Io(
                    "Connection closed while reading chunk size".into(),
                ));
            }
            size_buf.push(byte[0]);
            if size_buf.ends_with(b"\r\n") {
                break;
            }
        }

        let size_str = String::from_utf8_lossy(&size_buf[..size_buf.len() - 2]);
        let size = usize::from_str_radix(size_str.trim(), 16)
            .map_err(|_| NetworkError::Other("Invalid chunk size".into()))?;

        if size == 0 {
            // Read trailing \r\n
            let mut trailing = [0u8; 2];
            let _ = stream.read_exact(&mut trailing);
            let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
                url: url.to_string(),
                chunk_data: Vec::new(),
                offset,
                is_final: true,
            });
            break;
        }

        let mut chunk = vec![0u8; size];
        stream
            .read_exact(&mut chunk)
            .map_err(|_| NetworkError::Io("Failed to read chunk data".into()))?;

        let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
            url: url.to_string(),
            chunk_data: chunk,
            offset,
            is_final: false,
        });
        offset += size;

        // Read trailing \r\n
        let mut trailing = [0u8; 2];
        stream
            .read_exact(&mut trailing)
            .map_err(|_| NetworkError::Io("Failed to read chunk trailing CRLF".into()))?;
    }
    Ok(())
}

fn stream_exact_body<R: std::io::Read>(
    stream: &mut R,
    length: usize,
    url: &str,
    sender: &std::sync::mpsc::Sender<crate::net::bus::ResourceBusEvent>,
) -> Result<(), NetworkError> {
    let mut remaining = length;
    let mut offset = 0;
    let mut buf = [0u8; 8192];

    while remaining > 0 {
        let to_read = std::cmp::min(remaining, buf.len());
        let n = stream
            .read(&mut buf[..to_read])
            .map_err(|_| NetworkError::Io("Read error".into()))?;
        if n == 0 {
            break;
        }
        remaining -= n;
        let is_final = remaining == 0;
        let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
            url: url.to_string(),
            chunk_data: buf[..n].to_vec(),
            offset,
            is_final,
        });
        offset += n;
    }

    // In case connection closed early
    if remaining > 0 {
        let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
            url: url.to_string(),
            chunk_data: Vec::new(),
            offset,
            is_final: true,
        });
    }

    Ok(())
}

fn stream_eof_body<R: std::io::Read>(
    stream: &mut R,
    url: &str,
    sender: &std::sync::mpsc::Sender<crate::net::bus::ResourceBusEvent>,
) -> Result<(), NetworkError> {
    let mut buf = [0u8; 8192];
    let mut offset = 0;
    loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
                    url: url.to_string(),
                    chunk_data: Vec::new(),
                    offset,
                    is_final: true,
                });
                break;
            }
            Ok(n) => {
                let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
                    url: url.to_string(),
                    chunk_data: buf[..n].to_vec(),
                    offset,
                    is_final: false,
                });
                offset += n;
            }
            Err(_) => {
                let _ = sender.send(crate::net::bus::ResourceBusEvent::ChunkReceived {
                    url: url.to_string(),
                    chunk_data: Vec::new(),
                    offset,
                    is_final: true,
                });
                break;
            }
        }
    }
    Ok(())
}

/// Resolves a redirect location against the current URL.
// ─── Tests ───────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parse_basic() {
        let url = Url::parse("http://example.com").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/");
    }

    #[test]
    fn test_url_parse_with_port() {
        let url = Url::parse("http://example.com:8080/page").unwrap();
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 8080);
        assert_eq!(url.path, "/page");
    }

    #[test]
    fn test_url_parse_with_path() {
        let url = Url::parse("http://example.com/path/to/page.html").unwrap();
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/path/to/page.html");
    }

    #[test]
    fn test_url_parse_no_scheme() {
        assert!(Url::parse("example.com").is_err());
    }

    #[test]
    fn test_url_parse_https_accepted() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.port, 443);
    }

    #[test]
    fn test_url_parse_empty_host() {
        assert!(Url::parse("http://").is_err());
    }

    #[test]
    fn test_url_host_port() {
        let url = Url::parse("http://example.com:8080").unwrap();
        assert_eq!(url.host_port(), "example.com:8080");
    }

    #[test]
    fn test_http_request_serialization() {
        let url = Url::parse("http://example.com/test").unwrap();
        let req = HttpRequest {
            method: HttpMethod::Get,
            url,
            headers: vec![("Custom-Header".to_string(), "Value".to_string())],
        };
        let bytes = req.to_request_bytes();
        let str_rep = String::from_utf8(bytes).unwrap();
        assert!(str_rep.contains("GET /test HTTP/1.1\r\n"));
        assert!(str_rep.contains("Host: example.com\r\n"));
        assert!(str_rep.contains("Custom-Header: Value\r\n"));
        assert!(str_rep.contains("Connection: keep-alive\r\n"));
        assert!(str_rep.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_http_response_header_lookup() {
        let resp = HttpResponse {
            status_code: 200,
            status_text: "OK".into(),
            headers: vec![("Content-Type".to_string(), "text/html".to_string())],
            body: vec![],
            url: "".into(),
        };
        assert_eq!(resp.header("content-type"), Some("text/html"));
        assert_eq!(resp.header("CONTENT-TYPE"), Some("text/html"));
        assert_eq!(resp.header("Accept"), None);
    }

    #[test]
    fn test_http_response_redirect_detection() {
        let mut resp = HttpResponse {
            status_code: 301,
            status_text: "Moved Permanently".into(),
            headers: vec![],
            body: vec![],
            url: "".into(),
        };
        assert!(resp.is_redirect());
        resp.status_code = 200;
        assert!(!resp.is_redirect());
    }

    #[test]
    fn test_http_response_success_detection() {
        let resp = HttpResponse {
            status_code: 204,
            status_text: "No Content".into(),
            headers: vec![],
            body: vec![],
            url: "".into(),
        };
        assert!(resp.is_success());
    }

    #[test]
    fn test_http_response_content_type() {
        let resp = HttpResponse {
            status_code: 200,
            status_text: "OK".into(),
            headers: vec![("Content-Type".to_string(), "application/json".to_string())],
            body: vec![],
            url: "".into(),
        };
        assert_eq!(resp.content_type(), Some("application/json"));
    }

    #[test]
    fn test_resolve_redirect_absolute() {
        let url = Url::parse("http://example.com/path").unwrap();
        let redirect = url.resolve_relative("http://new.com/page").unwrap();
        assert_eq!(redirect.host, "new.com");
        assert_eq!(redirect.path, "/page");
    }

    #[test]
    fn test_resolve_redirect_relative() {
        let url = Url::parse("http://example.com/dir/page.html").unwrap();
        let redirect = url.resolve_relative("/new-page.html").unwrap();
        assert_eq!(redirect.host, "example.com");
        assert_eq!(redirect.path, "/new-page.html");

        let redirect2 = url.resolve_relative("other.html").unwrap();
        assert_eq!(redirect2.path, "/dir/other.html");
    }
}
