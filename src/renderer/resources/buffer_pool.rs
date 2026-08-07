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
