use crate::renderer::graph::render_pass::RenderPass;

pub struct ImagePass;

impl ImagePass {
    pub fn new() -> Self {
        Self
    }
}

impl RenderPass for ImagePass {
    fn prepare(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {}
    fn render<'a>(&'a self, _pass: &mut wgpu::RenderPass<'a>) {}
}
