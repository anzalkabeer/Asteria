use crate::renderer::commands::command_builder::RenderCommand;
use crate::scene::SceneGraph;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

pub struct BatchBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BatchBuilder {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn build_batches(&mut self, commands: &[RenderCommand]) {
        self.vertices.clear();
        self.indices.clear();

        for cmd in commands {
            if let RenderCommand::SolidRect { rect, rgba } = cmd {
                let base_idx = self.vertices.len() as u16;

                // 4 corners of the quad
                let (x, y, w, h) = (rect[0], rect[1], rect[2], rect[3]);

                self.vertices.push(Vertex {
                    position: [x, y],
                    color: *rgba,
                });
                self.vertices.push(Vertex {
                    position: [x + w, y],
                    color: *rgba,
                });
                self.vertices.push(Vertex {
                    position: [x + w, y + h],
                    color: *rgba,
                });
                self.vertices.push(Vertex {
                    position: [x, y + h],
                    color: *rgba,
                });

                // 2 triangles per quad
                self.indices.extend_from_slice(&[
                    base_idx,
                    base_idx + 1,
                    base_idx + 2,
                    base_idx,
                    base_idx + 2,
                    base_idx + 3,
                ]);
            }
        }
    }

    /// Build vertex/index buffers only if there are dirty ViewportSegments.
    /// Skips work entirely when 0 tiles are dirty (0% GPU/VRAM overhead).
    pub fn build_dirty_batches(&mut self, commands: &[RenderCommand], scene: &SceneGraph) {
        let dirty_segs = scene.dirty_segments();
        if dirty_segs.is_empty() {
            return;
        }
        self.build_batches(commands);
    }
}
