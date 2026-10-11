# ISSUE: AST-INC-001 — Dead Memory Infrastructure: Arena, Pool, Symbol

- **Classification:** INCOMPLETE IMPLEMENTATION
- **Severity:** Low
- **Priority:** P2
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Memory & Allocation Utilities
- **Specification Link:** [`../specifications/SPEC-11.md`](../specifications/SPEC-11.md)

---

## 1. Problem Statement
Asteria includes several sophisticated custom memory structures:
- `FrameArena` (`src/arena.rs`): A bump allocator for per-frame temporary allocations.
- `Pool<T>` (`src/pool.rs`): A reusable object pool for layout boxes and render commands.
- `Symbol` interner (`src/intern.rs`): A string interner pre-seeded with 140+ CSS property names and HTML tag names.
None of these structures are utilized anywhere in engine hot paths; their call sites exist exclusively in unit tests.

---

## 2. Evidence & Source Location
- **Location:** [`src/arena.rs`](../../../src/arena.rs), [`src/pool.rs`](../../../src/pool.rs), [`src/intern.rs`](../../../src/intern.rs)
Zero references in `dom.rs`, `style.rs`, `layout.rs`, or `renderer.rs`.

---

## 3. Root Cause
These memory utilities were developed speculatively during early infrastructure work but were bypassed during feature sprints in favor of standard heap `Vec` and `String` allocations.

---

## 4. Desired Behaviour
Either activate `Symbol` interning across the DOM and Style engines to reduce string allocation overhead, or remove unused dead structures to keep the codebase lean and free of compiler warnings.
