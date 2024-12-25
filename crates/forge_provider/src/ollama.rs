use std::collections::HashMap;

use forge_tool::{Tool, ToolId};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;

use super::error::Result;
use super::provider::{InnerProvider, Provider};
use crate::model::{AnyMessage, Assistant, Role, System, ToolUse, UseId, User};
use crate::ResultStream;

const DEFAULT_MODEL: &str = "ollama/default-model";

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Model {
    id: String,
    name: String,
    created: u64,
    description: String,
    context_length: u64,
    architecture: Architecture,
    pricing: Pricing,
    top_provider: TopProvider,
    per_request_limits: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Architecture {
    modality: String,
    tokenizer: String,
    instruct_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Pricing {
    prompt: String,
    completion: String,
    image: String,
    request: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct TopProvider {
    context_length: Option<u64>,
    max_completion_tokens: Option<u64>,
    is_moderated: bool,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Serialize)]
struct ListModelResponse {
    data: Vec<Model>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct Request {
    #[serde(skip_serializing_if = "Option::is_none")]
    messages: Option<Vec<Message>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt: Option<String>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    logit_bias: Option<HashMap<u32, f32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_logprobs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_a: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prediction: Option<Prediction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    models: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    route: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<ProviderPreferences>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TextContent {
    r#type: String,
    text: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ImageContentPart {
    r#type: String,
    image_url: ImageUrl,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ImageUrl {
    url: String,
    detail: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
enum ContentPart {
    Text(TextContent),
    Image(ImageContentPart),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Message {
    role: String,
    content: String,
    name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FunctionDescription {
    description: Option<String>,
    name: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct OllamaTool {
    r#type: String,
    function: FunctionDescription,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
enum ToolChoice {
    None,
    Auto,
    Function { name: String },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ResponseFormat {
    r#type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Prediction {
    r#type: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Response {
    id: String,
    provider: String,
    model: String,
    choices: Vec<Choice>,
    created: u64,
    object: String,
    system_fingerprint: Option<String>,
    usage: Option<ResponseUsage>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ResponseUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
enum Choice {
    NonChat {
        finish_reason: Option<String>,
        text: String,
        error: Option<ErrorResponse>,
    },
    NonStreaming {
        logprobs: Option<serde_json::Value>,
        index: u32,
        finish_reason: Option<String>,
        message: ResponseMessage,
        error: Option<ErrorResponse>,
    },
    Streaming {
        finish_reason: Option<String>,
        delta: ResponseMessage,
        error: Option<ErrorResponse>,
    },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ErrorResponse {
    code: u32,
    message: String,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ResponseMessage {
    content: Option<String>,
    role: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
    refusal: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ToolCall {
    id: Option<String>,
    r#type: String,
    function: FunctionCall,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct FunctionCall {
    name: String,
    arguments: String,
}

impl From<Tool> for OllamaTool {
    fn from(value: Tool) -> Self {
        OllamaTool {
            r#type: "function".to_string(),
            function: FunctionDescription {
                description: Some(value.description),
                name: value.id.into_string(),
                parameters: serde_json::to_value(value.input_schema).unwrap(),
            },
        }
    }
}

impl From<AnyMessage> for Message {
    fn from(value: AnyMessage) -> Self {
        match value {
            AnyMessage::Assistant(assistant) => Message {
                role: Assistant::name(),
                content: assistant.content,
                name: None,
            },
            AnyMessage::System(sys) => {
                Message { role: System::name(), content: sys.content, name: None }
            }
            AnyMessage::User(usr) => {
                Message { role: User::name(), content: usr.content, name: None }
            }
        }
    }
}

impl From<crate::model::Request> for Request {
    fn from(value: crate::model::Request) -> Self {
        Request {
            messages: {
                let result = value
                    .tool_result
                    .into_iter()
                    .map(|tool_result| {
                        let value = tool_result.content;

                        let mut content = HashMap::new();
                        content.insert("content", value.to_string());
                        content.insert("role", "tool".to_string());

                        if let Some(id) = tool_result.tool_use_id {
                            content.insert("tool_use_id", id.0);
                        }
                        Message {
                            role: User::name(),
                            content: serde_json::to_string(&content).unwrap(),
                            name: None,
                        }
                    })
                    .collect::<Vec<_>>();

                let mut messages = value
                    .context
                    .into_iter()
                    .map(Message::from)
                    .collect::<Vec<_>>();

                messages.extend(result);

                Some(messages)
            },
            tools: {
                let tools = value
                    .tools
                    .into_iter()
                    .map(OllamaTool::from)
                    .collect::<Vec<_>>();
                if tools.is_empty() {
                    None
                } else {
                    Some(tools)
                }
            },
            ..Default::default()
        }
    }
}

impl TryFrom<Response> for crate::model::Response {
    type Error = crate::error::Error;

    fn try_from(res: Response) -> Result<Self> {
        if let Some(choice) = res.choices.first() {
            let response = match choice {
                Choice::NonChat { text, .. } => crate::model::Response::new(text.clone()),
                Choice::NonStreaming { message, .. } => {
                    let mut resp =
                        crate::model::Response::new(message.content.clone().unwrap_or_default());
                    if let Some(tool_calls) = &message.tool_calls {
                        for tool_call in tool_calls {
                            resp = resp.add_call(ToolUse {
                                tool_use_id: tool_call.id.clone().map(UseId),
                                tool_id: ToolId::new(&tool_call.function.name),
                                input: serde_json::from_str(&tool_call.function.arguments)?,
                            });
                        }
                    }
                    resp
                }
                Choice::Streaming { delta, .. } => {
                    let mut resp =
                        crate::model::Response::new(delta.content.clone().unwrap_or_default());
                    if let Some(tool_calls) = &delta.tool_calls {
                        for tool_call in tool_calls {
                            resp = resp.add_call(ToolUse {
                                tool_use_id: tool_call.id.clone().map(UseId),
                                tool_id: ToolId::new(&tool_call.function.name),
                                input: serde_json::from_str(&tool_call.function.arguments)?,
                            });
                        }
                    }
                    resp
                }
            };
            Ok(response)
        } else {
            Err(crate::error::Error::empty_response("Ollama"))
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ProviderPreferences {
    // Define fields as necessary
}

#[derive(Clone)]
struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    fn new(model: Option<String>, base_url: Option<String>) -> Self {
        let client = Client::builder().build().unwrap();

        Self {
            client,
            base_url: base_url.unwrap_or("https://localhost:1134".to_string()),
            model: model.unwrap_or(DEFAULT_MODEL.to_string()),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

#[async_trait::async_trait]
impl InnerProvider for OllamaProvider {
    type Request = crate::model::Request;
    type Response = crate::model::Response;
    type Error = crate::error::Error;

    async fn chat(
        &self,
        request: Self::Request,
    ) -> ResultStream<Self::Response, Self::Error> {
        let mut new_request = Request::from(request);

        new_request.model = self.model.clone();
        new_request.stream = Some(true); // Ensure streaming is enabled

        let body = serde_json::to_string(&new_request)?;

        let response_stream = self
            .client
            .post(self.url("/chat/completions"))
            .body(body)
            .send()
            .await?
            .bytes_stream();

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        tokio::spawn(async move {
            let mut response_stream = response_stream;
            while let Some(chunk_result) = response_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        if let Ok(response) = serde_json::from_slice::<Response>(&chunk) {
                            if let Ok(model_response) = crate::model::Response::try_from(response) {
                                if tx.send(Ok(model_response)).await.is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(Err(crate::error::Error::from(err))).await;
                        break;
                    }
                }
            }
        });

        let processed_stream = ReceiverStream::new(rx);

        Ok(Box::pin(Box::new(processed_stream)))
    }

    async fn models(&self) -> Result<Vec<String>> {
        let text = self
            .client
            .get(self.url("/models"))
            .send()
            .await?
            .text()
            .await?;

        let response: ListModelResponse = serde_json::from_str(&text)?;

        Ok(response
            .data
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<String>>())
    }
}

impl Provider {
    pub fn ollama(model: Option<String>, base_url: Option<String>) -> Self {
        Provider::new(OllamaProvider::new(model, base_url))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn models() -> &'static str {
        include_str!("./models.json")
    }

    #[test]
    fn test_de_ser_of_models() {
        let _: ListModelResponse = serde_json::from_str(models()).unwrap();
    }

    #[test]
    fn test_de_ser_of_response() {
        let response = r#"{
            "id": "ollama-12345",
            "provider": "Ollama",
            "model": "ollama/gpt-4-stream",
            "object": "chat.completion",
            "created": 1700000000,
            "choices": [{
                "delta": {
                    "content": "Hello! How can I assist you today?"
                },
                "finish_reason": "end_turn",
                "index": 0,
                "error": null
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }"#;

        let _: Response = serde_json::from_str(response).unwrap();
    }

    #[tokio::test]
    async fn test_chat() {
        let provider = Provider::new(OllamaProvider::new(None, None));

        let result_stream = provider
            .chat(
                crate::model::Request::default()
                    .add_message(crate::model::Message::user("Hello Forge!")),
            )
            .await
            .unwrap();

        let mut stream = result_stream;

        println!("Streaming Ollama response:");

        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    println!("{:#?}", response);
                }
                Err(err) => {
                    eprintln!("Error: {:#?}", err);
                }
            }
        }
    }
}
