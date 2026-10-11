# ISSUE: AST-ARC-005 — Dual Scheduler Divergence: Priority vs FIFO

- **Classification:** ARCHITECTURAL WEAKNESS
- **Severity:** Medium
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Concurrency & Task Scheduling
- **Specification Link:** [`../specifications/SPEC-09.md`](../specifications/SPEC-09.md)

---

## 1. Problem Statement
Asteria contains two divergent scheduler implementations in `src/scheduler.rs`:
1. `TaskScheduler` (`src/scheduler.rs:26-107`): An advanced 5-bucket priority queue (`High`, `Render`, `Layout`, `Network`, `Idle`).
2. `ThreadedScheduler` (`src/scheduler.rs:142-198`): A simple FIFO worker threadpool.
The runtime engine only executes `ThreadedScheduler`, leaving the priority queue unused outside unit tests. Consequently, network and layout tasks can starve urgent frame rendering tasks.

---

## 2. Evidence & Source Location
- **Location:** [`src/scheduler.rs:26-107`](../../../src/scheduler.rs#L26-L107) & [`src/scheduler.rs:142-198`](../../../src/scheduler.rs#L142-L198)
Production engine entry points in `main.rs` and `pipeline.rs` do not dispatch tasks through `TaskScheduler`'s priority queue.

---

## 3. Root Cause
The priority scheduler was developed as an experimental prototype and was never integrated into the application event loop or pipeline.

---

## 4. Desired Behaviour
Unify the schedulers into a single `PriorityTaskScheduler` driving the background threadpool, ensuring user input and rendering events preempt background network fetches.
