# SPEC-03: RFC 9110 Compliant HTTP URL Normalization & Request Target

- **Related Issue:** [`../issues/AST-DEF-003.md`](../issues/AST-DEF-003.md)
- **Target Subsystem:** Networking Client (`src/net/http.rs`, `src/net/url.rs`)
- **Stage:** Stage 1 (P0)

---

## 1. Problem Statement
In `src/net/http.rs:47-70`, Asteria formats the HTTP request line using `url.path` verbatim without stripping fragment identifiers (`#`), transmitting `#` fragments across the wire and violating RFC 9110 Section 7.1.

---

## 2. Root Cause
The URL parser stores raw path segments without separating client-side fragments from server request targets.

---

## 3. Recommended Solution
Implement RFC 9110 request target normalization in `src/net/http.rs`:
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

---

## 4. Affected Source Locations
- `src/net/http.rs`
- `src/net/url.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_url_request_target_strips_fragment`: URL `http://example.com/doc#sec` formats request line as `GET /doc HTTP/1.1`.
2. `test_url_request_target_preserves_query`: URL `http://example.com/search?q=rust#top` formats request line as `GET /search?q=rust HTTP/1.1`.
