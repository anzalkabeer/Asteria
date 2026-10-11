# HANDOFF.md — Contributor & Agent Session Briefing

> **Last Updated:** October 11, 2026  
> **Target Audience:** Any incoming contributor, Anzal, Keshav, or AI coding agent resuming Asteria development.  
> **Instruction:** Read this document first to immediately orient yourself without needing conversation history.

---

## 1. Executive Status Briefing

- **Repository State:** Asteria Browser Engine (`Asteria/Asteria`)
- **Current Git Revision:** `3cdd5155caf544794c3d11f45dfc839665d9759b` (Branch `main`)
- **Working Tree:** `Cargo.lock` has unstaged dependency resolution modifications (retain as-is). All source files in `src/` and `tests/` are pristine.
- **Active Implementation Task:** **NONE (Awaiting Stage 1 Authorization)**.
- **Recently Completed Work:**
  - Completed comprehensive forensic audit of all 33 Asteria modules (`ASTERIA_IMPROVEMENT_AUDIT/`).
  - Completed comparative analysis against Servo (`asteria_servo_audit/`).
  - Consolidated canonical persistent engineering memory in `docs/engineering/` and `AGENTS.md`.
  - Reconciled historic issue ID collisions across all documents.
- **Baseline Test Verification:**
  - `cargo test --no-run` verified with exit code `0` in 302.26s (~5m 02s). All 11 test suites compile successfully.

---

## 2. Key Forensic Discoveries to Keep in Mind

1. **Rendering Performance Bottleneck:** User scrolling or hovering causes immediate-mode CPU walks of the display list and complete vertex/index buffer reallocations on every tick ([`AST-DEF-001`](issues/AST-DEF-001.md), [`AST-PERF-001`](issues/AST-PERF-001.md)).
2. **Missing Image Pipeline:** `image.rs` returns mock gradient bytes, `ImagePass` is uncalled in `renderer.rs`, and images are drawn as grey boxes with `[IMG]` labels ([`AST-DEF-002`](issues/AST-DEF-002.md)).
3. **HTTP RFC 9110 Violation:** HTTP GET requests transmit `#` fragments in request lines ([`AST-DEF-003`](issues/AST-DEF-003.md)).
4. **Layout Text Measurement Mismatch:** Layout line breaking uses `estimate_char_width_ratio` hardcoded ratios rather than actual font metrics from `FontContext` ([`AST-ARC-002`](issues/AST-ARC-002.md)).
5. **Servo Integration Philosophy:** Do **not** import WebRender or SpiderMonkey (too heavy, C++ dependencies). Keep Asteria 100% pure Rust; adopt Taffy for Flexbox/Grid and Boa for JavaScript.

---

## 3. Recommended Next Implementation Action

Once the technical lead or repository owner authorizes implementation work:

1. **Execute Stage 1: Core Engine Stabilization (P0):**
   - **Step 1:** Implement [`SPEC-01`](specifications/SPEC-01.md) — Retained GPU vertex/index buffer pool in `src/renderer/gpu/renderer.rs` to stop per-frame VRAM churn.
   - **Step 2:** Implement [`SPEC-02`](specifications/SPEC-02.md) — Real PNG/JPEG image decoding via `image` crate and activate `ImagePass` in `renderer.rs`.
   - **Step 3:** Implement [`SPEC-03`](specifications/SPEC-03.md) — Strip `#` fragments from HTTP request lines in `src/net/http.rs`.
   - **Step 4:** Implement [`SPEC-04`](specifications/SPEC-04.md) — Add bounded streaming buffer size checks (100 MB max) in `src/net/http.rs`.

2. **Documents to Consult Before Acting:**
   - [`AGENTS.md`](../../AGENTS.md) — Operational instructions.
   - [`PROJECT_STATE.md`](PROJECT_STATE.md) — Verified capabilities and constraints.
   - [`ROADMAP.md`](ROADMAP.md) — Stage 1 milestones and exit criteria.
   - [`specifications/SPEC-01.md`](specifications/SPEC-01.md) through [`specifications/SPEC-04.md`](specifications/SPEC-04.md) — Detailed technical specifications.
   - [`TESTING.md`](TESTING.md) — Commands for running unit tests.
