# ISSUE: AST-PERF-001 — Excessive Per-Frame GPU Buffer Reallocation

- **Classification:** PERFORMANCE CONCERN
- **Severity:** High
- **Priority:** P0
- **Status:** **RESOLVED**
- **Affected Subsystem:** GPU Renderer & Batch Builder
- **Specification Link:** [`../specifications/SPEC-01.md`](../specifications/SPEC-01.md)

---

## 1. Problem Statement
In `src/renderer/gpu/renderer.rs:310-380` and `src/renderer/gpu/batch_builder.rs:45-120`, Asteria invokes `device.create_buffer_init` on every frame for both vertex buffers and index buffers. This causes continuous CPU and driver memory allocation, GPU VRAM heap fragmentation, and dropped frames.

---

## 2. Evidence & Source Location
- **Location:** [`src/renderer/gpu/renderer.rs:310-380`](../../../src/renderer/gpu/renderer.rs#L310-L380)
```rust
let vertex_buffer = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Vertex Buffer"),
    contents: bytemuck::cast_slice(&batch.vertices),
    usage: wgpu::BufferUsages::VERTEX,
}); // Called every frame!
```

---

## 3. Root Cause
The renderer was written with a naive immediate-mode pattern instead of maintaining persistent, pre-allocated GPU ring buffers.

---

## 4. Desired Behaviour
Implement a persistent `GeometryBufferPool` that retains pre-allocated vertex and index buffers across frames, updating existing VRAM via `queue.write_buffer()` and only resizing when geometry exceeds capacity.

---

## 5. Resolution & Verification
- Implemented `GeometryBufferPool` in `src/renderer/resources/buffer_pool.rs` with geometric 1.5x buffer capacity expansion.
- Integrated `GeometryBufferPool` into `RectPass` (`src/renderer/passes/rect_pass.rs`) and `ImagePass` (`src/renderer/passes/image_pass.rs`).
- Connected queue write updates in `src/renderer/window/window.rs`.
- Verified via `test_retained_buffer_pool`: 100 consecutive frames with static geometry make zero new calls to `device.create_buffer()`.
