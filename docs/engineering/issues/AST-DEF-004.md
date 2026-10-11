# ISSUE: AST-DEF-004 — Unbounded Synchronous Network Buffer Streaming

- **Classification:** CONFIRMED DEFECT / RELIABILITY
- **Severity:** High
- **Priority:** P0
- **Status:** **CONFIRMED**
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
Enforce a strict 100 MB body limit default, reject oversize payloads with an explicit error, and support streaming chunks into the HTML parser.
