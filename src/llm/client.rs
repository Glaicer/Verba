use std::time::Duration;

use reqwest::{Client, StatusCode};
use url::Url;

use crate::{
    config::{Preset, ProviderConfig},
    llm::{
        errors::{LlmError, LlmErrorKind},
        prompt::build_system_prompt,
        schema::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage},
    },
};

#[derive(Clone, Debug)]
pub struct LlmClient {
    client: Client,
    provider: ProviderConfig,
    endpoint: Url,
}

impl LlmClient {
    pub fn new(provider: ProviderConfig) -> Result<Self, LlmError> {
        if provider.model_name.trim().is_empty() {
            return Err(LlmError::new(
                LlmErrorKind::MissingConfiguration,
                "model name is not configured",
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(provider.timeout_secs))
            .build()
            .map_err(|err| LlmError::new(LlmErrorKind::Network, err.to_string()))?;

        let endpoint = build_endpoint(&provider.base_url)?;

        Ok(Self {
            client,
            provider,
            endpoint,
        })
    }

    pub async fn translate(
        &self,
        api_key: &str,
        language: &str,
        preset: &Preset,
        input: &str,
    ) -> Result<String, LlmError> {
        if api_key.trim().is_empty() {
            return Err(LlmError::new(
                LlmErrorKind::MissingConfiguration,
                "API key is not configured",
            ));
        }

        let request = ChatCompletionRequest {
            model: self.provider.model_name.clone(),
            temperature: self.provider.temperature,
            max_tokens: self.provider.max_tokens,
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: build_system_prompt(language, preset),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: input.to_string(),
                },
            ],
        };

        let response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(api_key)
            .json(&request)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| LlmError::new(LlmErrorKind::Network, err.to_string()))?;

        if !status.is_success() {
            return Err(LlmError::from_status(status.as_u16(), &body));
        }

        parse_completion(&body)
    }
}

fn build_endpoint(base_url: &str) -> Result<Url, LlmError> {
    let mut base = Url::parse(base_url)
        .map_err(|err| LlmError::new(LlmErrorKind::MissingConfiguration, err.to_string()))?;
    let path = base.path().trim_end_matches('/').to_string();
    base.set_path(&format!("{path}/v1/chat/completions"));
    Ok(base)
}

fn parse_completion(body: &str) -> Result<String, LlmError> {
    let parsed: ChatCompletionResponse = serde_json::from_str(body)
        .map_err(|err| LlmError::new(LlmErrorKind::InvalidResponse, err.to_string()))?;
    let content = parsed
        .choices
        .first()
        .and_then(|choice| choice.message.content.as_deref())
        .ok_or_else(|| {
            LlmError::new(
                LlmErrorKind::InvalidResponse,
                "response did not include choices[0].message.content",
            )
        })?;

    if content.is_empty() {
        return Err(LlmError::new(
            LlmErrorKind::InvalidResponse,
            "response message content was empty",
        ));
    }

    Ok(content.to_string())
}

fn map_reqwest_error(err: reqwest::Error) -> LlmError {
    if err.is_timeout() {
        return LlmError::new(LlmErrorKind::Timeout, "request timed out");
    }

    if let Some(status) = err.status() {
        return LlmError::from_status(status.as_u16(), "");
    }

    LlmError::new(LlmErrorKind::Network, err.to_string())
}

#[allow(dead_code)]
fn map_status(status: StatusCode, body: &str) -> LlmError {
    LlmError::from_status(status.as_u16(), body)
}
