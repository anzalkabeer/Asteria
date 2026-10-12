# ISSUE: AST-DEF-004 — Unbounded Synchronous Network Buffer Streaming

- **Classification:** CONFIRMED DEFECT / RELIABILITY
- **Severity:** High
- **Priority:** P0
- **Status:** **RESOLVED**
- **Affected Subsystem:** Networking & HTTP Client
- **Specification Link:** [`../specifications/SPEC-04.md`](../specifications/SPEC-04.md)

---

## 1. Problem Statement
The HTTP client in `src/net/http.rs:163-195` reads response bodies by continuously appending socket chunks into an unbounded `Vec<u8>`. If a malicious or misconfigured server streams an infinite or massive payload, Asteria will exhaust available system RAM and crash with an out-of-memory (OOM) panic.

---

## 2. Evidence & Source Location
- **Location:** [`src/net/http.rs:163-195`](../../../src/net/http.rs#L163-L195)
```rust
let mut body = Vec::new();
stream.read_to_end(&mut body)?; // Unbounded read with zero size limit or backpressure
```

---

## 3. Root Cause
The socket reader lacks a configurable maximum response size check and does not support bounded streaming chunk transfers.

---

## 4. Desired Behaviour
Enforce a strict 100 MiB body limit default, reject oversize payloads with an explicit error, and support streaming chunks into the HTML parser.

---

## 5. Resolution & Verification
- Enforced `pub const MAX_RESPONSE_BODY_SIZE: usize = 100 * 1024 * 1024;` (100 MiB) limit across all body reading routines in `src/net/http.rs`.
- Guarded `read_exact_body`, `read_chunked_body` (both single chunk and accumulated chunks), and unbounded EOF loops against exceeding `MAX_RESPONSE_BODY_SIZE`.
- Guarded streaming pipelines (`stream_chunked_body`, `stream_exact_body`, `stream_eof_body`) with bounded size checks.
- Verified via unit tests: `test_read_exact_body_rejects_oversized`, `test_read_exact_body_accepts_within_limit`, `test_read_chunked_body_bounded_rejects_oversized`.
