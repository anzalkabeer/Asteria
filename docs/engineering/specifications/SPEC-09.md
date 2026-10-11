# SPEC-09: Unified Priority Event Loop & Task Scheduler

- **Related Issue:** [`../issues/AST-ARC-005.md`](../issues/AST-ARC-005.md)
- **Target Subsystem:** Concurrency & Scheduler (`src/scheduler.rs`, `src/pipeline.rs`)
- **Stage:** Stage 3 (P2)

---

## 1. Problem Statement
Asteria contains an advanced 5-bucket `TaskScheduler` (`src/scheduler.rs:26-107`) that is only called in unit tests, while production runs on an unprioritized FIFO threadpool (`ThreadedScheduler`).

---

## 2. Root Cause
The priority scheduler was never wired into the pipeline orchestration loop.

---

## 3. Recommended Solution
Merge the two implementations into a unified `PriorityThreadPoolScheduler`:
```rust
pub enum TaskPriority {
    High,    // User input, resize, urgent animations
    Render,  // Frame composition, GPU command submission
    Layout,  // Box tree calculations, font metrics
    Network, // HTTP socket polling, resource fetches
    Idle,    // Garbage collection, cache pruning
}
```
Worker threads poll tasks using priority-weighted work-stealing, ensuring input and rendering tasks are executed before background fetches.

---

## 4. Affected Source Locations
- `src/scheduler.rs`
- `src/pipeline.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_scheduler_priority_ordering`: Tasks submitted with `High` priority execute before earlier `Network` tasks.
