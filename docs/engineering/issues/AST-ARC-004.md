# ISSUE: AST-ARC-004 — Geometric Bounding-Box Heuristic in Scene Tree

- **Classification:** ARCHITECTURAL WEAKNESS
- **Severity:** Medium
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Scene Graph & Display List
- **Specification Link:** [`../specifications/SPEC-07.md`](../specifications/SPEC-07.md)

---

## 1. Problem Statement
In `src/scene.rs:360-372`, parent-child relationships in the scene graph are inferred using a geometric bounding box containment heuristic (`parent.rect.contains(child.rect)`). This heuristic fails catastrophically for zero-sized elements, empty containers, and children that intentionally overflow their parents (e.g. tooltips, dropdown menus, negative margins).

---

## 2. Evidence & Source Location
- **Location:** [`src/scene.rs:360-372`](../../../src/scene.rs#L360-L372)
```rust
if parent_rect.contains(child_rect) {
    // Inferred child assignment based purely on geometric bounds
}
```

---

## 3. Root Cause
The scene graph generator was built to operate solely on flat geometric command streams without preserving the original DOM node hierarchy.

---

## 4. Desired Behaviour
Preserve the DOM node tree hierarchy during display command generation and scene graph construction, ensuring parent-child relationships mirror the DOM regardless of geometric bounding box sizes.
