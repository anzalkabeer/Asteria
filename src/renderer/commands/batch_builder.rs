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
            match cmd {
                RenderCommand::SolidRect { rect, rgba } => {
                    self.add_quad(rect[0], rect[1], rect[2], rect[3], *rgba);
                }
                RenderCommand::Text {
                    text,
                    rect,
                    rgba,
                    font_size,
                } => {
                    let char_w = font_size * 0.55;
                    let char_h = font_size * 0.85;
                    let text_color = if rgba[3] > 0.0 {
                        *rgba
                    } else {
                        [0.0, 0.0, 0.0, 1.0]
                    };

                    for (idx, ch) in text.chars().enumerate() {
                        if ch.is_whitespace() {
                            continue;
                        }
                        let cx = rect[0] + (idx as f32) * char_w;
                        let cy = rect[1] + font_size * 0.1;
                        self.add_quad(cx, cy, char_w * 0.8, char_h, text_color);
                    }
                }
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
        self.build_batches(commands, 800.0);
    }

    fn add_quad(&mut self, x: f32, y: f32, w: f32, h: f32, rgba: [f32; 4]) {
        let base_idx = self.vertices.len() as u16;

        self.vertices.push(Vertex {
            position: [x, y],
            color: rgba,
        });
        self.vertices.push(Vertex {
            position: [x + w, y],
            color: rgba,
        });
        self.vertices.push(Vertex {
            position: [x + w, y + h],
            color: rgba,
        });
        self.vertices.push(Vertex {
            position: [x, y + h],
            color: rgba,
        });

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
