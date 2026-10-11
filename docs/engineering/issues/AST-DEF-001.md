# ISSUE: AST-DEF-001 — Immediate-Mode Redraw on Scroll & Hover

- **Classification:** CONFIRMED DEFECT
- **Severity:** High
- **Priority:** P1
- **Status:** **CONFIRMED**
- **Affected Subsystem:** Shell Window & GPU Renderer
- **Specification Link:** [`../specifications/SPEC-08.md`](../specifications/SPEC-08.md)

---

## 1. Problem Statement
When a user scrolls the window with a mouse wheel or moves the cursor, the event loop in `src/renderer/window/window.rs:747-794` triggers a complete rebuild of the display command list and invokes `render()`. This causes a full CPU walk of the scene tree, rebuilds all vertex/index batches, and reallocates GPU vertex buffers on every single scroll tick, causing high CPU usage and frame stutter.

---

## 2. Evidence & Source Location
- **Location:** [`src/renderer/window/window.rs:747-794`](../../../src/renderer/window/window.rs#L747-L794)
```rust
WindowEvent::MouseWheel { delta, .. } => {
    // Rebuilds entire command stream and forces redraw
    self.pipeline.rebuild_display_commands(&self.scene, self.viewport_offset_y);
    self.window.request_redraw();
}
```

---

## 3. Root Cause
The renderer lacks a spatial transform tree or GPU viewport offset uniform buffer. Geometry coordinates are baked into CPU vertex streams instead of being translated dynamically in the vertex shader.

---

## 4. Desired Behaviour
User scrolling should update a 16-byte WGSL uniform buffer via `queue.write_buffer()`, bypassing CPU display list rebuilding and vertex buffer regeneration completely, delivering 120 FPS scrolling.
