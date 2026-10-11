# SPEC-10: Taffy Layout Adapter Delegation for Flexbox & Grid

- **Related Issue:** [`../issues/AST-ARC-003.md`](../issues/AST-ARC-003.md)
- **Target Subsystem:** Layout Engine (`src/layout.rs`)
- **Stage:** Stage 3 (P1)
- **ADR Reference:** [`../decisions/ADR-004-taffy-layout-delegation.md`](../decisions/ADR-004-taffy-layout-delegation.md)

---

## 1. Problem Statement
Asteria's handwritten flexbox solver in `src/layout.rs:1350-1685` lacks multi-line wrapping, flex-grow/shrink proportion scaling, and CSS Grid layout is missing entirely.

---

## 2. Root Cause
Handcrafting full CSS Flexbox and Grid engines requires immense mathematical and geometric complexity.

---

## 3. Recommended Solution
1. Add `taffy = "0.14"` to `Cargo.toml`.
2. Implement an adapter module in `crates/asteria_layout` that converts Asteria's `ComputedStyle` into `taffy::style::Style`.
3. When `layout.rs` encounters a container with `display: flex` or `display: grid`:
   - Create a sub-tree in a Taffy instance.
   - Run Taffy layout calculation: `taffy.compute_layout(root, available_space)`.
   - Read back the computed `Layout` rectangles and assign them to child `LayoutBox` coordinates.

---

## 4. Affected Source Locations
- `src/layout.rs`
- Target crate: `crates/asteria_layout`

---

## 5. Verification & Acceptance Criteria
1. `test_taffy_flexbox_wrap`: Elements with `flex-wrap: wrap` wrap onto multiple lines when container width is exceeded.
2. `test_taffy_css_grid`: Grid layouts with `grid-template-columns: repeat(3, 1fr)` position children into 3 equal columns.
