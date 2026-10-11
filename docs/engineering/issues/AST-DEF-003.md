# ISSUE: AST-DEF-003 — RFC 9110 URL Query/Fragment Parser Bug

- **Classification:** CONFIRMED DEFECT
- **Severity:** High
- **Priority:** P0
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Networking Client & URL Parser
- **Specification Link:** [`../specifications/SPEC-03.md`](../specifications/SPEC-03.md)

---

## 1. Problem Statement
When fetching URLs containing fragment identifiers (e.g. `http://example.com/page#section`), Asteria transmits the fragment `#section` across the wire in the HTTP request line (e.g. `GET /page#section HTTP/1.1`), directly violating RFC 9110 and causing strict web servers to return HTTP 400 Bad Request.

---

## 2. Evidence & Source Location
- **Location:** [`src/net/http.rs:47-70`](../../../src/net/http.rs#L47-L70) & [`src/net/url.rs:48-96`](../../../src/net/url.rs#L48-L96)
```rust
let request = format!(
    "GET {} HTTP/1.1\r\nHost: {}\r\n...",
    url.path, url.host // url.path includes `#section`
);
```

---

## 3. Root Cause
The URL parser fails to separate client-side fragments (`#`) from request targets prior to formatting HTTP request lines.

---

## 4. Desired Behaviour
Implement RFC 9110 request target normalization: strip fragments from the request line while preserving valid query strings (`?key=val`).
