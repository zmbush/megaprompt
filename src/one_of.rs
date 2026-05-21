use std::path::Path;

use log::trace;
use prompt_buffer::{PluginSpeed, PromptBufferPlugin, PromptLines, ShellType};

pub struct OneOf {
    options: Vec<Box<dyn PromptBufferPlugin>>,
}

impl OneOf {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
        }
    }

    pub fn with(mut self, plugin: impl PromptBufferPlugin + 'static) -> Self {
        self.options.push(Box::new(plugin));
        self
    }
}

#[async_trait::async_trait(?Send)]
impl PromptBufferPlugin for OneOf {
    async fn run(
        &mut self,
        speed: PluginSpeed,
        shell: ShellType,
        path: &Path,
        lines: &mut PromptLines,
    ) -> Result<(), eyre::Report> {
        for option in &mut self.options {
            match option.run(speed, shell, path, lines).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    trace!("OneOf plugin failed: {e:?}");
                }
            }
        }

        Err(eyre::eyre!("No option selected in OneOf plugin"))
    }
}
