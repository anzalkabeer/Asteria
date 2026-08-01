pub trait RenderPass {
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);
}
