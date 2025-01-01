use std::sync::Arc;

use forge_env::Environment;
use forge_provider::{Model, ModelId, Provider, Request};
use forge_tool::{ToolDefinition, ToolEngine};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

use crate::app::{Action, App, ChatRequest, ChatResponse};
use crate::completion::{Completion, File};
use crate::executor::ChatCommandExecutor;
use crate::runtime::ApplicationRuntime;
use crate::Result;

#[derive(Clone)]
pub struct Server {
    provider: Arc<Provider>,
    tools: Arc<ToolEngine>,
    completions: Arc<Completion>,
    runtime: Arc<ApplicationRuntime<App>>,
    env: Environment,
    api_key: String,
}

impl Server {
    pub fn new(env: Environment, api_key: impl Into<String>) -> Server {
        let tools = ToolEngine::new();

        let request = Request::new(ModelId::default());

        let cwd: String = env.cwd.clone();
        let api_key: String = api_key.into();

        Self {
            env,
            provider: Arc::new(Provider::open_router(api_key.clone(), None)),
            tools: Arc::new(tools),
            completions: Arc::new(Completion::new(cwd.clone())),
            runtime: Arc::new(ApplicationRuntime::new(App::new(request))),
            api_key,
        }
    }

    pub async fn completions(&self) -> Result<Vec<File>> {
        self.completions.list().await
    }

    pub fn tools(&self) -> Vec<ToolDefinition> {
        self.tools.list()
    }

    pub async fn context(&self) -> Request {
        self.runtime.state().await.request().clone()
    }

    pub async fn models(&self) -> Result<Vec<Model>> {
        Ok(self.provider.models().await?)
    }

    pub async fn chat(&self, chat: ChatRequest) -> Result<impl Stream<Item = ChatResponse> + Send> {
        let (tx, rx) = mpsc::channel::<ChatResponse>(100);
        let executor = ChatCommandExecutor::new(self.env.clone(), self.api_key.clone(), tx);
        let runtime = self.runtime.clone();
        let message = format!("<task>{}</task>", chat.content);

        tokio::spawn(async move {
            runtime
                .clone()
                .execute(
                    Action::UserMessage(chat.content(message)),
                    Arc::new(executor),
                )
                .await
        });

        Ok(ReceiverStream::new(rx))
    }
}
