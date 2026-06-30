use std::path::Path;

use prompt_buffer::{PluginSpeed, PromptBufferPlugin, PromptLines, ShellType};

pub struct Tap(Box<dyn PromptBufferPlugin>);

impl Tap {
    pub fn new(plugin: impl PromptBufferPlugin + 'static) -> Self {
        Self(Box::new(plugin))
    }
}

#[async_trait::async_trait(?Send)]
impl PromptBufferPlugin for Tap {
    async fn run(
        &mut self,
        speed: PluginSpeed,
        shell: ShellType,
        path: &Path,
    ) -> Result<PromptLines, eyre::Report> {
        let output = self.0.run(speed, shell, path).await;
        if let Err(e) = &output {
            println!("Tap plugin failed: {e:?}");
        }
        output
    }
}
