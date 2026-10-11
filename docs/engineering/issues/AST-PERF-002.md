# ISSUE: AST-PERF-002 — Synchronous Blocking IO on Background Threadpool

- **Classification:** PERFORMANCE CONCERN
- **Severity:** Medium
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Networking & Threadpool
- **Specification Link:** [`../specifications/SPEC-04.md`](../specifications/SPEC-04.md)

---

## 1. Problem Statement
In `src/net/fetch.rs:45-98`, network requests are executed via blocking standard library `TcpStream` sockets dispatched onto a fixed worker threadpool. When a webpage loads multiple slow subresources (images, fonts, stylesheets), worker threads become blocked in kernel socket reads, starving parsing and layout tasks.

---

## 2. Evidence & Source Location
- **Location:** [`src/net/fetch.rs:45-98`](../../../src/net/fetch.rs#L45-L98)
Synchronous socket operations hold worker threads indefinitely during network latency.

---

## 3. Root Cause
Asteria avoided asynchronous runtimes (like Tokio) to keep the initial prototype simple.

---

## 4. Desired Behaviour
Transition the networking layer to non-blocking asynchronous IO (via Tokio / `asteria_net`), or configure strict non-blocking socket timeouts to prevent thread starvation.
