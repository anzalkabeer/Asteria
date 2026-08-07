use crate::renderer::commands::command_builder::RenderCommand;

pub struct BatchPlanner;

impl BatchPlanner {
    pub fn plan(commands: &[RenderCommand]) -> Vec<Vec<&RenderCommand>> {
        let mut batches: Vec<Vec<&RenderCommand>> = Vec::new();
        let mut last_is_text: Option<bool> = None;

        for cmd in commands {
            let is_text = matches!(cmd, RenderCommand::Text { .. });
            if last_is_text != Some(is_text) {
                batches.push(Vec::new());
                last_is_text = Some(is_text);
            }
            batches
                .last_mut()
                .expect("a batch was created above")
                .push(cmd);
        }

        batches
    }
}
