# SPEC-04: Bounded Streaming Network Buffer with Backpressure

- **Related Issue:** [`../issues/AST-DEF-004.md`](../issues/AST-DEF-004.md), [`../issues/AST-PERF-002.md`](../issues/AST-PERF-002.md)
- **Target Subsystem:** Networking Client (`src/net/http.rs`, `src/net/fetch.rs`)
- **Stage:** Stage 1 (P0)

---

## 1. Problem Statement
In `src/net/http.rs:163-195`, socket response bodies are read continuously into an unbounded `Vec<u8>`, risking memory exhaustion panics on large or malicious web resources.

---

## 2. Root Cause
Lack of a maximum response size limit and chunked stream bounding.

---

## 3. Recommended Solution
Enforce a 100 MB maximum body limit with bounded chunk streaming:
```rust
pub const MAX_RESPONSE_BODY_SIZE: usize = 100 * 1024 * 1024; // 100 MB

pub fn read_bounded_body<R: std::io::Read>(mut reader: R) -> Result<Vec<u8>, HttpError> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 { break; }
        if buffer.len() + n > MAX_RESPONSE_BODY_SIZE {
            return Err(HttpError::PayloadTooLarge);
        }
        buffer.extend_from_slice(&chunk[..n]);
    }
    Ok(buffer)
}
```

---

## 4. Affected Source Locations
- `src/net/http.rs`
- `src/net/fetch.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_bounded_body_rejects_oversized_payload`: Payloads exceeding 100 MB return `HttpError::PayloadTooLarge` without panic.
2. Standard web payloads under 100 MB stream cleanly into the engine.
