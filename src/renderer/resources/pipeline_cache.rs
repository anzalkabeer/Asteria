use std::collections::HashMap;

pub struct PipelineCache {
    pipelines: HashMap<String, wgpu::RenderPipeline>,
}

impl Default for PipelineCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineCache {
    pub fn new() -> Self {
        Self {
            pipelines: HashMap::new(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&wgpu::RenderPipeline> {
        self.pipelines.get(name)
    }

    pub fn insert(&mut self, name: String, pipeline: wgpu::RenderPipeline) {
        self.pipelines.insert(name, pipeline);
    }
}
