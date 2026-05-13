use std::fs;
use std::path::PathBuf;

use llm::open_router::OpenRouterClient;
use llm::{ChatMessage, LlmClient, ToolChoice, ToolDefinition};
use serde_json::json;

fn load_openrouter_key_from_repo() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let key_path = manifest_dir
        .parent()
        .expect("llm crate must be inside backend workspace")
        .join("openrouter_key");

    let key = fs::read_to_string(&key_path)
        .unwrap_or_else(|e| panic!("Failed to read OpenRouter key at {:?}: {e}", key_path));
    let trimmed = key.trim().to_string();
    assert!(
        !trimmed.is_empty(),
        "OpenRouter key file exists but is empty: {:?}",
        key_path
    );
    trimmed
}

#[tokio::test]
#[ignore = "live integration test: requires network and backend/openrouter_key"]
async fn openrouter_live_tool_call_contract() {
    let api_key = load_openrouter_key_from_repo();
    let client = OpenRouterClient::new(api_key);

    let tool_name = "integration_echo";
    let tool = ToolDefinition {
        name: tool_name.to_string(),
        description: "Echo probe payload for integration testing.".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "probe": {
                    "type": "string",
                    "enum": ["openrouter-live"]
                }
            },
            "required": ["probe"],
            "additionalProperties": false
        }),
    };

    let result = client
        .chat_completion_with_tools(
            "openai/gpt-5-mini",
            vec![
                ChatMessage::system(
                    "You are running an integration test. Call the provided tool exactly once.",
                ),
                ChatMessage::user("Call the integration tool now."),
            ],
            vec![tool],
            Some(ToolChoice::Function {
                name: tool_name.to_string(),
            }),
            Some(false),
            None,
        )
        .await
        .expect("OpenRouter live tool-call request should succeed");

    assert!(
        !result.tool_calls.is_empty(),
        "Expected at least one tool call in live response, got none. finish_reason={:?}, content={:?}",
        result.finish_reason,
        result.content
    );
    assert_eq!(
        result.tool_calls[0].name, tool_name,
        "Expected first tool call to target requested function"
    );
    assert!(
        result.tool_calls[0].arguments_parse_error.is_none(),
        "Expected tool arguments to be valid JSON, got parse error: {:?}",
        result.tool_calls[0].arguments_parse_error
    );
    assert_eq!(
        result.tool_calls[0].arguments["probe"], "openrouter-live",
        "Expected tool argument contract field 'probe=openrouter-live'"
    );
    assert_eq!(
        result.finish_reason.as_deref(),
        Some("tool_calls"),
        "Expected finish_reason=tool_calls for forced function-call request"
    );
}
