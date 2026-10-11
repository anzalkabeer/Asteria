# ROADMAP.md — Issue-Driven Implementation Roadmap

> **Authority:** Actionable, dependency-aware engineering sequence for Asteria.  
> **Status:** Ratified Blueprint awaiting Stage 1 authorization.  
> **Rule:** Stage $N$ must satisfy its exit criteria before Stage $N+1$ begins.

---

## Roadmap Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│ STAGE 1: Critical Correctness & Core Engine Stabilization (P0)          │
│ Retained Buffers (SPEC-01) ── Image Pipeline (SPEC-02)                 │
│ RFC 9110 URL (SPEC-03)    ── Bounded Stream Buffer (SPEC-04)          │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ Unblocks stable rendering & IO
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│ STAGE 2: Architectural Boundaries & Layout Correctness (P1)            │
│ Workspace Modularization (SPEC-05) ── True Font Metrics (SPEC-06)      │
│ Scene Tree Hierarchy (SPEC-07)    ── File Scheme Jailing (SPEC-12)     │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ Unblocks isolated crate testing
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│ STAGE 3: Performance, Rendering & Layout Modernization (P1-P2)         │
│ Retained GPU Scrolling (SPEC-08) ── Unified Priority Sched (SPEC-09)   │
│ Taffy Flex/Grid (SPEC-10)       ── Dead Memory Pruning (SPEC-11)      │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ Unblocks 120 FPS scrolling & CSS3
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│ STAGE 4: Quality Assurance, Hardening & Developer Tooling (P2)         │
│ Headless Reftest Harness ── CSP & Origin Isolation ── Benchmarks       │
└────────────────────────────────────┬───────────────────────────────────┘
                                     │ Unblocks platform compliance
                                     ▼
┌────────────────────────────────────────────────────────────────────────┐
│ STAGE 5: Advanced Platform Capabilities (P3)                           │
│ Pure-Rust Boa ECMAScript (ENH-01) ── Chrome DevTools Protocol (ENH-02) │
│ Embeddable WebView (ENH-06)       ── SDF Text & WebGPU API (ENH-04/07) │
└────────────────────────────────────────────────────────────────────────┘
```

---

## Stage Details & Execution Gates

### Stage 1: Critical Correctness & Core Engine Stabilization (Priority: P0)
- **Primary Objective:** Fix active rendering memory thrashing, complete non-functional image rendering, and repair broken HTTP networking.
- **Included Issues:**
  - [`AST-PERF-001`](issues/AST-PERF-001.md) — GPU Buffer Thrashing (`SPEC-01`)
  - [`AST-DEF-002`](issues/AST-DEF-002.md) — Image Pipeline Non-Functional (`SPEC-02`)
  - [`AST-DEF-003`](issues/AST-DEF-003.md) — RFC 9110 URL Query/Fragment Bug (`SPEC-03`)
  - [`AST-DEF-004`](issues/AST-DEF-004.md) — Unbounded Socket Buffer Streaming (`SPEC-04`)
- **Prerequisites:** None. Directly actionable on current codebase.
- **Target Files:** `src/renderer/gpu/renderer.rs`, `src/image.rs`, `src/renderer/command_builder.rs`, `src/net/http.rs`, `src/net/fetch.rs`.
- **Exit Criteria:**
  1. 100 consecutive frames allocate zero new GPU vertex/index buffers.
  2. HTML documents with `<img src="...">` render decoded PNG/JPEG bitmaps instead of grey placeholder boxes.
  3. HTTP GET request line formats RFC 9110 compliant targets (fragments stripped).
  4. Socket stream reader rejects or truncates bodies exceeding 100 MB safely.

---

### Stage 2: Architectural Boundaries & Layout Correctness (Priority: P1)
- **Primary Objective:** Partition the monolithic package into a 7-crate workspace, synchronize text line breaking with font metrics, and repair scene tree parenting.
- **Included Issues:**
  - [`AST-ARC-001`](issues/AST-ARC-001.md) — Workspace Modularization (`SPEC-05`)
  - [`AST-ARC-002`](issues/AST-ARC-002.md) — Font Metric Measurement (`SPEC-06`)
  - [`AST-ARC-004`](issues/AST-ARC-004.md) — Scene Tree Parenting (`SPEC-07`)
  - [`AST-SEC-001`](issues/AST-SEC-001.md) — Local File Scheme Jail (`SPEC-12`)
- **Prerequisites:** Stage 1 complete and verified.
- **Exit Criteria:**
  1. `cargo check --workspace` passes cleanly across all 7 newly partitioned crates.
  2. Text line wrapping in layout exactly matches rendered GPU glyph pixel widths.
  3. Overflowing and zero-size elements preserve correct DOM parent-child hierarchy in the scene tree.
  4. Local `file://` fetches outside the designated document jail return an access denial error.

---

### Stage 3: Performance, Rendering & Layout Modernization (Priority: P1–P2)
- **Primary Objective:** Deliver 120 FPS GPU scrolling, delegate Flexbox and Grid to Taffy, and unify thread scheduling.
- **Included Issues:**
  - [`AST-DEF-001`](issues/AST-DEF-001.md) — Retained GPU Scrolling via Uniforms (`SPEC-08`)
  - [`AST-ARC-005`](issues/AST-ARC-005.md) — Unified Priority Event Loop (`SPEC-09`)
  - [`AST-ARC-003`](issues/AST-ARC-003.md) — Taffy Flexbox & Grid Delegation (`SPEC-10`)
  - [`AST-INC-001`](issues/AST-INC-001.md) — Dead Memory Infrastructure Pruning (`SPEC-11`)
- **Prerequisites:** Stage 2 complete.
- **Exit Criteria:**
  1. User scrolling produces zero CPU display command rebuilds and runs at monitor refresh rate.
  2. Multi-line flexbox wrapping and CSS Grid layouts render properly.
  3. `TaskScheduler` priority queue drives all engine pipeline tasks.
  4. Zero compiler warnings for unused memory pool structures.

---

### Stage 4: Quality Assurance, Hardening & Developer Tooling (Priority: P2)
- **Primary Objective:** Implement automated visual regression testing, security sandboxing, and performance profiling.
- **Included Issues:**
  - [`AST-ENG-001`](issues/AST-ENG-001.md) — Automated Headless Visual Reftest Harness
  - [`AST-SEC-002`](issues/AST-SEC-002.md) — Content Security Policy & Origin Isolation
  - [`AST-PERF-003`](issues/AST-PERF-003.md) — Style Cascade Rule Tree Optimization
- **Prerequisites:** Stage 3 complete.
- **Exit Criteria:**
  1. Headless CI runs automated pixel-diff reftests against baseline reference images.
  2. Cross-origin script and stylesheet injections are blocked according to CSP rules.

---

### Stage 5: Advanced Platform Capabilities (Priority: P3)
- **Primary Objective:** Transform Asteria into a dynamic web platform with JavaScript execution and developer tooling.
- **Included Opportunities:**
  - [`AST-INC-003`](issues/AST-INC-003.md) — Pure-Rust Boa ECMAScript Integration (`ENH-01`)
  - Chrome DevTools Protocol (CDP) WebSocket Server (`ENH-02`)
  - Embeddable Native WebView Component Library (`ENH-06`)
  - Signed Distance Field (SDF) Text Rendering (`ENH-07`)
