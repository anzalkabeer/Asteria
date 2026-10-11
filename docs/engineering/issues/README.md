# Master Issues Directory & Canonical Issue Records

> **Overview:** Detailed technical records for all substantive findings discovered in the Asteria engine.  
> **Lifecycle:** `UNVERIFIED` → `CONFIRMED` → `PLANNED` → `IN PROGRESS` → `FIXED` → `VERIFIED`.

---

## Canonical Issue Directory

### Confirmed Defects (Functional Broken Behavior)
- **[`AST-DEF-001`](AST-DEF-001.md):** Immediate-Mode Redraw on Scroll & Hover (CPU thrashing).
- **[`AST-DEF-002`](AST-DEF-002.md):** Mock Image Pipeline & Uncalled `ImagePass` (Images render as `[IMG]` grey boxes).
- **[`AST-DEF-003`](AST-DEF-003.md):** RFC 9110 URL Query/Fragment Parser Bug (HTTP GET transmits `#` fragment).
- **[`AST-DEF-004`](AST-DEF-004.md):** Unbounded Synchronous Network Buffer Streaming (Memory exhaustion risk).

### Architectural Weaknesses (Structural Fragility & Coupling)
- **[`AST-ARC-001`](AST-ARC-001.md):** Monolithic Single-Crate Cargo Architecture (Slow builds, leaky boundaries).
- **[`AST-ARC-002`](AST-ARC-002.md):** Disconnect Between Font Metrics & Layout Sizing (Text line-wrap mismatch).
- **[`AST-ARC-003`](AST-ARC-003.md):** Ad-Hoc Flexbox Without Grid or Multi-Line Wrap (Incomplete modern CSS).
- **[`AST-ARC-004`](AST-ARC-004.md):** Geometric Bounding-Box Heuristic in Scene Tree (Fails on overflow elements).
- **[`AST-ARC-005`](AST-ARC-005.md):** Dual Scheduler Divergence: Priority vs FIFO (Unconnected priority queues).

### Incomplete Implementations (Unfinished Scaffolds)
- **[`AST-INC-001`](AST-INC-001.md):** Dead Memory Infrastructure (`FrameArena`, `Pool<T>`, `Symbol` interner unused).
- **[`AST-INC-002`](AST-INC-002.md):** Incomplete CSS Specificity & Combinator Matching (Missing `>`, `+`, `~`, pseudo-elements).
- **[`AST-INC-003`](AST-INC-003.md):** Missing JavaScript Execution Runtime (Read-only static DOM).

### Performance Concerns (Measured Inefficiencies & Overhead)
- **[`AST-PERF-001`](AST-PERF-001.md):** Excessive Per-Frame GPU Buffer Reallocation (Heap thrashing).
- **[`AST-PERF-002`](AST-PERF-002.md):** Synchronous Blocking IO on Background Threadpool (Thread starvation).
- **[`AST-PERF-003`](AST-PERF-003.md):** Naive O(N) Linear Style Cascade Matching ($O(N \times R)$ traversal).
- **[`AST-PERF-004`](AST-PERF-004.md):** Full Document Relayout on DOM Mutation (Missing dirty subtree flags).

### Security Concerns (Trust Boundaries & Confinement)
- **[`AST-SEC-001`](AST-SEC-001.md):** Unrestricted Local File URI Reading (Arbitrary filesystem reads via `file://`).
- **[`AST-SEC-002`](AST-SEC-002.md):** Missing Content Security Policy & Origin Isolation (No CSP headers enforced).

### Engineering Practice Gaps (Verification & Tooling)
- **[`AST-ENG-001`](AST-ENG-001.md):** Missing Visual Reftests & Automated Pixel CI (Zero automated regression reftests).
