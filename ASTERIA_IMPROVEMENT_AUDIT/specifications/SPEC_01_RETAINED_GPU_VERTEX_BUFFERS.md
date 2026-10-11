# TECHNICAL SPECIFICATION: SPEC-01
## Retained GPU Vertex Buffers & Persistent Geometry Pool

---

## 1. Problem Statement & Motivation
Currently in `src/renderer/gpu/renderer.rs:310-380` and `src/renderer/gpu/batch_builder.rs:45-120`, Asteria invokes `device.create_buffer_init` on every frame for both vertex buffers and index buffers (`AST-PERF-001`). This introduces massive CPU/GPU heap thrashing, continuous driver memory allocation, and frame stutter.

---

## 2. Root Cause
`BatchBuilder` and `Renderer` were designed with a naive immediate-mode pattern where geometry buffers are discarded and recreated every frame rather than maintaining retained, pre-allocated GPU ring buffers.

---

## 3. Desired Architecture
The renderer must allocate persistent vertex and index buffers sized for typical document workloads (e.g. 1 MB vertex buffer, 256 KB index buffer). When geometry is updated:
1. If the existing buffer capacity is sufficient, write new vertex slices into the existing buffer via `queue.write_buffer()`.
2. Only reallocate the GPU buffer if the required geometry size exceeds the current capacity (with a 1.5x growth factor).

```rust
pub struct GeometryBufferPool {
    vertex_buffer: wgpu::Buffer,
    vertex_capacity_bytes: usize,
    index_buffer: wgpu::Buffer,
    index_capacity_bytes: usize,
}

impl GeometryBufferPool {
    pub fn new(device: &wgpu::Device, initial_vertices: usize, initial_indices: usize) -> Self { ... }
    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex], indices: &[u16]) {
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
        // Same logic for indices...
    }
}
```

---

## 4. Affected Source Locations
- `src/renderer/gpu/renderer.rs`
- `src/renderer/gpu/batch_builder.rs`
- Target crate in modular workspace: `crates/asteria_render`

---

## 5. Verification & Acceptance Criteria
1. `test_retained_buffer_pool_reallocation`: Validate that 100 consecutive frames with identical geometry execute zero calls to `device.create_buffer()`.
2. GPU memory profiling shows steady VRAM allocation without per-frame churn.
