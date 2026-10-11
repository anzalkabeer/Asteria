# TECHNICAL SPECIFICATION: SPEC-03
## RFC 9110 Compliant HTTP URL Normalization & Request Line

---

## 1. Problem Statement & Motivation
Currently in `src/net/http.rs:47-70` and `src/net/url.rs:48-96`, Asteria formats the HTTP request line using `url.path` verbatim without stripping fragment identifiers (`#`) or properly handling queries (`?`), causing non-compliant requests that fail on standards-compliant HTTP servers (`AST-DEF-003`).

---

## 2. Root Cause
The URL parser stores raw URL path segments without distinguishing the origin request target from client-side URL fragments.

---

## 3. Desired Architecture
1. **RFC 9110 Request Target Normalization:**
In `src/net/http.rs`:
```rust
pub fn format_request_target(url: &Url) -> String {
    let path = if url.path.is_empty() { "/" } else { &url.path };
    let path_without_fragment = path.split('#').next().unwrap_or("/");
    match &url.query {
        Some(query) if !query.is_empty() => format!("{}?{}", path_without_fragment, query),
        _ => path_without_fragment.to_string(),
    }
}
```
2. **Request Line Format:**
Ensure the request line matches:
```http
GET /path?key=value HTTP/1.1\r\n
Host: example.com\r\n
User-Agent: Asteria/0.1\r\n
Connection: close\r\n
\r\n
```

---

## 4. Affected Source Locations
- `src/net/http.rs`
- `src/net/url.rs`
- Target crate: `crates/asteria_net`

---

## 5. Verification & Acceptance Criteria
1. `test_url_request_target_strips_fragment`: URL `http://example.com/doc#section1` formats request line as `GET /doc HTTP/1.1`.
2. `test_url_request_target_preserves_query`: URL `http://example.com/search?q=rust#top` formats request line as `GET /search?q=rust HTTP/1.1`.
