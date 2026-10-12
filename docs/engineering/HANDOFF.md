# HANDOFF.md — Contributor & Agent Session Briefing

> **Last Updated:** October 12, 2026  
> **Target Audience:** Any incoming contributor, Anzal, Keshav, or AI coding agent resuming Asteria development.  
> **Instruction:** Read this document first to immediately orient yourself without needing conversation history.

---

## 1. Executive Status Briefing

- **Repository State:** Asteria Browser Engine (`Asteria/Asteria`)
- **Current Git Revision:** Branch `main`
- **Working Tree:** Stage 1 implementation applied cleanly with zero regressions across all 11 test suites.
- **Active Implementation Task:** **Awaiting Stage 2 Authorization**.
- **Recently Completed Work:**
  - **Completed Stage 1: Critical Correctness & Core Engine Stabilization (P0)**:
    - `SPEC-01`: Persistent retained `GeometryBufferPool` in `src/renderer/resources/buffer_pool.rs`, `RectPass`, and `ImagePass` (zero GPU buffer allocations over 100 frames).
    - `SPEC-02`: Real PNG/JPEG image decoding via `image = "0.25"` crate in `src/image.rs` and active `RenderCommand::Image` emission.
    - `SPEC-03`: RFC 9110 compliant request target formatting (`Url::request_target`) stripping `#fragment` identifiers over the wire in `src/net/http.rs`.
    - `SPEC-04`: 100 MB maximum body buffer limits (`MAX_RESPONSE_BODY_SIZE`) guarding exact, chunked, and streaming response pipelines in `src/net/http.rs`.
  - All 11 test suites (`cargo test`) verified with exit code `0` and 0 failures.

---

## 2. Key Forensic Discoveries to Keep in Mind

1. **Rendering Performance Bottleneck:** User scrolling or hovering causes immediate-mode CPU walks of the display list and full batch rebuilds ([`AST-DEF-001`](issues/AST-DEF-001.md)). Scheduled for Stage 3 via uniform GPU scroll offsets ([`SPEC-08`](specifications/SPEC-08.md)).
2. **Layout Text Measurement Mismatch:** Layout line breaking uses `estimate_char_width_ratio` hardcoded ratios rather than actual font metrics from `FontContext` ([`AST-ARC-002`](issues/AST-ARC-002.md)). Scheduled for Stage 2 via [`SPEC-06`](specifications/SPEC-06.md).
3. **Workspace Monolith:** Engine currently resides in a monolithic 33-module root crate ([`AST-ARC-001`](issues/AST-ARC-001.md)). Scheduled for Stage 2 modularization via [`SPEC-05`](specifications/SPEC-05.md).
4. **Servo Integration Philosophy:** Do **not** import WebRender or SpiderMonkey (too heavy, C++ dependencies). Keep Asteria 100% pure Rust; adopt Taffy for Flexbox/Grid and Boa for JavaScript.

---

## 3. Recommended Next Implementation Action

Once the technical lead or repository owner authorizes the next stage:

1. **Execute Stage 2: Architectural Boundaries & Layout Correctness (P1):**
   - **Step 1:** Implement [`SPEC-05`](specifications/SPEC-05.md) — Partition monolithic root crate into a 7-crate Cargo workspace (`asteria-core`, `asteria-html`, `asteria-css`, `asteria-layout`, `asteria-render`, `asteria-net`, `asteria-app`).
   - **Step 2:** Implement [`SPEC-06`](specifications/SPEC-06.md) — Connect `layout.rs` line wrapping directly to true glyph metrics from `FontContext`.
   - **Step 3:** Implement [`SPEC-07`](specifications/SPEC-07.md) — Ensure zero-size and overflow elements preserve correct DOM parenting in `scene.rs`.
   - **Step 4:** Implement [`SPEC-12`](specifications/SPEC-12.md) — Enforce document jail path canonicalization on local `file://` fetch URLs in `src/net/fetch.rs`.

2. **Documents to Consult Before Acting:**
   - [`AGENTS.md`](../../AGENTS.md) — Operational instructions.
   - [`PROJECT_STATE.md`](PROJECT_STATE.md) — Verified capabilities and constraints.
   - [`ROADMAP.md`](ROADMAP.md) — Stage 2 milestones and exit criteria.
   - [`specifications/SPEC-05.md`](specifications/SPEC-05.md) through [`specifications/SPEC-12.md`](specifications/SPEC-12.md) — Detailed technical specifications.
   - [`TESTING.md`](TESTING.md) — Commands for running unit tests.
