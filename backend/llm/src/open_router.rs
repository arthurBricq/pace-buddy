use crate::{
    ChatCompletionRequest, ChatCompletionResponse, ChatCompletionResult, ChatMessage,
    JsonSchemaDefinition, LlmClient, LlmError, LlmUsage, ModelInfo, ReasoningConfig,
    ResponseFormat,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

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

    async fn send_chat_completion_request(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResult, LlmError> {
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://pacebuddy.app")
            .header("X-Title", "Pace Buddy")
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
        log::debug!("OpenRouter completion payload: {:?}", completion);

        let usage = completion
            .usage
            .map(|u| LlmUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                cost: u.cost.unwrap_or(0.0),
            })
            .unwrap_or_default();

        let content = completion
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or(LlmError::NoContent)?;

        if content.trim().is_empty() {
            return Err(LlmError::NoContent);
        }

        Ok(ChatCompletionResult { content, usage })
    }
}

/// Curated list of allowed models
const ALLOWED_MODELS: &[&str] = &[
    "x-ai/grok-code-fast-1",
    "deepseek/deepseek-v3.2",
    "google/gemini-2.5-flash",
    "google/gemini-3-flash-preview",
    "google/gemini-2.5-flash-lite",
    "google/gemini-3-pro-preview",
    "google/gemini-3.1-pro-preview",
    "anthropic/claude-sonnet-4.5",
    "anthropic/claude-opus-4.5",
    "anthropic/claude-haiku-4.5",
    "anthropic/claude-sonnet-4",
    "openai/gpt-5.2",
    "openai/gpt-5-mini",
    "openai/gpt-5.3-chat",
    "openai/gpt-5.4",
];

fn fallback_allowed_models() -> Vec<ModelInfo> {
    ALLOWED_MODELS
        .iter()
        .map(|id| ModelInfo {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            pricing: None,
            context_length: None,
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    #[serde(default)]
    data: Vec<ModelInfo>,
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        let allowed_set = ALLOWED_MODELS.iter().copied().collect::<HashSet<_>>();

        let response = match self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                log::warn!(
                    "OpenRouter model list request failed, using fallback model list: {}",
                    err
                );
                return Ok(fallback_allowed_models());
            }
        };

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::warn!(
                "OpenRouter model list returned non-success status {}, using fallback list. Body: {}",
                status,
                body
            );
            return Ok(fallback_allowed_models());
        }

        let parsed: ModelsResponse = match response.json().await {
            Ok(models) => models,
            Err(err) => {
                log::warn!(
                    "OpenRouter model list parsing failed, using fallback model list: {}",
                    err
                );
                return Ok(fallback_allowed_models());
            }
        };

        let mut by_id = parsed
            .data
            .into_iter()
            .filter(|m| allowed_set.contains(m.id.as_str()))
            .map(|m| (m.id.clone(), m))
            .collect::<HashMap<_, _>>();

        let ordered_models = ALLOWED_MODELS
            .iter()
            .map(|id| {
                by_id.remove(*id).unwrap_or(ModelInfo {
                    id: id.to_string(),
                    name: id.to_string(),
                    description: None,
                    pricing: None,
                    context_length: None,
                })
            })
            .collect::<Vec<_>>();

        Ok(ordered_models)
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<ChatCompletionResult, LlmError> {
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
            response_format: None,
        };

        self.send_chat_completion_request(request).await
    }

    async fn chat_completion_with_schema(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        json_schema: JsonSchemaDefinition,
        reasoning_effort: Option<&str>,
    ) -> Result<ChatCompletionResult, LlmError> {
        let reasoning = reasoning_effort.map(|effort| ReasoningConfig {
            effort: Some(effort.to_string()),
            max_tokens: None,
            exclude: Some(false),
        });

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens: None,
            temperature: Some(0.2),
            reasoning,
            response_format: Some(ResponseFormat::from_json_schema(json_schema)),
        };

        self.send_chat_completion_request(request).await
    }
}
