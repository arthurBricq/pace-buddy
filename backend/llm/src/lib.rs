//! LLM client abstraction for making chess moves.
//!
//! This module provides a trait-based abstraction for LLM clients,
//! with a concrete implementation for OpenRouter API.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

/// OpenRouter clients for LLM
pub mod open_router;

/// A scripted sequence client for debugging, plays predetermined moves with configurable delay.
pub mod sequence;

use open_router::OpenRouterClient;
use sequence::DualSequenceClient;

/// Enum wrapper for different LLM client implementations.
///
/// This allows switching between client types at runtime based on CLI flags.
pub enum LlmClientKind {
    /// Production client using OpenRouter API
    OpenRouter(OpenRouterClient),
    /// Debug client that plays a scripted sequence of moves
    Sequence(DualSequenceClient),
}

#[async_trait]
impl LlmClient for LlmClientKind {
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError> {
        match self {
            Self::OpenRouter(client) => client.list_models().await,
            Self::Sequence(client) => client.list_models().await,
        }
    }

    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<String, LlmError> {
        match self {
            Self::OpenRouter(client) => {
                client
                    .chat_completion(model, messages, reasoning_effort)
                    .await
            }
            Self::Sequence(client) => {
                client
                    .chat_completion(model, messages, reasoning_effort)
                    .await
            }
        }
    }
}

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

/// Trait for LLM clients that can be used for chess games.
///
/// This abstraction allows for different implementations:
/// - `OpenRouterClient` for production use with OpenRouter API
/// - Mock clients for testing
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Fetches the list of available models
    async fn list_models(&self) -> Result<Vec<ModelInfo>, LlmError>;

    /// Sends a chat completion request and returns the response text
    ///
    /// # Arguments
    /// * `model` - The model ID to use
    /// * `messages` - The conversation messages
    /// * `reasoning_effort` - Optional reasoning effort level ("low", "medium", "high")
    async fn chat_completion(
        &self,
        model: &str,
        messages: Vec<ChatMessage>,
        reasoning_effort: Option<&str>,
    ) -> Result<String, LlmError>;
}

/// Chat message for the API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[CHAT] {}: {}", self.role, self.content)
    }
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Reasoning configuration for models that support extended thinking
#[derive(Debug, Serialize)]
pub struct ReasoningConfig {
    /// Effort level: "low", "medium", or "high"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    /// Maximum tokens for reasoning (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether to exclude reasoning from response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    pub choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatChoice {
    pub message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatChoiceMessage {
    pub content: Option<String>,
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

/// Default rules message for chess games.
/// The $COLOR variable is substituted at runtime with "White" or "Black".
pub const DEFAULT_CHESS_RULES_MESSAGE: &str = r#"You are playing a game of chess as $COLOR.

RULES:
1. Respond with ONLY your move in UCI notation (e.g., "e2e4", "g1f3").
2. For castling: "e1g1" (white kingside), "e1c1" (white queenside), "e8g8" (black kingside), "e8c8" (black queenside).
3. For pawn promotion, append the piece letter: "e7e8q" for queen.
4. Do not include any explanation, just the move.
5. Invalid moves or wrong format = forfeit."#;
