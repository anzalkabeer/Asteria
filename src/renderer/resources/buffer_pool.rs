use std::collections::VecDeque;

const MAX_FREE_BUFFERS: usize = 8;

struct PooledBuffer {
    buffer: wgpu::Buffer,
    capacity: usize,
}

pub struct BufferPool {
    free_vertex_buffers: VecDeque<PooledBuffer>,
    free_index_buffers: VecDeque<PooledBuffer>,
}

impl Default for BufferPool {
    fn default() -> Self {
        Self::new()
    }
}

fn build_aligned_payload(data: &[u8], required_capacity: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(required_capacity);
    payload.extend_from_slice(data);
    if payload.len() < required_capacity {
        payload.resize(required_capacity, 0);
    }
    payload
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            free_vertex_buffers: VecDeque::new(),
            free_index_buffers: VecDeque::new(),
        }
    }

    pub fn acquire_vertex_buffer(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        data: &[u8],
    ) -> wgpu::Buffer {
        let required_capacity = (data.len().max(1) as u64)
            .max(wgpu::COPY_BUFFER_ALIGNMENT)
            .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) as usize;
        let payload = build_aligned_payload(data, required_capacity);

        if let Some(index) = self
            .free_vertex_buffers
            .iter()
            .position(|pooled| pooled.capacity >= required_capacity)
        {
            let pooled = self.free_vertex_buffers.remove(index).unwrap();
            queue.write_buffer(&pooled.buffer, 0, &payload);
            return pooled.buffer;
        }

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: required_capacity as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, &payload);
        buffer
    }

    pub fn acquire_index_buffer(
        &mut self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        data: &[u8],
    ) -> wgpu::Buffer {
        let required_capacity = (data.len().max(1) as u64)
            .max(wgpu::COPY_BUFFER_ALIGNMENT)
            .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) as usize;
        let payload = build_aligned_payload(data, required_capacity);

        if let Some(index) = self
            .free_index_buffers
            .iter()
            .position(|pooled| pooled.capacity >= required_capacity)
        {
            let pooled = self.free_index_buffers.remove(index).unwrap();
            queue.write_buffer(&pooled.buffer, 0, &payload);
            return pooled.buffer;
        }

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Index Buffer"),
            size: required_capacity as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&buffer, 0, &payload);
        buffer
    }

    pub fn release_vertex_buffer(&mut self, buffer: wgpu::Buffer) {
        if self.free_vertex_buffers.len() >= MAX_FREE_BUFFERS {
            return;
        }

        let capacity = buffer.size() as usize;
        self.free_vertex_buffers
            .push_back(PooledBuffer { buffer, capacity });
    }

    pub fn release_index_buffer(&mut self, buffer: wgpu::Buffer) {
        if self.free_index_buffers.len() >= MAX_FREE_BUFFERS {
            return;
        }

        let capacity = buffer.size() as usize;
        self.free_index_buffers
            .push_back(PooledBuffer { buffer, capacity });
    }
}

/// Persistent retained geometry buffer pool to eliminate per-frame GPU buffer reallocations.
/// Implements SPEC-01 (AST-PERF-001).
pub struct GeometryBufferPool {
    vertex_buffer: Option<wgpu::Buffer>,
    vertex_capacity_bytes: usize,
    index_buffer: Option<wgpu::Buffer>,
    index_capacity_bytes: usize,
    pub allocations_count: usize,
}

impl Default for GeometryBufferPool {
    fn default() -> Self {
        Self::new()
    }
}

impl GeometryBufferPool {
    pub fn new() -> Self {
        Self {
            vertex_buffer: None,
            vertex_capacity_bytes: 0,
            index_buffer: None,
            index_capacity_bytes: 0,
            allocations_count: 0,
        }
    }

    pub fn vertex_buffer(&self) -> Option<&wgpu::Buffer> {
        self.vertex_buffer.as_ref()
    }

    pub fn index_buffer(&self) -> Option<&wgpu::Buffer> {
        self.index_buffer.as_ref()
    }

    pub fn vertex_capacity(&self) -> usize {
        self.vertex_capacity_bytes
    }

    pub fn index_capacity(&self) -> usize {
        self.index_capacity_bytes
    }

    /// Uploads vertex and index data. If existing buffers have sufficient capacity,
    /// updates them in-place via `queue.write_buffer()` with 0 buffer allocations.
    /// Resizes geometrically (1.5x) only when geometry exceeds current capacity.
    pub fn upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        vertex_data: &[u8],
        index_data: &[u8],
    ) {
        if vertex_data.is_empty() && index_data.is_empty() {
            return;
        }

        if !vertex_data.is_empty() {
            let aligned_len = (vertex_data.len() as u64)
                .max(wgpu::COPY_BUFFER_ALIGNMENT)
                .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) as usize;

            if self.vertex_buffer.is_none() || aligned_len > self.vertex_capacity_bytes {
                self.vertex_capacity_bytes = aligned_len
                    .max(self.vertex_capacity_bytes * 3 / 2)
                    .max(1024);
                self.vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Asteria Retained Vertex Buffer"),
                    size: self.vertex_capacity_bytes as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                self.allocations_count += 1;
            }

            if let Some(vb) = &self.vertex_buffer {
                queue.write_buffer(vb, 0, vertex_data);
            }
        }

        if !index_data.is_empty() {
            let aligned_len = (index_data.len() as u64)
                .max(wgpu::COPY_BUFFER_ALIGNMENT)
                .next_multiple_of(wgpu::COPY_BUFFER_ALIGNMENT) as usize;

            if self.index_buffer.is_none() || aligned_len > self.index_capacity_bytes {
                self.index_capacity_bytes = aligned_len
                    .max(self.index_capacity_bytes * 3 / 2)
                    .max(1024);
                self.index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Asteria Retained Index Buffer"),
                    size: self.index_capacity_bytes as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }));
                self.allocations_count += 1;
            }

            if let Some(ib) = &self.index_buffer {
                queue.write_buffer(ib, 0, index_data);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geometry_buffer_pool_initial_state() {
        let pool = GeometryBufferPool::new();
        assert_eq!(pool.vertex_capacity(), 0);
        assert_eq!(pool.index_capacity(), 0);
        assert_eq!(pool.allocations_count, 0);
        assert!(pool.vertex_buffer().is_none());
        assert!(pool.index_buffer().is_none());
    }

    #[test]
    fn test_retained_buffer_pool() {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        }));

        let (device, queue) = match adapter {
            Some(ad) => {
                match pollster::block_on(ad.request_device(&wgpu::DeviceDescriptor::default(), None))
                {
                    Ok(dq) => dq,
                    Err(_) => return,
                }
            }
            None => return,
        };

        let mut pool = GeometryBufferPool::new();
        let vertices = [1.0f32, 2.0, 3.0, 4.0];
        let indices = [0u16, 1, 2, 3];
        let v_bytes = bytemuck::cast_slice(&vertices);
        let i_bytes = bytemuck::cast_slice(&indices);

        // Frame 1: Initial upload creates 2 buffers
        pool.upload(&device, &queue, v_bytes, i_bytes);
        assert_eq!(pool.allocations_count, 2);
        assert!(pool.vertex_buffer().is_some());
        assert!(pool.index_buffer().is_some());

        // Frames 2..=100: 99 consecutive frames with static geometry make ZERO new allocations
        for _ in 2..=100 {
            pool.upload(&device, &queue, v_bytes, i_bytes);
        }
        assert_eq!(
            pool.allocations_count, 2,
            "100 consecutive frames allocate zero new GPU buffers"
        );
    }
}
