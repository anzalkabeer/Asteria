# SPEC-11: Dead Memory Infrastructure Pruning & Integration

- **Related Issue:** [`../issues/AST-INC-001.md`](../issues/AST-INC-001.md)
- **Target Subsystem:** Memory System (`src/arena.rs`, `src/pool.rs`, `src/intern.rs`)
- **Stage:** Stage 3 (P2)

---

## 1. Problem Statement
`FrameArena`, `Pool<T>`, and 140+ pre-seeded `Symbol` constants exist in the codebase but have zero callers outside unit tests, creating dead code maintenance burdens and compiler warnings.

---

## 2. Root Cause
These memory utilities were designed speculatively and never integrated into the actual DOM or Style pipelines.

---

## 3. Recommended Solution
1. **Activate String Interning:** Wire up `src/intern.rs` (`Symbol`) to store tag names in `src/dom.rs` and CSS property names in `src/style.rs`, reducing heap string allocations.
2. **Prune Unused Arenas:** Remove `src/arena.rs` and `src/pool.rs` if standard Rust allocators provide equal performance with simpler ownership semantics.

---

## 4. Affected Source Locations
- `src/arena.rs`
- `src/pool.rs`
- `src/intern.rs`
- `src/dom.rs`
- `src/style.rs`

---

## 5. Verification & Acceptance Criteria
1. Running `cargo clippy -- -D warnings` reports zero dead code or unused struct warnings.
