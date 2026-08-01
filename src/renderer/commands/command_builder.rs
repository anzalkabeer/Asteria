use crate::scene::{SceneGraph, SceneNode};

pub enum RenderCommand {
    SolidRect { rect: [f32; 4], rgba: [f32; 4] },
    Text { text: String, rect: [f32; 4], rgba: [f32; 4], font_size: f32 },
    // Image { rect: [f32; 4], image_id: String },
}

pub struct CommandBuilder {
    pub commands: Vec<RenderCommand>,
}

impl CommandBuilder {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn build_from_scene(&mut self, scene: &SceneGraph) {
        self.commands.clear();

        for (i, node) in scene.nodes.iter().enumerate() {
            let color = scene.colors[i];
            match &node.kind {
                crate::scene::SceneNodeKind::SolidRect => {
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [node.rect.x, node.rect.y, node.rect.width, node.rect.height],
                        rgba: color,
                    });
                }
                crate::scene::SceneNodeKind::Text { font_size } => {
                    if let Some(text_run) = &scene.texts[i] {
                        self.commands.push(RenderCommand::Text {
                            rect: [node.rect.x, node.rect.y, node.rect.width, node.rect.height],
                            rgba: color,
                            text: text_run.text.clone(),
                            font_size: *font_size,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}
