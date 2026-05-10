use domain::{DomainError, RunningCoachMemoryData};
use llm::{ChatMessage, JsonSchemaDefinition, LlmClient};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::classifier::parse_json_response;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNormalizerOutput {
    pub pinned_facts: Vec<String>,
    pub active_coaching_plan: String,
    pub episodic_memory: Vec<String>,
    pub rolling_summary: String,
}

pub struct MemoryNormalizer;

impl Default for MemoryNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryNormalizer {
    pub fn new() -> Self {
        Self
    }

    pub async fn normalize(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        model: &str,
        memory: &RunningCoachMemoryData,
    ) -> Result<RunningCoachMemoryData, DomainError> {
        let schema = JsonSchemaDefinition {
            name: "memory_normalizer".to_string(),
            strict: true,
            schema: normalizer_schema(),
        };

        let messages = vec![
            ChatMessage::system(NORMALIZER_SYSTEM_PROMPT),
            ChatMessage::user(format!(
                "Normalize this memory snapshot while keeping only long-term useful information:\n{}",
                serde_json::to_string_pretty(memory).unwrap_or_else(|_| "{}".to_string())
            )),
        ];

        let result = llm_client
            .chat_completion_with_schema(model, messages, schema, None)
            .await
            .map_err(|e| DomainError::Internal(format!("Memory normalizer call failed: {e}")))?;

        parse_normalizer_output(&result.content, &memory.recent_tool_results)
    }
}

const NORMALIZER_SYSTEM_PROMPT: &str = "You are a memory normalizer for a running coach system.
Return JSON only.
Deduplicate, shorten, and remove stale information.
Keep memory compact and specific.";

fn normalizer_schema() -> serde_json::Value {
    json!({
      "type": "object",
      "additionalProperties": false,
      "required": [
        "pinned_facts",
        "active_coaching_plan",
        "episodic_memory",
        "rolling_summary"
      ],
      "properties": {
        "pinned_facts": {
          "type": "array",
          "items": { "type": "string" },
          "maxItems": 12
        },
        "active_coaching_plan": { "type": "string" },
        "episodic_memory": {
          "type": "array",
          "items": { "type": "string" },
          "maxItems": 20
        },
        "rolling_summary": { "type": "string" }
      }
    })
}

pub(crate) fn parse_normalizer_output(
    raw: &str,
    recent_tool_results: &[String],
) -> Result<RunningCoachMemoryData, DomainError> {
    let normalized = parse_json_response::<MemoryNormalizerOutput>(raw, "memory normalizer")?;
    Ok(RunningCoachMemoryData {
        pinned_facts: normalized.pinned_facts,
        active_coaching_plan: normalized.active_coaching_plan,
        episodic_memory: normalized.episodic_memory,
        rolling_summary: normalized.rolling_summary,
        recent_tool_results: recent_tool_results.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_normalizer_output;

    #[test]
    fn parse_normalizer_output_accepts_json_fence() {
        let raw = r#"```json
{
  "pinned_facts": ["Race in 6 weeks"],
  "active_coaching_plan": "Build threshold with one long run weekly",
  "episodic_memory": ["Skipped Tuesday due to calf soreness"],
  "rolling_summary": "Good consistency this week."
}
```"#;

        let parsed = parse_normalizer_output(raw, &["tool summary".to_string()])
            .expect("normalizer output should parse");
        assert_eq!(parsed.pinned_facts, vec!["Race in 6 weeks".to_string()]);
        assert_eq!(
            parsed.active_coaching_plan,
            "Build threshold with one long run weekly"
        );
        assert_eq!(
            parsed.episodic_memory,
            vec!["Skipped Tuesday due to calf soreness".to_string()]
        );
        assert_eq!(parsed.rolling_summary, "Good consistency this week.");
        assert_eq!(parsed.recent_tool_results, vec!["tool summary".to_string()]);
    }
}
