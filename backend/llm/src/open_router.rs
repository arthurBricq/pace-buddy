use crate::{
    parse_message_content, parse_tool_call, ChatCompletionRequest, ChatCompletionRequestTool,
    ChatCompletionResponse, ChatCompletionResult, ChatMessage, JsonSchemaDefinition, LlmClient,
    LlmError, LlmUsage, ModelInfo, ReasoningConfig, ResponseFormat, ToolChoice,
    ToolCompletionResult, ToolDefinition,
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
    ) -> Result<ToolCompletionResult, LlmError> {
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

        parse_completion_payload(completion)
    }
}

fn parse_completion_payload(
    completion: ChatCompletionResponse,
) -> Result<ToolCompletionResult, LlmError> {
    let usage = completion
        .usage
        .map(|u| LlmUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
            cost: u.cost.unwrap_or(0.0),
        })
        .unwrap_or_default();

    let choice = completion
        .choices
        .into_iter()
        .next()
        .ok_or(LlmError::NoContent)?;

    let tool_calls = choice
        .message
        .tool_calls
        .unwrap_or_default()
        .into_iter()
        .map(parse_tool_call)
        .collect();

    let content = parse_message_content(choice.message.content);

    Ok(ToolCompletionResult {
        content,
        tool_calls,
        finish_reason: choice.finish_reason,
        usage,
    })
}

/// Curated list of allowed models
const ALLOWED_MODELS: &[&str] = &[
    "x-ai/grok-code-fast-1",
    "deepseek/deepseek-v3.2",
    "google/gemini-2.5-flash",
    "google/gemini-3.1-pro-preview",
    "anthropic/claude-sonnet-4.5",
    "anthropic/claude-opus-4.5",
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
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
        };

        let tool_result = self.send_chat_completion_request(request).await?;
        let content = tool_result.content.ok_or(LlmError::NoContent)?;
        if content.trim().is_empty() {
            return Err(LlmError::NoContent);
        }

        Ok(ChatCompletionResult {
            content,
            usage: tool_result.usage,
        })
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
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
        };

        let tool_result = self.send_chat_completion_request(request).await?;
        let content = tool_result.content.ok_or(LlmError::NoContent)?;
        if content.trim().is_empty() {
            return Err(LlmError::NoContent);
        }

        Ok(ChatCompletionResult {
            content,
            usage: tool_result.usage,
        })
    }

    async fn chat_completion_with_tools(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        tool_choice: Option<ToolChoice>,
        parallel_tool_calls: Option<bool>,
        reasoning_effort: Option<&str>,
    ) -> Result<ToolCompletionResult, LlmError> {
        let reasoning = reasoning_effort.map(|effort| ReasoningConfig {
            effort: Some(effort.to_string()),
            max_tokens: None,
            exclude: Some(false),
        });

        let request_tools = if tools.is_empty() {
            None
        } else {
            Some(tools.iter().map(ChatCompletionRequestTool::from).collect())
        };

        let request = ChatCompletionRequest {
            model: model.to_string(),
            messages,
            max_tokens: None,
            temperature: Some(0.2),
            reasoning,
            response_format: None,
            tools: request_tools,
            tool_choice: tool_choice.map(|choice| choice.to_request_value()),
            parallel_tool_calls,
        };

        self.send_chat_completion_request(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::parse_completion_payload;
    use crate::{
        ChatCompletionRequest, ChatCompletionRequestTool, ChatCompletionResponse, ChatMessage,
        ToolChoice, ToolDefinition,
    };
    use serde_json::json;

    #[test]
    fn parses_normal_text_response() {
        let payload = r#"{
          "choices": [{
            "message": { "content": "Hello runner" },
            "finish_reason": "stop"
          }],
          "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15, "cost": 0.01 }
        }"#;

        let parsed: ChatCompletionResponse = serde_json::from_str(payload).expect("valid payload");
        let result = parse_completion_payload(parsed).expect("parsed");

        assert_eq!(result.content.as_deref(), Some("Hello runner"));
        assert!(result.tool_calls.is_empty());
        assert_eq!(result.finish_reason.as_deref(), Some("stop"));
        assert_eq!(result.usage.total_tokens, 15);
    }

    #[test]
    fn parses_tool_call_response_with_empty_content() {
        let payload = r#"{
          "choices": [{
            "message": {
              "content": "",
              "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                  "name": "search_sessions",
                  "arguments": "{\"query\":\"last race\"}"
                }
              }]
            },
            "finish_reason": "tool_calls"
          }],
          "usage": { "prompt_tokens": 20, "completion_tokens": 8, "total_tokens": 28, "cost": 0.03 }
        }"#;

        let parsed: ChatCompletionResponse = serde_json::from_str(payload).expect("valid payload");
        let result = parse_completion_payload(parsed).expect("parsed");

        assert!(result.content.is_none());
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "search_sessions");
        assert!(result.tool_calls[0].arguments_parse_error.is_none());
        assert_eq!(result.finish_reason.as_deref(), Some("tool_calls"));
    }

    #[test]
    fn keeps_invalid_tool_arguments_without_failing() {
        let payload = r#"{
          "choices": [{
            "message": {
              "tool_calls": [{
                "id": "call_1",
                "type": "function",
                "function": {
                  "name": "get_session_detail",
                  "arguments": "{invalid json}"
                }
              }]
            },
            "finish_reason": "tool_calls"
          }],
          "usage": { "prompt_tokens": 2, "completion_tokens": 1, "total_tokens": 3, "cost": 0.0 }
        }"#;

        let parsed: ChatCompletionResponse = serde_json::from_str(payload).expect("valid payload");
        let result = parse_completion_payload(parsed).expect("parsed");

        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].name, "get_session_detail");
        assert!(result.tool_calls[0].arguments_parse_error.is_some());
        assert_eq!(result.tool_calls[0].arguments, serde_json::Value::Null);
    }

    #[test]
    fn serializes_tool_call_request_fields() {
        let tool = ToolDefinition {
            name: "search_sessions".to_string(),
            description: "Search activities".to_string(),
            parameters: json!({
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            }),
        };

        let request = ChatCompletionRequest {
            model: "openai/gpt-5.4".to_string(),
            messages: vec![ChatMessage::user("find my last interval")],
            max_tokens: None,
            temperature: Some(0.2),
            reasoning: None,
            response_format: None,
            tools: Some(vec![ChatCompletionRequestTool::from(&tool)]),
            tool_choice: Some(ToolChoice::Auto.to_request_value()),
            parallel_tool_calls: Some(false),
        };

        let value = serde_json::to_value(request).expect("request json");
        assert_eq!(value["model"], "openai/gpt-5.4");
        assert_eq!(value["tool_choice"], "auto");
        assert_eq!(value["parallel_tool_calls"], false);
        assert_eq!(value["tools"][0]["type"], "function");
        assert_eq!(value["tools"][0]["function"]["name"], "search_sessions");
        assert_eq!(
            value["tools"][0]["function"]["description"],
            "Search activities"
        );
        assert_eq!(
            value["tools"][0]["function"]["parameters"]["type"],
            "object"
        );
        assert_eq!(value["messages"][0]["role"], "user");
    }
}
