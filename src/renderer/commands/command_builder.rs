use crate::scene::SceneGraph;

#[derive(Debug, Clone)]
pub enum RenderCommand {
    SolidRect {
        rect: [f32; 4],
        rgba: [f32; 4],
        transform: [f32; 6],
    },
    Text {
        text: String,
        rect: [f32; 4],
        rgba: [f32; 4],
        font_size: f32,
    },
    Image {
        rect: [f32; 4],
        image_id: String,
        transform: [f32; 6],
    },
}

pub struct CommandBuilder {
    pub commands: Vec<RenderCommand>,
}

impl Default for CommandBuilder {
    fn default() -> Self {
        Self::new()
    }
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
            let transform = node.transform;
            match &node.kind {
                crate::scene::SceneNodeKind::SolidRect
                | crate::scene::SceneNodeKind::RoundedRect { .. } => {
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [node.rect.x, node.rect.y, node.rect.width, node.rect.height],
                        rgba: color,
                        transform,
                    });
                }
                crate::scene::SceneNodeKind::BoxShadow { .. } => {}
                crate::scene::SceneNodeKind::Border { widths } => {
                    let x = node.rect.x;
                    let y = node.rect.y;
                    let w = node.rect.width;
                    let h = node.rect.height;

                    if widths.top > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y, w, widths.top],
                            rgba: color,
                            transform,
                        });
                    }
                    if widths.bottom > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y + (h - widths.bottom).max(0.0), w, widths.bottom],
                            rgba: color,
                            transform,
                        });
                    }
                    if widths.left > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y, widths.left, h],
                            rgba: color,
                            transform,
                        });
                    }
                    if widths.right > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x + (w - widths.right).max(0.0), y, widths.right, h],
                            rgba: color,
                            transform,
                        });
                    }
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
                crate::scene::SceneNodeKind::Image => {
                    let x = node.rect.x;
                    let y = node.rect.y;
                    let w = node.rect.width;
                    let h = node.rect.height;

                    let image_id = if let Some(text_run) = &scene.texts[i] {
                        text_run.text.clone()
                    } else {
                        String::new()
                    };

                    self.commands.push(RenderCommand::Image {
                        rect: [x, y, w, h],
                        image_id,
                        transform,
                    });
                }
                _ => {}
            }
        }
    }
}
