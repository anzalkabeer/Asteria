# SPEC-07: DOM-Preserving Scene Tree Hierarchy Construction

- **Related Issue:** [`../issues/AST-ARC-004.md`](../issues/AST-ARC-004.md)
- **Target Subsystem:** Scene Graph (`src/scene.rs`, `src/renderer/command_builder.rs`)
- **Stage:** Stage 2 (P1)

---

## 1. Problem Statement
In `src/scene.rs:360-372`, parent-child relationships in the scene graph are determined via a geometric containment check (`parent.rect.contains(child.rect)`), failing on zero-size elements and intentionally overflowing children (tooltips, dropdowns).

---

## 2. Root Cause
Display command generation discards the original DOM tree hierarchy and passes a flat list of geometric items to the scene builder.

---

## 3. Recommended Solution
Attach `dom_node_id` and `parent_node_id` metadata to each `DisplayItem` and `RenderCommand`. Construct the scene graph tree directly from the preserved DOM hierarchy:
```rust
pub struct SceneNode {
    pub id: SceneNodeId,
    pub dom_id: NodeId,
    pub parent: Option<SceneNodeId>,
    pub children: Vec<SceneNodeId>,
    pub rect: Rect,
}
```

---

## 4. Affected Source Locations
- `src/scene.rs`
- `src/renderer/command_builder.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_scene_tree_overflow_parenting`: An element with `margin-left: -50px` positioned outside its parent's bounds retains its correct parent node in the scene tree.
