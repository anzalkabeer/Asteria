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
                    let text_color = if rgba[3] > 0.0 {
                        *rgba
                    } else {
                        [0.1, 0.1, 0.1, 1.0]
                    };

                    let scale = (font_size / 9.0).max(1.0);
                    let px = (scale * 0.9).max(1.0);
                    let char_spacing = 6.0 * scale;

                    for (idx, ch) in text.chars().enumerate() {
                        if ch == ' ' {
                            continue;
                        }
                        let cx = rect[0] + (idx as f32) * char_spacing;
                        let cy = rect[1];

                        let font_cols = get_char_5x7(ch);
                        for (col_idx, &col_bits) in font_cols.iter().enumerate() {
                            for row_idx in 0..7 {
                                if (col_bits >> row_idx) & 1 == 1 {
                                    let pixel_x = cx + (col_idx as f32) * px;
                                    let pixel_y = cy + (row_idx as f32) * px;
                                    self.add_quad(pixel_x, pixel_y, px, px, text_color);
                                }
                            }
                        }
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

/// Compact 5x7 bitmap font patterns for ASCII characters
fn get_char_5x7(c: char) -> [u8; 5] {
    match c {
        'A' | 'a' => [0x7c, 0x12, 0x11, 0x12, 0x7c],
        'B' | 'b' => [0x7f, 0x49, 0x49, 0x49, 0x36],
        'C' | 'c' => [0x3e, 0x41, 0x41, 0x41, 0x22],
        'D' | 'd' => [0x7f, 0x41, 0x41, 0x22, 0x1c],
        'E' | 'e' => [0x7f, 0x49, 0x49, 0x49, 0x41],
        'F' | 'f' => [0x7f, 0x09, 0x09, 0x09, 0x01],
        'G' | 'g' => [0x3e, 0x41, 0x49, 0x49, 0x7a],
        'H' | 'h' => [0x7f, 0x08, 0x08, 0x08, 0x7f],
        'I' | 'i' => [0x00, 0x41, 0x7f, 0x41, 0x00],
        'J' | 'j' => [0x20, 0x40, 0x41, 0x3f, 0x01],
        'K' | 'k' => [0x7f, 0x08, 0x14, 0x22, 0x41],
        'L' | 'l' => [0x7f, 0x40, 0x40, 0x40, 0x40],
        'M' | 'm' => [0x7f, 0x02, 0x0c, 0x02, 0x7f],
        'N' | 'n' => [0x7f, 0x04, 0x08, 0x10, 0x7f],
        'O' | 'o' => [0x3e, 0x41, 0x41, 0x41, 0x3e],
        'P' | 'p' => [0x7f, 0x09, 0x09, 0x09, 0x06],
        'Q' | 'q' => [0x3e, 0x41, 0x51, 0x21, 0x5e],
        'R' | 'r' => [0x7f, 0x09, 0x19, 0x29, 0x46],
        'S' | 's' => [0x46, 0x49, 0x49, 0x49, 0x31],
        'T' | 't' => [0x01, 0x01, 0x7f, 0x01, 0x01],
        'U' | 'u' => [0x3f, 0x40, 0x40, 0x40, 0x3f],
        'V' | 'v' => [0x1f, 0x20, 0x40, 0x20, 0x1f],
        'W' | 'w' => [0x3f, 0x40, 0x38, 0x40, 0x3f],
        'X' | 'x' => [0x63, 0x14, 0x08, 0x14, 0x63],
        'Y' | 'y' => [0x07, 0x08, 0x70, 0x08, 0x07],
        'Z' | 'z' => [0x61, 0x51, 0x49, 0x45, 0x43],
        '0' => [0x3e, 0x51, 0x49, 0x45, 0x3e],
        '1' => [0x00, 0x42, 0x7f, 0x40, 0x00],
        '2' => [0x42, 0x61, 0x51, 0x49, 0x46],
        '3' => [0x21, 0x41, 0x45, 0x4b, 0x31],
        '4' => [0x18, 0x14, 0x12, 0x7f, 0x10],
        '5' => [0x27, 0x45, 0x45, 0x45, 0x39],
        '6' => [0x3c, 0x4a, 0x49, 0x49, 0x30],
        '7' => [0x01, 0x71, 0x09, 0x05, 0x03],
        '8' => [0x36, 0x49, 0x49, 0x49, 0x36],
        '9' => [0x06, 0x49, 0x49, 0x29, 0x1e],
        ',' => [0x00, 0x50, 0x30, 0x00, 0x00],
        '.' => [0x00, 0x60, 0x60, 0x00, 0x00],
        '!' => [0x00, 0x00, 0x5f, 0x00, 0x00],
        '?' => [0x02, 0x01, 0x51, 0x09, 0x06],
        ':' => [0x00, 0x36, 0x36, 0x00, 0x00],
        '-' => [0x08, 0x08, 0x08, 0x08, 0x08],
        _ => [0x7f, 0x41, 0x41, 0x41, 0x7f],
    }
}
