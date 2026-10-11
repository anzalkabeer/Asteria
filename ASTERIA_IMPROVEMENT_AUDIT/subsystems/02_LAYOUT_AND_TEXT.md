# ASTERIA SUBSYSTEM AUDIT: LAYOUT AND TEXT

## 1. Subsystem Scope & Files
- **Primary Source Modules:**
  - [`src/layout.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/layout.rs)
  - [`src/text.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/text.rs)
  - [`src/geometry.rs`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/geometry.rs)

---

## 2. Architectural Analysis

### Strengths
1. **Clear LayoutBox Hierarchy:** `LayoutBox` accurately models CSS box model properties (content, padding, border, margin).
2. **Block and Inline Formatting Basics:** Basic CSS block formatting (normal flow, margin collapse) and inline text wrapping function correctly for simple documents.

### Deficiencies Identified
1. **Font Metric Disconnect (`AST-ARC-002`):** In `layout.rs:2534-2558`, text line breaking uses `estimate_char_width_ratio` instead of querying actual font metrics from `FontContext`. As a result, text line-breaking decisions made during layout do not match the rendered pixel widths produced by the GPU glyph rasterizer.
2. **Incomplete Flexbox and Missing Grid (`AST-ARC-003`):** The flexbox engine in `layout.rs:1350-1685` lacks multi-line wrapping, flex-grow/shrink factors, and CSS Grid layout is entirely missing.
3. **No Incremental Layout (`AST-PERF-004`):** Any DOM modification triggers a full recomputation of the entire `LayoutBox` tree from the document root.

---

## 3. Comparison with Servo (Layout 2020)
Servo uses Layout 2020 (`asteria_servo_audit/subsystems/05_LAYOUT_2020.md`), an independent multi-pass parallel layout engine using Rayon. While Servo achieves high throughput across multicore CPUs, its architecture is highly complex.

**Decision:** Rather than writing a massive custom layout engine or importing Servo's Layout 2020, Asteria should delegate Flexbox and Grid calculations to the industry-standard **Taffy** crate (`SPEC-10`), while integrating true font metrics into inline layout (`SPEC-06`).
