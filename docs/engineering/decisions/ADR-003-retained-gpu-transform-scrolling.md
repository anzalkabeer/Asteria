# ADR-003: Retained GPU Spatial Transform Scrolling

- **Status:** **PROPOSED**
- **Date:** October 2026
- **Proposers:** Principal Architect & Antigravity AI Pair
- **Related Issues:** [`AST-DEF-001`](../issues/AST-DEF-001.md), [`AST-PERF-001`](../issues/AST-PERF-001.md)
- **Specification:** [`SPEC-08`](../specifications/SPEC-08.md)

---

## 1. Context & Problem Statement
Currently, Asteria executes an immediate-mode redraw on every mouse scroll tick or cursor motion (`src/renderer/window/window.rs:747-794`). This forces a complete CPU walk of the scene tree, rebuilds all display commands, and allocates fresh GPU vertex buffers on every tick, causing 100% CPU core spikes and stutter during scrolling.

---

## 2. Decision Drivers
1. **120 FPS Fluid Scrolling:** Scrolling must execute purely on the GPU compositing stage at the display refresh rate without CPU display-list rebuilding.
2. **Zero Per-Frame VRAM Allocation:** Avoid destroying and re-allocating GPU buffers during user interaction.

---

## 3. Considered Options
- **Option A (Status Quo):** Immediate-mode display command rebuild on every scroll event.
- **Option B (WebRender Clone):** Import Servo's complete `SpatialNodeTree` with complex multi-clip transform hierarchies.
- **Option C (Selected Proposal):** Introduce a WGSL `ViewportUniform` buffer in Asteria's existing shader pipelines. During scroll events, only update the uniform buffer offset via `queue.write_buffer()`, bypassing CPU command generation completely.

---

## 4. Decision & Rationale
Adopt Option C. By updating a 16-byte viewport offset uniform buffer on scroll, vertex positions cached in VRAM are translated in the GPU vertex shader. This achieves the smoothness of WebRender without introducing WebRender's massive complexity.

---

## 5. Consequences & Trade-Offs
- **Positive:** Drops CPU usage during scrolling to near 0%; enables smooth 60/120 FPS scrolling on large documents.
- **Negative:** Fixed-position elements (`position: fixed`) require separate render passes or uniform flags to prevent them from scrolling with content.
