# ISSUE: AST-ARC-002 — Disconnect Between Font Metrics & Layout Sizing

- **Classification:** ARCHITECTURAL WEAKNESS
- **Severity:** High
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Layout & Text Rendering
- **Specification Link:** [`../specifications/SPEC-06.md`](../specifications/SPEC-06.md)

---

## 1. Problem Statement
In `src/layout.rs:2534-2558`, inline text measurement uses a hardcoded ratio lookup function `estimate_char_width_ratio` instead of querying actual font metrics. Consequently, text line-breaking decisions in layout desync from the actual pixel widths rendered by the GPU glyph rasterizer, causing awkward word wraps or overflowing text.

---

## 2. Evidence & Source Location
- **Location:** [`src/layout.rs:2534-2558`](../../../src/layout.rs#L2534-L2558) & [`src/text.rs:188-232`](../../../src/text.rs#L188-L232)
```rust
fn estimate_char_width_ratio(c: char) -> f32 {
    match c {
        'i' | 'l' | 'j' => 0.25,
        'w' | 'm' | 'W' | 'M' => 0.85,
        _ => 0.55, // Hardcoded heuristic
    }
}
```

---

## 3. Root Cause
The layout engine was built before the text rasterizer (`renderer/font.rs`) was mature, and the two subsystems were never linked via a shared `FontContext`.

---

## 4. Desired Behaviour
Expose a shared `FontContext` from `asteria_paint` to `asteria_layout` that provides exact font ascent, descent, and glyph advance measurements during layout.
