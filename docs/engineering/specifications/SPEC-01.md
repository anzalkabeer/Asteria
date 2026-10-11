# SPEC-01: Retained GPU Vertex Buffers & Persistent Geometry Pool

- **Related Issue:** [`../issues/AST-PERF-001.md`](../issues/AST-PERF-001.md)
- **Target Subsystem:** GPU Renderer (`src/renderer/gpu/renderer.rs`, `src/renderer/gpu/batch_builder.rs`)
- **Stage:** Stage 1 (P0)

---

## 1. Problem Statement
In `src/renderer/gpu/renderer.rs:310-380`, Asteria calls `device.create_buffer_init` on every frame for both vertex buffers and index buffers. This causes continuous VRAM heap churn, driver overhead, and frame drops.

---

## 2. Root Cause
The renderer lacks a retained geometry pool and reallocates GPU memory unconditionally rather than updating existing buffers via `queue.write_buffer()`.

---

## 3. Recommended Solution
Implement a persistent `GeometryBufferPool`:
```rust
pub struct GeometryBufferPool {
    vertex_buffer: wgpu::Buffer,
    vertex_capacity_bytes: usize,
    index_buffer: wgpu::Buffer,
    index_capacity_bytes: usize,
}

impl GeometryBufferPool {
    pub fn new(device: &wgpu::Device, initial_vertex_kb: usize, initial_index_kb: usize) -> Self { ... }

    pub fn upload(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex], indices: &[u16]) {
        let v_bytes = bytemuck::cast_slice(vertices);
        if v_bytes.len() > self.vertex_capacity_bytes {
            self.vertex_capacity_bytes = v_bytes.len().max(self.vertex_capacity_bytes * 3 / 2);
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Asteria Retained Vertex Buffer"),
                size: self.vertex_capacity_bytes as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.vertex_buffer, 0, v_bytes);
        // Equivalent growth & write logic for indices...
    }
}
```

---

## 4. Affected Source Locations
- `src/renderer/gpu/renderer.rs`
- `src/renderer/gpu/batch_builder.rs`

---

## 5. Verification & Acceptance Criteria
1. `test_retained_buffer_pool`: Assert that 100 consecutive frames with static geometry make zero calls to `device.create_buffer()`.
2. GPU VRAM allocation remains flat during continuous animation/rendering.
