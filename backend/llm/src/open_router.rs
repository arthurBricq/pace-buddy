use async_trait::async_trait;
use crate::{
    ChatCompletionRequest, ChatCompletionResponse, ChatMessage, LlmClient, LlmError, ModelInfo,
    ReasoningConfig,
};
use reqwest::Client;

/// OpenRouter API client
#[derive(Clone)]
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenRouterClient {
    /// Creates a new OpenRouter client
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_string(),
        }
    }
}

/// Curated list of allowed models
const ALLOWED_MODELS: &[&str] = &[
    "anthropic/claude-sonnet-4.5",
    "x-ai/grok-code-fast-1",
    "google/gemini-2.5-flash",
    "google/gemini-3-flash-preview",
    "deepseek/deepseek-v3.2",
    "anthropic/claude-opus-4.5",
    "google/gemini-2.5-flash-lite",
    "google/gemini-3-pro-preview",
    "openai/gpt-4o-mini",
    "anthropic/claude-haiku-4.5",
    "openai/gpt-5.2",
    "anthropic/claude-sonnet-4",
    "openai/gpt-5-mini",
];

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        Ok(ALLOWED_MODELS
            .iter()
            .map(|id| ModelInfo {
                id: id.to_string(),
                name: id.to_string(),
                description: None,
                pricing: None,
                context_length: None,
            })
            .collect())
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<String, LlmError> {
        // API spec: https://openrouter.ai/docs/api/api-reference/chat/send-chat-completion-request
        // Reasoning tokens: https://openrouter.ai/docs/guides/best-practices/reasoning-tokens
        let reasoning = reasoning_effort.map(|effort| ReasoningConfig {
            effort: Some(effort.to_string()),
            max_tokens: None,
            exclude: Some(false),
        });

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            // TODO: this limits the output size of the thinking models
            max_tokens: None,
            temperature: Some(0.7),
            reasoning,
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://running-tool.app")
            .header("X-Title", "Running Tool")
            .json(&request)
            .send()
            .await?;

        log::debug!("OpenRouter response: {:?}", response);

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(LlmError::RateLimited);
        }

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::Api(error_text));
        }

        let completion: ChatCompletionResponse = response.json().await?;

        log::debug!("OpenRouter response: {:?}", completion);

        let content = completion
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(LlmError::NoContent)?;

        // Treat empty responses as NoContent
        if content.trim().is_empty() {
            return Err(LlmError::NoContent);
        }

        Ok(content)
    }
}
