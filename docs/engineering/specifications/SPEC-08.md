# SPEC-08: Retained GPU Scrolling via Spatial Transform Uniforms

- **Related Issue:** [`../issues/AST-DEF-001.md`](../issues/AST-DEF-001.md)
- **Target Subsystem:** GPU Renderer & Shell (`src/renderer/gpu/renderer.rs`, `src/renderer/window/window.rs`)
- **Stage:** Stage 3 (P1)
- **ADR Reference:** [`../decisions/ADR-003-retained-gpu-transform-scrolling.md`](../decisions/ADR-003-retained-gpu-transform-scrolling.md)

---

## 1. Problem Statement
In `src/renderer/window/window.rs:747-794`, user scrolling triggers a full CPU rebuild of all display commands and reallocates vertex/index buffers on every tick.

---

## 2. Root Cause
The renderer lacks a GPU viewport offset uniform buffer and bakes scrolled coordinates into CPU vertex positions.

---

## 3. Recommended Solution
1. Add a `ViewportUniform` buffer in WGSL shader pipelines:
```wgsl
struct ViewportUniform {
    scroll_offset: vec2<f32>,
    viewport_size: vec2<f32>,
};
@group(0) @binding(0) var<uniform> u_viewport: ViewportUniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - u_viewport.scroll_offset;
    let clip_pos = (world_pos / (u_viewport.viewport_size * 0.5)) - vec2<f32>(1.0, 1.0);
    out.position = vec4<f32>(clip_pos.x, -clip_pos.y, 0.0, 1.0);
    return out;
}
```
2. On `MouseWheel` events, update the uniform buffer via `queue.write_buffer()` and request a redraw without re-running `rebuild_display_commands`.

---

## 4. Affected Source Locations
- `src/renderer/gpu/renderer.rs`
- `src/renderer/gpu/shader.rs`
- `src/renderer/window/window.rs`

---

## 5. Verification & Acceptance Criteria
1. Scrolling through a 10,000-line document consumes <2% CPU on modern hardware.
2. Zero calls to `BatchBuilder::build()` occur during pure scrolling.
