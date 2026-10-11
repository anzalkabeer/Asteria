# SPEC-06: True Font Metric Integration in Layout Engine

- **Related Issue:** [`../issues/AST-ARC-002.md`](../issues/AST-ARC-002.md)
- **Target Subsystem:** Layout & Text (`src/layout.rs`, `src/text.rs`, `src/renderer/font.rs`)
- **Stage:** Stage 2 (P1)

---

## 1. Problem Statement
In `src/layout.rs:2534-2558`, text line breaking uses `estimate_char_width_ratio` hardcoded ratios, causing layout line breaks to diverge from the pixel widths rendered by the GPU glyph rasterizer.

---

## 2. Root Cause
The layout engine does not query typeface advance widths from `FontContext`.

---

## 3. Recommended Solution
Define a `FontMetricsProvider` trait in `asteria_paint` and pass an implementation to `asteria_layout`:
```rust
pub trait FontMetricsProvider {
    fn measure_text(&self, text: &str, font_size: f32, font_family: &str) -> f32;
    fn line_height(&self, font_size: f32, font_family: &str) -> f32;
}
```
During inline text formatting in `layout.rs`, invoke `provider.measure_text()` to calculate word breaking boundaries based on actual typeface advances.

---

## 4. Affected Source Locations
- `src/layout.rs`
- `src/text.rs`
- `src/renderer/font.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_layout_text_exact_alignment`: Rendered text runs align with layout container boundaries without premature word wraps or pixel clipping.
