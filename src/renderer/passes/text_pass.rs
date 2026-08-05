use crate::renderer::graph::render_pass::RenderPass;
use glyphon::{
    Attrs, Buffer, Cache, Color, FontSystem, Metrics, Resolution, Shaping, SwashCache, TextArea,
    TextAtlas, TextBounds, TextRenderer, Viewport,
};

pub struct TextPass {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    renderer: TextRenderer,
    viewport: Viewport,
    buffers: Vec<(Buffer, [f32; 2], Color)>,
    width: u32,
    height: u32,
}

impl TextPass {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let renderer =
            TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        let mut viewport = Viewport::new(device, &cache);
        viewport.update(queue, Resolution { width, height });

        Self {
            font_system,
            swash_cache,
            atlas,
            renderer,
            viewport,
            buffers: Vec::new(),
            width,
            height,
        }
    }

    pub fn add_text(&mut self, text: &str, pos: [f32; 2], font_size: f32, color: [f32; 4]) {
        let line_height = font_size * 1.2;
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(font_size, line_height));

        let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        let (r, g, b, a) = (
            to_u8(color[0]),
            to_u8(color[1]),
            to_u8(color[2]),
            to_u8(color[3]),
        );
        let glyphon_color = Color::rgba(r, g, b, a);

        buffer.set_size(
            &mut self.font_system,
            Some(self.width as f32),
            Some(self.height as f32),
        );
        buffer.set_text(&mut self.font_system, text, Attrs::new(), Shaping::Advanced);

        self.buffers.push((buffer, pos, glyphon_color));
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    pub fn clear(&mut self) {
        self.buffers.clear();
    }
}

impl RenderPass for TextPass {
    fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.viewport.update(
            queue,
            Resolution {
                width: self.width,
                height: self.height,
            },
        );

        let text_areas: Vec<TextArea> = self
            .buffers
            .iter()
            .map(|(buffer, pos, color)| TextArea {
                buffer,
                left: pos[0],
                top: pos[1],
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: self.width as i32,
                    bottom: self.height as i32,
                },
                default_color: *color,
                custom_glyphs: &[],
            })
            .collect();

        let _ = self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        );
    }

    fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let _ = self.renderer.render(&self.atlas, &self.viewport, pass);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
