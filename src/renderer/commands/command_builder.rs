use crate::scene::SceneGraph;

#[derive(Debug, Clone)]
pub enum RenderCommand {
    SolidRect {
        rect: [f32; 4],
        rgba: [f32; 4],
    },
    Text {
        text: String,
        rect: [f32; 4],
        rgba: [f32; 4],
        font_size: f32,
    },
    // Image { rect: [f32; 4], image_id: String },
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
            match &node.kind {
                crate::scene::SceneNodeKind::SolidRect => {
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [node.rect.x, node.rect.y, node.rect.width, node.rect.height],
                        rgba: color,
                    });
                }
                crate::scene::SceneNodeKind::Border { widths } => {
                    let x = node.rect.x;
                    let y = node.rect.y;
                    let w = node.rect.width;
                    let h = node.rect.height;

                    if widths.top > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y, w, widths.top],
                            rgba: color,
                        });
                    }
                    if widths.bottom > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y + (h - widths.bottom).max(0.0), w, widths.bottom],
                            rgba: color,
                        });
                    }
                    if widths.left > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x, y, widths.left, h],
                            rgba: color,
                        });
                    }
                    if widths.right > 0.0 {
                        self.commands.push(RenderCommand::SolidRect {
                            rect: [x + (w - widths.right).max(0.0), y, widths.right, h],
                            rgba: color,
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
                    let w = node.rect.width.max(120.0);
                    let h = node.rect.height.max(80.0);

                    self.commands.push(RenderCommand::SolidRect {
                        rect: [x, y, w, h],
                        rgba: [0.88, 0.91, 0.94, 1.0],
                    });
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [x, y, w, 1.0],
                        rgba: [0.80, 0.84, 0.88, 1.0],
                    });
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [x, y + (h - 1.0).max(0.0), w, 1.0],
                        rgba: [0.80, 0.84, 0.88, 1.0],
                    });
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [x, y, 1.0, h],
                        rgba: [0.80, 0.84, 0.88, 1.0],
                    });
                    self.commands.push(RenderCommand::SolidRect {
                        rect: [x + (w - 1.0).max(0.0), y, 1.0, h],
                        rgba: [0.80, 0.84, 0.88, 1.0],
                    });

                    if let Some(text_run) = &scene.texts[i] {
                        let raw = &text_run.text;
                        let label = if raw.is_empty() {
                            "[IMG]".to_string()
                        } else if raw.len() > 14 {
                            format!("[IMG: {}..]", &raw[..12.min(raw.len())])
                        } else {
                            raw.clone()
                        };
                        self.commands.push(RenderCommand::Text {
                            rect: [x + 8.0, y + (h / 2.0) - 8.0, w - 16.0, 16.0],
                            rgba: [0.2, 0.35, 0.55, 1.0],
                            text: label,
                            font_size: 12.0,
                        });
                    }
                }
                _ => {}
            }
        }
    }
}
