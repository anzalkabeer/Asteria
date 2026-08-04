use crate::renderer::commands::batch_builder::{BatchBuilder, Vertex};
use crate::renderer::graph::render_pass::RenderPass;
use wgpu::util::DeviceExt;

pub struct RectPass {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    num_indices: u32,
}

impl RectPass {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Rect Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rect Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Rect Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            pipeline,
            vertex_buffer: None,
            index_buffer: None,
            num_indices: 0,
        }
    }

    pub fn update_buffers(&mut self, device: &wgpu::Device, batch: &BatchBuilder) {
        if batch.indices.is_empty() {
            self.num_indices = 0;
            return;
        }

        self.vertex_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Rect Vertex Buffer"),
                contents: bytemuck::cast_slice(&batch.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
        );

        self.index_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Rect Index Buffer"),
                contents: bytemuck::cast_slice(&batch.indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
        );

        self.num_indices = batch.indices.len() as u32;
    }
}

impl RenderPass for RectPass {
    fn prepare(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {
        // Buffers are updated explicitly via `update_buffers` from the scheduler before prepare is called in this simplified version.
    }

    #[allow(clippy::collapsible_if)]
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.num_indices > 0 {
            if let (Some(vb), Some(ib)) = (&self.vertex_buffer, &self.index_buffer) {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, vb.slice(..));
                pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint16);
                pass.draw_indexed(0..self.num_indices, 0, 0..1);
            }
        }
    }
}
