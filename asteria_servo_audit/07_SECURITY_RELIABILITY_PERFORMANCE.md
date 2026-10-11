# 07 Security, Reliability, Performance & Resource Management

> **Evidence Status:** VERIFIED  
> **Source Commit:** `7ca9f80268a1d76e698560db82463e7960aa7845`  
> **Primary References:** `components/constellation/sandboxing.rs:1–60`, `components/background_hang_monitor/`, `components/malloc_size_of/`, `deny.toml:1–60`  
> **Classification Standard:** Rigorous distinction between VERIFIED facts, INFERRED risks, and RECOMMENDATIONS for Asteria.

---

## 1. Security Architecture & Threat Boundaries

### 1.1 Process Boundaries & Sandboxing (`sandboxing.rs`)
- **[VERIFIED] Multi-Process Mode:** When compiled with `feature = "multiprocess"`, content execution (`ScriptThread`, `Layout`) is isolated from the browser supervisor (`Constellation`) via separate OS processes communicating over OS pipes (`ipc-channel`).
- **[VERIFIED] OS Sandbox Limitations:** Sandboxing is governed by the `gaol` crate (`sandboxing.rs:8–20`). However:
  - macOS uses Seatbelt sandbox profiles (`content_process_sandbox_profile()`).
  - Linux uses seccomp / namespace isolation.
  - **Windows, Android, iOS, and OpenHarmony explicitly exclude `gaol`:**
    ```rust
    // components/constellation/sandboxing.rs:8-20
    #[cfg(any(
        target_os = "macos",
        all(not(target_os = "windows"), not(target_os = "ios"), not(target_os = "android"), not(target_env = "ohos"))
    ))]
    use gaol::profile::Profile;
    ```
  - **Finding `SEC-001` [VERIFIED]:** On Windows, Servo's content processes rely on memory boundary separation without OS token restriction or AppContainer sandboxing.

### 1.2 Origin Boundaries & Fetch Security
- **[VERIFIED] WHATWG Origin Invariants:** `ServoUrl` enforces origin equality. Cross-origin requests trigger CORS preflight (`OPTIONS`) in `components/net/fetch/cors_cache.rs`.
- **[VERIFIED] Content Security Policy (CSP):** Verified in `components/net/fetch/methods.rs:12`. CSP headers (`default-src`, `script-src`, `connect-src`) are parsed and enforced by the `content-security-policy` crate before dispatching network sockets.
- **[VERIFIED] Subresource Integrity (SRI):** Cryptographic hash checks (`sha256`, `sha384`) verify integrity on `<script>` and `<link>` tags in `subresource_integrity.rs`.

### 1.3 Memory Safety & Unsafe Rust Audit
- **[VERIFIED] Unsafe Containment:** Concrete crates enforce `#![deny(unsafe_code)]` at their root manifests wherever possible:
  - `components/script/lib.rs:6`: `#![deny(unsafe_code)]`.
  - `components/paint/lib.rs:5`: `#![deny(unsafe_code)]`.
  - `components/layout/lib.rs:5`: `#![deny(unsafe_code)]`.
- **[VERIFIED] Unsafe Boundaries:** Unsafe code is deliberately quarantined to:
  - `components/script_bindings`: SpiderMonkey GC pointer dereferencing and C++ FFI trampolines.
  - `components/fonts/shapers/harfbuzz.rs`: C FFI calls to HarfBuzz pointers.
  - `ffi/capi/lib.rs`: Exported `extern "C"` ABI functions.
  - `components/shared/layout/layout_element.rs:60–68`: `unsafe fn dangerous_style_element()`.

---

## 2. Reliability & Fault Tolerance

### 2.1 Supervisor Crash Isolation
- **Finding `REL-001` [VERIFIED]:** A panic in a web page's JavaScript execution or layout does **not** crash the browser shell.
  - The script thread wraps top-level task loops. If a thread panics, the crossbeam / IPC channel disconnects.
  - Constellation detects the dropped sender, marks the pipeline dead, sends `ServoError::LostConnectionWithBackend` to `ServoDelegate`, and presents an error page (`resources/error.html`).

### 2.2 Background Hang Monitor (`components/background_hang_monitor`)
- **Finding `REL-002` [VERIFIED]:** Monitors worker thread responsiveness.
  - Worker threads register with `BackgroundHangMonitorRegister` and ping a monotonic heartbeat.
  - If a script thread enters an infinite loop (`while(true) {}`) and fails to tick within the hang deadline (default: 5 seconds), the monitor sends `HangAlert` to the Constellation.
  - The embedder is notified via `ServoDelegate` to offer the user a "Kill Page" prompt.

### 2.3 IPC Deadlock Prevention & Router Threads
- **Finding `REL-003` [VERIFIED]:** Operating system pipe buffers (`ipc-channel`) block on write when filled (typically 64KB on Windows). If thread $A$ blocks sending to thread $B$ while thread $B$ blocks sending to thread $A$, a permanent deadlock occurs.
- Servo resolves this via dedicated `RouterThread` instances that continuously drain IPC pipes into unbounded in-memory crossbeam queues (`components/shared/base/generic_channel.rs`).

---

## 3. Performance & Resource Management

### 3.1 Heap Footprint Measurement (`components/malloc_size_of`)
- **Finding `PERF-001` [VERIFIED]:** Every long-lived heap object implements the `MallocSizeOf` trait (`malloc_size_of::MallocSizeOf`).
- Allows Servo to accurately report live memory across all subsystems:
  - DOM tree heap size.
  - Stylo RuleTree and cached stylesheet memory.
  - WebRender font and image texture cache footprint.
- Cooperative memory reporting (`create_memory_report`) allows the embedder to flush image caches under OS memory pressure.

### 3.2 Rayon Parallelism in Style & Layout
- **Finding `PERF-002` [VERIFIED]:** Stylo distributes selector matching across a fixed thread pool (`StyleThreadPool`) sized to available logical CPU cores (`num_cpus`).
- Subtree partitioning ensures parallel traversal without locks.

### 3.3 Allocator Configuration (`components/allocator`)
- **Finding `PERF-003` [VERIFIED]:** Configured to use `tikv-jemallocator` on desktop platforms, reducing heap fragmentation caused by thousands of small DOM node allocations.

---

## 4. Key Takeaways for Asteria

| Engineering Risk | Servo Mitigation (Audited) | Asteria Actionable Recommendation |
| :--- | :--- | :--- |
| **Script Hangs** | Dedicated `BackgroundHangMonitor` thread flagging unresponsive worker pings. | Add heartbeat timeouts to Asteria's worker scheduler (`src/scheduler.rs`). |
| **Heap Bloat & Leaks**| `MallocSizeOf` trait instrumented on every major struct. | Implement a memory reporting trait on Asteria's DOM arena and LRU image cache. |
| **IPC Deadlocks** | Dedicated routing worker threads unbuffering OS pipes. | If Asteria ever adopts multi-process, avoid raw synchronous blocking pipes. |
| **Supply Chain Security**| `deny.toml` enforcing license whitelists and vulnerability bans. | Add `cargo-deny` to Asteria's CI pipeline (`.github/workflows/ci.yml`). |
