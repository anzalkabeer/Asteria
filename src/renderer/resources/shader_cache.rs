use std::collections::HashMap;

pub struct ShaderCache {
    modules: HashMap<String, wgpu::ShaderModule>,
}

impl ShaderCache {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&wgpu::ShaderModule> {
        self.modules.get(name)
    }

    pub fn insert(&mut self, name: String, module: wgpu::ShaderModule) {
        self.modules.insert(name, module);
    }
}
