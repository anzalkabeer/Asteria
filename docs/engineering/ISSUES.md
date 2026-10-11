# ISSUES.md — Canonical Master Issue Index & ID Collision Reconciliation

> **Authority:** Canonical Tracker for all defects, weaknesses, and enhancements.  
> **Status Lifecycle:** `UNVERIFIED` → `CONFIRMED` → `PLANNED` → `IN PROGRESS` → `FIXED` → `VERIFIED` (or `DEFERRED` / `WONT FIX`).  
> **Important Invariant:** An issue marked `FIXED` means code changes are drafted/merged; an issue marked `VERIFIED` requires passing empirical unit/integration tests and regression suites.

---

## 1. Master Issue & Audit ID Reconciliation Mapping

To resolve historic collisions between early Servo audit candidate IDs (`AST-001` to `AST-012`), draft comparative citations, and forensic classified IDs, use the following authoritative mapping:

| Authoritative Issue ID | Canonical Title | Historical / Draft Alias | Related Spec ID | Root Cause / Scope Distinction |
| :--- | :--- | :--- | :--- | :--- |
| **AST-DEF-001** | Immediate-Mode Redraw on Scroll & Hover | `AST-005`, `AST-PERF-001` (draft) | [`SPEC-08`](specifications/SPEC-08.md) | Viewport scroll triggers full CPU display command rebuild and redraw. Distinct from buffer allocation. |
| **AST-DEF-002** | Mock Image Pipeline & Uncalled ImagePass | `AST-008` (draft), `FLAW-1.12` | [`SPEC-02`](specifications/SPEC-02.md) | `decode_image` produces dummy bytes; `ImagePass` unexecuted; `<img>` renders as grey rect. |
| **AST-DEF-003** | RFC 9110 URL Query/Fragment Parser Bug | `NET-BUG-01` | [`SPEC-03`](specifications/SPEC-03.md) | HTTP GET request line transmits `#` fragments verbatim, violating RFC 9110. |
| **AST-DEF-004** | Unbounded Synchronous Network Buffer Streaming | `NET-BUG-02` | [`SPEC-04`](specifications/SPEC-04.md) | Synchronous accumulation of socket bytes into unbounded `Vec<u8>`. |
| **AST-ARC-001** | Monolithic Single-Crate Cargo Architecture | `AST-009` | [`SPEC-05`](specifications/SPEC-05.md) | Monolith package prevents crate-level unit testing and prolongs clean compile times to 5m+. |
| **AST-ARC-002** | Disconnect Between Font Metrics & Layout Sizing | `AST-003`, `AST-DEF-003` (draft) | [`SPEC-06`](specifications/SPEC-06.md) | `estimate_char_width_ratio` hardcodes character width ratios instead of querying typeface metrics. |
| **AST-ARC-003** | Ad-Hoc Flexbox Without Grid or Multi-Line Wrap | `AST-002` | [`SPEC-10`](specifications/SPEC-10.md) | Handwritten flex solver misses flex-wrap, grow/shrink factors, and CSS Grid. |
| **AST-ARC-004** | Geometric Bounding-Box Heuristic in Scene Tree | `SCENE-TREE-BUG` | [`SPEC-07`](specifications/SPEC-07.md) | Infers parent-child hierarchy via geometric bounding box containment instead of DOM structure. |
| **AST-ARC-005** | Dual Scheduler Divergence: Priority vs FIFO | `SCHED-DIV-01` | [`SPEC-09`](specifications/SPEC-09.md) | 5-bucket `TaskScheduler` is dead in production; `ThreadedScheduler` only runs FIFO threadpool. |
| **AST-INC-001** | Dead Memory Infrastructure: Arena, Pool, Symbol | `AST-007` | [`SPEC-11`](specifications/SPEC-11.md) | `FrameArena`, `Pool<T>`, and 140+ `Symbol` constants are 100% unused in engine hot paths. |
| **AST-INC-002** | Incomplete CSS Specificity & Combinator Matching | `STYLE-MATCH-01` | [`SPEC-13`](specifications/README.md) | Misses child (`>`), sibling (`+`, `~`) combinators and pseudo-elements. |
| **AST-INC-003** | Missing JavaScript Execution Runtime | `AST-010` | [`ENH-01`](../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-01-embedded-pure-rust-ecmascript-runtime-boa-engine) | Passive DOM; `<script>` tags skipped; no event listener dispatcher. |
| **AST-PERF-001** | Excessive Per-Frame GPU Buffer Reallocation | `AST-005b` | [`SPEC-01`](specifications/SPEC-01.md) | `BatchBuilder` calls `device.create_buffer_init` on every frame, thrashing VRAM heaps. |
| **AST-PERF-002** | Synchronous Blocking IO on Background Threadpool | `AST-011` | [`SPEC-04`](specifications/SPEC-04.md) | Socket reads block worker threads; lacks async non-blocking task polling. |
| **AST-PERF-003** | Naive O(N) Linear Style Cascade Matching | `AST-004` | [`SPEC-14`](specifications/README.md) | Exhaustive linear search over all CSS rules for every DOM node without rule sharing. |
| **AST-PERF-004** | Full Document Relayout on DOM Mutation | `LAYOUT-RELAYOUT-01` | [`SPEC-15`](specifications/README.md) | Entire `LayoutBox` tree rebuilt from root on any mutation; lacks dirty subtree tracking. |
| **AST-SEC-001** | Unrestricted Local File URI Reading | `SEC-FILE-01` | [`SPEC-12`](specifications/SPEC-12.md) | Loads arbitrary `file://` filesystem paths without directory jailing or traversal checks. |
| **AST-SEC-002** | Missing Content Security Policy & Origin Isolation | `SEC-CSP-01` | [`SPEC-16`](specifications/README.md) | No CSP headers or Same-Origin checks enforced on external resources. |
| **AST-ENG-001** | Missing Visual Reftests & Automated Pixel CI | `TEST-RETEST-01` | [`ENH-03`](../../ASTERIA_IMPROVEMENT_AUDIT/06_ENHANCEMENT_OPPORTUNITIES.md#enh-03-headless-gpu-accelerated-visual-reftest-harness) | Unit tests only; zero automated visual regression tests or headless pixel comparisons. |

---

## 2. Canonical Master Issue Directory

| Issue ID | Title | Classification | Sev / Pri | Conf | Status | Subsystem | Target Source Location | Spec / Detail Link |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **AST-DEF-001** | Immediate-Mode Redraw on Scroll & Hover | CONFIRMED DEFECT | High / P1 | High | **CONFIRMED** | Shell / Render | [`src/renderer/window/window.rs:747-794`](../../src/renderer/window/window.rs#L747-L794) | [`issues/AST-DEF-001.md`](issues/AST-DEF-001.md) |
| **AST-DEF-002** | Mock Image Pipeline & Uncalled ImagePass | CONFIRMED DEFECT | High / P0 | High | **CONFIRMED** | Media / Render | [`src/image.rs:97-100`](../../src/image.rs#L97-L100)<br>[`src/renderer/gpu/renderer.rs:248-275`](../../src/renderer/gpu/renderer.rs#L248-L275) | [`issues/AST-DEF-002.md`](issues/AST-DEF-002.md) |
| **AST-DEF-003** | RFC 9110 URL Query/Fragment Parser Bug | CONFIRMED DEFECT | High / P0 | High | **CONFIRMED** | Networking | [`src/net/http.rs:47-70`](../../src/net/http.rs#L47-L70)<br>[`src/net/url.rs:48-96`](../../src/net/url.rs#L48-L96) | [`issues/AST-DEF-003.md`](issues/AST-DEF-003.md) |
| **AST-DEF-004** | Unbounded Synchronous Network Buffer Streaming | CONFIRMED DEFECT | High / P0 | High | **CONFIRMED** | Networking | [`src/net/http.rs:163-195`](../../src/net/http.rs#L163-L195)<br>[`src/net/fetch.rs:104-142`](../../src/net/fetch.rs#L104-L142) | [`issues/AST-DEF-004.md`](issues/AST-DEF-004.md) |
| **AST-ARC-001** | Monolithic Single-Crate Cargo Architecture | ARCHITECTURAL WEAKNESS | High / P1 | High | **CONFIRMED** | Build / Workspace | [`Cargo.toml:1-48`](../../Cargo.toml#L1-L48) | [`issues/AST-ARC-001.md`](issues/AST-ARC-001.md) |
| **AST-ARC-002** | Disconnect Between Font Metrics & Layout Sizing | ARCHITECTURAL WEAKNESS | High / P1 | High | **CONFIRMED** | Text / Layout | [`src/layout.rs:2534-2558`](../../src/layout.rs#L2534-L2558)<br>[`src/text.rs:188-232`](../../src/text.rs#L188-L232) | [`issues/AST-ARC-002.md`](issues/AST-ARC-002.md) |
| **AST-ARC-003** | Ad-Hoc Flexbox Without Grid or Multi-Line Wrap | ARCHITECTURAL WEAKNESS | High / P1 | High | **CONFIRMED** | Layout Engine | [`src/layout.rs:1350-1685`](../../src/layout.rs#L1350-L1685) | [`issues/AST-ARC-003.md`](issues/AST-ARC-003.md) |
| **AST-ARC-004** | Geometric Bounding-Box Heuristic in Scene Tree | ARCHITECTURAL WEAKNESS | Med / P1 | High | **CONFIRMED** | Scene Graph | [`src/scene.rs:360-372`](../../src/scene.rs#L360-L372) | [`issues/AST-ARC-004.md`](issues/AST-ARC-004.md) |
| **AST-ARC-005** | Dual Scheduler Divergence: Priority vs FIFO | ARCHITECTURAL WEAKNESS | Med / P2 | High | **CONFIRMED** | Concurrency | [`src/scheduler.rs:26-107`](../../src/scheduler.rs#L26-L107) | [`issues/AST-ARC-005.md`](issues/AST-ARC-005.md) |
| **AST-INC-001** | Dead Memory Infrastructure: Arena, Pool, Symbol | INCOMPLETE IMPLEMENTATION | Low / P2 | High | **CONFIRMED** | Memory System | [`src/arena.rs:1-120`](../../src/arena.rs#L1-L120)<br>[`src/intern.rs:1-180`](../../src/intern.rs#L1-L180) | [`issues/AST-INC-001.md`](issues/AST-INC-001.md) |
| **AST-INC-002** | Incomplete CSS Specificity & Combinator Matching | INCOMPLETE IMPLEMENTATION | Med / P2 | High | **CONFIRMED** | Style Engine | [`src/css.rs:88-142`](../../src/css.rs#L88-L142)<br>[`src/style.rs:215-320`](../../src/style.rs#L215-L320) | [`issues/AST-INC-002.md`](issues/AST-INC-002.md) |
| **AST-INC-003** | Missing JavaScript Execution Runtime | INCOMPLETE IMPLEMENTATION | High / P3 | High | **CONFIRMED** | Scripting | [`src/dom.rs:1-420`](../../src/dom.rs#L1-L420) | [`issues/AST-INC-003.md`](issues/AST-INC-003.md) |
| **AST-PERF-001** | Excessive Per-Frame GPU Buffer Reallocation | PERFORMANCE CONCERN | High / P0 | High | **CONFIRMED** | GPU Renderer | [`src/renderer/gpu/renderer.rs:310-380`](../../src/renderer/gpu/renderer.rs#L310-L380) | [`issues/AST-PERF-001.md`](issues/AST-PERF-001.md) |
| **AST-PERF-002** | Synchronous Blocking IO on Background Threadpool | PERFORMANCE CONCERN | Med / P1 | High | **CONFIRMED** | Networking | [`src/net/fetch.rs:45-98`](../../src/net/fetch.rs#L45-L98) | [`issues/AST-PERF-002.md`](issues/AST-PERF-002.md) |
| **AST-PERF-003** | Naive O(N) Linear Style Cascade Matching | PERFORMANCE CONCERN | Med / P2 | High | **CONFIRMED** | Style Engine | [`src/style.rs:110-185`](../../src/style.rs#L110-L185) | [`issues/AST-PERF-003.md`](issues/AST-PERF-003.md) |
| **AST-PERF-004** | Full Document Relayout on DOM Mutation | PERFORMANCE CONCERN | Med / P2 | High | **CONFIRMED** | Layout Engine | [`src/renderer/window/window.rs:810-850`](../../src/renderer/window/window.rs#L810-L850) | [`issues/AST-PERF-004.md`](issues/AST-PERF-004.md) |
| **AST-SEC-001** | Unrestricted Local File URI Reading | SECURITY CONCERN | High / P1 | High | **CONFIRMED** | Security / Net | [`src/net/fetch.rs:145-180`](../../src/net/fetch.rs#L145-L180) | [`issues/AST-SEC-001.md`](issues/AST-SEC-001.md) |
| **AST-SEC-002** | Missing Content Security Policy & Origin Isolation | SECURITY CONCERN | Med / P2 | High | **CONFIRMED** | Web Security | [`src/html.rs:40-110`](../../src/html.rs#L40-L110) | [`issues/AST-SEC-002.md`](issues/AST-SEC-002.md) |
| **AST-ENG-001** | Missing Visual Reftests & Automated Pixel CI | ENGINEERING PRACTICE GAP | High / P2 | High | **CONFIRMED** | Quality Assurance| [`tests/`](../../tests/) | [`issues/AST-ENG-001.md`](issues/AST-ENG-001.md) |
