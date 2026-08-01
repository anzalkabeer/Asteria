use crate::renderer::commands::command_builder::RenderCommand;

pub struct BatchPlanner;

impl BatchPlanner {
    pub fn plan(commands: &[RenderCommand]) -> Vec<Vec<&RenderCommand>> {
        // Group commands by type to minimize pipeline state changes
        let mut rect_batch = Vec::new();
        let mut text_batch = Vec::new();

        for cmd in commands {
            match cmd {
                RenderCommand::SolidRect { .. } => rect_batch.push(cmd),
                RenderCommand::Text { .. } => text_batch.push(cmd),
            }
        }

        vec![rect_batch, text_batch]
    }
}
