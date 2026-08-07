use crate::renderer::graph::render_pass::RenderPass;

pub struct RenderGraph {
    passes: Vec<Box<dyn RenderPass>>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderGraph {
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    pub fn add_pass(&mut self, pass: Box<dyn RenderPass>) {
        self.passes.push(pass);
    }

    /// Downcast a pass at `index` to a concrete type `T`.
    /// Returns `None` if the index is out of bounds or the type doesn't match.
    pub fn pass_downcast_mut<T: 'static>(&mut self, index: usize) -> Option<&mut T> {
        self.passes.get_mut(index)?.as_any_mut().downcast_mut::<T>()
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        for pass in &mut self.passes {
            pass.prepare(device, queue);
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for pass in &self.passes {
            pass.render(render_pass);
        }
    }
}
