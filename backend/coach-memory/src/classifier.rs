use domain::{DomainError, RunningCoachMemoryData};
use llm::{ChatMessage, JsonSchemaDefinition, LlmClient};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryClassifierOutput {
    pub meaningful: bool,
    pub pinned_facts: Vec<String>,
    pub active_coaching_plan: Option<String>,
    pub episodic_memory: Vec<String>,
    pub rolling_summary: Option<String>,
}

pub struct MemoryClassifier;

impl Default for MemoryClassifier {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryClassifier {
    pub fn new() -> Self {
        Self
    }

    pub async fn classify(
        &self,
        llm_client: &(impl LlmClient + ?Sized),
        model: &str,
        memory: &RunningCoachMemoryData,
        latest_exchange: &str,
    ) -> Result<MemoryClassifierOutput, DomainError> {
        let schema = JsonSchemaDefinition {
            name: "memory_classifier".to_string(),
            strict: true,
            schema: classifier_schema(),
        };

        let messages = vec![
            ChatMessage::system(CLASSIFIER_SYSTEM_PROMPT),
            ChatMessage::user(format!(
                "Current memory snapshot:\n{}\n\nLatest exchange:\n{}",
                serde_json::to_string_pretty(memory).unwrap_or_else(|_| "{}".to_string()),
                latest_exchange
            )),
        ];

        let result = llm_client
            .chat_completion_with_schema(model, messages, schema, None)
            .await
            .map_err(|e| DomainError::Internal(format!("Memory classifier call failed: {e}")))?;

        parse_json_response::<MemoryClassifierOutput>(&result.content, "memory classifier")
    }

    pub fn apply_output(
        &self,
        current: &mut RunningCoachMemoryData,
        output: MemoryClassifierOutput,
    ) {
        if !output.meaningful {
            return;
        }

        merge_unique_lines(&mut current.pinned_facts, output.pinned_facts, 12);
        merge_unique_lines(&mut current.episodic_memory, output.episodic_memory, 20);

        if let Some(plan) = output.active_coaching_plan {
            let plan = plan.trim();
            if !plan.is_empty() {
                current.active_coaching_plan = plan.to_string();
            }
        }

        if let Some(summary) = output.rolling_summary {
            let summary = summary.trim();
            if !summary.is_empty() {
                current.rolling_summary = summary.to_string();
            }
        }
    }
}

const CLASSIFIER_SYSTEM_PROMPT: &str = "You are a memory extractor for a running coach system.
Return JSON only.
Decide whether the latest exchange is meaningful for future coaching.
Keep only durable constraints, decisions, and insights.";

fn classifier_schema() -> serde_json::Value {
    json!({
      "type": "object",
      "additionalProperties": false,
      "required": [
        "meaningful",
        "pinned_facts",
        "active_coaching_plan",
        "episodic_memory",
        "rolling_summary"
      ],
      "properties": {
        "meaningful": { "type": "boolean" },
        "pinned_facts": {
          "type": "array",
          "items": { "type": "string" },
          "maxItems": 12
        },
        "active_coaching_plan": {
          "anyOf": [{ "type": "string" }, { "type": "null" }]
        },
        "episodic_memory": {
          "type": "array",
          "items": { "type": "string" },
          "maxItems": 20
        },
        "rolling_summary": {
          "anyOf": [{ "type": "string" }, { "type": "null" }]
        }
      }
    })
}

pub(crate) fn parse_json_response<T: serde::de::DeserializeOwned>(
    raw: &str,
    label: &str,
) -> Result<T, DomainError> {
    let raw = raw.trim();
    let cleaned = if raw.starts_with("```") {
        raw.trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
            .to_string()
    } else {
        raw.to_string()
    };

    serde_json::from_str::<T>(&cleaned)
        .map_err(|e| DomainError::Internal(format!("Failed to parse {label} output: {e}")))
}

fn merge_unique_lines(target: &mut Vec<String>, mut additions: Vec<String>, cap: usize) {
    additions.retain(|line| !line.trim().is_empty());
    for line in additions {
        let normalized = normalize_line(&line);
        if target
            .iter()
            .any(|existing| normalize_line(existing) == normalized)
        {
            continue;
        }
        target.push(line.trim().to_string());
    }
    if target.len() > cap {
        let start = target.len() - cap;
        *target = target[start..].to_vec();
    }
}

fn normalize_line(s: &str) -> String {
    s.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{MemoryClassifier, MemoryClassifierOutput};
    use domain::RunningCoachMemoryData;

    #[test]
    fn apply_output_dedupes_and_caps_memory() {
        let classifier = MemoryClassifier::new();
        let mut current = RunningCoachMemoryData {
            pinned_facts: (0..11).map(|i| format!("fact-{i}")).collect(),
            active_coaching_plan: "existing plan".to_string(),
            episodic_memory: (0..19).map(|i| format!("episode-{i}")).collect(),
            rolling_summary: "existing summary".to_string(),
            recent_tool_results: vec!["tool-1".to_string()],
        };

        classifier.apply_output(
            &mut current,
            MemoryClassifierOutput {
                meaningful: true,
                pinned_facts: vec![
                    "fact-3".to_string(),
                    "fact-11".to_string(),
                    "fact-12".to_string(),
                ],
                active_coaching_plan: Some("new plan".to_string()),
                episodic_memory: vec![
                    "episode-1".to_string(),
                    "episode-19".to_string(),
                    "episode-20".to_string(),
                    "episode-21".to_string(),
                ],
                rolling_summary: Some("new summary".to_string()),
            },
        );

        assert_eq!(current.pinned_facts.len(), 12);
        assert_eq!(current.pinned_facts.first().unwrap(), "fact-1");
        assert_eq!(current.pinned_facts.last().unwrap(), "fact-12");
        assert_eq!(current.episodic_memory.len(), 20);
        assert_eq!(current.episodic_memory.first().unwrap(), "episode-2");
        assert_eq!(current.episodic_memory.last().unwrap(), "episode-21");
        assert_eq!(current.active_coaching_plan, "new plan");
        assert_eq!(current.rolling_summary, "new summary");
        assert_eq!(current.recent_tool_results, vec!["tool-1".to_string()]);
    }
}
