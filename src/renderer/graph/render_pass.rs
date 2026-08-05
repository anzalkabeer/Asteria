use std::any::Any;

/// Trait object interface for all GPU render passes.
///
/// Passes are stored as `Box<dyn RenderPass>` inside the RenderGraph.
/// The `as_any_mut()` method enables safe downcasting to concrete types
/// when pass-specific methods (e.g. `update_buffers`, `add_text`) are needed.
pub trait RenderPass: Any {
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue);
    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>);

    /// Downcast support — enables RenderGraph to access pass-specific methods.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
