use crate::renderer::graph::render_pass::RenderPass;

pub struct TextPass;

impl TextPass {
    pub fn new() -> Self {
        Self
    }
}

impl RenderPass for TextPass {
    fn prepare(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) {}
    fn render<'a>(&'a self, _pass: &mut wgpu::RenderPass<'a>) {}
}
