use std::collections::HashMap;

pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub bind_group: wgpu::BindGroup,
}

pub struct TextureCache {
    textures: HashMap<String, GpuTexture>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TextureCache {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }

    pub fn get(&self, id: &str) -> Option<&GpuTexture> {
        self.textures.get(id)
    }

    pub fn insert(&mut self, id: String, texture: GpuTexture) {
        self.textures.insert(id, texture);
    }
}
