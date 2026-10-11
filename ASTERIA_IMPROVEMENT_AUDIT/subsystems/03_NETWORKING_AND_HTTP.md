# ASTERIA SUBSYSTEM AUDIT: NETWORKING AND HTTP

## 1. Subsystem Scope & Files
- **Primary Source Modules:**
  - [`src/net/http.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/http.rs)
  - [`src/net/fetch.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/fetch.rs)
  - [`src/net/url.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/net/url.rs)

---

## 2. Architectural Analysis

### Strengths
1. **Zero External Heavyweight Networking Crate Overhead:** Asteria implements custom TCP socket networking using standard library `std::net::TcpStream`, keeping the engine lightweight.
2. **Support for HTTPS via rustls:** Asteria supports TLS encrypted sessions using pure-Rust `rustls` without requiring OpenSSL platform dependencies.

### Deficiencies Identified
1. **RFC 9110 URL Query/Fragment Bug (`AST-DEF-003`):** In `http.rs:47-70`, HTTP GET request lines transmit `url.path` directly without stripping `#` fragments or separating `?` query strings properly.
2. **Unbounded Response Buffering (`AST-DEF-004`):** Large responses are accumulated synchronously into an unbounded `Vec<u8>`, creating potential out-of-memory denial-of-service risks.
3. **Synchronous Blocking IO on Threadpool (`AST-PERF-002`):** Networking operations block threads in the background threadpool, limiting concurrency during slow subresource fetches.
4. **Unrestricted `file://` Scheme Access (`AST-SEC-001`):** Local file URIs are loaded directly from disk without directory jailing or path traversal protections.

---

## 3. Comparison with Servo (Net & Resource Pipeline)
Servo uses an asynchronous IPC-isolated network process (`asteria_servo_audit/subsystems/07_NETWORKING_AND_RESOURCE_LOADING.md`) based on Hyper, Tokio, and cross-thread channels.

**Decision:** In Stage 1, Asteria should fix the RFC 9110 parsing and buffer bounding bugs (`SPEC-03`, `SPEC-04`). In Stage 4/5, Asteria should migrate to an asynchronous Tokio/reqwest network stack (`ENH-05`) within the isolated `asteria_net` crate.
