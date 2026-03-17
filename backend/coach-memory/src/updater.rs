use chrono::Utc;
use domain::{RunningCoachMemory, RunningCoachMessage, RunningCoachSettings};
use llm::LlmClient;

use crate::{MemoryClassifier, MemoryNormalizer};

pub async fn update_memory_after_exchange(
    llm_client: &(impl LlmClient + ?Sized),
    settings: &RunningCoachSettings,
    user_message: &RunningCoachMessage,
    assistant_message: &RunningCoachMessage,
    memory: &mut RunningCoachMemory,
) {
    let classifier = MemoryClassifier::new();
    let normalizer = MemoryNormalizer::new();
    let exchange = format!(
        "User:\n{}\n\nAssistant:\n{}",
        user_message.content, assistant_message.content
    );

    let classification = classifier
        .classify(llm_client, &settings.model, &memory.data, &exchange)
        .await;

    match classification {
        Ok(output) => {
            log::info!(
                "coach.memory.classification_done meaningful={} extracted_pinned={} extracted_episodic={} has_plan={} has_summary={}",
                output.meaningful,
                output.pinned_facts.len(),
                output.episodic_memory.len(),
                output
                    .active_coaching_plan
                    .as_deref()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false),
                output
                    .rolling_summary
                    .as_deref()
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            );
            if output.meaningful {
                classifier.apply_output(&mut memory.data, output);
                memory.message_count_since_normalization += 1;
                log::info!(
                    "coach.memory.classification_applied mem_norm_count={} mem_pinned={} mem_episodic={}",
                    memory.message_count_since_normalization,
                    memory.data.pinned_facts.len(),
                    memory.data.episodic_memory.len()
                );
            }
        }
        Err(err) => {
            log::warn!("Running coach memory classification failed: {}", err);
        }
    }

    let threshold = clamp(settings.normalizer_every_n_messages, 1, 20, 6);
    if memory.message_count_since_normalization >= threshold {
        log::info!(
            "coach.memory.normalization_start mem_norm_count={} threshold={}",
            memory.message_count_since_normalization,
            threshold
        );
        match normalizer
            .normalize(llm_client, &settings.model, &memory.data)
            .await
        {
            Ok(normalized) => {
                let pinned_len = normalized.pinned_facts.len();
                let episodic_len = normalized.episodic_memory.len();
                memory.data = normalized;
                memory.message_count_since_normalization = 0;
                log::info!(
                    "coach.memory.normalization_done mem_pinned={} mem_episodic={} mem_norm_count={}",
                    pinned_len,
                    episodic_len,
                    memory.message_count_since_normalization
                );
            }
            Err(err) => {
                log::warn!("Running coach memory normalization failed: {}", err);
            }
        }
    }

    memory.updated_at = Utc::now();
}

fn clamp(value: i32, min: i32, max: i32, fallback: i32) -> i32 {
    let val = if value <= 0 { fallback } else { value };
    val.clamp(min, max)
}
