# Master Technical Specifications Directory

> **Overview:** Detailed implementation-oriented engineering blueprints for all major Asteria refactorings, bug fixes, and performance improvements.  
> **Rule:** Every specification contains exact problem statements, root causes, target architecture, affected files, implementation sequences, and acceptance criteria.

---

## Specifications Directory

### Stage 1: Critical Correctness & Core Engine Stabilization (P0)
- **[`SPEC-01.md`](SPEC-01.md):** Retained GPU Vertex Buffers & Persistent Geometry Pool (`AST-PERF-001`)
- **[`SPEC-02.md`](SPEC-02.md):** Real PNG/JPEG Image Decoding Pipeline & GPU Texture Pass (`AST-DEF-002`)
- **[`SPEC-03.md`](SPEC-03.md):** RFC 9110 Compliant HTTP URL Normalization & Request Target (`AST-DEF-003`)
- **[`SPEC-04.md`](SPEC-04.md):** Bounded Streaming Network Buffer with Backpressure (`AST-DEF-004`, `AST-PERF-002`)

### Stage 2: Architectural Boundaries & Layout Correctness (P1)
- **[`SPEC-05.md`](SPEC-05.md):** 7-Crate Workspace Modularization Architecture (`AST-ARC-001`)
- **[`SPEC-06.md`](SPEC-06.md):** True Font Metric Integration in Layout Engine (`AST-ARC-002`)
- **[`SPEC-07.md`](SPEC-07.md):** DOM-Preserving Scene Tree Hierarchy Construction (`AST-ARC-004`)
- **[`SPEC-12.md`](SPEC-12.md):** Local File Scheme Jailing & Origin Security Policy (`AST-SEC-001`)

### Stage 3: Performance, Rendering & Layout Modernization (P1–P2)
- **[`SPEC-08.md`](SPEC-08.md):** Retained GPU Scrolling via Spatial Transform Uniforms (`AST-DEF-001`)
- **[`SPEC-09.md`](SPEC-09.md):** Unified Priority Event Loop & Task Scheduler (`AST-ARC-005`)
- **[`SPEC-10.md`](SPEC-10.md):** Taffy Layout Adapter Delegation for Flexbox & Grid (`AST-ARC-003`)
- **[`SPEC-11.md`](SPEC-11.md):** Dead Memory Infrastructure Pruning (`AST-INC-001`)
