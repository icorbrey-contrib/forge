mod completion;
mod error;
mod walker;
use std::path::PathBuf;
mod prompt;
use completion::Completion;
pub use error::*;
use futures::future::join_all;
use futures::FutureExt;
use prompt::Prompt;
use serde::{Deserialize, Serialize};
use walker::Walker;

pub struct UserPrompt {
    walker: Walker,
}

#[derive(Serialize, Deserialize)]
pub struct ResolvePrompt {
    pub message: String,
    pub files: Vec<File>,
}

#[derive(Serialize, Deserialize)]
pub struct File {
    pub path: String,
    pub content: String,
}

impl UserPrompt {
    pub fn new(cwd: PathBuf) -> Self {
        Self { walker: Walker::new(cwd) }
    }

    pub async fn ask(&self, message: Option<&str>) -> Result<ResolvePrompt> {
        let suggestions = self.walker.get()?;
        let completions = Completion::new(suggestions.iter().map(|s| format!("@{}", s)).collect());

        let input = inquire::Text::new(message.unwrap_or(""))
            .with_autocomplete(completions)
            .prompt()?;

        let prompt = Prompt::parse(input).map_err(Error::Parse)?;

        let files = join_all(prompt.files().into_iter().map(|path| {
            tokio::fs::read_to_string(path.clone())
                .map(|result| result.map(|content| File { path, content }))
        }))
        .await;

        Ok(ResolvePrompt {
            message: prompt.message(),
            files: files.into_iter().flatten().collect(),
        })
    }
}
