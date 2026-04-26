use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ChatChoice {
    pub message: ChatResponseMessage,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct ChatResponseMessage {
    pub content: Option<String>,
}
