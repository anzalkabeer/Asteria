# FINDING RECORD: AST-DEF-001

## 1. Classification & Identity
- **Finding ID:** `AST-DEF-001`
- **Classification:** CONFIRMED DEFECT
- **Severity:** High
- **Subsystem:** Compositing, Window & Shell
- **Confidence Level:** High (Directly verified in source and runtime event loops)

---

## 2. Location & Evidence
- **Source Location:** [`src/renderer/window/window.rs:747-794`](file:///z:/1mportant%20stuff/Codes%20written%20by%20me%20(New)/Asteria/Asteria/src/renderer/window/window.rs#L747-L794)
- **Relevant Code Excerpt:**
```rust
WindowEvent::MouseWheel { delta, .. } => {
    let scroll_y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y * 40.0,
        MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
    };
    self.viewport_offset_y = (self.viewport_offset_y + scroll_y).clamp(0.0, max_scroll);
    // CRITICAL: Forces full rebuild of command list and re-renders entire document
    self.pipeline.rebuild_display_commands(&self.scene, self.viewport_offset_y);
    self.window.request_redraw();
}
```
- **Call-Chain Analysis:**
  - `WindowEvent::MouseWheel` or `CursorMoved` received in `winit` event loop.
  - Updates `viewport_offset_y`.
  - Dispatches `rebuild_display_commands`, iterating over all display nodes in the scene.
  - In `src/renderer/gpu/renderer.rs:310-380`, `BatchBuilder::build()` is called, allocating fresh vertex and index buffers for every single scroll tick.
  - CPU usage spikes to 100% of a core during fast scrolling on complex pages, introducing dropped frames and input lag.

---

## 3. Root Cause Analysis
The renderer lacks a spatial transform tree or GPU viewport offset uniform. Instead of rendering the cached display list geometry with an offset translation in the vertex shader (`pos.xy + uniform.scroll_offset`), Asteria recalculates geometry bounds in CPU space and re-generates GPU vertex buffers on every tick.

---

## 4. Servo Architectural Reference
- Reference: `asteria_servo_audit/subsystems/04_WEBRENDER_AND_COMPOSITING.md`
- Mechanism: WebRender establishes Spatial Nodes in a transform tree. Scrolling updates the `ScrollNode` offset matrix. WebRender re-executes the GPU composition pass without altering display list items or regenerating vertex streams.

---

## 5. Remediation Plan
Implemented via `SPEC-08` and `SPEC-01`:
1. Introduce a `ViewportUniform` buffer in WGSL:
   ```wgsl
   struct ViewportUniform {
       scroll_offset: vec2<f32>,
       viewport_size: vec2<f32>,
   };
   @group(0) @binding(0) var<uniform> u_viewport: ViewportUniform;
   ```
2. When scrolling, update the uniform buffer via `queue.write_buffer(&self.viewport_buffer, 0, bytemuck::cast_slice(&[offset]))`.
3. Bypass `rebuild_display_commands` entirely during pure scroll events.
