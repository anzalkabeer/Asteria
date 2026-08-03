use std::collections::VecDeque;
use wgpu::util::DeviceExt;

pub struct BufferPool {
    free_vertex_buffers: VecDeque<wgpu::Buffer>,
    free_index_buffers: VecDeque<wgpu::Buffer>,
}

impl BufferPool {
    pub fn new() -> Self {
        Self {
            free_vertex_buffers: VecDeque::new(),
            free_index_buffers: VecDeque::new(),
        }
    }

    pub fn acquire_vertex_buffer(&mut self, device: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
        // In a real robust implementation, we would reuse buffers that fit the size.
        // For this milestone, we'll implement simple reallocation if empty.
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: data,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn acquire_index_buffer(&mut self, device: &wgpu::Device, data: &[u8]) -> wgpu::Buffer {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: data,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn release_vertex_buffer(&mut self, buffer: wgpu::Buffer) {
        self.free_vertex_buffers.push_back(buffer);
    }

    pub fn release_index_buffer(&mut self, buffer: wgpu::Buffer) {
        self.free_index_buffers.push_back(buffer);
    }
}
