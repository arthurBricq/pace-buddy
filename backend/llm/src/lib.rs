//! LLM client abstraction for making API calls to language models.
//!
//! This module provides a trait-based abstraction for LLM clients,
//! with a concrete implementation for OpenRouter API.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
}

#[derive(Debug, Clone)]
pub struct ChatCompletionResult {
    pub content: String,
    pub usage: LlmUsage,
}

#[derive(Debug, Clone)]
pub struct ToolCompletionResult {
    pub content: Option<String>,
    pub tool_calls: Vec<ToolCall>,
    pub finish_reason: Option<String>,
    pub usage: LlmUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    Function { name: String },
}

impl ToolChoice {
    pub(crate) fn to_request_value(&self) -> Value {
        match self {
            Self::Auto => Value::String("auto".to_string()),
            Self::None => Value::String("none".to_string()),
            Self::Required => Value::String("required".to_string()),
            Self::Function { name } => serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                }
            }),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
    pub arguments_raw: String,
    pub arguments_parse_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ToolMessage {
    AssistantToolCalls(Vec<ToolCall>),
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

/// OpenRouter clients for LLM
pub mod open_router;

#[derive(Debug, Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("No response content")]
    NoContent,
    #[error("Rate limited")]
    RateLimited,
}

/// Trait for LLM clients.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Fetches the list of available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError>;

    /// Sends a chat completion request and returns the response with usage info.
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<ChatCompletionResult, LlmError>;

    /// Sends a chat completion request with strict JSON-schema output.
    async fn chat_completion_with_schema(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        json_schema: JsonSchemaDefinition,
        reasoning_effort: Option<&str>,
    ) -> Result<ChatCompletionResult, LlmError>;

    /// Sends a chat completion request with tool calling support.
    async fn chat_completion_with_tools(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        tools: Vec<ToolDefinition>,
        tool_choice: Option<ToolChoice>,
        parallel_tool_calls: Option<bool>,
        reasoning_effort: Option<&str>,
    ) -> Result<ToolCompletionResult, LlmError>;
}

/// Chat message for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ChatToolCall>>,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[CHAT] {}: {}", self.role, self.content)
    }
}

impl ChatMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::new("system", content)
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self::new("user", content)
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::new("assistant", content)
    }

    pub fn assistant_tool_calls(tool_calls: &[ToolCall]) -> Self {
        let calls = tool_calls
            .iter()
            .map(|call| ChatToolCall {
                id: call.id.clone(),
                call_type: "function".to_string(),
                function: ChatToolFunction {
                    name: call.name.clone(),
                    arguments: if call.arguments_raw.trim().is_empty() {
                        call.arguments.to_string()
                    } else {
                        call.arguments_raw.clone()
                    },
                },
            })
            .collect();

        Self {
            role: "assistant".to_string(),
            content: String::new(),
            tool_call_id: None,
            tool_calls: Some(calls),
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }

    pub fn from_tool_message(message: ToolMessage) -> Self {
        match message {
            ToolMessage::AssistantToolCalls(calls) => Self::assistant_tool_calls(&calls),
            ToolMessage::ToolResult {
                tool_call_id,
                content,
            } => Self::tool(tool_call_id, content),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ChatToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatToolFunction {
    pub name: String,
    pub arguments: String,
}

/// Reasoning configuration for models that support extended thinking
#[derive(Debug, Serialize)]
pub struct ReasoningConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ReasoningConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ChatCompletionRequestTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequestTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: ChatCompletionRequestToolFunction,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequestToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl From<&ToolDefinition> for ChatCompletionRequestTool {
    fn from(value: &ToolDefinition) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: ChatCompletionRequestToolFunction {
                name: value.name.clone(),
                description: value.description.clone(),
                parameters: value.parameters.clone(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonSchemaDefinition {
    pub name: String,
    pub schema: Value,
    pub strict: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseFormat {
    #[serde(rename = "type")]
    format_type: String,
    json_schema: ResponseFormatJsonSchema,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseFormatJsonSchema {
    name: String,
    strict: bool,
    schema: Value,
}

impl ResponseFormat {
    pub(crate) fn from_json_schema(def: JsonSchemaDefinition) -> Self {
        Self {
            format_type: "json_schema".to_string(),
            json_schema: ResponseFormatJsonSchema {
                name: def.name,
                strict: def.strict,
                schema: def.schema,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawUsage {
    #[serde(default)]
    pub prompt_tokens: u32,
    #[serde(default)]
    pub completion_tokens: u32,
    #[serde(default)]
    pub total_tokens: u32,
    #[serde(default)]
    pub cost: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    #[serde(default)]
    pub choices: Vec<ChatChoice>,
    pub usage: Option<RawUsage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatChoice {
    pub message: ChatChoiceMessage,
    #[serde(default)]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatChoiceMessage {
    pub content: Option<Value>,
    #[serde(default)]
    pub tool_calls: Option<Vec<RawToolCall>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawToolCall {
    pub id: String,
    #[serde(default, rename = "type")]
    pub _call_type: Option<String>,
    pub function: RawToolCallFunction,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawToolCallFunction {
    pub name: String,
    pub arguments: String,
}

pub(crate) fn parse_message_content(content: Option<Value>) -> Option<String> {
    let content = content?;
    match content {
        Value::String(text) => {
            let trimmed = text.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Value::Array(parts) => {
            let mut text_parts = Vec::new();
            for part in parts {
                if let Some(text) = part
                    .get("text")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                {
                    if !text.is_empty() {
                        text_parts.push(text);
                    }
                }
            }
            if text_parts.is_empty() {
                None
            } else {
                Some(text_parts.join("\n"))
            }
        }
        _ => None,
    }
}

pub(crate) fn parse_tool_call(raw: RawToolCall) -> ToolCall {
    match serde_json::from_str::<Value>(&raw.function.arguments) {
        Ok(arguments) => ToolCall {
            id: raw.id,
            name: raw.function.name,
            arguments,
            arguments_raw: raw.function.arguments,
            arguments_parse_error: None,
        },
        Err(err) => ToolCall {
            id: raw.id,
            name: raw.function.name,
            arguments: Value::Null,
            arguments_raw: raw.function.arguments,
            arguments_parse_error: Some(err.to_string()),
        },
    }
}

/// Information about an available model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub pricing: Option<Pricing>,
    #[serde(default)]
    pub context_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    pub prompt: String,
    pub completion: String,
}
